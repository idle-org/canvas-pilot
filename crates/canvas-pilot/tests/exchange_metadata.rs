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
    for codec in [
        Codec::Dwg,
        Codec::Figma,
        Codec::Figcraft,
        Codec::Illustrator,
    ] {
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
