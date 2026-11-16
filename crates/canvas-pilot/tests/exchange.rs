use base64::{Engine, engine::general_purpose::STANDARD};
use canvas_pilot::{
    Geometry, Layout,
    exchange::{self, Format, Options},
    import::parse_values,
    project::{Project, Resolution},
};
use std::io::Write;

fn fixture() -> Project {
    let mut layout = Layout::default();
    layout
        .apply(
            0,
            Geometry {
                x: -12.5,
                y: 271.4,
                width: 945.2,
                height: 531.58,
            },
        )
        .unwrap();
    layout
        .rename_layer(0, "Camera & <left> \"wide\"", "")
        .unwrap();
    Project::from_layout(&layout)
}

fn template(resolution: [u32; 2]) -> Project {
    let mut project = Project::from_layout(&Layout::default());
    let scale = resolution[0] as f64 / 1920.0;
    project.output_resolution = Resolution::from(resolution);
    for layer in &mut project.layers {
        layer.bounds.x *= scale;
        layer.bounds.y *= scale;
        layer.bounds.width *= scale;
        layer.bounds.height *= scale;
    }
    project
}

#[test]
fn readme_templates_match_the_exporter_and_contain_two_layers() {
    for (resolution, documented) in [
        (
            [1920, 1080],
            [
                (
                    Format::Svg,
                    include_str!(
                        "../../../docs/templates/canvas-pilot/canvas-pilot-1080p-2-layers.svg"
                    ),
                ),
                (
                    Format::Drawio,
                    include_str!(
                        "../../../docs/templates/canvas-pilot/canvas-pilot-1080p-2-layers.drawio"
                    ),
                ),
                (
                    Format::Json,
                    include_str!(
                        "../../../docs/templates/canvas-pilot/canvas-pilot-1080p-2-layers.json"
                    ),
                ),
                (
                    Format::Csv,
                    include_str!(
                        "../../../docs/templates/canvas-pilot/canvas-pilot-1080p-2-layers.csv"
                    ),
                ),
                (
                    Format::Dxf,
                    include_str!(
                        "../../../docs/templates/canvas-pilot/canvas-pilot-1080p-2-layers.dxf"
                    ),
                ),
                (
                    Format::Html,
                    include_str!(
                        "../../../docs/templates/canvas-pilot/canvas-pilot-1080p-2-layers.html"
                    ),
                ),
            ],
        ),
        (
            [3840, 2160],
            [
                (
                    Format::Svg,
                    include_str!(
                        "../../../docs/templates/canvas-pilot/canvas-pilot-4k-2-layers.svg"
                    ),
                ),
                (
                    Format::Drawio,
                    include_str!(
                        "../../../docs/templates/canvas-pilot/canvas-pilot-4k-2-layers.drawio"
                    ),
                ),
                (
                    Format::Json,
                    include_str!(
                        "../../../docs/templates/canvas-pilot/canvas-pilot-4k-2-layers.json"
                    ),
                ),
                (
                    Format::Csv,
                    include_str!(
                        "../../../docs/templates/canvas-pilot/canvas-pilot-4k-2-layers.csv"
                    ),
                ),
                (
                    Format::Dxf,
                    include_str!(
                        "../../../docs/templates/canvas-pilot/canvas-pilot-4k-2-layers.dxf"
                    ),
                ),
                (
                    Format::Html,
                    include_str!(
                        "../../../docs/templates/canvas-pilot/canvas-pilot-4k-2-layers.html"
                    ),
                ),
            ],
        ),
    ] {
        let project = template(resolution);
        let ids = project
            .layers
            .iter()
            .map(|layer| layer.id)
            .collect::<Vec<_>>();
        for (format, documented) in documented {
            assert_eq!(
                exchange::export(&project, &ids, format, Options::default()).unwrap(),
                documented
            );
            let imported = parse_values(documented).unwrap();
            assert_eq!(imported.shapes.len(), 2);
            assert!(imported.shapes.iter().all(|shape| !shape.background_only));
            assert_eq!(
                imported
                    .shapes
                    .iter()
                    .map(|shape| shape.geometry)
                    .collect::<Vec<_>>(),
                project
                    .layers
                    .iter()
                    .map(|layer| layer.bounds)
                    .collect::<Vec<_>>()
            );
        }
    }
}
#[test]
fn every_format_and_artwork_option_roundtrips_geometry_and_order() {
    let project = fixture();
    let ids: Vec<_> = project.layers.iter().map(|l| l.id).collect();
    for format in Format::ALL {
        for background in [false, true] {
            for placeholders in [false, true] {
                if placeholders && !format.supports_placeholders() {
                    continue;
                }
                let text = exchange::export(
                    &project,
                    &ids,
                    format,
                    Options {
                        extra_parameters: false,
                        background,
                        placeholders,
                    },
                )
                .unwrap();
                if format.supports_placeholders() && (background || placeholders) {
                    assert!(text.contains("iVBOR"), "missing embedded PNG in {format:?}");
                }
                let doc = parse_values(&text).unwrap();
                assert_eq!(
                    doc.shapes.iter().filter(|s| s.background_only).count(),
                    usize::from(background)
                );
                let shapes: Vec<_> = doc.shapes.iter().filter(|s| !s.background_only).collect();
                assert_eq!(shapes.len(), ids.len());
                let mut imported = Layout::default();
                let mappings: Vec<_> = shapes
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        let g = project.layers[i].bounds;
                        let expected = Geometry {
                            x: g.x.round(),
                            y: g.y.round(),
                            width: g.width.round(),
                            height: g.height.round(),
                        };
                        assert_eq!(s.geometry, expected);
                        assert_eq!(s.outline, [0.0, 0.0]);
                        (Some(imported.layer_info(i).id), s.geometry, None)
                    })
                    .collect();
                imported.import_device_layers(&mappings).unwrap();
                let reexport = exchange::export(
                    &Project::from_layout(&imported),
                    &ids,
                    format,
                    Options {
                        extra_parameters: false,
                        background,
                        placeholders,
                    },
                )
                .unwrap();
                let reread = parse_values(&reexport).unwrap();
                assert_eq!(
                    reread
                        .shapes
                        .iter()
                        .filter(|s| !s.background_only)
                        .map(|s| s.geometry)
                        .collect::<Vec<_>>(),
                    shapes.iter().map(|s| s.geometry).collect::<Vec<_>>()
                );
            }
        }
    }
}
#[test]
fn single_full_screen_layer_is_never_mistaken_for_background() {
    let mut p = Project::from_layout(&Layout::default());
    p.layers[0].bounds = Geometry {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };
    for format in Format::ALL {
        for placeholders in [false, true] {
            if placeholders && !format.supports_placeholders() {
                continue;
            }
            let text = exchange::export(
                &p,
                &[p.layers[0].id],
                format,
                Options {
                    extra_parameters: false,
                    background: true,
                    placeholders,
                },
            )
            .unwrap();
            let doc = parse_values(&text).unwrap();
            assert_eq!(doc.shapes.len(), 2);
            assert!(doc.shapes[0].background_only);
            assert!(!doc.shapes[1].background);
            assert!(!doc.shapes[1].background_only);
        }
    }
}
#[test]
fn drawio_compressed_resave_and_percent_encoded_clipboard_are_supported() {
    let p = fixture();
    let xml = exchange::export(
        &p,
        &[p.layers[0].id],
        Format::Drawio,
        Options {
            extra_parameters: false,
            background: true,
            placeholders: true,
        },
    )
    .unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let model = doc
        .descendants()
        .find(|n| n.has_tag_name("mxGraphModel"))
        .unwrap();
    let model = &xml[model.range()];
    let encoded = model
        .bytes()
        .map(|b| format!("%{b:02X}"))
        .collect::<String>();
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(encoded.as_bytes()).unwrap();
    let compressed = STANDARD.encode(encoder.finish().unwrap());
    let wrapped = format!("<mxfile><diagram>{compressed}</diagram></mxfile>");
    for text in [&xml, &wrapped, &encoded] {
        let shapes = parse_values(text).unwrap().shapes;
        assert_eq!(shapes.len(), 2);
        assert!(shapes[0].background_only);
        assert_eq!(shapes[1].geometry.x, -13.0);
    }
}
#[test]
fn reference_image_is_embedded_and_missing_ids_or_invalid_versions_fail() {
    let mut p = fixture();
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    p.references.push(canvas_pilot::project::Reference {
        name: "red".into(),
        image_base64: STANDARD.encode(png.into_inner()),
        visible: true,
        opacity: 1.0,
        preserve_ratio: false,
        native_size: false,
        offset: [0.0, 0.0],
    });
    let xml = exchange::export(
        &p,
        &[p.layers[0].id],
        Format::Svg,
        Options {
            extra_parameters: false,
            background: true,
            placeholders: false,
        },
    )
    .unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let data = doc
        .descendants()
        .find(|n| n.has_tag_name("image"))
        .unwrap()
        .attribute(("http://www.w3.org/1999/xlink", "href"))
        .unwrap();
    let bytes = STANDARD
        .decode(data.strip_prefix("data:image/png;base64,").unwrap())
        .unwrap();
    let image = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(*image.get_pixel(0, 0), image::Rgba([255, 0, 0, 255]));
    assert!(parse_values(&xml.replace("geometry=\"2\"", "geometry=\"99\"")).is_err());
    assert!(exchange::export(&p, &[], Format::Svg, Options::default()).is_err());
    assert!(exchange::export(&p, &[999], Format::Drawio, Options::default()).is_err());
    assert!(exchange::export(&p, &[1, 1], Format::Svg, Options::default()).is_err());
    assert!(parse_values("<mxfile><diagram/><diagram/></mxfile>").is_err());
    assert!(parse_values("<mxfile><diagram>garbage</diagram></mxfile>").is_err());
}

#[test]
fn open_text_adapters_reject_wrong_versions_canvases_and_non_rectangles() {
    let project = template([1920, 1080]);
    let ids = project
        .layers
        .iter()
        .map(|layer| layer.id)
        .collect::<Vec<_>>();
    assert!(
        exchange::export(
            &project,
            &ids,
            Format::Csv,
            Options {
                extra_parameters: false,
                background: false,
                placeholders: true,
            },
        )
        .unwrap_err()
        .contains("cannot embed")
    );

    let json = exchange::export(&project, &ids, Format::Json, Options::default()).unwrap();
    assert!(parse_values(&json.replace("\"version\": 1", "\"version\": 2")).is_err());

    let csv = exchange::export(&project, &ids, Format::Csv, Options::default()).unwrap();
    let inconsistent = csv.replacen("1,1920,1080,layer-1", "1,3840,1080,layer-1", 1);
    assert!(parse_values(&inconsistent).is_err());

    let html = exchange::export(&project, &ids, Format::Html, Options::default()).unwrap();
    assert!(
        parse_values(&html.replace("data-midra-role=\"layer\"", "data-midra-role=\"guide\""))
            .is_err()
    );

    let dxf = exchange::export(&project, &ids, Format::Dxf, Options::default()).unwrap();
    let mut tags = craft_dxf::parse(dxf.as_bytes()).unwrap();
    let start = tags
        .iter()
        .position(|t| t.code == 0 && t.str() == "LWPOLYLINE")
        .unwrap();
    let closed = tags[start + 1..]
        .iter_mut()
        .take_while(|t| t.code != 0)
        .find(|t| t.code == 70)
        .unwrap();
    *closed = craft_dxf::Tag::i(70, 0);
    assert!(parse_values(&craft_dxf::write_ascii_as(&tags, true)).is_err());
}

#[test]
fn documented_examples_import_and_limits_are_enforced() {
    for text in [
        include_str!("../../../docs/interfaces/canvas-pilot-svg-v2.example.svg"),
        include_str!("../../../docs/interfaces/canvas-pilot-drawio-v1.example.drawio"),
    ] {
        let doc = parse_values(text).unwrap();
        assert_eq!(doc.shapes.len(), 2);
        assert!(doc.shapes.iter().all(|s| !s.background_only));
    }
    assert!(
        parse_values(&"x".repeat(exchange::MAX_EXCHANGE_BYTES + 1))
            .unwrap_err()
            .contains("16 MiB")
    );
    let mut p = fixture();
    p.layers[0].bounds.width = 0.1;
    assert!(exchange::export(&p, &[p.layers[0].id], Format::Svg, Options::default()).is_err());
}

#[test]
fn exported_background_composites_visible_references_in_order() {
    let mut project = fixture();
    for (name, color, visible) in [
        ("Red", [255, 0, 0, 255], true),
        ("Blue", [0, 0, 255, 128], true),
        ("Hidden green", [0, 255, 0, 255], false),
    ] {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 2, image::Rgba(color))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        project.references.push(canvas_pilot::project::Reference {
            name: name.into(),
            image_base64: STANDARD.encode(png.into_inner()),
            visible,
            opacity: 1.0,
            preserve_ratio: false,
            native_size: false,
            offset: [0.; 2],
        });
    }
    let xml = exchange::export(
        &project,
        &[project.layers[0].id],
        Format::Svg,
        Options {
            extra_parameters: false,
            background: true,
            placeholders: false,
        },
    )
    .unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let data = doc
        .descendants()
        .find(|n| n.has_tag_name("image"))
        .unwrap()
        .attribute(("http://www.w3.org/1999/xlink", "href"))
        .unwrap();
    let image = image::load_from_memory(
        &STANDARD
            .decode(data.strip_prefix("data:image/png;base64,").unwrap())
            .unwrap(),
    )
    .unwrap()
    .to_rgba8();
    assert_eq!(*image.get_pixel(0, 0), image::Rgba([127, 0, 128, 255]));
    let shapes = parse_values(&xml).unwrap().shapes;
    assert_eq!(shapes.len(), 2); // one flattened background and one real layer
    assert!(shapes[0].background_only);
}

#[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
fn check_native_templates(codecs: &[canvas_pilot::interchange::Codec]) {
    use canvas_pilot::interchange;
    for (label, resolution) in [("1080p", [1920, 1080]), ("4k", [3840, 2160])] {
        let project = template(resolution);
        let expected: Vec<_> = project.layers.iter().map(|l| l.bounds).collect();
        let ids: Vec<_> = project.layers.iter().map(|l| l.id).collect();
        for &codec in codecs {
            let filename = format!("canvas-pilot-{label}-2-layers.{}", codec.extension());
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../docs/templates/canvas-pilot")
                .join(&filename);
            let saved = std::fs::read(path).unwrap();
            let fresh = interchange::export(&project, &ids, codec, Options::default()).unwrap();
            // Native encoders may vary ZIP metadata or document IDs; compare imported geometry.
            for bytes in [saved, fresh] {
                let pages = interchange::import(&bytes, &filename).unwrap();
                assert_eq!(pages.len(), 1, "{filename}");
                assert!(pages[0].document.shapes.iter().all(|s| !s.background_only));
                assert_eq!(
                    pages[0]
                        .document
                        .shapes
                        .iter()
                        .map(|s| s.geometry)
                        .collect::<Vec<_>>(),
                    expected,
                    "{filename}"
                );
            }
        }
    }
}

#[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
#[test]
fn native_readme_templates_preserve_two_layer_geometry() {
    use canvas_pilot::interchange::Codec;
    check_native_templates(&[Codec::Pptx, Codec::Figma, Codec::Figcraft, Codec::Dwg]);
}

#[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
#[test]
#[ignore = "Requires a real VectorCraft CLI; set MIDRA_VECTORCRAFT_CLI"]
fn illustrator_readme_templates_preserve_two_layer_geometry() {
    check_native_templates(&[canvas_pilot::interchange::Codec::Illustrator]);
}
