use canvas_pilot::{
    Layout,
    clipboard::{self, Appearance},
    formats::{Crop, Framing, LayerFit, Mask},
    import::parse_values,
    project::Project,
};

fn copied(fit: Option<LayerFit>) -> (Layout, String) {
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
    if fit.is_none() {
        layout
            .set_framing(
                0,
                Framing {
                    zoom: 2.25,
                    offset: [17.125, -8.75],
                },
            )
            .unwrap();
    }
    layout
        .set_mask(
            0,
            Mask {
                origin: Default::default(),
                left: 0.125,
                right: 0.0625,
                top: 0.25,
                bottom: 0.125,
            },
        )
        .unwrap();
    let text = clipboard::encode(&layout, &[0], "Readable values").unwrap();
    (layout, text)
}
#[test]
fn every_fit_roundtrips_exact_crop_mask_and_offset_with_atomic_undo() {
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
            let (mut layout, _) = copied(fit);
            let mut mask = layout.formats().masks[0];
            mask.origin = origin;
            layout.set_mask(0, mask).unwrap();
            let text = clipboard::encode(&layout, &[0], "Readable values").unwrap();
            let shape = parse_values(&text).unwrap().shapes.remove(0);
            let before = Project::from_layout(&layout);
            let a = shape.appearance.unwrap();
            assert_eq!(a.fit, fit);
            assert_eq!(a.crop, layout.formats().advanced[0]);
            layout
                .apply_mapped_clipboard(&[(1, shape.geometry)], &[], &[(1, a)], true, true)
                .unwrap();
            let after = Project::from_layout(&layout);
            assert_eq!(layout.formats().advanced[1], layout.formats().advanced[0]);
            assert_eq!(layout.formats().layer_fit[1], fit);
            assert_eq!(layout.formats().masks[1], layout.formats().masks[0]);
            if fit.is_none() {
                assert_eq!(
                    layout.formats().mapping(1, shape.geometry).unwrap(),
                    layout.formats().mapping(0, shape.geometry).unwrap()
                );
            }
            assert_eq!(after.inputs, before.inputs);
            assert_eq!(after.layers[1].source_id, before.layers[1].source_id);
            assert_eq!(after.layers[1].device_link, before.layers[1].device_link);
            assert_eq!(after.layers[1].name, before.layers[1].name);
            let second = clipboard::encode(&layout, &[1], "Second copy").unwrap();
            assert_eq!(parse_values(&second).unwrap().shapes[0].appearance, Some(a));
            layout.undo();
            assert_eq!(Project::from_layout(&layout), before);
            layout.redo();
            assert_eq!(Project::from_layout(&layout), after);
        }
    }
}
#[test]
fn appearance_can_be_applied_independently_and_source_mismatch_is_atomic() {
    let (mut layout, text) = copied(Some(LayerFit::Fullscreen));
    let a = parse_values(&text).unwrap().shapes[0].appearance.unwrap();
    let before = Project::from_layout(&layout);
    let g = layout.layers()[1];
    let incompatible = Appearance {
        input_size: [1000., 500.],
        ..a
    };
    assert!(
        layout
            .apply_mapped_clipboard(&[(1, g)], &[], &[(1, incompatible)], true, true)
            .unwrap_err()
            .contains("matching source dimensions")
    );
    assert_eq!(Project::from_layout(&layout), before);
    layout
        .apply_mapped_clipboard(&[(1, g)], &[], &[(1, incompatible)], false, true)
        .unwrap();
    assert_eq!(layout.formats().masks[1], a.mask);
    assert_eq!(layout.formats().advanced[1], before.layers[1].advanced_crop);
    assert_eq!(layout.formats().layer_fit[1], before.layers[1].layer_fit);
    layout.undo();
    layout
        .apply_mapped_clipboard(&[(1, g)], &[], &[(1, a)], true, false)
        .unwrap();
    assert_eq!(layout.formats().layer_fit[1], a.fit);
    assert_eq!(layout.formats().masks[1], before.layers[1].mask);
    layout.undo();
    let mut invalid = a;
    invalid.mask.left = 2.;
    assert!(
        layout
            .apply_mapped_clipboard(&[(0, g), (1, g)], &[], &[(0, a), (1, invalid)], true, true)
            .is_err()
    );
    assert_eq!(Project::from_layout(&layout), before);
    assert!(
        layout
            .apply_mapped_clipboard(&[(1, g)], &[], &[(1, a), (1, a)], true, true)
            .is_err()
    );
    assert!(
        layout
            .apply_mapped_clipboard(&[(1, g)], &[(1, 2.)], &[(1, a)], true, true)
            .is_err()
    );
    assert!(
        layout
            .apply_mapped_clipboard(&[(999, g)], &[], &[(999, a)], true, true)
            .is_err()
    );
    layout.set_locked(1, true);
    assert!(
        layout
            .apply_mapped_clipboard(&[(1, g)], &[], &[(1, a)], true, true)
            .is_err()
    );
}
#[test]
fn normal_crop_requires_matching_aspect_and_does_not_silently_reframe() {
    let (mut layout, text) = copied(None);
    let a = parse_values(&text).unwrap().shapes[0].appearance.unwrap();
    let before = Project::from_layout(&layout);
    let mut g = layout.layers()[1];
    g.width = g.height;
    assert!(
        layout
            .apply_mapped_clipboard(&[(1, g)], &[], &[(1, a)], true, true)
            .unwrap_err()
            .contains("Include Format")
    );
    assert_eq!(Project::from_layout(&layout), before);
}
#[test]
fn clipboard_schema_is_bounded_versioned_and_never_falls_back_on_invalid_payload() {
    let (layout, text) = copied(None);
    let (_, json) = text.rsplit_once(clipboard::MARKER).unwrap();
    let good: serde_json::Value = serde_json::from_str(json).unwrap();
    for (pointer, value) in [
        ("/version", serde_json::json!(4)),
        ("/format", serde_json::json!("other")),
        ("/layers/0/appearance/fit", serde_json::json!("unknown")),
        ("/layers/0/appearance/input_size/0", serde_json::json!(0)),
        ("/layers/0/appearance/crop/left", serde_json::json!(-1)),
        ("/layers/0/appearance/mask/top", serde_json::json!(1.5)),
        ("/layers", serde_json::json!([])),
    ] {
        let mut invalid = good.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(
            parse_values(&format!("Readable text{}{invalid}", clipboard::MARKER)).is_err(),
            "{pointer}"
        );
    }
    for field in ["fit", "crop", "mask", "input_size"] {
        let mut invalid = good.clone();
        invalid["layers"][0]["appearance"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(clipboard::parse(&invalid.to_string()).is_err());
    }
    let mut invalid = good.clone();
    invalid["layers"][0]["device_link"] = serde_json::json!({});
    assert!(clipboard::parse(&invalid.to_string()).is_err());
    invalid = good.clone();
    invalid["layers"] = serde_json::json!(vec![good["layers"][0].clone(); 257]);
    assert!(clipboard::parse(&invalid.to_string()).is_err());
    assert!(parse_values(&format!("{text}garbage")).is_err());
    assert!(clipboard::encode(&layout, &[], "").is_err());
    assert!(clipboard::encode(&layout, &[999], "").is_err());
    assert!(parse_values(&"x".repeat(canvas_pilot::exchange::MAX_EXCHANGE_BYTES + 1)).is_err());
}
#[test]
fn documented_fixture_and_multiple_layers_preserve_fractional_bounds() {
    let fixture = include_str!("../../../docs/interfaces/canvas-pilot-clipboard-v1.example.txt");
    let shape = parse_values(fixture).unwrap().shapes.remove(0);
    assert_eq!(shape.geometry.x, 120.125);
    assert_eq!(shape.appearance.unwrap().crop.left, 600.);
    let (layout, _) = copied(None);
    let multi = clipboard::encode(&layout, &[0, 1], "Readable values").unwrap();
    assert_eq!(parse_values(&multi).unwrap().shapes.len(), 2);
    let legacy = "Old — visible box (device container unverified)\nHorizontal: 320 px\nVertical: 180 px\nWidth: 640 px\nHeight: 360 px\nCrop zoom: 200 %";
    let shape = parse_values(legacy).unwrap().shapes.remove(0);
    assert_eq!(shape.zoom, Some(2.));
    assert!(shape.appearance.is_none());
}

#[test]
fn names_are_explicit_optional_atomic_and_preserve_identity_and_folders() {
    let (mut layout, _) = copied(None);
    layout
        .rename_layer(0, "Caméra & <wide>", "Sources")
        .unwrap();
    layout
        .rename_layer(1, "Destination", "Keep this folder")
        .unwrap();
    let text = clipboard::encode(&layout, &[0], "Summary").unwrap();
    let shape = parse_values(&text).unwrap().shapes.remove(0);
    assert_eq!(shape.name.as_deref(), Some("Caméra & <wide>"));
    let legacy = include_str!("../../../docs/interfaces/canvas-pilot-clipboard-v1.example.txt");
    assert!(parse_values(legacy).unwrap().shapes[0].name.is_none());
    let current = include_str!("../../../docs/interfaces/canvas-pilot-clipboard-v2.example.txt");
    assert_eq!(
        parse_values(current).unwrap().shapes[0].name.as_deref(),
        Some("Presenter")
    );
    let before = Project::from_layout(&layout);
    let g = layout.layers()[1];
    layout
        .apply_mapped_clipboard_with_names(
            &[(1, g)],
            &[],
            &[],
            false,
            false,
            &[(1, shape.name.unwrap())],
        )
        .unwrap();
    assert_eq!(layout.layer_info(1).name, "Caméra & <wide>");
    assert_eq!(layout.layer_info(1).folder, "Keep this folder");
    assert_eq!(layout.layer_info(1).id, before.layers[1].id);
    layout.undo();
    assert_eq!(Project::from_layout(&layout), before);
    for names in [
        vec![(1, "".into())],
        vec![(1, "x".repeat(129))],
        vec![(1, "One".into()), (1, "Two".into())],
        vec![(999, "No".into())],
    ] {
        assert!(
            layout
                .apply_mapped_clipboard_with_names(&[(1, g)], &[], &[], false, false, &names)
                .is_err()
        );
        assert_eq!(Project::from_layout(&layout), before);
    }
}

#[test]
fn external_names_survive_exchange_without_promoting_generated_ids() {
    use canvas_pilot::{
        exchange::Options,
        interchange::{self, Codec},
    };
    let mut layout = Layout::default();
    layout.rename_layer(0, "Caméra & wide", "").unwrap();
    let project = Project::from_layout(&layout);
    for codec in Codec::available()
        .into_iter()
        .filter(|c| *c != Codec::Illustrator)
    {
        let bytes =
            interchange::export(&project, &[project.layers[0].id], codec, Options::default())
                .unwrap();
        let pages = interchange::import(&bytes, &format!("test.{}", codec.extension())).unwrap();
        assert_eq!(
            pages[0].document.shapes[0].name.as_deref(),
            Some("Caméra & wide"),
            "{codec:?}"
        );
    }
    for (svg, name) in [
        (
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect width="100" height="50"/></svg>"#,
            None,
        ),
        (
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect id="Camera" width="100" height="50"/></svg>"#,
            Some("Camera"),
        ),
        (
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect id="path1" width="100" height="50"><title>Presenter</title></rect></svg>"#,
            Some("Presenter"),
        ),
    ] {
        assert_eq!(parse_values(svg).unwrap().shapes[0].name.as_deref(), name);
    }
    let xml = |value: &str, style: &str| {
        format!(
            r#"<mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/><mxCell id="generated-id" value="{value}" style="{style}" vertex="1" parent="1"><mxGeometry width="100" height="50" as="geometry"/></mxCell></root></mxGraphModel>"#
        )
    };
    for (value, style, name) in [
        ("", "html=0", None),
        ("Caméra &amp; wide", "html=0", Some("Caméra & wide")),
        ("&lt;b&gt;Presenter&lt;/b&gt;", "html=1", Some("Presenter")),
    ] {
        assert_eq!(
            parse_values(&xml(value, style)).unwrap().shapes[0]
                .name
                .as_deref(),
            name
        );
    }
}

#[test]
fn svg_group_names_and_labels_are_used_but_document_titles_are_not() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="500" height="500"><title>Document title</title><g id="Camera"><rect width="100" height="50"/></g><rect id="path2" inkscape:label="Lectern" width="100" height="50"><title>Other title</title></rect><rect width="100" height="50"/></svg>"#;
    let names: Vec<_> = parse_values(svg)
        .unwrap()
        .shapes
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(
        names,
        vec![Some("Camera".into()), Some("Lectern".into()), None]
    );
}
