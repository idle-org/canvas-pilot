//! Optional ArtCraft codecs. No device access or remote conversion.
use crate::{
    Geometry,
    exchange::{self, ExportShape},
    import::{DetectedShape, ImportDocument},
    interchange::{Codec, Page},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::{io::Read, sync::Arc};

fn shape(name: String, geometry: Geometry, background: bool) -> Result<DetectedShape, String> {
    Ok(DetectedShape {
        name: crate::import::source_name(Some(&name)),
        id: name,
        geometry: geometry.validate()?,
        outline: [0.0; 2],
        background,
        background_only: background,
        zoom: None,
        appearance: None,
    })
}
fn page(name: String, shapes: Vec<DetectedShape>, note: String) -> Result<Page, String> {
    if shapes.len() > 257 {
        return Err(format!(
            "{name} has more than 256 layers plus background. Choose a smaller frame or slide."
        ));
    }
    Ok(Page {
        name,
        document: ImportDocument {
            shapes,
            svg: false,
            note,
        },
    })
}
fn rounded(g: Geometry) -> Geometry {
    Geometry {
        x: g.x.round(),
        y: g.y.round(),
        width: g.width.round(),
        height: g.height.round(),
    }
}

pub(crate) fn export(
    codec: Codec,
    canvas: [u32; 2],
    shapes: &[ExportShape],
) -> Result<Vec<u8>, String> {
    if matches!(codec, Codec::Pptx | Codec::Figma | Codec::Figcraft)
        && shapes
            .iter()
            .any(|s| !s.background && s.name == "midra-background")
    {
        return Err("Rename the layer called ‘midra-background’ before native export; that name identifies the optional background.".into());
    }
    match codec {
        Codec::Dwg => {
            let dxf = exchange::encode_shapes(exchange::Format::Dxf, canvas, shapes)?;
            let (bytes, omitted) = craft_dwg::dxf_to_dwg_noted(dxf.as_bytes(), "AC1021")?;
            if !omitted.is_empty() {
                return Err(format!("DWG export omitted records: {omitted:?}"));
            }
            Ok(bytes)
        }
        Codec::Pptx => export_pptx(canvas, shapes),
        Codec::Figma => export_fig(canvas, shapes, false),
        Codec::Figcraft => export_fig(canvas, shapes, true),
        Codec::Illustrator => {
            let svg = exchange::encode_shapes(exchange::Format::Svg, canvas, shapes)?;
            let (bytes, warnings) = convert_ai(svg.as_bytes(), "svg", "ai", None)?;
            if !warnings.is_empty() {
                return Err(format!(
                    "Illustrator export reported losses: {}",
                    warnings.join("; ")
                ));
            }
            Ok(bytes)
        }
        _ => Err("Not a vendor format.".into()),
    }
}
pub(crate) fn import(bytes: &[u8], extension: &str, filename: &str) -> Result<Vec<Page>, String> {
    if extension == "dwg" || craft_dwg::is_dwg(bytes) {
        let (dxf, notes) = craft_dwg::dwg_to_dxf_noted(
            bytes,
            craft_dwg::Limits {
                max_dwg_bytes: exchange::MAX_EXCHANGE_BYTES,
                max_dxf_bytes: exchange::MAX_EXCHANGE_BYTES,
            },
        )?;
        let mut document = crate::geometry_formats::parse_dxf_bytes(&dxf)?;
        document.note = format!("DWG via Craft. {} {}", document.note, notes.join("; "));
        return Ok(vec![Page {
            name: filename.into(),
            document,
        }]);
    }
    if extension == "ai" {
        return import_ai(bytes);
    }
    if deckcraft_pptx::sniff(bytes) {
        return import_pptx(bytes);
    }
    import_fig(bytes, filename, extension == "figcraft")
}

fn export_pptx([w, h]: [u32; 2], shapes: &[ExportShape]) -> Result<Vec<u8>, String> {
    use deckcraft_model::{
        MediaId, MediaItem, Presentation, Shape, ShapeId, ShapeKind, Slide, SlideId,
    };
    // CSS pixels → points → OOXML EMUs (DeckCraft performs the final conversion).
    let mut doc = Presentation {
        slide_size: deckcraft_geom::Size::new(w as f64 * 0.75, h as f64 * 0.75),
        ..Default::default()
    };
    let mut slide = Slide {
        id: SlideId(doc.alloc_id()),
        name: "Canvas Pilot".into(),
        show_master_shapes: false,
        ..Default::default()
    };
    for s in shapes {
        let g = rounded(s.geometry);
        let mut layer = Shape {
            id: ShapeId(doc.alloc_id()),
            name: if s.background {
                "midra-background".into()
            } else {
                s.name.clone()
            },
            xfrm: Some(deckcraft_geom::Xfrm::new(
                g.x * 0.75,
                g.y * 0.75,
                g.width * 0.75,
                g.height * 0.75,
            )),
            fill: Some(deckcraft_model::Fill::solid(
                deckcraft_model::ColorRef::rgb(deckcraft_model::Rgba::rgb(35, 69, 87)),
            )),
            line: Some(deckcraft_model::Line {
                fill: Some(deckcraft_model::Fill::None),
                ..Default::default()
            }),
            ..Default::default()
        };
        if let Some(png) = &s.image_png_base64 {
            let media = MediaId(doc.alloc_id());
            doc.media.push(MediaItem {
                id: media,
                name: format!("{}.png", s.id),
                content_type: "image/png".into(),
                data: Arc::new(STANDARD.decode(png).map_err(|e| e.to_string())?),
                ..Default::default()
            });
            layer.kind = ShapeKind::Picture {
                fill: deckcraft_model::style::PictureFill {
                    media,
                    ..Default::default()
                },
            };
        }
        slide.shapes.push(layer);
    }
    doc.slides = vec![Arc::new(slide)];
    deckcraft_pptx::export(&doc).map_err(|e| e.to_string())
}

fn import_pptx(bytes: &[u8]) -> Result<Vec<Page>, String> {
    let doc = deckcraft_pptx::import(bytes).map_err(|e| e.to_string())?;
    let mut pages = Vec::new();
    for (index, slide) in doc.slides.iter().enumerate() {
        let mut shapes = Vec::new();
        let mut skipped = 0;
        collect_pptx(
            &slide.shapes,
            deckcraft_geom::Affine::IDENTITY,
            &mut shapes,
            &mut skipped,
            0,
        )?;
        pages.push(page(format!("Slide {} · {}", index + 1, slide.name), shapes, format!("PPTX: 96 px/in; slide {:.1} × {:.1} px. {skipped} unsupported, hidden or unpositioned objects omitted. Rotated objects use enclosing boxes; masters and inherited placeholders are not imported.", doc.slide_size.width / 0.75, doc.slide_size.height / 0.75))?);
    }
    if pages.is_empty() {
        return Err("The presentation has no readable slides.".into());
    }
    Ok(pages)
}
fn collect_pptx(
    items: &[deckcraft_model::Shape],
    parent: deckcraft_geom::Affine,
    out: &mut Vec<DetectedShape>,
    skipped: &mut usize,
    depth: usize,
) -> Result<(), String> {
    use deckcraft_model::ShapeKind;
    if depth > 64 {
        return Err("PPTX groups are nested too deeply.".into());
    }
    for item in items {
        if item.hidden {
            *skipped += 1;
            continue;
        }
        let Some(x) = item.xfrm else {
            *skipped += 1;
            continue;
        };
        if let ShapeKind::Group { children, child } = &item.kind {
            collect_pptx(
                children,
                parent * deckcraft_geom::group_child_affine(&x, child.rect()),
                out,
                skipped,
                depth + 1,
            )?;
            continue;
        }
        if matches!(
            item.kind,
            ShapeKind::Connector { .. } | ShapeKind::Opaque { .. }
        ) || x.w <= 0.0
            || x.h <= 0.0
        {
            *skipped += 1;
            continue;
        }
        let r = (parent * x.affine())
            .transform_rect_bbox(deckcraft_geom::Rect::new(0.0, 0.0, x.w, x.h));
        out.push(shape(
            item.name.clone(),
            Geometry {
                x: r.x0 / 0.75,
                y: r.y0 / 0.75,
                width: r.width() / 0.75,
                height: r.height() / 0.75,
            },
            item.name == "midra-background",
        )?);
        if out.len() > 257 {
            return Err("Slide exceeds 256 layers plus background.".into());
        }
    }
    Ok(())
}

fn export_fig([w, h]: [u32; 2], shapes: &[ExportShape], native: bool) -> Result<Vec<u8>, String> {
    use figcraft_doc::{
        Document, Node, NodeKind,
        color::{ImageScaleMode, Paint},
    };
    let mut doc = Document::new("Canvas Pilot");
    let page = doc
        .pages
        .first()
        .copied()
        .unwrap_or_else(|| doc.add_page("Layout"));
    let frame = Node::new(
        doc.alloc_id(),
        NodeKind::Frame,
        "Canvas Pilot",
        w as f64,
        h as f64,
    );
    let frame_id = doc.insert(frame, page, None).map_err(|e| e.to_string())?;
    for s in shapes {
        let g = rounded(s.geometry);
        let mut node = Node::new(
            doc.alloc_id(),
            NodeKind::Rectangle,
            if s.background {
                "midra-background"
            } else {
                &s.name
            },
            g.width,
            g.height,
        );
        node.transform = figcraft_geom::Affine::translate((g.x, g.y));
        if let Some(png) = &s.image_png_base64 {
            let (hash, _, _) = figcraft_format::add_image(
                &mut doc,
                STANDARD.decode(png).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            node.fills = vec![Paint::image(hash, ImageScaleMode::Fill)];
        }
        doc.insert(node, frame_id, None)
            .map_err(|e| e.to_string())?;
    }
    if native {
        return figcraft_format::save_bytes(&doc, None).map_err(|e| e.to_string());
    }
    let (bytes, warnings) = figcraft_format::fig::export(&doc, None).map_err(|e| e.to_string())?;
    if !warnings.is_empty() {
        return Err(format!(
            "Figma export would lose content: {}",
            warnings.join("; ")
        ));
    }
    Ok(bytes)
}

fn import_fig(bytes: &[u8], filename: &str, native: bool) -> Result<Vec<Page>, String> {
    use figcraft_doc::NodeKind;
    let (doc, warnings) = if native {
        figcraft_format::load_bytes(bytes)
    } else {
        figcraft_format::fig::import(bytes, filename)
    }
    .map_err(|e| e.to_string())?;
    let mut pages = Vec::new();
    for id in &doc.pages {
        let Some(p) = doc.node(*id) else {
            continue;
        };
        // Frames are individual canvases, while loose nodes can be reviewed as a whole page.
        for root in p
            .children
            .iter()
            .filter_map(|id| doc.node(*id))
            .filter(|n| matches!(n.kind, NodeKind::Frame | NodeKind::Component { .. }))
        {
            let mut shapes = Vec::new();
            let inverse = doc.world_transform(root.id).inverse();
            collect_fig(&doc, &root.children, inverse, &mut shapes, 0)?;
            pages.push(page(
                format!("{} / {}", p.name, root.name),
                shapes,
                format!(
                    "Frame coordinates in pixels. Rotated objects use enclosing boxes. {}",
                    warnings.join("; ")
                ),
            )?);
        }
        if p.children
            .iter()
            .filter_map(|id| doc.node(*id))
            .any(|n| !matches!(n.kind, NodeKind::Frame | NodeKind::Component { .. }))
            || pages.is_empty()
        {
            let mut shapes = Vec::new();
            collect_fig(
                &doc,
                &p.children,
                figcraft_geom::Affine::IDENTITY,
                &mut shapes,
                0,
            )?;
            pages.push(page(
                p.name.clone(),
                shapes,
                format!(
                    "Page coordinates in pixels. Rotated objects use enclosing boxes. {}",
                    warnings.join("; ")
                ),
            )?);
        }
    }
    if pages.is_empty() {
        return Err("The Figma file has no readable pages or frames.".into());
    }
    Ok(pages)
}
fn collect_fig(
    doc: &figcraft_doc::Document,
    ids: &[figcraft_doc::NodeId],
    origin: figcraft_geom::Affine,
    out: &mut Vec<DetectedShape>,
    depth: usize,
) -> Result<(), String> {
    if depth > 64 {
        return Err("Figma groups are nested too deeply.".into());
    }
    for id in ids {
        let Some(n) = doc.node(*id) else {
            continue;
        };
        if !n.visible {
            continue;
        }
        if !n.children.is_empty() {
            collect_fig(doc, &n.children, origin, out, depth + 1)?;
            continue;
        }
        if n.width <= 0.0 || n.height <= 0.0 {
            continue;
        }
        let r = (origin * doc.world_transform(*id))
            .transform_rect_bbox(figcraft_geom::Rect::new(0.0, 0.0, n.width, n.height));
        out.push(shape(
            n.name.clone(),
            Geometry {
                x: r.x0,
                y: r.y0,
                width: r.width(),
                height: r.height(),
            },
            n.name == "midra-background",
        )?);
        if out.len() > 257 {
            return Err("Figma canvas exceeds 256 layers plus background.".into());
        }
    }
    Ok(())
}

fn vectorcraft() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os("MIDRA_VECTORCRAFT_CLI") {
        return path.into();
    }
    let name = if cfg!(windows) {
        "vectorcraft-cli.exe"
    } else {
        "vectorcraft-cli"
    };
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        for path in [parent.join(name), parent.join("bin").join(name)] {
            if path.is_file() {
                return path;
            }
        }
    }
    name.into()
}
fn import_ai(bytes: &[u8]) -> Result<Vec<Page>, String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let input = dir.path().join("input.ai");
    std::fs::write(&input, bytes).map_err(|e| e.to_string())?;
    let mut command = std::process::Command::new(vectorcraft());
    command.arg("--safe-mode").arg("info").arg(&input);
    let info = run_vectorcraft(&mut command, dir.path())?;
    let artboards = info
        .get("artboards")
        .and_then(|v| v.as_array())
        .ok_or("VectorCraft did not report any artboards.")?;
    if artboards.is_empty() || artboards.len() > 64 {
        return Err("Illustrator import supports 1–64 artboards.".into());
    }
    let mut pages = Vec::new();
    for (index, artboard) in artboards.iter().enumerate() {
        let (svg, warnings) = convert_ai(bytes, "ai", "svg", Some(index))?;
        let text = std::str::from_utf8(&svg).map_err(|e| e.to_string())?;
        let mut document = crate::import::parse_values(text)?;
        document.note = format!(
            "Illustrator artboard converted locally by VectorCraft. {} {}",
            document.note,
            warnings.join("; ")
        );
        pages.push(Page {
            name: format!(
                "Artboard {} · {}",
                index + 1,
                artboard
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unnamed")
            ),
            document,
        });
    }
    Ok(pages)
}

pub(crate) fn reference_ai(bytes: &[u8]) -> Result<Vec<(String, String)>, String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let input = dir.path().join("input.ai");
    std::fs::write(&input, bytes).map_err(|e| e.to_string())?;
    let mut command = std::process::Command::new(vectorcraft());
    command.arg("--safe-mode").arg("info").arg(&input);
    let info = run_vectorcraft(&mut command, dir.path())?;
    let boards = info
        .get("artboards")
        .and_then(|v| v.as_array())
        .ok_or("No Illustrator artboards")?;
    if boards.is_empty() || boards.len() > 64 {
        return Err("Illustrator supports 1–64 artboards.".into());
    }
    boards
        .iter()
        .enumerate()
        .map(|(index, board)| {
            let (svg, warnings) = convert_ai(bytes, "ai", "svg", Some(index))?;
            let label = format!(
                "Artboard {} · {} {}",
                index + 1,
                board
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unnamed"),
                warnings.join("; ")
            );
            Ok((label, String::from_utf8(svg).map_err(|e| e.to_string())?))
        })
        .collect()
}

pub(crate) fn reference_fig(
    bytes: &[u8],
    name: &str,
    native: bool,
) -> Result<(Vec<(String, String)>, String), String> {
    use figcraft_doc::NodeKind;
    let (doc, warnings) = if native {
        figcraft_format::load_bytes(bytes)
    } else {
        figcraft_format::fig::import(bytes, name)
    }
    .map_err(|e| e.to_string())?;
    let mut pages = Vec::new();
    for id in &doc.pages {
        let Some(page) = doc.node(*id) else { continue };
        let mut loose = Vec::new();
        for id in &page.children {
            let Some(node) = doc.node(*id) else { continue };
            if matches!(node.kind, NodeKind::Frame | NodeKind::Component { .. }) {
                let svg = figcraft_svg::export_svg(&doc, &[*id], &Default::default())
                    .ok_or("Cannot render Figma frame")?;
                pages.push((format!("{} / {}", page.name, node.name), svg));
            } else {
                loose.push(*id);
            }
        }
        if !loose.is_empty() {
            let svg = figcraft_svg::export_svg(&doc, &loose, &Default::default())
                .ok_or("Cannot render Figma page")?;
            pages.push((page.name.clone(), svg));
        }
        if pages.len() > 256 {
            return Err("Reference supports up to 256 pages/frames.".into());
        }
    }
    if pages.is_empty() {
        return Err("No visible Figma pages/frames.".into());
    }
    Ok((pages, warnings.join("; ")))
}

fn convert_ai(
    bytes: &[u8],
    from: &str,
    to: &str,
    artboard: Option<usize>,
) -> Result<(Vec<u8>, Vec<String>), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let input = dir.path().join(format!("input.{from}"));
    let output = dir.path().join(format!("output.{to}"));
    std::fs::write(&input, bytes).map_err(|e| e.to_string())?;
    let mut command = std::process::Command::new(vectorcraft());
    command
        .arg("--safe-mode")
        .arg("convert")
        .arg(&input)
        .arg(&output);
    if let Some(index) = artboard {
        command.arg("--artboard").arg(index.to_string());
    }
    let result = run_vectorcraft(&mut command, dir.path())?;
    let warnings = ["warnings", "importWarnings"]
        .into_iter()
        .flat_map(|key| {
            result
                .get(key)
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
        })
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| v.to_string())
        })
        .collect();
    let mut bytes = Vec::new();
    std::fs::File::open(output)
        .and_then(|f| {
            f.take(exchange::MAX_EXCHANGE_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
        })
        .map_err(|e| e.to_string())?;
    if bytes.len() > exchange::MAX_EXCHANGE_BYTES {
        return Err("Converted file exceeds 16 MiB.".into());
    }
    Ok((bytes, warnings))
}

fn run_vectorcraft(
    command: &mut std::process::Command,
    directory: &std::path::Path,
) -> Result<serde_json::Value, String> {
    use std::{
        process::Stdio,
        time::{Duration, Instant},
    };
    let log = std::fs::File::create(directory.join("converter.log")).map_err(|e| e.to_string())?;
    let report = std::fs::File::create(directory.join("report.json")).map_err(|e| e.to_string())?;
    let mut child = command.stdin(Stdio::null()).stdout(report).stderr(log).spawn()
        .map_err(|e| format!("Cannot start vectorcraft-cli: {e}. Install VectorCraft CLI or set MIDRA_VECTORCRAFT_CLI to its executable."))?;
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < Duration::from_secs(60) => {
                std::thread::sleep(Duration::from_millis(50))
            }
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "VectorCraft conversion did not finish within 60 seconds: {result:?}"
                ));
            }
        }
    };
    if !status.success() {
        let mut log = String::new();
        let _ = std::fs::File::open(directory.join("converter.log"))
            .and_then(|f| f.take(8192).read_to_string(&mut log));
        return Err(format!("VectorCraft conversion failed ({status}): {log}"));
    }
    let mut report = Vec::new();
    std::fs::File::open(directory.join("report.json"))
        .and_then(|f| f.take(1024 * 1024).read_to_end(&mut report))
        .map_err(|e| e.to_string())?;
    serde_json::from_slice(&report)
        .map_err(|e| format!("Invalid VectorCraft conversion report: {e}"))
}
