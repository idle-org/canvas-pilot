//! One byte-oriented entry point and capability registry for layer exchange.
use crate::{
    exchange::{self, Format, Options},
    import::{self, ImportDocument},
    project::Project,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Codec {
    #[default]
    Svg,
    Drawio,
    Json,
    Csv,
    Dxf,
    Dwg,
    Html,
    Pptx,
    Figma,
    Figcraft,
    Illustrator,
}

impl Codec {
    pub fn available() -> Vec<Self> {
        let mut formats = vec![
            Self::Svg,
            Self::Drawio,
            Self::Json,
            Self::Csv,
            Self::Dxf,
            Self::Html,
        ];
        if cfg!(all(
            feature = "vendor_formats",
            not(feature = "no_proprietary")
        )) {
            formats.extend([
                Self::Dwg,
                Self::Pptx,
                Self::Figma,
                Self::Figcraft,
                Self::Illustrator,
            ]);
        }
        formats
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Svg => "SVG · vector editors",
            Self::Drawio => "draw.io",
            Self::Json => "Geometry JSON",
            Self::Csv => "Geometry CSV",
            Self::Dxf => "DXF · CAD",
            Self::Dwg => "DWG · CAD (Craft)",
            Self::Html => "HTML / CSS",
            Self::Pptx => "PowerPoint · PPTX",
            Self::Figma => "Figma · FIG",
            Self::Figcraft => "FigCraft · FIGCRAFT",
            Self::Illustrator => "Illustrator (VectorCraft)",
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Drawio => "drawio",
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Dxf => "dxf",
            Self::Dwg => "dwg",
            Self::Html => "html",
            Self::Pptx => "pptx",
            Self::Figma => "fig",
            Self::Figcraft => "figcraft",
            Self::Illustrator => "ai",
        }
    }
    pub fn artwork(self) -> bool {
        !matches!(self, Self::Csv | Self::Dxf | Self::Dwg)
    }
    pub fn help(self) -> &'static str {
        match self {
            Self::Dwg => {
                "Editable CAD rectangles via Craft. Unitless drawing: one unit equals one screen pixel; no pictures."
            }
            Self::Pptx => {
                "One editable slide. Coordinates use 96 pixels per inch. Import lets you choose a slide."
            }
            Self::Figcraft => "Editable FigCraft document. Import lets you choose a page or frame.",
            Self::Figma => {
                "Editable Figma frame. Import lets you choose a page or frame. Uses FigCraft's best-effort native codec."
            }
            Self::Illustrator => {
                "Requires vectorcraft-cli beside this app, in bin/, or on PATH (or MIDRA_VECTORCRAFT_CLI). Converts through SVG locally."
            }
            _ => {
                "Layer boxes in screen pixels. Save a Canvas Pilot project to retain source settings, crop and device links."
            }
        }
    }
    fn text(self) -> Option<Format> {
        Some(match self {
            Self::Svg => Format::Svg,
            Self::Drawio => Format::Drawio,
            Self::Json => Format::Json,
            Self::Csv => Format::Csv,
            Self::Dxf => Format::Dxf,
            Self::Html => Format::Html,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Page {
    pub name: String,
    pub document: ImportDocument,
}

pub fn extensions() -> Vec<&'static str> {
    let mut extensions: Vec<_> = Codec::available().iter().map(|c| c.extension()).collect();
    extensions.push("xml");
    extensions
}

pub fn export(
    project: &Project,
    ids: &[usize],
    codec: Codec,
    options: Options,
) -> Result<Vec<u8>, String> {
    if !Codec::available().contains(&codec) {
        return Err(
            "This build excludes vendor formats; rebuild with vendor_formats enabled.".into(),
        );
    }
    if options.extra_parameters && !crate::exchange_metadata::supported(codec) {
        return Err("Extra parameters are not supported by this format’s current codec.".into());
    }
    let shapes = exchange::prepare_shapes(project, ids, options, codec.artwork())?;
    let canvas = [
        project.output_resolution.width,
        project.output_resolution.height,
    ];
    let bytes = if let Some(format) = codec.text() {
        exchange::encode_shapes(format, canvas, &shapes)?.into_bytes()
    } else {
        #[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
        {
            crate::vendor_formats::export(codec, canvas, &shapes)?
        }
        #[cfg(any(not(feature = "vendor_formats"), feature = "no_proprietary"))]
        {
            return Err("Vendor formats are unavailable in this build.".into());
        }
    };
    let bytes = if options.extra_parameters {
        crate::exchange_metadata::embed(bytes, codec, project, ids)?
    } else {
        bytes
    };
    if bytes.len() > exchange::MAX_EXCHANGE_BYTES {
        return Err("Export exceeds 16 MiB; disable artwork or export fewer layers.".into());
    }
    Ok(bytes)
}

pub fn import(bytes: &[u8], filename: &str) -> Result<Vec<Page>, String> {
    if bytes.len() > exchange::MAX_EXCHANGE_BYTES {
        return Err("Import exceeds 16 MiB.".into());
    }
    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let vendor = matches!(
        extension.as_str(),
        "pptx" | "fig" | "figcraft" | "ai" | "dwg"
    ) || bytes.starts_with(b"PK")
        || bytes.starts_with(b"fig-")
        || bytes.starts_with(b"AC10");
    if vendor {
        if cfg!(feature = "no_proprietary") {
            return Err("Vendor format support is disabled in this no_proprietary build.".into());
        }
        #[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
        {
            return crate::vendor_formats::import(bytes, &extension, filename);
        }
        #[cfg(any(not(feature = "vendor_formats"), feature = "no_proprietary"))]
        {
            return Err("Vendor format support is not compiled in. Enable vendor_formats.".into());
        }
    }
    if extension == "dxf" || bytes.starts_with(b"AutoCAD Binary DXF") {
        return Ok(vec![Page {
            name: filename.into(),
            document: crate::geometry_formats::parse_dxf_bytes(bytes)?,
        }]);
    }
    let text = std::str::from_utf8(bytes).map_err(
        |_| "Unrecognized binary format. Choose PPTX, FIG, FIGCRAFT, Illustrator or a supported geometry file.",
    )?;
    Ok(vec![Page {
        name: filename.into(),
        document: import::parse_values(text)?,
    }])
}

/// Explicit recovery of extra parameters; the artwork path never becomes the save path.
pub fn read_embedded_project(path: &std::path::Path) -> Result<Project, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(exchange::MAX_EXCHANGE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let codec = Codec::available()
        .into_iter()
        .find(|c| c.extension() == ext || (ext == "xml" && *c == Codec::Drawio))
        .ok_or("Unsupported extra-parameter format")?;
    crate::exchange_metadata::extract(&bytes, codec)?.ok_or(
        "No supported Canvas Pilot parameters found. Use geometry import for this file.".into(),
    )
}

/// The boolean distinguishes a native project path from a recovered export snapshot.
pub fn read_project_or_parameters(path: &std::path::Path) -> Result<(Project, bool), String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(crate::project::MAX_PROJECT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > crate::project::MAX_PROJECT_BYTES {
        return Err("Project exceeds 96 MiB.".into());
    }
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && value["format"] == crate::project::FORMAT
    {
        return Project::from_json(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)
            .map(|p| (p, true));
    }
    read_embedded_project(path).map(|p| (p, false))
}
