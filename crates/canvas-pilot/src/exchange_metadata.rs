//! Optional, inert project snapshots carried by geometry files.
use crate::{interchange::Codec, project::Project};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::io::{Cursor, Read, Write};

pub const KEY: &str = "beffroi_midra_parameters";
const MAX_METADATA: usize = 1024 * 1024;
const PREFIX: &str = "beffroi-midra-parameters:";

pub fn supported(codec: Codec) -> bool {
    !matches!(codec, Codec::Dwg | Codec::Figma | Codec::Illustrator)
}

/// Exclude unexported layers, unused inputs, image bytes and device capture bindings.
pub fn snapshot(project: &Project, ids: &[usize]) -> Result<Project, String> {
    project.layout()?;
    let mut p = project.clone();
    p.layers.retain(|l| ids.contains(&l.id));
    p.input_proposals.retain(|p| ids.contains(&p.layer_id));
    p.editor.selected_layer_ids.retain(|id| ids.contains(id));
    p.folders
        .retain(|f| p.layers.iter().any(|l| &l.folder == f));
    p.references.clear();
    p.workspace.previews.clear();
    p.workspace
        .collapsed_folders
        .retain(|f| p.layers.iter().any(|l| &l.folder == f));
    p.inputs
        .retain(|s| p.layers.iter().any(|l| l.source_id == s.id));
    for (i, source) in p.inputs.iter_mut().enumerate() {
        for layer in &mut p.layers {
            if layer.source_id == source.id {
                layer.source_id = i + 1;
            }
        }
        source.id = i + 1;
    }
    p.layout()?;
    Ok(p)
}
fn encode(project: &Project, ids: &[usize]) -> Result<String, String> {
    let json = snapshot(project, ids)?.to_json()?;
    if json.len() > MAX_METADATA {
        return Err("Extra parameters exceed 1 MiB.".into());
    }
    Ok(STANDARD.encode(json))
}
fn decode(encoded: &str) -> Result<Project, String> {
    if encoded.len() > MAX_METADATA * 4 / 3 + 4 {
        return Err("Extra parameters exceed 1 MiB.".into());
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|e| format!("Invalid extra parameters: {e}"))?;
    let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    Project::from_json(text)
}

pub(crate) fn embed(
    bytes: Vec<u8>,
    codec: Codec,
    project: &Project,
    ids: &[usize],
) -> Result<Vec<u8>, String> {
    if !supported(codec) {
        return Err("Extra parameters are not supported by this format's current codec.".into());
    }
    let payload = encode(project, ids)?;
    if codec == Codec::Figcraft {
        return embed_figcraft(&bytes, &payload);
    }
    if codec == Codec::Pptx {
        return embed_pptx(&bytes, &payload);
    }
    let mut text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    match codec {
        Codec::Svg => {
            let at = text.rfind("</svg>").ok_or("Missing SVG root")?;
            text.insert_str(at, &format!("<metadata><parameters xmlns=\"urn:beffroi:midra:parameters:1\">{payload}</parameters></metadata>\n"));
        }
        Codec::Drawio => {
            // Global custom properties live on the root cell's UserObject.
            text = text.replacen(
                "<mxCell id=\"0\"/>",
                &format!("<object id=\"0\" {KEY}=\"{payload}\"><mxCell/></object>"),
                1,
            );
        }
        Codec::Html => {
            let at = text.find("</head>").ok_or("Missing HTML head")?;
            text.insert_str(
                at,
                &format!("<meta name=\"{KEY}\" content=\"{payload}\"/>\n"),
            );
        }
        Codec::Json => {
            let mut value: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| e.to_string())?;
            value["version"] = 2.into();
            value[KEY] = payload.into();
            text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
        }
        Codec::Csv => {
            text = csv_metadata(&text, Some(&payload))?;
        }
        Codec::Dxf => {
            let mut comments = String::new();
            for chunk in payload.as_bytes().chunks(200) {
                comments.push_str(&format!(
                    "999\n{PREFIX}{}\n",
                    std::str::from_utf8(chunk).unwrap()
                ));
            }
            let at = text.split_inclusive('\n').take(2).map(str::len).sum();
            text.insert_str(at, &comments);
        }
        _ => unreachable!(),
    }
    Ok(text.into_bytes())
}

fn csv_metadata(text: &str, payload: Option<&str>) -> Result<String, String> {
    let mut reader = csv::Reader::from_reader(text.as_bytes());
    let header = reader.headers().map_err(|e| e.to_string())?.clone();
    let extra = header.iter().position(|v| v == KEY);
    let version = header
        .iter()
        .position(|v| v == "midra_geometry_version")
        .ok_or("Missing CSV version")?;
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    let mut out: Vec<_> = header
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != extra)
        .map(|(_, v)| v.to_string())
        .collect();
    if payload.is_some() {
        out.push(KEY.into());
    }
    writer.write_record(out).map_err(|e| e.to_string())?;
    for (row, record) in reader.records().enumerate() {
        let record = record.map_err(|e| e.to_string())?;
        let expected = if payload.is_some() { "1" } else { "2" };
        if record.get(version) != Some(expected) {
            return Err("Unsupported metadata CSV version.".into());
        }
        let mut out: Vec<_> = record
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != extra)
            .map(|(i, v)| {
                if i == version {
                    if payload.is_some() {
                        "2".into()
                    } else {
                        "1".into()
                    }
                } else {
                    v.to_string()
                }
            })
            .collect();
        if let Some(payload) = payload {
            out.push(if row == 0 {
                payload.into()
            } else {
                String::new()
            });
        }
        writer.write_record(out).map_err(|e| e.to_string())?;
    }
    String::from_utf8(writer.into_inner().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// Strip only the known v2 extension before the existing strict geometry parser.
/// Bad optional metadata must not prevent ordinary geometry import.
pub(crate) fn geometry_text(text: &str) -> Result<Option<String>, String> {
    if text.trim_start().starts_with('{') {
        let mut value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if value["format"] == "beffroi.midra-geometry" && value["version"] == 2 {
            if value.as_object_mut().unwrap().remove(KEY).is_none() {
                return Err("Geometry JSON v2 requires extra parameters.".into());
            }
            value["version"] = 1.into();
            return serde_json::to_string(&value)
                .map(Some)
                .map_err(|e| e.to_string());
        }
    } else if text
        .lines()
        .next()
        .is_some_and(|h| h.contains(KEY) && h.contains("midra_geometry_version"))
    {
        return csv_metadata(text, None).map(Some);
    }
    Ok(None)
}

/// Restore the original exported snapshot explicitly; never overwrite edited geometry implicitly.
pub fn extract(bytes: &[u8], codec: Codec) -> Result<Option<Project>, String> {
    if bytes.len() > crate::exchange::MAX_EXCHANGE_BYTES {
        return Err("Import exceeds 16 MiB.".into());
    }
    if !supported(codec) {
        return Ok(None);
    }
    let payload = if codec == Codec::Figcraft {
        figcraft_payload(bytes)?
    } else if codec == Codec::Pptx {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
        let text = match zip.by_name("docProps/custom.xml") {
            Ok(file) => read_bounded(file)?,
            Err(zip::result::ZipError::FileNotFound) => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let xml = roxmltree::Document::parse(&text).map_err(|e| e.to_string())?;
        xml.descendants()
            .find(|n| n.has_tag_name("property") && n.attribute("name") == Some(KEY))
            .and_then(|n| n.children().find(|c| c.has_tag_name("lpwstr")))
            .and_then(|n| n.text())
            .map(str::to_owned)
    } else {
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        match codec {
            Codec::Json => serde_json::from_str::<serde_json::Value>(text)
                .map_err(|e| e.to_string())?[KEY]
                .as_str()
                .map(str::to_owned),
            Codec::Csv => {
                let mut reader = csv::Reader::from_reader(bytes);
                let index = reader
                    .headers()
                    .map_err(|e| e.to_string())?
                    .iter()
                    .position(|v| v == KEY);
                match index {
                    None => None,
                    Some(i) => {
                        let mut payload = None;
                        for row in reader.records() {
                            let row = row.map_err(|e| e.to_string())?;
                            if let Some(value) = row.get(i).filter(|v| !v.is_empty()) {
                                if payload.is_some() {
                                    return Err("Multiple extra-parameter snapshots in CSV.".into());
                                }
                                payload = Some(value.to_string());
                            }
                        }
                        payload
                    }
                }
            }
            Codec::Dxf => {
                let lines: Vec<_> = text.lines().collect();
                let payload: String = lines
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .filter(|p| p[0].trim() == "999")
                    .filter_map(|p| p[1].strip_prefix(PREFIX))
                    .collect();
                (!payload.is_empty()).then_some(payload)
            }
            _ => {
                let xml = roxmltree::Document::parse_with_options(
                    text,
                    roxmltree::ParsingOptions {
                        allow_dtd: true,
                        nodes_limit: 200_000,
                    },
                )
                .map_err(|e| e.to_string())?;
                let direct = xml
                    .descendants()
                    .find_map(|n| {
                        if n.has_tag_name(("urn:beffroi:midra:parameters:1", "parameters")) {
                            n.text()
                        } else if n.has_tag_name("meta") && n.attribute("name") == Some(KEY) {
                            n.attribute("content")
                        } else {
                            n.attribute(KEY)
                        }
                    })
                    .map(str::to_owned);
                if direct.is_some() {
                    direct
                } else if codec == Codec::Drawio
                    || (codec == Codec::Svg && xml.root_element().attribute("content").is_some())
                {
                    let model = crate::drawio_model(
                        xml.root_element().attribute("content").unwrap_or(text),
                    )?;
                    let model = roxmltree::Document::parse(&model).map_err(|e| e.to_string())?;
                    model
                        .descendants()
                        .find_map(|n| n.attribute(KEY))
                        .map(str::to_owned)
                } else {
                    None
                }
            }
        }
    };
    payload.map(|p| decode(&p)).transpose()
}
fn read_bounded(reader: impl Read) -> Result<String, String> {
    let mut text = String::new();
    reader
        .take((MAX_METADATA * 2 + 1) as u64)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if text.len() > MAX_METADATA * 2 {
        return Err("Metadata part exceeds 2 MiB.".into());
    }
    Ok(text)
}
fn embed_pptx(bytes: &[u8], payload: &str) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();
        if name == "docProps/custom.xml" {
            return Err("Exporter already contains custom properties.".into());
        }
        if name == "[Content_Types].xml" || name == "_rels/.rels" {
            let text = read_bounded(&mut file)?;
            let text = if name == "[Content_Types].xml" {
                text.replace("</Types>", "<Override PartName=\"/docProps/custom.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.custom-properties+xml\"/></Types>")
            } else {
                text.replace("</Relationships>", "<Relationship Id=\"rIdMidraParameters\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties\" Target=\"docProps/custom.xml\"/></Relationships>")
            };
            writer
                .start_file(name, options)
                .map_err(|e| e.to_string())?;
            writer
                .write_all(text.as_bytes())
                .map_err(|e| e.to_string())?;
        } else {
            writer.raw_copy_file(file).map_err(|e| e.to_string())?;
        }
    }
    writer
        .start_file("docProps/custom.xml", options)
        .map_err(|e| e.to_string())?;
    write!(writer, "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/custom-properties\" xmlns:vt=\"http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes\"><property fmtid=\"{{D5CDD505-2E9C-101B-9397-08002B2CF9AE}}\" pid=\"2\" name=\"{KEY}\"><vt:lpwstr>{payload}</vt:lpwstr></property></Properties>").map_err(|e| e.to_string())?;
    writer
        .finish()
        .map(|c| c.into_inner())
        .map_err(|e| e.to_string())
}

// FigCraft's native node plugin_data survives its normal load/save cycle. This is
// deliberately not used for FIG: that exporter drops plugin_data.
fn figcraft_document(bytes: &[u8]) -> Result<serde_json::Value, String> {
    let mut text = Vec::new();
    if bytes.starts_with(b"PK") {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
        if zip
            .file_names()
            .filter(|name| *name == "document.json")
            .count()
            != 1
        {
            return Err("FigCraft requires exactly one document.json.".into());
        }
        zip.by_name("document.json")
            .map_err(|e| e.to_string())?
            .take((crate::exchange::MAX_EXCHANGE_BYTES + 1) as u64)
            .read_to_end(&mut text)
            .map_err(|e| e.to_string())?;
    } else {
        text.extend_from_slice(bytes);
    }
    if text.len() > crate::exchange::MAX_EXCHANGE_BYTES {
        return Err("FigCraft document exceeds 16 MiB.".into());
    }
    let document: serde_json::Value = serde_json::from_slice(&text).map_err(|e| e.to_string())?;
    if document["format"] != "figcraft" || document["version"] != 1 {
        return Err("Unsupported FigCraft metadata carrier version.".into());
    }
    Ok(document)
}
fn figcraft_payload(bytes: &[u8]) -> Result<Option<String>, String> {
    let document = figcraft_document(bytes)?;
    let nodes = document["nodes"]
        .as_object()
        .ok_or("Missing FigCraft nodes")?;
    let mut found = None;
    for node in nodes.values() {
        if let Some(value) = node.get("plugin_data").and_then(|v| v.get(KEY)) {
            if found.is_some() {
                return Err("Multiple extra-parameter snapshots in FigCraft.".into());
            }
            found = Some(
                value
                    .as_str()
                    .ok_or("Invalid FigCraft parameters")?
                    .to_owned(),
            );
        }
    }
    Ok(found)
}
fn embed_figcraft(bytes: &[u8], payload: &str) -> Result<Vec<u8>, String> {
    let mut document = figcraft_document(bytes)?;
    let page = document["pages"]
        .as_array()
        .and_then(|p| p.first())
        .and_then(|p| p.as_u64())
        .ok_or("Missing FigCraft page")?
        .to_string();
    let node = document["nodes"]
        .get_mut(&page)
        .ok_or("Missing FigCraft page node")?;
    node["plugin_data"][KEY] = payload.into();
    let json = serde_json::to_vec(&document).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| e.to_string())?;
        if file.name() == "document.json" {
            writer
                .start_file(
                    "document.json",
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .map_err(|e| e.to_string())?;
            writer.write_all(&json).map_err(|e| e.to_string())?;
        } else {
            writer.raw_copy_file(file).map_err(|e| e.to_string())?;
        }
    }
    writer
        .finish()
        .map(|c| c.into_inner())
        .map_err(|e| e.to_string())
}
