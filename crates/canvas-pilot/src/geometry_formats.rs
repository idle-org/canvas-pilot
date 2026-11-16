//! Open, text-based geometry adapters beyond SVG and draw.io.
use crate::{
    Geometry,
    exchange::{ExportShape, Format, MAX_EXCHANGE_BYTES},
    import::{DetectedShape, ImportDocument},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const GEOMETRY_FORMAT: &str = "beffroi.midra-geometry";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonCanvas {
    width: u32,
    height: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonShape {
    id: String,
    name: String,
    role: String,
    x: i64,
    y: i64,
    width: i64,
    height: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image_png_base64: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonDocument {
    format: String,
    version: u32,
    units: String,
    canvas: JsonCanvas,
    layers: Vec<JsonShape>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CsvShape {
    midra_geometry_version: u32,
    canvas_width: u32,
    canvas_height: u32,
    id: String,
    name: String,
    role: String,
    x: i64,
    y: i64,
    width: i64,
    height: i64,
}

fn rounded(geometry: Geometry) -> [i64; 4] {
    [
        geometry.x.round() as i64,
        geometry.y.round() as i64,
        geometry.width.round() as i64,
        geometry.height.round() as i64,
    ]
}

fn role(background: bool) -> &'static str {
    if background { "background" } else { "layer" }
}

fn validate_canvas(canvas: [u32; 2]) -> Result<(), String> {
    crate::formats::validate_resolution(canvas)
}

fn document(
    format_name: &str,
    canvas: [u32; 2],
    shapes: Vec<JsonShape>,
) -> Result<ImportDocument, String> {
    validate_canvas(canvas)?;
    if shapes.is_empty() || shapes.len() > 257 {
        return Err("Geometry exchange requires 1–256 layers and at most one background.".into());
    }
    let mut ids = HashSet::new();
    let mut backgrounds = 0;
    let mut detected = Vec::with_capacity(shapes.len());
    for shape in shapes {
        if shape.id.is_empty() || shape.id.chars().count() > 255 || !ids.insert(shape.id.clone()) {
            return Err(
                "Geometry IDs must be unique, non-empty and at most 255 characters.".into(),
            );
        }
        if shape.name.chars().count() > 1024 || shape.name.chars().any(char::is_control) {
            return Err("Geometry names must be at most 1024 characters without controls.".into());
        }
        let background = match shape.role.as_str() {
            "layer" => false,
            "background" => {
                backgrounds += 1;
                true
            }
            _ => return Err("Geometry role must be layer or background.".into()),
        };
        let geometry = Geometry {
            x: shape.x as f64,
            y: shape.y as f64,
            width: shape.width as f64,
            height: shape.height as f64,
        }
        .validate()?;
        if background
            && (geometry.x != 0.0
                || geometry.y != 0.0
                || geometry.width != canvas[0] as f64
                || geometry.height != canvas[1] as f64)
        {
            return Err("The geometry background must cover the complete canvas.".into());
        }
        detected.push(DetectedShape {
            id: if shape.name.is_empty() {
                shape.id
            } else {
                format!("{} · {}", shape.id, shape.name)
            },
            geometry,
            outline: [0.0; 2],
            background,
            background_only: background,
            zoom: None,
        });
    }
    if backgrounds > 1
        || detected
            .iter()
            .filter(|shape| !shape.background_only)
            .count()
            > 256
    {
        return Err("Geometry exchange allows at most 256 layers and one background.".into());
    }
    Ok(ImportDocument {
        shapes: detected,
        svg: false,
        note: format!(
            "{format_name}: {} × {} px canvas; top-left whole-pixel layer boxes.",
            canvas[0], canvas[1]
        ),
    })
}

pub(crate) fn export(
    format: Format,
    canvas: [u32; 2],
    shapes: &[ExportShape],
) -> Result<String, String> {
    let output = match format {
        Format::Json => export_json(canvas, shapes)?,
        Format::Csv => export_csv(canvas, shapes)?,
        Format::Dxf => export_dxf(canvas, shapes),
        Format::Html => export_html(canvas, shapes),
        _ => unreachable!(),
    };
    if output.len() > MAX_EXCHANGE_BYTES {
        return Err("Export exceeds 16 MiB; disable embedded artwork.".into());
    }
    Ok(output)
}

fn export_json(canvas: [u32; 2], shapes: &[ExportShape]) -> Result<String, String> {
    let layers = shapes
        .iter()
        .map(|shape| {
            let [x, y, width, height] = rounded(shape.geometry);
            JsonShape {
                id: shape.id.clone(),
                name: shape.name.clone(),
                role: role(shape.background).into(),
                x,
                y,
                width,
                height,
                image_png_base64: shape.image_png_base64.clone(),
            }
        })
        .collect();
    let document = JsonDocument {
        format: GEOMETRY_FORMAT.into(),
        version: 1,
        units: "px".into(),
        canvas: JsonCanvas {
            width: canvas[0],
            height: canvas[1],
        },
        layers,
    };
    serde_json::to_string_pretty(&document)
        .map(|mut output| {
            output.push('\n');
            output
        })
        .map_err(|error| error.to_string())
}

fn export_csv(canvas: [u32; 2], shapes: &[ExportShape]) -> Result<String, String> {
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    for shape in shapes {
        let [x, y, width, height] = rounded(shape.geometry);
        writer
            .serialize(CsvShape {
                midra_geometry_version: 1,
                canvas_width: canvas[0],
                canvas_height: canvas[1],
                id: shape.id.clone(),
                name: shape.name.clone(),
                role: role(shape.background).into(),
                x,
                y,
                width,
                height,
            })
            .map_err(|error| error.to_string())?;
    }
    String::from_utf8(writer.into_inner().map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

fn export_html(canvas: [u32; 2], shapes: &[ExportShape]) -> String {
    let mut output = format!(
        "<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" data-midra-layout-geometry=\"1\"><head><meta charset=\"utf-8\"/><title>Canvas Pilot geometry</title><style>html,body{{margin:0}} #midra-canvas{{position:relative;width:{}px;height:{}px;background:#122531;overflow:hidden}} .midra-layer{{position:absolute;box-sizing:border-box;background:#173b50;color:white;display:flex;align-items:center;justify-content:center;font:16px sans-serif}} img.midra-layer{{object-fit:fill}}</style></head><body><main id=\"midra-canvas\" data-width=\"{}\" data-height=\"{}\">\n",
        canvas[0], canvas[1], canvas[0], canvas[1]
    );
    for shape in shapes {
        let [x, y, width, height] = rounded(shape.geometry);
        let common = format!(
            "id=\"{}\" class=\"midra-layer\" data-midra-role=\"{}\" data-name=\"{}\" data-x=\"{x}\" data-y=\"{y}\" data-width=\"{width}\" data-height=\"{height}\" style=\"left:{x}px;top:{y}px;width:{width}px;height:{height}px\"",
            crate::exchange::escape(&shape.id),
            role(shape.background),
            crate::exchange::escape(&shape.name)
        );
        if let Some(image) = &shape.image_png_base64 {
            output.push_str(&format!(
                "<img {common} alt=\"{}\" src=\"data:image/png;base64,{image}\"/>\n",
                crate::exchange::escape(&shape.name)
            ));
        } else {
            output.push_str(&format!(
                "<div {common}>{}</div>\n",
                crate::exchange::escape(&shape.name)
            ));
        }
    }
    output.push_str("</main></body></html>\n");
    output
}

fn export_dxf(canvas: [u32; 2], shapes: &[ExportShape]) -> String {
    use craft_dxf::model::{
        Color, Common, Entity, EntityKind, Layer, Lineweight, LwPolyline, LwVertex, Transparency,
        write::{Drawing, Format},
    };
    let bounds = [0.0, 0.0, f64::from(canvas[0]), f64::from(canvas[1])];
    let mut drawing = Drawing {
        format: Format::R2007,
        extents: bounds,
        limits: bounds,
        insunits: 0,
        ..Default::default()
    };
    for (index, shape) in shapes.iter().enumerate() {
        drawing.layers.push(Layer {
            handle: None,
            name: shape.id.clone(),
            color: Color::Index(if index % 2 == 0 { 4 } else { 5 }),
            on: true,
            frozen: false,
            vp_freeze_new: false,
            locked: false,
            plot: true,
            linetype: "Continuous".into(),
            lineweight: Lineweight::Default,
            plot_style: None,
            transparency: Transparency::ByLayer,
            description: Some(shape.name.clone()),
        });
        let [x, y, width, height] = rounded(shape.geometry).map(|v| v as f64);
        let top = f64::from(canvas[1]) - y;
        let bottom = top - height;
        let mut common = Common::read(&[]);
        common.layer = shape.id.clone();
        drawing.entities.push(Entity {
            common,
            kind: EntityKind::LwPolyline(LwPolyline {
                vertices: [[x, top], [x + width, top], [x + width, bottom], [x, bottom]]
                    .into_iter()
                    .map(|[x, y]| LwVertex {
                        x,
                        y,
                        ..Default::default()
                    })
                    .collect(),
                closed: true,
                plinegen: false,
                const_width: 0.0,
                elevation: 0.0,
            }),
        });
    }
    let (tags, _) = drawing.tags();
    // QCAD seeds stock objects at low handles before importing. Reserve that range
    // and remap definitions and references together with Craft's model helper.
    let mut tags = craft_dxf::model::write::remap_handles(&tags, |handle| Some(handle + 0x100));
    tags.insert(0, craft_dxf::Tag::s(999, "MIDRA_LAYOUT_GEOMETRY 1"));
    craft_dxf::write_ascii_as(&tags, true)
}

pub(crate) fn parse_json(text: &str) -> Result<ImportDocument, String> {
    let parsed: JsonDocument =
        serde_json::from_str(text).map_err(|error| format!("Invalid geometry JSON: {error}"))?;
    if parsed.format != GEOMETRY_FORMAT || parsed.version != 1 || parsed.units != "px" {
        return Err("Expected beffroi.midra-geometry version 1 in px.".into());
    }
    document(
        "Geometry JSON v1",
        [parsed.canvas.width, parsed.canvas.height],
        parsed.layers,
    )
}

pub(crate) fn parse_csv(text: &str) -> Result<ImportDocument, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut reader = csv::ReaderBuilder::new().from_reader(text.as_bytes());
    let mut canvas = None;
    let mut layers = Vec::new();
    for row in reader.deserialize::<CsvShape>() {
        let row = row.map_err(|error| format!("Invalid geometry CSV: {error}"))?;
        if row.midra_geometry_version != 1 {
            return Err("Expected geometry CSV version 1.".into());
        }
        let row_canvas = [row.canvas_width, row.canvas_height];
        if canvas
            .replace(row_canvas)
            .is_some_and(|previous| previous != row_canvas)
        {
            return Err("Every geometry CSV row must use the same canvas.".into());
        }
        layers.push(JsonShape {
            id: row.id,
            name: row.name,
            role: row.role,
            x: row.x,
            y: row.y,
            width: row.width,
            height: row.height,
            image_png_base64: None,
        });
    }
    document(
        "Geometry CSV v1",
        canvas.ok_or("Geometry CSV has no layer rows.")?,
        layers,
    )
}

fn xml_number(node: roxmltree::Node, name: &str) -> Result<i64, String> {
    node.attribute(name)
        .ok_or_else(|| format!("Missing HTML {name}."))?
        .parse::<i64>()
        .map_err(|_| format!("Invalid HTML {name}."))
}

pub(crate) fn parse_html(text: &str) -> Result<ImportDocument, String> {
    let document_xml = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            nodes_limit: 200_000,
        },
    )
    .map_err(|error| format!("Invalid geometry HTML: {error}"))?;
    let root = document_xml.root_element();
    if !root.has_tag_name("html") || root.attribute("data-midra-layout-geometry") != Some("1") {
        return Err("Expected Canvas Pilot HTML geometry version 1.".into());
    }
    let canvas = root
        .descendants()
        .find(|node| node.attribute("id") == Some("midra-canvas"))
        .ok_or("HTML geometry is missing #midra-canvas.")?;
    let size = [
        xml_number(canvas, "data-width")?,
        xml_number(canvas, "data-height")?,
    ];
    if size
        .iter()
        .any(|value| *value <= 0 || *value > i64::from(u32::MAX))
    {
        return Err("Invalid HTML canvas size.".into());
    }
    let mut layers = Vec::new();
    for node in canvas
        .descendants()
        .filter(|node| node.is_element() && node.attribute("data-midra-role").is_some())
    {
        layers.push(JsonShape {
            id: node.attribute("id").unwrap_or("").into(),
            name: node.attribute("data-name").unwrap_or("").into(),
            role: node.attribute("data-midra-role").unwrap().into(),
            x: xml_number(node, "data-x")?,
            y: xml_number(node, "data-y")?,
            width: xml_number(node, "data-width")?,
            height: xml_number(node, "data-height")?,
            image_png_base64: None,
        });
    }
    document(
        "HTML/CSS geometry v1",
        [size[0] as u32, size[1] as u32],
        layers,
    )
}

pub(crate) fn parse_dxf(text: &str) -> Result<ImportDocument, String> {
    parse_dxf_bytes(text.as_bytes())
}

pub(crate) fn parse_dxf_bytes(bytes: &[u8]) -> Result<ImportDocument, String> {
    let tags = craft_dxf::parse_with_limit(bytes, MAX_EXCHANGE_BYTES / 4)
        .map_err(|e| format!("Invalid DXF: {e}"))?;
    let values: Vec<_> = tags.iter().map(|tag| (tag.code, tag.str())).collect();
    let pairs: Vec<_> = values
        .iter()
        .map(|(code, value)| (*code, value.trim()))
        .collect();
    if !pairs
        .iter()
        .any(|(code, value)| *code == 0 && *value == "LWPOLYLINE")
    {
        return Err("DXF import requires closed LWPOLYLINE rectangles.".into());
    }
    let header_point = |name: &str| -> Result<[f64; 2], String> {
        let start = pairs
            .iter()
            .position(|pair| *pair == (9, name))
            .ok_or_else(|| format!("DXF requires {name} canvas metadata."))?;
        let mut x = None;
        let mut y = None;
        for (code, value) in &pairs[start + 1..] {
            if matches!(code, 0 | 9) {
                break;
            }
            if *code == 10 {
                x = Some(
                    value
                        .parse::<f64>()
                        .map_err(|_| format!("Invalid DXF {name} X coordinate."))?,
                );
            } else if *code == 20 {
                y = Some(
                    value
                        .parse::<f64>()
                        .map_err(|_| format!("Invalid DXF {name} Y coordinate."))?,
                );
            }
        }
        Ok([
            x.ok_or_else(|| format!("DXF {name} is missing X."))?,
            y.ok_or_else(|| format!("DXF {name} is missing Y."))?,
        ])
    };
    let min = header_point("$LIMMIN").or_else(|_| header_point("$EXTMIN"))?;
    let max = header_point("$LIMMAX").or_else(|_| header_point("$EXTMAX"))?;
    let canvas_f = [max[0] - min[0], max[1] - min[1]];
    if canvas_f
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0 || value.fract() != 0.0)
    {
        return Err("DXF canvas extents must be positive whole pixels.".into());
    }
    let canvas = [canvas_f[0] as u32, canvas_f[1] as u32];
    validate_canvas(canvas)?;
    let mut layers = Vec::new();
    let mut index = 0;
    while index < pairs.len() {
        if pairs[index] != (0, "LWPOLYLINE") {
            index += 1;
            continue;
        }
        let start = index + 1;
        index = start;
        while index < pairs.len() && pairs[index].0 != 0 {
            index += 1;
        }
        let entity = &pairs[start..index];
        let id = entity
            .iter()
            .find(|(code, _)| *code == 8)
            .map(|(_, value)| *value)
            .ok_or("DXF rectangle is missing a layer name.")?;
        let closed = entity
            .iter()
            .find(|(code, _)| *code == 70)
            .and_then(|(_, value)| value.parse::<u32>().ok())
            .is_some_and(|flags| flags & 1 != 0);
        if !closed {
            return Err("DXF rectangles must be closed LWPOLYLINE entities.".into());
        }
        let mut points = Vec::new();
        let mut point = None;
        for (code, value) in entity {
            if *code == 10 {
                if point.is_some() {
                    return Err("DXF vertex is missing its Y coordinate.".into());
                }
                point = Some(
                    value
                        .parse::<f64>()
                        .map_err(|_| "Invalid DXF X coordinate.")?,
                );
            } else if *code == 20 {
                let x = point.take().ok_or("DXF vertex Y appears before X.")?;
                points.push([
                    x,
                    value
                        .parse::<f64>()
                        .map_err(|_| "Invalid DXF Y coordinate.")?,
                ]);
            }
        }
        if point.is_some() || points.len() != 4 {
            return Err("DXF rectangles require exactly four X/Y vertices.".into());
        }
        let cad = super::import::rectangle_points(points)?;
        let [x, y, width, height] = [
            cad.x - min[0],
            max[1] - (cad.y + cad.height),
            cad.width,
            cad.height,
        ]
        .map(|value| value.round() as i64);
        layers.push(JsonShape {
            id: id.into(),
            name: String::new(),
            role: if id == "midra-background" {
                "background"
            } else {
                "layer"
            }
            .into(),
            x,
            y,
            width,
            height,
            image_png_base64: None,
        });
    }
    document("DXF geometry", canvas, layers)
}

pub(crate) fn looks_like_csv(text: &str) -> bool {
    text.trim_start_matches(['\u{feff}', ' ', '\t', '\r', '\n'])
        .starts_with("midra_geometry_version,canvas_width,canvas_height,")
}

pub(crate) fn looks_like_dxf(text: &str) -> bool {
    let start = text.trim_start_matches(['\u{feff}', ' ', '\t', '\r', '\n']);
    start.starts_with("999\r\nMIDRA_LAYOUT_GEOMETRY")
        || start.starts_with("999\nMIDRA_LAYOUT_GEOMETRY")
        || (start.starts_with("0\r\nSECTION") && text.contains("LWPOLYLINE"))
        || (start.starts_with("0\nSECTION") && text.contains("LWPOLYLINE"))
}
