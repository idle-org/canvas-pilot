use canvas_pilot::{
    Geometry, Layout,
    formats::{Crop, Framing, Source},
    project::{Project, write_file},
};

// Old files retain their original labels/IDs rather than adopting new-project defaults.
fn legacy_default() -> Project {
    let mut p = Project::from_layout(&Layout::default());
    p.layers[0].id = 1;
    p.layers[0].name = "Presenter".into();
    p.layers[1].id = 2;
    p.layers[1].name = "Slides".into();
    p.editor.selected_layer_ids = vec![2];
    p
}

#[test]
fn full_precision_project_roundtrip_restores_sources_crops_flags_and_selection() {
    let mut layout = Layout::default();
    let g = Geometry {
        x: -12.123456789,
        y: 371.23456789,
        width: 300.125,
        height: 250.25,
    };
    let source = Source {
        input_crop: Crop {
            top: 40.0,
            bottom: 40.0,
            ..Crop::default()
        },
        ..Source::default()
    };
    layout.configure(1, 0, source, g, true).unwrap();
    layout
        .set_framing(
            1,
            Framing {
                zoom: 2.25,
                offset: [10.0, -15.0],
            },
        )
        .unwrap();
    layout.set_visible(0, false);
    layout.set_locked(0, true);
    layout.set_output([2560, 1600]).unwrap();
    let project = Project::from_layout(&layout);
    let json = project.to_json().unwrap();
    assert!(json.contains("\"advanced_crop\""));
    let decoded = Project::from_json(&json).unwrap();
    assert_eq!(decoded, project);
    let mut loaded = decoded.layout().unwrap();
    assert_eq!(loaded.layers(), layout.layers());
    assert_eq!(loaded.formats(), layout.formats());
    assert!(loaded.locked(0));
    assert!(!loaded.visible(0));
    assert_eq!(loaded.selection_mask(), layout.selection_mask());
    loaded.undo();
    assert_eq!(loaded.layers(), layout.layers());
}
#[test]
fn malformed_or_incompatible_projects_are_refused() {
    let original = serde_json::to_value(Project::from_layout(&Layout::default())).unwrap();
    for (pointer, value) in [
        ("/version", serde_json::json!(999)),
        ("/format", serde_json::json!("other")),
        ("/layers/0/source_id", serde_json::json!(0)),
        ("/inputs/1/id", serde_json::json!(1)),
        ("/layers/0/bounds/width", serde_json::json!(-1)),
        ("/layers/1/advanced_crop/left", serde_json::json!(1900)),
        ("/editor/selected_layer_ids", serde_json::json!([2, 2])),
        ("/layers/1/locked", serde_json::json!(true)),
        ("/editor/viewport_zoom", serde_json::json!(0)),
        ("/inputs/0/native_resolution/width", serde_json::json!(0)),
    ] {
        let mut invalid = original.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(
            Project::from_json(&invalid.to_string()).is_err(),
            "{pointer}"
        );
    }
    let mut extra = original.clone();
    extra["unexpected"] = serde_json::json!(true);
    assert!(Project::from_json(&extra.to_string()).is_err());
    let mut missing = original.clone();
    missing.as_object_mut().unwrap().remove("version");
    assert!(Project::from_json(&missing.to_string()).is_err());
    assert!(Project::from_json("{broken").is_err());
}
#[test]
fn exported_svg_roundtrips_rounded_geometry_including_full_screen_layers() {
    let mut layout = Layout::default();
    layout
        .apply(
            0,
            Geometry {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
        )
        .unwrap();
    layout
        .apply(
            1,
            Geometry {
                x: -12.6,
                y: 27.4,
                width: 300.8,
                height: 250.2,
            },
        )
        .unwrap();
    layout.set_visible(1, false);
    let p = Project::from_layout(&layout);
    let doc = canvas_pilot::import::parse_svg(&p.to_svg(false).unwrap()).unwrap();
    assert_eq!(doc.shapes.len(), 2);
    assert!(!doc.shapes[0].background);
    assert_eq!(doc.shapes[0].geometry, layout.layers()[0]);
    assert_eq!(
        doc.shapes[1].geometry,
        Geometry {
            x: -13.0,
            y: 27.0,
            width: 301.0,
            height: 250.0
        }
    );
    assert_eq!(doc.shapes[1].outline, [0.0; 2]);
    assert_eq!(
        canvas_pilot::import::parse_svg(&p.to_svg(true).unwrap())
            .unwrap()
            .shapes
            .len(),
        1
    );
    layout.set_visible(0, false);
    assert!(Project::from_layout(&layout).to_svg(true).is_err());
}
#[test]
fn publication_requires_overwrite_and_read_matches_written_document() {
    for filename in ["layout.pilot.json", "arbitrary.json", "layout.midra.json"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(filename);
        let p = Project::from_layout(&Layout::default());
        write_file(&path, p.to_json().unwrap().as_bytes(), false).unwrap();
        assert_eq!(Project::read(&path).unwrap(), p);
        assert!(write_file(&path, b"replacement", false).is_err());
        assert_eq!(Project::read(&path).unwrap(), p);
        write_file(&path, b"replacement", true).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"replacement");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}

#[test]
fn documented_example_is_a_valid_version_one_project() {
    let example = include_str!("../../../docs/interfaces/canvas-pilot-v1.example.json");
    assert_eq!(Project::from_json(example).unwrap(), legacy_default());
}

#[test]
fn version_two_requires_captions_preference_and_preserves_it() {
    let mut project = Project::from_layout(&Layout::default());
    project.editor.show_toolbar_icon_names = true;
    assert_eq!(
        Project::from_json(&project.to_json().unwrap()).unwrap(),
        project
    );
    let mut missing = serde_json::to_value(&project).unwrap();
    missing["editor"]
        .as_object_mut()
        .unwrap()
        .remove("show_toolbar_icon_names");
    assert!(Project::from_json(&missing.to_string()).is_err());
    let example = include_str!("../../../docs/interfaces/canvas-pilot-v2.example.json");
    assert_eq!(Project::from_json(example).unwrap(), legacy_default());
}
#[test]
fn v4_example_and_old_version_migrations_preserve_defaults() {
    assert_eq!(
        Project::from_json(include_str!(
            "../../../docs/interfaces/canvas-pilot-v4.example.json"
        ))
        .unwrap(),
        legacy_default()
    );
    let mut value = serde_json::to_value(Project::from_layout(&Layout::default())).unwrap();
    value["version"] = 0.into();
    assert!(Project::from_json(&value.to_string()).is_err());
    value["version"] = 4.into();
    value["editor"]
        .as_object_mut()
        .unwrap()
        .remove("outline_layers");
    assert!(Project::from_json(&value.to_string()).is_err());
}

#[test]
fn persistent_device_links_roundtrip_migrate_and_undo() {
    use canvas_pilot::DeviceLink;
    let mut layout = Layout::default();
    let link = DeviceLink {
        family: Default::default(),
        screen: "2".into(),
        layer: "1".into(),
    };
    let first_link = DeviceLink {
        family: Default::default(),
        screen: "1".into(),
        layer: "1".into(),
    };
    layout.set_device_link(0, Some(first_link.clone())).unwrap();
    layout.set_device_link(1, Some(link.clone())).unwrap();
    layout.select(None);
    let project = Project::from_layout(&layout);
    let restored = Project::from_json(&project.to_json().unwrap())
        .unwrap()
        .layout()
        .unwrap();
    assert_eq!(restored.layer_info(0).load_mapping, Some(first_link));
    assert_eq!(restored.layer_info(1).load_mapping, Some(link.clone()));
    layout.undo();
    assert!(layout.layer_info(1).load_mapping.is_none());
    layout.redo();
    assert_eq!(layout.layer_info(1).load_mapping, Some(link));
    layout.set_device_link(1, None).unwrap();
    assert!(layout.layer_info(1).load_mapping.is_none());
    for text in [
        include_str!("../../../docs/interfaces/canvas-pilot-v4.example.json"),
        include_str!("../../../docs/interfaces/canvas-pilot-v5.example.json"),
    ] {
        let p = Project::from_json(text).unwrap();
        assert_eq!(p.version, 19);
        assert!(p.layers.iter().all(|l| l.load_mapping.is_none()));
    }
    let mut value = serde_json::to_value(&project).unwrap();
    value["layers"][1]["load_mapping"]["screen"] = "../2".into();
    assert!(Project::from_json(&value.to_string()).is_err());
    value["layers"][1]["load_mapping"] = serde_json::Value::Null;
    value["layers"][1]
        .as_object_mut()
        .unwrap()
        .remove("load_mapping");
    assert!(Project::from_json(&value.to_string()).is_err());
}

#[test]
fn positioning_points_roundtrip_and_keep_their_reference_when_resized() {
    use canvas_pilot::PositionPoint;
    let output = [1921, 1081];
    let g = Geometry {
        x: 100.25,
        y: 200.5,
        width: 321.5,
        height: 181.25,
    };
    for point in [
        PositionPoint::TopLeft,
        PositionPoint::TopRight,
        PositionPoint::BottomLeft,
        PositionPoint::BottomRight,
        PositionPoint::Center,
    ] {
        let [dx, dy] = point.offsets_on(g, output);
        assert_eq!(
            point
                .geometry_on(dx, dy, g.width, g.height, output)
                .unwrap(),
            g
        );
        let resized = point.geometry_on(dx, dy, 123.5, 99.25, output).unwrap();
        assert_eq!(point.offsets_on(resized, output), [dx, dy]);
        if point == PositionPoint::Center {
            assert!(dx < 0.0 && dy < 0.0);
            assert_eq!(resized.center(), g.center());
        }
        let mut project = Project::from_layout(&Layout::default());
        project.editor.position_point = point;
        assert_eq!(
            Project::from_json(&project.to_json().unwrap()).unwrap(),
            project
        );
    }
    let centered = PositionPoint::Center
        .geometry_on(0.0, 0.0, 320.0, 180.0, output)
        .unwrap();
    assert_eq!(centered.center(), [960.5, 540.5]);
}

#[test]
fn position_point_schema_migrates_corners_and_rejects_mislabeled_files() {
    use canvas_pilot::PositionPoint;
    let current = Project::from_json(include_str!(
        "../../../docs/interfaces/canvas-pilot-v6.example.json"
    ))
    .unwrap();
    assert_eq!(current, legacy_default());
    for (name, expected) in [
        ("top_left", PositionPoint::TopLeft),
        ("top_right", PositionPoint::TopRight),
        ("bottom_left", PositionPoint::BottomLeft),
        ("bottom_right", PositionPoint::BottomRight),
    ] {
        let mut old: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/interfaces/canvas-pilot-v5.example.json"
        ))
        .unwrap();
        old["editor"]["position_corner"] = name.into();
        let loaded = Project::from_json(&old.to_string()).unwrap();
        assert_eq!(loaded.version, 19);
        assert_eq!(loaded.editor.position_point, expected);
        old["editor"]["position_corner"] = "center".into();
        assert!(Project::from_json(&old.to_string()).is_err());
        old["editor"]["position_corner"] = name.into();
        old["editor"]["position_point"] = name.into();
        assert!(Project::from_json(&old.to_string()).is_err());
    }
    let value: serde_json::Value = serde_json::from_str(&current.to_json().unwrap()).unwrap();
    for version in [0, 20] {
        let mut invalid = value.clone();
        invalid["version"] = version.into();
        assert!(
            Project::from_json(&invalid.to_string())
                .unwrap_err()
                .contains("Unsupported")
        );
    }
    let mut invalid = value.clone();
    invalid["editor"]
        .as_object_mut()
        .unwrap()
        .remove("position_point");
    assert!(Project::from_json(&invalid.to_string()).is_err());
    invalid["editor"]["position_corner"] = "top_left".into();
    assert!(Project::from_json(&invalid.to_string()).is_err());
}

#[test]
fn backdrop_title_roundtrips_and_older_projects_gain_the_default() {
    use canvas_pilot::project::DEFAULT_BACKDROP_TITLE;
    let current = Project::from_json(include_str!(
        "../../../docs/interfaces/canvas-pilot-v7.example.json"
    ))
    .unwrap();
    assert_eq!(current, legacy_default());
    for text in ["", "Conférence · Main stage", &"é".repeat(128)] {
        let mut project = current.clone();
        project.editor.backdrop_title = text.into();
        assert_eq!(
            Project::from_json(&project.to_json().unwrap()).unwrap(),
            project
        );
        assert_eq!(
            project.to_svg(false).unwrap(),
            current.to_svg(false).unwrap()
        );
    }
    let old = include_str!("../../../docs/interfaces/canvas-pilot-v6.example.json");
    let migrated = Project::from_json(old).unwrap();
    assert_eq!(migrated.version, 19);
    assert_eq!(migrated.editor.backdrop_title, DEFAULT_BACKDROP_TITLE);
    let mut invalid: serde_json::Value = serde_json::from_str(old).unwrap();
    invalid["editor"]["backdrop_title"] = "Unexpected".into();
    assert!(Project::from_json(&invalid.to_string()).is_err());
    let value: serde_json::Value = serde_json::from_str(&current.to_json().unwrap()).unwrap();
    for title in ["a".repeat(129), "line\nbreak".into(), "tab\there".into()] {
        let mut invalid = value.clone();
        invalid["editor"]["backdrop_title"] = title.into();
        assert!(Project::from_json(&invalid.to_string()).is_err());
    }
    let mut missing = value;
    missing["editor"]
        .as_object_mut()
        .unwrap()
        .remove("backdrop_title");
    assert!(Project::from_json(&missing.to_string()).is_err());
}

#[test]
fn independent_layer_point_is_explicit_and_old_projects_keep_matched_points() {
    use canvas_pilot::PositionPoint;
    let current = Project::from_json(include_str!(
        "../../../docs/interfaces/canvas-pilot-v8.example.json"
    ))
    .unwrap();
    assert_eq!(current, legacy_default());
    for point in [
        None,
        Some(PositionPoint::Center),
        Some(PositionPoint::BottomRight),
    ] {
        let mut project = current.clone();
        project.editor.position_layer_point = point;
        assert_eq!(
            Project::from_json(&project.to_json().unwrap()).unwrap(),
            project
        );
    }
    let old = include_str!("../../../docs/interfaces/canvas-pilot-v7.example.json");
    assert!(
        Project::from_json(old)
            .unwrap()
            .editor
            .position_layer_point
            .is_none()
    );
    let mut invalid: serde_json::Value = serde_json::from_str(old).unwrap();
    invalid["editor"]["position_layer_point"] = "center".into();
    assert!(Project::from_json(&invalid.to_string()).is_err());
    let mut missing: serde_json::Value = serde_json::from_str(&current.to_json().unwrap()).unwrap();
    missing["editor"]
        .as_object_mut()
        .unwrap()
        .remove("position_layer_point");
    assert!(Project::from_json(&missing.to_string()).is_err());
}

#[test]
fn saved_pivots_roundtrip_and_old_projects_default_to_center() {
    let mut layout = Layout::default();
    let custom = [-0.123456789, 1.25];
    layout.set_layer_pivots(&[(0, custom)]).unwrap();
    let project = Project::from_layout(&layout);
    let loaded = Project::from_json(&project.to_json().unwrap())
        .unwrap()
        .layout()
        .unwrap();
    assert_eq!(loaded.layer_info(0).pivot, custom);
    assert_eq!(loaded.layer_info(1).pivot, [0.5; 2]);
    let example = include_str!("../../../docs/interfaces/canvas-pilot-v9.example.json");
    assert_eq!(Project::from_json(example).unwrap().version, 19);
    let old = include_str!("../../../docs/interfaces/canvas-pilot-v8.example.json");
    let migrated = Project::from_json(old).unwrap();
    assert!(migrated.layers.iter().all(|l| l.pivot == [0.5; 2]));
    let mut old: serde_json::Value = serde_json::from_str(old).unwrap();
    old["layers"][0]["pivot"] = serde_json::json!([0.2, 0.3]);
    assert!(Project::from_json(&old.to_string()).is_err());
    for invalid in [
        serde_json::json!([1e7, 0.5]),
        serde_json::json!([0.5]),
        serde_json::json!([0.5, 0.5, 0.5]),
        serde_json::Value::Null,
    ] {
        let mut value = serde_json::to_value(&project).unwrap();
        value["layers"][0]["pivot"] = invalid;
        assert!(Project::from_json(&value.to_string()).is_err());
    }
    let mut value = serde_json::to_value(&project).unwrap();
    value["layers"][0].as_object_mut().unwrap().remove("pivot");
    assert!(Project::from_json(&value.to_string()).is_err());
}

#[test]
fn reference_collection_migrates_single_images_and_rejects_mislabeled_or_oversized_collections() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let mut old: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/interfaces/canvas-pilot-v9.example.json"
    ))
    .unwrap();
    let reference = serde_json::json!({"name":"guide.png","image_base64":STANDARD.encode(b"image bytes"),"visible":false,"opacity":0.4,"preserve_ratio":true,"native_size":false,"offset":[0.0,0.0]});
    old["reference"] = reference.clone();
    let migrated = Project::from_json(&old.to_string()).unwrap();
    assert_eq!(migrated.version, 19);
    assert_eq!(migrated.references.len(), 1);
    assert!(!migrated.references[0].visible);
    assert_eq!(migrated.references[0].opacity, 0.4);
    let mut gallery = migrated.clone();
    gallery.references.push(migrated.references[0].clone());
    gallery.references[1].name = "other.png".into();
    gallery.references[1].visible = true;
    assert_eq!(
        Project::from_json(&gallery.to_json().unwrap()).unwrap(),
        gallery
    );
    old["references"] = serde_json::json!([]);
    assert!(Project::from_json(&old.to_string()).is_err());
    let mut wrong = serde_json::to_value(&gallery).unwrap();
    wrong["reference"] = reference;
    assert!(Project::from_json(&wrong.to_string()).is_err());
    gallery.references = vec![migrated.references[0].clone(); 33];
    assert!(gallery.to_json().unwrap_err().contains("32 reference"));
    let example = include_str!("../../../docs/interfaces/canvas-pilot-v10.example.json");
    assert_eq!(Project::from_json(example).unwrap().version, 19);
}

#[test]
fn editor_inputs_grow_independently_and_roundtrip_with_undo() {
    let mut layout = Layout::default();
    let before = layout.formats().clone();
    let geometry = layout.layers()[0];
    layout
        .configure(0, 7, Source::default(), geometry, true)
        .unwrap();
    assert_eq!(layout.layers().len(), 2);
    assert_eq!(layout.formats().sources.len(), 8);
    assert_eq!(layout.formats().source_for, [7, 1]);
    layout.undo();
    assert_eq!(layout.formats(), &before);
    layout.redo();
    layout
        .configure(1, 7, Source::default(), layout.layers()[1], true)
        .unwrap();
    let project = Project::from_layout(&layout);
    let restored = Project::from_json(&project.to_json().unwrap())
        .unwrap()
        .layout()
        .unwrap();
    assert_eq!(restored.formats(), layout.formats());
    assert_eq!(project.inputs.len(), 8);
    assert_eq!(project.layers[0].source_id, 8);
    layout
        .configure(0, 8, Source::default(), geometry, true)
        .unwrap();
    assert_eq!(layout.formats().sources.len(), 9);
    let before = layout.formats().clone();
    assert!(
        layout
            .configure(0, 1024, Source::default(), geometry, true)
            .is_err()
    );
    assert_eq!(layout.formats(), &before);
    layout.set_locked(0, true);
    assert!(
        layout
            .configure(0, 9, Source::default(), geometry, true)
            .is_err()
    );
    assert_eq!(layout.formats(), &before);
}

#[test]
fn workspace_roundtrip_migration_and_validation() {
    use canvas_pilot::project::{Preview, Workspace};
    let old = include_str!("../../../docs/interfaces/canvas-pilot-v10.example.json");
    let migrated = Project::from_json(old).unwrap();
    assert_eq!(migrated.workspace, Workspace::default());
    let current = include_str!("../../../docs/interfaces/canvas-pilot-v11.example.json");
    assert_eq!(Project::from_json(current).unwrap(), migrated);
    let mut p = Project::from_layout(&Layout::default());
    p.layers[0].folder = "Memory A".into();
    p.workspace.collapsed_folders = vec!["Memory A".into()];
    p.workspace.resize_mode = canvas_pilot::ResizeMode::Opposite;
    p.workspace.grid_spacing = Some(100);
    p.workspace.snapping.grid_divisions = [13, 7];
    p.workspace.snapping.enabled = true;
    p.workspace.capture_interval_seconds = 37;
    p.workspace.show_empty_inputs = true;
    p.workspace.previews.push(Preview {
        layer_id: p.layers[0].id,
        source_id: p.layers[0].source_id,
        input: 8,
        host: "192.168.10.146:3000".into(),
        enabled: true,
        automatic: true,
        guide_opacity: 0.27,
        name: "Input 8".into(),
        image_base64: String::new(),
    });
    assert_eq!(Project::from_json(&p.to_json().unwrap()).unwrap(), p);
    let good = serde_json::to_value(&p).unwrap();
    for (path, value) in [
        ("/workspace/capture_interval_seconds", serde_json::json!(0)),
        ("/workspace/grid_spacing", serde_json::json!(0)),
        (
            "/workspace/snapping/grid_divisions/0",
            serde_json::json!(65),
        ),
        ("/workspace/resize_mode", serde_json::json!("unknown")),
        (
            "/workspace/collapsed_folders/0",
            serde_json::json!("missing"),
        ),
        ("/workspace/previews/0/layer_id", serde_json::json!(999)),
        ("/workspace/previews/0/source_id", serde_json::json!(999)),
        ("/workspace/previews/0/input", serde_json::json!(1025)),
        (
            "/workspace/previews/0/host",
            serde_json::json!("https://example.org/"),
        ),
        (
            "/workspace/previews/0/guide_opacity",
            serde_json::json!(1.1),
        ),
        (
            "/workspace/previews/0/image_base64",
            serde_json::json!("!bad"),
        ),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(path).unwrap() = value;
        assert!(Project::from_json(&bad.to_string()).is_err(), "{path}");
    }
    let mut bad = p.clone();
    bad.workspace
        .previews
        .push(bad.workspace.previews[0].clone());
    assert!(bad.to_json().is_err());
    let mut old: serde_json::Value = serde_json::from_str(old).unwrap();
    old["workspace"] = good["workspace"].clone();
    assert!(Project::from_json(&old.to_string()).is_err());
    let mut missing = good.clone();
    missing.as_object_mut().unwrap().remove("workspace");
    assert!(Project::from_json(&missing.to_string()).is_err());
}

#[test]
fn input_proposals_migrate_validate_and_require_explicit_version_16() {
    let old = include_str!("../../../docs/interfaces/canvas-pilot-v15.example.json");
    let migrated = Project::from_json(old).unwrap();
    assert_eq!(migrated.version, 19);
    assert!(migrated.input_proposals.is_empty());
    let fixture = include_str!("../../../docs/interfaces/canvas-pilot-v16.example.json");
    let project = Project::from_json(fixture).unwrap();
    assert_eq!(project.input_proposals.len(), 1);
    assert_eq!(
        Project::from_json(&project.to_json().unwrap()).unwrap(),
        project
    );
    let mut json = serde_json::to_value(&project).unwrap();
    json.as_object_mut().unwrap().remove("input_proposals");
    assert!(Project::from_json(&json.to_string()).is_err());
    for (pointer, value) in [
        ("/version", serde_json::json!(15)),
        ("/input_proposals/0/layer_id", serde_json::json!(999)),
        (
            "/input_proposals/0/input",
            serde_json::json!("INPUT_4/../../source"),
        ),
        ("/input_proposals/0/plug", serde_json::json!("0")),
        (
            "/input_proposals/0/device_link/layer",
            serde_json::json!("999"),
        ),
        (
            "/input_proposals/0/settings/aspect~1pp~1customRatio",
            serde_json::json!(0),
        ),
    ] {
        let mut invalid = serde_json::to_value(&project).unwrap();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(
            Project::from_json(&invalid.to_string()).is_err(),
            "{pointer}"
        );
    }
    let mut invalid = project.clone();
    invalid.input_proposals[0]
        .settings
        .insert("source/pp/input".into(), serde_json::json!("INPUT_9"));
    assert!(invalid.to_json().is_err());
    let mut duplicate = project.clone();
    duplicate
        .input_proposals
        .push(project.input_proposals[0].clone());
    assert!(duplicate.to_json().is_err());
}

#[test]
fn mask_snapping_migration_and_strict_round_trip() {
    let old = include_str!("../../../docs/interfaces/canvas-pilot-v17.example.json");
    let current = include_str!("../../../docs/interfaces/canvas-pilot-v18.example.json");
    let mut project = Project::from_json(old).unwrap();
    assert_eq!(project, Project::from_json(current).unwrap());
    assert!(!project.workspace.snapping.mask_edges);
    project.workspace.snapping.mask_edges = true;
    assert_eq!(
        Project::from_json(&project.to_json().unwrap()).unwrap(),
        project
    );
    let mut value = serde_json::to_value(project).unwrap();
    value["version"] = 17.into();
    assert!(Project::from_json(&value.to_string()).is_err());
    value["version"] = 19.into();
    value["workspace"]["snapping"]
        .as_object_mut()
        .unwrap()
        .remove("mask_edges");
    assert!(Project::from_json(&value.to_string()).is_err());
}
