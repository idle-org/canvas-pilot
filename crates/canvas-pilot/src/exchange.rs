//! Versioned geometry interchange with self-contained optional artwork.
use crate::{
    Geometry, export_pixels,
    project::{Layer, Project},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{Rgba, RgbaImage};
use std::io::Cursor;

pub const MAX_EXCHANGE_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    Svg,
    Drawio,
    Json,
    Csv,
    Dxf,
    Html,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    pub extra_parameters: bool,
    pub background: bool,
    pub placeholders: bool,
}
impl Format {
    pub const ALL: [Self; 6] = [
        Self::Svg,
        Self::Drawio,
        Self::Json,
        Self::Csv,
        Self::Dxf,
        Self::Html,
    ];
    pub fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Drawio => "drawio",
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Dxf => "dxf",
            Self::Html => "html",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Svg => "SVG",
            Self::Drawio => "draw.io",
            Self::Json => "Geometry JSON",
            Self::Csv => "Geometry CSV",
            Self::Dxf => "DXF",
            Self::Html => "HTML / CSS",
        }
    }
    pub fn supports_placeholders(self) -> bool {
        matches!(self, Self::Svg | Self::Drawio | Self::Json | Self::Html)
    }
}

#[derive(Clone)]
pub(crate) struct ExportShape {
    pub id: String,
    pub name: String,
    pub geometry: Geometry,
    pub image_png_base64: Option<String>,
    pub background: bool,
}
pub(crate) fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn png(image: RgbaImage) -> Result<String, String> {
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(STANDARD.encode(bytes.into_inner()))
}
fn thumbnail_size(w: f64, h: f64, max: f64) -> [u32; 2] {
    let scale = (max / w.max(h)).min(1.0);
    [
        (w * scale).round().max(1.0) as u32,
        (h * scale).round().max(1.0) as u32,
    ]
}
fn placeholder(project: &Project, layer: &Layer) -> Result<String, String> {
    let source = project
        .inputs
        .iter()
        .find(|i| i.id == layer.source_id)
        .ok_or("Missing source")?;
    let native = [
        source.native_resolution.width as f64,
        source.native_resolution.height as f64,
    ];
    let input = native;
    let remaining = layer.advanced_crop.remaining(input)?;
    let fitted = if let Some(mode) = layer.layer_fit {
        mode.image(remaining, layer.bounds)?
    } else {
        layer.bounds
    };
    let masked = layer.mask.bounds(layer.mask.reference(
        input,
        layer.advanced_crop,
        layer.layer_fit,
        layer.bounds,
    )?);
    let [w, h] = thumbnail_size(layer.bounds.width, layer.bounds.height, 512.0);
    let image = RgbaImage::from_fn(w, h, |x, y| {
        let ox = layer.bounds.x + (x as f64 + 0.5) / w as f64 * layer.bounds.width;
        let oy = layer.bounds.y + (y as f64 + 0.5) / h as f64 * layer.bounds.height;
        if ox < masked.x
            || ox >= masked.x + masked.width
            || oy < masked.y
            || oy >= masked.y + masked.height
            || ox < fitted.x
            || ox >= fitted.x + fitted.width
            || oy < fitted.y
            || oy >= fitted.y + fitted.height
        {
            return Rgba([0, 0, 0, 0]);
        }

        let sx = layer.advanced_crop.left + (ox - fitted.x) / fitted.width * remaining[0];
        let sy = layer.advanced_crop.top + (oy - fitted.y) / fitted.height * remaining[1];
        let gx = sx / native[0] * 10.0;
        let gy = sy / native[1] * 6.0;
        if gx.fract() < 0.025 || gy.fract() < 0.025 {
            Rgba([210, 221, 230, 255])
        } else if (gx as u32 + gy as u32).is_multiple_of(2) {
            Rgba([35, 69, 87, 255])
        } else {
            Rgba([23, 47, 63, 255])
        }
    });
    png(image)
}
fn background(project: &Project) -> Result<String, String> {
    let ow = project.output_resolution.width as f64;
    let oh = project.output_resolution.height as f64;
    let [w, h] = thumbnail_size(ow, oh, 1920.0);
    let mut canvas = RgbaImage::from_pixel(w, h, Rgba([18, 37, 49, 255]));
    for reference in project.references.iter().filter(|r| r.visible) {
        let bytes = reference.bytes()?;
        let mut reader = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|e| e.to_string())?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(256 * 1024 * 1024);
        reader.limits(limits);
        let image = reader
            .decode()
            .map_err(|e| format!("Cannot decode reference: {e}"))?
            .to_rgba8();
        let iw = image.width() as f64;
        let ih = image.height() as f64;
        let (rw, rh, x, y) = if reference.native_size {
            (iw, ih, 0.0, 0.0)
        } else if reference.preserve_ratio {
            let k = (ow / iw).min(oh / ih);
            (iw * k, ih * k, (ow - iw * k) / 2.0, (oh - ih * k) / 2.0)
        } else {
            (ow, oh, 0.0, 0.0)
        };
        for (px, py, out) in canvas.enumerate_pixels_mut() {
            let sx = ((px as f64 + 0.5) * ow / w as f64 - x - reference.offset[0]) / rw * iw;
            let sy = ((py as f64 + 0.5) * oh / h as f64 - y - reference.offset[1]) / rh * ih;
            if sx >= 0.0 && sy >= 0.0 && sx < iw && sy < ih {
                let source = image.get_pixel(sx as u32, sy as u32);
                let alpha = source[3] as f32 / 255.0 * reference.opacity;
                for c in 0..3 {
                    out[c] =
                        (source[c] as f32 * alpha + out[c] as f32 * (1.0 - alpha)).round() as u8;
                }
            }
        }
    }
    png(canvas)
}
/// Export explicit stable IDs, in document order; coordinates stay on the output canvas.
pub fn export(
    project: &Project,
    ids: &[usize],
    format: Format,
    options: Options,
) -> Result<String, String> {
    let shapes = prepare_shapes(project, ids, options, format.supports_placeholders())?;
    let w = project.output_resolution.width;
    let h = project.output_resolution.height;
    let text = encode_shapes(format, [w, h], &shapes)?;
    if options.extra_parameters {
        let codec = match format {
            Format::Svg => crate::interchange::Codec::Svg,
            Format::Drawio => crate::interchange::Codec::Drawio,
            Format::Json => crate::interchange::Codec::Json,
            Format::Csv => crate::interchange::Codec::Csv,
            Format::Dxf => crate::interchange::Codec::Dxf,
            Format::Html => crate::interchange::Codec::Html,
        };
        let bytes = crate::exchange_metadata::embed(text.into_bytes(), codec, project, ids)?;
        if bytes.len() > MAX_EXCHANGE_BYTES {
            return Err("Export exceeds 16 MiB.".into());
        }
        String::from_utf8(bytes).map_err(|e| e.to_string())
    } else {
        Ok(text)
    }
}

pub(crate) fn prepare_shapes(
    project: &Project,
    ids: &[usize],
    options: Options,
    artwork: bool,
) -> Result<Vec<ExportShape>, String> {
    project.layout()?;
    if ids.is_empty()
        || ids
            .iter()
            .enumerate()
            .any(|(i, id)| ids[..i].contains(id) || !project.layers.iter().any(|l| l.id == *id))
    {
        return Err("Choose distinct existing layer IDs to export.".into());
    }
    let w = project.output_resolution.width;
    let h = project.output_resolution.height;
    if options.placeholders && !artwork {
        return Err("This format cannot embed placeholder pictures.".into());
    }
    let mut shapes = Vec::new();
    if options.background {
        let image_png_base64 = artwork.then(|| background(project)).transpose()?;
        shapes.push(ExportShape {
            id: "midra-background".into(),
            name: "Background".into(),
            geometry: Geometry {
                x: 0.0,
                y: 0.0,
                width: w as f64,
                height: h as f64,
            },
            image_png_base64,
            background: true,
        });
    }
    for layer in project.layers.iter().filter(|l| ids.contains(&l.id)) {
        let g = layer.bounds;
        if g.width.round() < 1.0 || g.height.round() < 1.0 {
            return Err("A layer rounds to zero pixels; enlarge it before export.".into());
        }
        let image_png_base64 = options
            .placeholders
            .then(|| placeholder(project, layer))
            .transpose()?;
        shapes.push(ExportShape {
            id: format!("layer-{}", layer.id),
            name: layer.name.clone(),
            geometry: g,
            image_png_base64,
            background: false,
        });
    }
    Ok(shapes)
}

pub(crate) fn encode_shapes(
    format: Format,
    [w, h]: [u32; 2],
    shapes: &[ExportShape],
) -> Result<String, String> {
    if matches!(
        format,
        Format::Json | Format::Csv | Format::Dxf | Format::Html
    ) {
        return crate::geometry_formats::export(format, [w, h], shapes);
    }
    let mut out = match format {
        Format::Svg => format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" data-midra-layout-geometry=\"2\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n<title>Canvas Pilot geometry</title>\n"
        ),
        Format::Drawio => format!(
            "<mxfile host=\"Canvas Pilot\" compressed=\"false\"><diagram id=\"canvas-pilot\" name=\"Layout\"><mxGraphModel page=\"1\" pageScale=\"1\" pageWidth=\"{w}\" pageHeight=\"{h}\" grid=\"0\" adaptiveColors=\"none\" data-midra-layout-geometry=\"1\"><root><mxCell id=\"0\"/><mxCell id=\"1\" parent=\"0\"/>\n"
        ),
        _ => unreachable!(),
    };
    for shape in shapes {
        element(
            &mut out,
            format,
            &shape.id,
            &shape.name,
            shape.geometry,
            shape.image_png_base64.as_deref(),
            shape.background,
        );
    }
    out.push_str(match format {
        Format::Svg => "</svg>\n",
        Format::Drawio => "</root></mxGraphModel></diagram></mxfile>\n",
        _ => unreachable!(),
    });
    if out.len() > MAX_EXCHANGE_BYTES {
        return Err(
            "Export exceeds 16 MiB; disable placeholder pictures or the background.".into(),
        );
    }
    Ok(out)
}
fn element(
    out: &mut String,
    format: Format,
    id: &str,
    name: &str,
    g: Geometry,
    image: Option<&str>,
    background: bool,
) {
    let [x, y, w, h] = [g.x, g.y, g.width, g.height].map(export_pixels);
    let role = if background { "background" } else { "layer" };
    let name = if background {
        String::new()
    } else {
        escape(name)
    };
    match format {
        Format::Svg => {
            let attributes = format!(
                "id=\"{id}\" data-midra-role=\"{role}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\""
            );
            if let Some(image) = image {
                out.push_str(&format!("<image {attributes} preserveAspectRatio=\"none\" xlink:href=\"data:image/png;base64,{image}\"><title>{name}</title></image>\n"));
            } else {
                out.push_str(&format!(
                    "<rect {attributes} fill=\"#173b50\"><title>{name}</title></rect>\n"
                ));
            }
        }
        Format::Drawio => {
            // mxGraph stores image data without the ';base64' delimiter inside styles.
            let image = image
                .map(|data| format!("shape=image;imageAspect=0;image=data:image/png,{data};"))
                .unwrap_or_default();
            out.push_str(&format!("<mxCell id=\"{id}\" value=\"{name}\" data-midra-role=\"{role}\" vertex=\"1\" parent=\"1\" style=\"rounded=0;whiteSpace=wrap;html=0;strokeColor=none;fillColor=#173b50;fontColor=#ffffff;{image}\"><mxGeometry x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" as=\"geometry\"/></mxCell>\n"));
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod fit_tests {
    use super::*;
    use crate::{
        Layout,
        formats::{Crop, LayerFit},
    };

    #[test]
    fn source_mask_artwork_uses_source_edges_before_layer_crop() {
        let mut layout = crate::Layout::default();
        layout
            .set_framing(
                0,
                crate::formats::Framing {
                    zoom: 2.0,
                    offset: [0.0; 2],
                },
            )
            .unwrap();
        let render = |layout: &crate::Layout| {
            let p = Project::from_layout(layout);
            placeholder(&p, &p.layers[0]).unwrap()
        };
        let unmasked = render(&layout);
        layout
            .set_mask(
                0,
                crate::formats::Mask {
                    origin: crate::formats::MaskOrigin::Source,
                    left: 0.2,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            render(&layout),
            unmasked,
            "mask before the crop has no visible effect"
        );
        layout
            .set_mask(
                0,
                crate::formats::Mask {
                    origin: crate::formats::MaskOrigin::Source,
                    left: 0.375,
                    ..Default::default()
                },
            )
            .unwrap();
        let source_masked = render(&layout);
        layout
            .set_mask(
                0,
                crate::formats::Mask {
                    left: 0.25,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(render(&layout), source_masked);
        assert_ne!(source_masked, unmasked);
    }

    #[test]
    fn mask_artwork_blanks_without_resampling_and_embedded_metadata_preserves_it() {
        let mut layout = Layout::default();
        for fit in [
            None,
            Some(LayerFit::OneToOne),
            Some(LayerFit::Centered),
            Some(LayerFit::Cropped),
            Some(LayerFit::Fullscreen),
        ] {
            layout.set_layer_fit(0, fit, Crop::default()).unwrap();
            layout.set_mask(0, Default::default()).unwrap();
            let p = Project::from_layout(&layout);
            let original = image::load_from_memory(
                &STANDARD
                    .decode(placeholder(&p, &p.layers[0]).unwrap())
                    .unwrap(),
            )
            .unwrap()
            .to_rgba8();
            layout
                .set_mask(
                    0,
                    crate::formats::Mask {
                        origin: Default::default(),
                        left: 0.25,
                        right: 0.25,
                        top: 0.25,
                        bottom: 0.25,
                    },
                )
                .unwrap();
            let p = Project::from_layout(&layout);
            let masked = image::load_from_memory(
                &STANDARD
                    .decode(placeholder(&p, &p.layers[0]).unwrap())
                    .unwrap(),
            )
            .unwrap()
            .to_rgba8();
            for (x, y, pixel) in masked.enumerate_pixels() {
                let fx = (x as f64 + 0.5) / masked.width() as f64;
                let fy = (y as f64 + 0.5) / masked.height() as f64;
                if (0.25..0.75).contains(&fx) && (0.25..0.75).contains(&fy) {
                    assert_eq!(pixel, original.get_pixel(x, y));
                } else {
                    assert_eq!(pixel[3], 0);
                }
            }
            let snapshot = crate::exchange_metadata::snapshot(&p, &[p.layers[0].id]).unwrap();
            assert_eq!(snapshot.layers[0].mask, p.layers[0].mask);
        }
    }
    #[test]
    fn exported_fit_artwork_has_transparent_margins_only_when_expected() {
        let mut layout = Layout::default();
        layout
            .apply(
                0,
                Geometry {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 600.0,
                },
            )
            .unwrap();
        for mode in LayerFit::ALL {
            layout
                .set_layer_fit(0, Some(mode), Crop::default())
                .unwrap();
            let project = Project::from_layout(&layout);
            let encoded = placeholder(&project, &project.layers[0]).unwrap();
            let image = image::load_from_memory(&STANDARD.decode(encoded).unwrap())
                .unwrap()
                .to_rgba8();
            assert_eq!(
                image.get_pixel(image.width() / 2, 0)[3],
                if mode == LayerFit::Centered { 0 } else { 255 }
            );
            assert_eq!(
                image.get_pixel(image.width() / 2, image.height() / 2)[3],
                255
            );
            let snapshot =
                crate::exchange_metadata::snapshot(&project, &[project.layers[0].id]).unwrap();
            assert_eq!(snapshot.layers[0].layer_fit, Some(mode));
            assert_eq!(
                Project::from_json(&snapshot.to_json().unwrap()).unwrap(),
                snapshot
            );
        }
    }
}
