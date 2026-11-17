//! Local document-to-picture imports. Sources are never modified or saved over.
use hayro::hayro_syntax::Pdf;
use std::{io::Read, path::Path, sync::Arc};

pub const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_PAGES: usize = 256;
static RENDER_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Clone)]
enum Source {
    Pdf(Arc<Vec<u8>>),
    Svg(Vec<String>),
}

#[derive(Clone)]
pub struct Document {
    pub name: String,
    pub pages: Vec<String>,
    pub note: String,
    source: Source,
}

/// Pictures plus every geometry format compiled into this build.
pub fn extensions() -> Vec<&'static str> {
    let mut extensions = crate::interchange::extensions();
    extensions.extend(["pdf", "png", "jpg", "jpeg", "bmp"]);
    extensions
}

pub fn is_document(name: &Path) -> bool {
    name.extension()
        .and_then(|v| v.to_str())
        .is_some_and(|ext| {
            let ext = ext.to_ascii_lowercase();
            !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp")
                && extensions().contains(&ext.as_str())
        })
}

pub fn read(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes))
        .map_err(|e| format!("Cannot read reference document: {e}"))?;
    check_size(&bytes)?;
    Ok(bytes)
}

fn check_size(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_BYTES {
        Err("Reference document exceeds 64 MiB.".into())
    } else {
        Ok(())
    }
}

impl Document {
    /// `layout_only` explicitly requests the existing geometry codec's bounds view.
    pub fn open(bytes: Vec<u8>, name: &str, layout_only: bool) -> Result<Self, String> {
        check_size(&bytes)?;
        let ext = Path::new(name)
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(ext.as_str(), "pptx" | "fig" | "figcraft" | "ai" | "dwg")
            && !crate::interchange::extensions().contains(&ext.as_str())
        {
            return Err("Vendor format support is disabled in this build.".into());
        }
        if ext == "pdf" || bytes.starts_with(b"%PDF-") {
            return Self::pdf(
                bytes,
                name,
                "PDF page rendered locally. Check the preview before adding it.",
            );
        }
        if !layout_only && ext == "svg" {
            return Ok(Self { name: name.into(), pages: vec!["Artwork".into()],
                note: "SVG artwork. Embedded images and installed fonts are rendered; linked external images are omitted.".into(),
                source: Source::Svg(vec![String::from_utf8(bytes).map_err(|e| e.to_string())?]) });
        }
        if !layout_only && matches!(ext.as_str(), "json" | "html") {
            let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
            let svg = crate::geometry_formats::reference_svg(text, ext == "html")?;
            return Ok(Self { name: name.into(), pages: vec!["Layout artwork".into()],
                note: "Canvas Pilot exchange artwork, including embedded PNG pictures. HTML is rendered from its geometry data; custom CSS and scripts are not executed.".into(), source: Source::Svg(vec![svg]) });
        }
        if !layout_only && ext == "pptx" {
            let pdf = office_pdf(&bytes)?;
            return Self::pdf(
                pdf,
                name,
                "Slides rendered by LibreOffice. Fonts and effects may differ from PowerPoint; check the preview.",
            );
        }
        if !layout_only && matches!(ext.as_str(), "drawio" | "xml") {
            let pdf = drawio_pdf(&bytes)?;
            return Self::pdf(
                pdf,
                name,
                "Diagram pages rendered by draw.io Desktop. Check the preview.",
            );
        }
        #[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
        if !layout_only && ext == "ai" {
            let pages = crate::vendor_formats::reference_ai(&bytes)?;
            return Ok(Self { name: name.into(), pages: pages.iter().map(|p| p.0.clone()).collect(),
                note: "Illustrator artwork converted by VectorCraft. Check the preview for conversion differences.".into(),
                source: Source::Svg(pages.into_iter().map(|p| p.1).collect()) });
        }
        #[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
        if !layout_only && matches!(ext.as_str(), "fig" | "figcraft") {
            let (pages, warnings) =
                crate::vendor_formats::reference_fig(&bytes, name, ext == "figcraft")?;
            return Ok(Self {
                name: name.into(),
                pages: pages.iter().map(|p| p.0.clone()).collect(),
                note: format!(
                    "Figma artwork rendered through FigCraft (best effort). Check the preview. {warnings}"
                ),
                source: Source::Svg(pages.into_iter().map(|p| p.1).collect()),
            });
        }
        if !layout_only && !matches!(ext.as_str(), "json" | "csv" | "dxf" | "dwg") {
            return Err("This format currently provides a layout preview only. Select ‘Layout boxes’ to render its supported geometry, or export PDF/SVG from the source application for the complete artwork.".into());
        }
        let pages = crate::interchange::import(&bytes, name)?;
        let mut pictures = Vec::new();
        for page in &pages {
            pictures.push(layout_svg(&page.document)?);
        }
        Ok(Self { name: name.into(), pages: pages.iter().map(|p| p.name.clone()).collect(),
            note: "Layout boxes only: labels and bounds from the geometry importer. Original artwork, fonts, pictures and decorative shapes are not reproduced. The preview fits the detected bounds.".into(),
            source: Source::Svg(pictures) })
    }

    fn pdf(bytes: Vec<u8>, name: &str, note: &str) -> Result<Self, String> {
        let pdf = parse_pdf(bytes.clone())?;
        let count = pdf.pages().len();
        if count == 0 || count > MAX_PAGES {
            return Err("Choose a PDF with 1–256 pages.".into());
        }
        Ok(Self {
            name: name.into(),
            pages: (1..=count).map(|n| format!("Page {n}")).collect(),
            note: note.into(),
            source: Source::Pdf(Arc::new(bytes)),
        })
    }

    /// Render only the chosen page, at a bounded size, and encode portable PNG bytes.
    pub fn render(&self, page: usize, longest_side: u32) -> Result<Vec<u8>, String> {
        if page >= self.pages.len() {
            return Err("Reference page is out of range.".into());
        }
        if !(64..=4096).contains(&longest_side) {
            return Err("Reference resolution must be 64–4096 pixels.".into());
        }
        let _guard = RENDER_LOCK
            .lock()
            .map_err(|_| "Document renderer stopped; please restart the app.")?;
        let bytes = match &self.source {
            Source::Pdf(bytes) => {
                let pdf = parse_pdf(bytes.as_ref().clone())?;
                let page = &pdf.pages()[page];
                let (w, h) = page.render_dimensions();
                let scale = render_scale(w, h, longest_side)?;
                let pixmap = hayro::render(
                    page,
                    &hayro::RenderCache::new(),
                    &Default::default(),
                    &Default::default(),
                    &hayro::PixmapSettings {
                        x_scale: scale,
                        y_scale: scale,
                        bg_color: hayro::vello_cpu::color::palette::css::WHITE,
                    },
                );
                pixmap
                    .into_png()
                    .map_err(|e| format!("Cannot encode rendered PDF: {e}"))?
            }
            Source::Svg(pages) => render_svg(pages[page].as_bytes(), longest_side)?,
        };
        check_size(&bytes)?;
        Ok(bytes)
    }
}

fn parse_pdf(bytes: Vec<u8>) -> Result<Pdf, String> {
    Pdf::new(bytes).map_err(|e| format!("Cannot open PDF ({e:?}). For a password-protected document, export an unlocked copy first."))
}

fn render_scale(width: f32, height: f32, longest: u32) -> Result<f32, String> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err("Invalid document page dimensions.".into());
    }
    let scale = longest as f32 / width.max(height);
    if width.min(height) * scale < 1.0 {
        return Err("Document page is too narrow to render.".into());
    }
    Ok(scale)
}

fn render_svg(bytes: &[u8], longest: u32) -> Result<Vec<u8>, String> {
    let mut options = resvg::usvg::Options::default();
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    options.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_data(bytes, &options)
        .map_err(|e| format!("Cannot render SVG: {e}"))?;
    let scale = render_scale(tree.size().width(), tree.size().height(), longest)?;
    let width = (tree.size().width() * scale).ceil() as u32;
    let height = (tree.size().height() * scale).ceil() as u32;
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).ok_or("Cannot allocate reference image.")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|e| e.to_string())
}

fn layout_svg(document: &crate::import::ImportDocument) -> Result<String, String> {
    if document.shapes.is_empty() {
        return Err("No supported layout shapes found.".into());
    }
    let mut bounds = [0.0_f64, 0.0_f64, 1.0_f64, 1.0_f64];
    for s in &document.shapes {
        let g = s.geometry.validate()?;
        bounds[0] = bounds[0].min(g.x);
        bounds[1] = bounds[1].min(g.y);
        bounds[2] = bounds[2].max(g.x + g.width);
        bounds[3] = bounds[3].max(g.y + g.height);
    }
    let [x, y, right, bottom] = bounds;
    let width = right - x;
    let height = bottom - y;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"{x} {y} {width} {height}\"><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" fill=\"#111720\"/>"
    );
    for s in &document.shapes {
        let g = s.geometry;
        let name = crate::exchange::escape(s.name.as_deref().unwrap_or(&s.id));
        let fill = if s.background { "#1b2330" } else { "#315066" };
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{fill}\" stroke=\"#66dbef\" stroke-width=\"{}\"/><text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"{}\" fill=\"white\">{name}</text>", g.x, g.y, g.width, g.height, width / 800.0, g.x + width / 200.0, g.y + height / 35.0, height / 45.0));
    }
    svg.push_str("</svg>");
    Ok(svg)
}

fn converter(env: &str, name: &str) -> std::path::PathBuf {
    if let Some(path) = std::env::var_os(env) {
        return path.into();
    }
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

fn office_pdf(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let input = dir.path().join("source.pptx");
    std::fs::write(&input, bytes).map_err(|e| e.to_string())?;
    // A separate profile prevents handing the document to an already running user session.
    let profile = url::Url::from_directory_path(dir.path().join("profile"))
        .map_err(|_| "Invalid converter profile path")?;
    let mut cmd = std::process::Command::new(converter(
        "CANVAS_PILOT_LIBREOFFICE",
        if cfg!(windows) {
            "soffice.exe"
        } else {
            "libreoffice"
        },
    ));
    cmd.arg(format!("-env:UserInstallation={profile}"))
        .args([
            "--headless",
            "--convert-to",
            "pdf:impress_pdf_Export",
            "--outdir",
        ])
        .arg(dir.path())
        .arg(input);
    run_converter(&mut cmd, "LibreOffice", "CANVAS_PILOT_LIBREOFFICE")?;
    read(&dir.path().join("source.pdf"))
}

fn drawio_pdf(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let input = dir.path().join("source.drawio");
    let output = dir.path().join("source.pdf");
    std::fs::write(&input, bytes).map_err(|e| e.to_string())?;
    let mut cmd = std::process::Command::new(converter(
        "CANVAS_PILOT_DRAWIO",
        if cfg!(windows) {
            "draw.io.exe"
        } else {
            "drawio"
        },
    ));
    cmd.args(["--export", "--format", "pdf", "--all-pages", "--output"])
        .arg(&output)
        .arg(input);
    run_converter(&mut cmd, "draw.io Desktop", "CANVAS_PILOT_DRAWIO")?;
    read(&output)
}

fn run_converter(cmd: &mut std::process::Command, label: &str, env: &str) -> Result<(), String> {
    use std::{
        process::Stdio,
        time::{Duration, Instant},
    };
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
        .map_err(|e| format!("Cannot start {label}: {e}. Install it or set {env} to its executable. You can also select Layout boxes or import an exported PDF/SVG."))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                return Err(format!(
                    "{label} conversion failed ({status}). Export PDF/SVG in the source application, or use Layout boxes."
                ));
            }
            Ok(None) if start.elapsed() < Duration::from_secs(60) => {
                std::thread::sleep(Duration::from_millis(50))
            }
            result => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{label} did not finish within 60 seconds: {result:?}"
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PDF: &[u8] = include_bytes!("../tests/fixtures/reference-pages.pdf");

    #[test]
    fn pdf_page_selection_crop_and_rotation_render_distinct_images() {
        let document = Document::open(PDF.to_vec(), "client.PDF", false).unwrap();
        assert_eq!(document.pages, ["Page 1", "Page 2"]);
        let red = image::load_from_memory(&document.render(0, 200).unwrap())
            .unwrap()
            .into_rgba8();
        assert_eq!(red.dimensions(), (200, 100));
        assert_eq!(red.get_pixel(50, 50).0, [255, 0, 0, 255]);
        let green = image::load_from_memory(&document.render(1, 200).unwrap())
            .unwrap()
            .into_rgba8();
        assert_eq!(green.dimensions(), (200, 200));
        assert_eq!(green.get_pixel(50, 50).0, [0, 255, 0, 255]);
        assert!(document.render(2, 200).is_err());
        assert!(document.render(0, 8192).is_err());
    }

    #[test]
    fn svg_artwork_keeps_color_transparency_and_does_not_fetch_linked_images() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect width="100" height="100" fill="red"/><image href="file:///not-a-reference.png" x="100" width="100" height="100"/></svg>"#;
        let doc = Document::open(svg.to_vec(), "art.svg", false).unwrap();
        let image = image::load_from_memory(&doc.render(0, 200).unwrap())
            .unwrap()
            .into_rgba8();
        assert_eq!(image.dimensions(), (200, 100));
        assert_eq!(image.get_pixel(10, 10).0, [255, 0, 0, 255]);
        assert_eq!(image.get_pixel(150, 50).0[3], 0);
        assert!(Document::open(b"broken".to_vec(), "bad.pdf", false).is_err());
        assert!(
            Document::open(b"broken".to_vec(), "bad.svg", false)
                .unwrap()
                .render(0, 200)
                .is_err()
        );
        assert!(render_scale(f32::INFINITY, 1.0, 200).is_err());
        assert!(render_scale(0.0, 1.0, 200).is_err());
        assert!(render_scale(1e20, 1.0, 200).is_err());
    }

    #[test]
    fn supported_exchange_documents_offer_explicit_layout_pictures() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/templates/canvas-pilot");
        for codec in crate::interchange::Codec::available() {
            if codec == crate::interchange::Codec::Illustrator {
                continue;
            }
            let path = root.join(format!("canvas-pilot-template.{}", codec.extension()));
            // The catalog owns the canonical starter basename.
            let path = if path.exists() {
                path
            } else {
                std::fs::read_dir(&root)
                    .unwrap()
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .find(|p| p.extension().and_then(|e| e.to_str()) == Some(codec.extension()))
                    .unwrap()
            };
            let doc = Document::open(
                read(&path).unwrap(),
                path.file_name().unwrap().to_str().unwrap(),
                true,
            )
            .unwrap();
            assert!(doc.note.contains("Layout boxes only"), "{}", codec.label());
            assert!(!doc.render(0, 320).unwrap().is_empty(), "{}", codec.label());
        }
        assert!(is_document(Path::new("drawing.PDF")));
        assert!(!is_document(Path::new("picture.PNG")));
        assert!(!is_document(Path::new("unknown.xyz")));
    }

    #[test]
    fn json_and_html_artwork_preserve_embedded_images_and_canvas() {
        use crate::exchange::{ExportShape, Format, encode_shapes};
        use base64::Engine;
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let shapes = [ExportShape {
            id: "red".into(),
            name: "Picture".into(),
            geometry: crate::Geometry {
                x: 0.,
                y: 0.,
                width: 200.,
                height: 100.,
            },
            image_png_base64: Some(
                base64::engine::general_purpose::STANDARD.encode(png.into_inner()),
            ),
            background: true,
        }];
        for format in [Format::Json, Format::Html] {
            let bytes = encode_shapes(format, [200, 100], &shapes)
                .unwrap()
                .into_bytes();
            let doc =
                Document::open(bytes, &format!("client.{}", format.extension()), false).unwrap();
            let image = image::load_from_memory(&doc.render(0, 200).unwrap())
                .unwrap()
                .into_rgba8();
            assert_eq!(image.dimensions(), (200, 100));
            assert_eq!(image.get_pixel(50, 50).0, [255, 0, 0, 255]);
        }
    }

    #[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
    #[test]
    fn figma_artwork_is_rendered_with_embedded_pictures() {
        use crate::exchange::ExportShape;
        use base64::Engine;
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let shapes = [ExportShape {
            id: "red".into(),
            name: "Picture".into(),
            geometry: crate::Geometry {
                x: 0.,
                y: 0.,
                width: 200.,
                height: 100.,
            },
            image_png_base64: Some(
                base64::engine::general_purpose::STANDARD.encode(png.into_inner()),
            ),
            background: false,
        }];
        for codec in [
            crate::interchange::Codec::Figma,
            crate::interchange::Codec::Figcraft,
        ] {
            let bytes = crate::vendor_formats::export(codec, [200, 100], &shapes).unwrap();
            let doc = Document::open(bytes, &format!("art.{}", codec.extension()), false).unwrap();
            let image = image::load_from_memory(&doc.render(0, 200).unwrap())
                .unwrap()
                .into_rgba8();
            assert_eq!(image.get_pixel(50, 50).0, [255, 0, 0, 255]);
        }
    }
}
