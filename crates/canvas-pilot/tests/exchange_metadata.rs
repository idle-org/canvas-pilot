use canvas_pilot::{
    Layout,
    exchange::Options,
    exchange_metadata as metadata,
    interchange::{self, Codec},
    project::Project,
};

fn fixture() -> Project {
    let mut layout = Layout::default();
    layout
        .set_framing(
            0,
            canvas_pilot::formats::Framing {
                zoom: 1.5,
                offset: [12.0, -8.0],
            },
        )
        .unwrap();
    let mut p = Project::from_layout(&layout);
    p.layers[0].name = "<Speaker> & \"guest\"".into();
    p.layers[0].folder = "Memory A".into();
    p.layers[0].device_link = Some(canvas_pilot::DeviceLink {
        family: Default::default(),
        screen: "2".into(),
        layer: "1".into(),
    });
    p.workspace.collapsed_folders = vec!["Memory A".into()];
    p
}
#[test]
fn supported_codecs_preserve_artwork_and_restore_selected_parameters() {
    let p = fixture();
    let ids = [p.layers[0].id];
    let expected = metadata::snapshot(&p, &ids).unwrap();
    for codec in Codec::available()
        .into_iter()
        .filter(|&c| metadata::supported(c))
    {
        for background in [false, true] {
            let plain = interchange::export(
                &p,
                &ids,
                codec,
                Options {
                    background,
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(
                metadata::extract(&plain, codec).unwrap().is_none(),
                "{codec:?}"
            );
            let bytes = interchange::export(
                &p,
                &ids,
                codec,
                Options {
                    background,
                    extra_parameters: true,
                    placeholders: codec.artwork(),
                },
            )
            .unwrap();
            assert_eq!(
                metadata::extract(&bytes, codec).unwrap(),
                Some(expected.clone()),
                "{codec:?}"
            );
            let filename = format!("layout.{}", codec.extension());
            let before = interchange::import(&plain, &filename).unwrap();
            let after = interchange::import(&bytes, &filename).unwrap();
            let geometries = |pages: Vec<interchange::Page>| {
                pages
                    .into_iter()
                    .flat_map(|p| {
                        p.document
                            .shapes
                            .into_iter()
                            .filter(|s| !s.background)
                            .map(|s| s.geometry)
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(geometries(before), geometries(after), "{codec:?}");
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(filename);
            std::fs::write(&path, &bytes).unwrap();
            assert_eq!(interchange::read_embedded_project(&path).unwrap(), expected);
            if let Ok(root) = std::env::var("MIDRA_METADATA_SAMPLES") {
                std::fs::create_dir_all(&root).unwrap();
                std::fs::write(
                    std::path::Path::new(&root).join(format!("sample.{}", codec.extension())),
                    &bytes,
                )
                .unwrap();
            }
        }
    }
}
#[test]
fn metadata_is_optional_versioned_and_not_silently_applied_to_edited_artwork() {
    let p = fixture();
    let ids = [p.layers[0].id];
    let bytes = interchange::export(
        &p,
        &ids,
        Codec::Json,
        Options {
            extra_parameters: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["version"], 2);
    json["layers"][0]["x"] = 777.into();
    let changed = serde_json::to_vec(&json).unwrap();
    assert_eq!(
        interchange::import(&changed, "x.json").unwrap()[0]
            .document
            .shapes[0]
            .geometry
            .x,
        777.0
    );
    assert_ne!(
        metadata::extract(&changed, Codec::Json)
            .unwrap()
            .unwrap()
            .layers[0]
            .bounds
            .x,
        777.0
    );
    json[metadata::KEY] = "not base64".into();
    let invalid = serde_json::to_vec(&json).unwrap();
    assert!(metadata::extract(&invalid, Codec::Json).is_err());
    assert!(interchange::import(&invalid, "x.json").is_ok());
    json.as_object_mut().unwrap().remove(metadata::KEY);
    assert!(interchange::import(&serde_json::to_vec(&json).unwrap(), "x.json").is_err());
    for codec in [Codec::Dwg, Codec::Figma, Codec::Illustrator] {
        assert!(!metadata::supported(codec));
        assert!(
            interchange::export(
                &p,
                &ids,
                codec,
                Options {
                    extra_parameters: true,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
#[test]
fn snapshot_omits_unexported_layers_unused_sources_and_capture_bindings() {
    let p = fixture();
    let result = metadata::snapshot(&p, &[p.layers[1].id]).unwrap();
    assert_eq!(result.layers.len(), 1);
    assert_eq!(result.inputs.len(), 1);
    assert_eq!(result.layers[0].source_id, 1);
    assert!(result.workspace.collapsed_folders.is_empty());
    assert!(result.references.is_empty() && result.workspace.previews.is_empty());
    assert!(metadata::snapshot(&p, &[]).is_err());
}

#[test]
fn optional_drawio_desktop_resave_recovers_embedded_snapshot() {
    let Ok(root) = std::env::var("MIDRA_METADATA_SAMPLES") else {
        return;
    };
    let path = std::path::Path::new(&root).join("drawio-resaved.svg");
    if !path.exists() {
        return;
    }
    let bytes = std::fs::read(path).unwrap();
    let p = fixture();
    assert_eq!(
        metadata::extract(&bytes, Codec::Svg).unwrap(),
        Some(metadata::snapshot(&p, &[p.layers[0].id]).unwrap())
    );
    assert!(interchange::import(&bytes, "resaved.svg").is_ok());
}

#[test]
fn opening_parameters_never_reuses_artwork_path_and_keeps_native_errors() {
    let p = fixture();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("layout.json");
    std::fs::write(&path, p.to_json().unwrap()).unwrap();
    assert_eq!(
        interchange::read_project_or_parameters(&path).unwrap(),
        (p.clone(), true)
    );
    let mut invalid = serde_json::to_value(&p).unwrap();
    invalid["version"] = 999.into();
    std::fs::write(&path, invalid.to_string()).unwrap();
    assert!(
        interchange::read_project_or_parameters(&path)
            .unwrap_err()
            .contains("version")
    );
    let bytes = interchange::export(
        &p,
        &[p.layers[0].id],
        Codec::Json,
        Options {
            extra_parameters: true,
            ..Default::default()
        },
    )
    .unwrap();
    std::fs::write(&path, bytes).unwrap();
    assert!(!interchange::read_project_or_parameters(&path).unwrap().1);
}

#[test]
fn documented_metadata_example_and_pptx_package_relationships_are_valid() {
    let example =
        include_bytes!("../../../docs/interfaces/midra-geometry-v2-metadata.example.json");
    assert_eq!(
        interchange::import(example, "example.json").unwrap()[0]
            .document
            .shapes
            .len(),
        2
    );
    metadata::extract(example, Codec::Json)
        .unwrap()
        .unwrap()
        .layout()
        .unwrap();
    if !Codec::available().contains(&Codec::Pptx) {
        return;
    }
    use std::io::Read;
    let p = fixture();
    let bytes = interchange::export(
        &p,
        &[p.layers[0].id],
        Codec::Pptx,
        Options {
            extra_parameters: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    for name in ["[Content_Types].xml", "_rels/.rels", "docProps/custom.xml"] {
        let mut text = String::new();
        zip.by_name(name)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        let xml = roxmltree::Document::parse(&text).unwrap();
        match name {
            "[Content_Types].xml" => assert!(xml.descendants().any(|n| n.attribute("PartName") == Some("/docProps/custom.xml") && n.attribute("ContentType") == Some("application/vnd.openxmlformats-officedocument.custom-properties+xml"))),
            "_rels/.rels" => assert!(xml.descendants().any(|n| n.attribute("Target") == Some("docProps/custom.xml") && n.attribute("Type") == Some("http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties"))),
            _ => assert!(xml.descendants().any(|n| n.attribute("name") == Some(metadata::KEY))),
        }
    }
}

#[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
#[test]
fn figcraft_native_resave_preserves_snapshot_after_artwork_edits() {
    let p = fixture();
    let ids = [p.layers[0].id];
    let bytes = interchange::export(
        &p,
        &ids,
        Codec::Figcraft,
        Options {
            extra_parameters: true,
            placeholders: true,
            background: true,
        },
    )
    .unwrap();
    let (mut document, warnings) = figcraft_format::load_bytes(&bytes).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let before = interchange::import(&bytes, "original.figcraft").unwrap();
    let id = document
        .nodes()
        .find(|n| n.name == p.layers[0].name)
        .unwrap()
        .id;
    document.node_mut(id).unwrap().width += 100.;
    document.name = "Edited in native codec".into();
    let bytes = figcraft_format::save_bytes(&document, None).unwrap();
    let after = interchange::import(&bytes, "edited.figcraft").unwrap();
    let widths = |pages: Vec<interchange::Page>| {
        pages
            .into_iter()
            .flat_map(|p| p.document.shapes.into_iter().map(|s| s.geometry.width))
            .collect::<Vec<_>>()
    };
    assert_ne!(
        widths(before),
        widths(after),
        "Geometry import must see the edited artwork"
    );
    assert_eq!(
        metadata::extract(&bytes, Codec::Figcraft).unwrap(),
        Some(metadata::snapshot(&p, &ids).unwrap())
    );
    assert!(
        !interchange::import(&bytes, "resaved.figcraft")
            .unwrap()
            .is_empty()
    );
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut json = String::new();
    archive
        .by_name("document.json")
        .unwrap()
        .read_to_string(&mut json)
        .unwrap();
    assert_eq!(
        metadata::extract(json.as_bytes(), Codec::Figcraft).unwrap(),
        Some(metadata::snapshot(&p, &ids).unwrap())
    );
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let nodes = value["nodes"].as_object_mut().unwrap();
    let payload = nodes
        .values()
        .find_map(|n| n["plugin_data"][metadata::KEY].as_str())
        .unwrap()
        .to_string();
    let other = nodes
        .values_mut()
        .find(|n| n["plugin_data"][metadata::KEY].is_null())
        .unwrap();
    other["plugin_data"][metadata::KEY] = payload.into();
    assert!(
        metadata::extract(&serde_json::to_vec(&value).unwrap(), Codec::Figcraft)
            .unwrap_err()
            .contains("Multiple")
    );
}

#[test]
fn every_snapshot_codec_roundtrips_all_fit_modes_masks_sources_links_and_proposals() {
    use canvas_pilot::{
        formats::{Crop, LayerFit, Mask},
        input_proposal::InputProposal,
    };
    for origin in [
        canvas_pilot::formats::MaskOrigin::Layer,
        canvas_pilot::formats::MaskOrigin::Source,
    ] {
        for fit in [
            None,
            Some(LayerFit::OneToOne),
            Some(LayerFit::Centered),
            Some(LayerFit::Fullscreen),
            Some(LayerFit::Cropped),
        ] {
            let mut layout = Layout::default();
            layout
                .set_layer_fit(
                    0,
                    fit,
                    Crop {
                        left: 40.25,
                        right: 70.5,
                        top: 20.125,
                        bottom: 35.75,
                    },
                )
                .unwrap();
            layout
                .set_mask(
                    0,
                    Mask {
                        origin,
                        left: 0.125,
                        right: 0.0625,
                        top: 0.25,
                        bottom: 0.125,
                    },
                )
                .unwrap();
            let mut p = Project::from_layout(&layout);
            p.inputs[0].input_crop = Crop {
                left: 10.25,
                right: 20.5,
                top: 5.25,
                bottom: 10.75,
            };
            p.layers[0].name = "Speaker — é".into();
            p.layers[0].folder = "Scene".into();
            p.folders = vec!["Scene".into(), "Unused".into()];
            p.layers[0].pivot = [0.25, 0.75];
            p.layers[0].device_link = Some(canvas_pilot::DeviceLink {
                family: Default::default(),
                screen: "1".into(),
                layer: "3".into(),
            });
            p.layers[1].source_id = p.layers[0].source_id;
            let native = &p.inputs[0].native_resolution;
            let input = p.inputs[0]
                .input_crop
                .remaining([native.width as f64, native.height as f64])
                .unwrap();
            for layer in &mut p.layers {
                if layer.layer_fit.is_none() {
                    layer.advanced_crop = canvas_pilot::formats::crop_from_framing(
                        input,
                        [layer.bounds.width, layer.bounds.height],
                        Default::default(),
                    )
                    .unwrap();
                }
            }

            p.layers[1].locked = true;
            p.editor.selected_layer_ids = vec![p.layers[0].id];
            p.workspace.collapsed_folders = vec!["Scene".into()];
            p.workspace.grid_spacing = Some(10);
            p.editor.viewport_pan = [17., -23.];
            p.editor.show_source_pattern = true;
            p.editor.viewport_zoom = 1.75;
            p.editor.backdrop_title = "Round trip".into();
            p.editor.keep_aspect_ratio = false;
            p.editor.outline_layers = true;
            p.editor.show_toolbar_icon_names = true;
            p.workspace.resize_mode = canvas_pilot::ResizeMode::Mirror;
            p.workspace.capture_interval_seconds = 17;
            p.workspace.show_empty_inputs = true;
            p.workspace.snapping.enabled = true;
            p.workspace.snapping.pixels = false;
            p.workspace.snapping.grid = true;
            p.workspace.snapping.grid_divisions = [7, 5];

            p.input_proposals.push(InputProposal {
                layer_id: p.layers[0].id,
                device_link: p.layers[0].device_link.clone().unwrap(),
                input: "INPUT_4".into(),
                plug: "2".into(),
                settings: serde_json::from_value(serde_json::json!({
                    "aspect/pp/signal":"16_9", "aspect/pp/transformTo":"CUSTOM",
                    "aspect/pp/customRatio":1778, "aspect/pp/layerFill":"CENTERED",
                    "cropping/control/pp/predefined":"NONE", "cropping/control/pp/left":41,
                    "cropping/control/pp/right":17, "cropping/control/pp/top":3,
                    "cropping/control/pp/bottom":9
                }))
                .unwrap(),
            });
            let ids: Vec<_> = p.layers.iter().map(|l| l.id).collect();
            assert_eq!(Project::from_json(&p.to_json().unwrap()).unwrap(), p);
            let expected = metadata::snapshot(&p, &ids).unwrap();
            assert_eq!(expected.layers, p.layers);
            assert_eq!(expected.inputs.len(), 1);
            assert_eq!(expected.input_proposals, p.input_proposals);
            assert_eq!(expected.layers[0].mask, p.layers[0].mask);
            assert_eq!(expected.layers[0].layer_fit, fit);
            assert!(
                metadata::snapshot(&p, &[p.layers[1].id])
                    .unwrap()
                    .input_proposals
                    .is_empty()
            );
            for codec in Codec::available()
                .into_iter()
                .filter(|&c| metadata::supported(c))
            {
                let bytes = interchange::export(
                    &p,
                    &ids,
                    codec,
                    Options {
                        extra_parameters: true,
                        ..Default::default()
                    },
                )
                .unwrap();
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join(format!("roundtrip.{}", codec.extension()));
                std::fs::write(&path, &bytes).unwrap();
                let (restored, native) = interchange::read_project_or_parameters(&path).unwrap();
                assert!(!native);
                assert_eq!(restored, expected, "{codec:?} / {fit:?}");
                assert_eq!(
                    restored.layout().unwrap().formats().masks,
                    expected.layout().unwrap().formats().masks
                );
                let second = interchange::export(
                    &restored,
                    &ids,
                    codec,
                    Options {
                        extra_parameters: true,
                        ..Default::default()
                    },
                )
                .unwrap();
                assert_eq!(
                    metadata::extract(&second, codec).unwrap(),
                    Some(expected.clone())
                );
                assert_eq!(
                    interchange::import(&bytes, &format!("x.{}", codec.extension())).unwrap()[0]
                        .document
                        .shapes
                        .len(),
                    2
                );
            }
        }
    }
}
