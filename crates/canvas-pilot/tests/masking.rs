use canvas_pilot::{
    Corner, Geometry, Layout, ResizeHandle,
    formats::{Crop, LayerAppearance, LayerFit, Mask},
    project::Project,
};
fn mask() -> Mask {
    Mask {
        origin: Default::default(),
        left: 0.2,
        right: 0.1,
        top: 0.15,
        bottom: 0.25,
    }
}
fn geometry() -> Geometry {
    Geometry {
        x: 100.0,
        y: 200.0,
        width: 800.0,
        height: 600.0,
    }
}
#[test]
fn masking_never_changes_crop_fit_source_or_geometry_and_undoes_as_one_gesture() {
    for fit in [
        None,
        Some(LayerFit::OneToOne),
        Some(LayerFit::Centered),
        Some(LayerFit::Cropped),
        Some(LayerFit::Fullscreen),
    ] {
        let mut l = Layout::default();
        l.select(Some(0));
        l.apply(0, geometry()).unwrap();
        l.set_layer_fit(0, fit, Crop::default()).unwrap();
        l.set_mask(0, mask()).unwrap();
        let before = Project::from_layout(&l);
        l.start_mask_drag(0, [0.0, 0.0], None);
        l.drag_to([40.0, 30.0]);
        l.drag_to([80.0, 60.0]);
        l.finish_drag();
        let mut expected = before.clone();
        expected.layers[0].mask = mask().translated([80.0, 60.0], geometry()).unwrap();
        assert_eq!(Project::from_layout(&l), expected);
        l.undo();
        assert_eq!(Project::from_layout(&l), before);
        l.redo();
        assert_eq!(Project::from_layout(&l), expected);
        l.start_mask_drag(0, [0.0, 0.0], Some(ResizeHandle::Left));
        l.drag_to([1000.0, 0.0]);
        l.cancel_drag();
        assert_eq!(Project::from_layout(&l), expected);
    }
}
#[test]
fn independent_handles_can_fully_blank_without_scaling_and_move_stops_at_edges() {
    for handle in [
        ResizeHandle::Left,
        ResizeHandle::Right,
        ResizeHandle::Top,
        ResizeHandle::Bottom,
        ResizeHandle::Corner(Corner::TopLeft),
        ResizeHandle::Corner(Corner::BottomRight),
    ] {
        let mut l = Layout::default();
        l.select(Some(0));
        l.apply(0, geometry()).unwrap();
        l.set_mask(0, mask()).unwrap();
        l.start_mask_drag(0, [0.0; 2], Some(handle));
        l.drag_to([80.0, 60.0]);
        l.finish_drag();
        let m = l.formats().masks[0];
        m.validate().unwrap();
        let horizontal = matches!(
            handle,
            ResizeHandle::Left | ResizeHandle::Right | ResizeHandle::Corner(_)
        );
        let vertical = matches!(
            handle,
            ResizeHandle::Top | ResizeHandle::Bottom | ResizeHandle::Corner(_)
        );
        assert_eq!(m.left != mask().left || m.right != mask().right, horizontal);
        assert_eq!(m.top != mask().top || m.bottom != mask().bottom, vertical);
        assert_eq!(l.layers()[0], geometry());
    }
    let mut l = Layout::default();
    l.select(Some(0));
    l.set_mask(0, mask()).unwrap();
    l.start_mask_drag(0, [0.0; 2], Some(ResizeHandle::Left));
    l.drag_to([1e6, 0.0]);
    l.finish_drag();
    assert_eq!(l.formats().masks[0].bounds(l.layers()[0]).width, 0.0);
    let moved = mask().translated([1e9, -1e9], geometry()).unwrap();
    assert_eq!(moved.right, 0.0);
    assert_eq!(moved.top, 0.0);
    assert!(mask().translated([f64::NAN, 0.0], geometry()).is_err());
}
#[test]
fn mask_schema_migration_validation_reorder_resize_and_atomic_import() {
    let mut l = Layout::default();
    l.set_mask(0, mask()).unwrap();
    let p = Project::from_layout(&l);
    assert_eq!(Project::from_json(&p.to_json().unwrap()).unwrap(), p);
    let old = include_str!("../../../docs/interfaces/canvas-pilot-v14.example.json");
    let migrated = canvas_pilot::project_recovery::from_json(old)
        .unwrap()
        .project;
    assert_eq!(migrated.version, 21);
    assert!(migrated.layers.iter().all(|l| l.mask == Mask::default()));
    let mut json = serde_json::to_value(&p).unwrap();
    json["layers"][0].as_object_mut().unwrap().remove("mask");
    assert!(Project::from_json(&json.to_string()).is_err());
    let mut json = serde_json::to_value(&p).unwrap();
    json["version"] = 14.into();
    assert!(Project::from_json(&json.to_string()).is_err());
    for invalid in [
        Mask {
            origin: Default::default(),
            left: -0.1,
            ..mask()
        },
        Mask {
            origin: Default::default(),
            left: f64::INFINITY,
            ..mask()
        },
        Mask {
            origin: Default::default(),
            left: 0.95,
            ..mask()
        },
    ] {
        assert!(l.set_mask(0, invalid).is_err());
    }
    let id = l.layer_info(0).id;
    l.move_layer(id, 1).unwrap();
    assert_eq!(l.formats().masks[1], mask());
    l.apply(1, geometry()).unwrap();
    assert_eq!(l.formats().masks[1], mask());
    let before = Project::from_layout(&l);
    assert!(
        l.import_device_layers_with_appearance(
            &[(Some(id), geometry(), None)],
            &[LayerAppearance {
                fit: None,
                mask: Mask {
                    origin: Default::default(),
                    left: 2.0,
                    ..mask()
                }
            }]
        )
        .is_err()
    );
    assert_eq!(Project::from_layout(&l), before);
    l.import_device_layers_with_appearance(
        &[(Some(id), geometry(), None)],
        &[LayerAppearance {
            fit: None,
            mask: Mask::default(),
        }],
    )
    .unwrap();
    assert_eq!(l.formats().masks[1], Mask::default());
    l.undo();
    assert_eq!(Project::from_layout(&l), before);
    Project::from_json(include_str!(
        "../../../docs/interfaces/canvas-pilot-v15.example.json"
    ))
    .unwrap();
}

#[test]
fn mask_alignment_moves_only_the_reveal_window() {
    use canvas_pilot::Alignment::*;
    for (axis, expected) in [
        (Left, [0.0, 0.3, 0.15, 0.25]),
        (Right, [0.3, 0.0, 0.15, 0.25]),
        (Top, [0.2, 0.1, 0.0, 0.4]),
        (Bottom, [0.2, 0.1, 0.4, 0.0]),
        (HorizontalCenter, [0.15, 0.15, 0.15, 0.25]),
        (VerticalCenter, [0.2, 0.1, 0.2, 0.2]),
    ] {
        let mut l = Layout::default();
        l.set_mask(0, mask()).unwrap();
        let before = Project::from_layout(&l);
        l.align_mask(0, &[axis]).unwrap();
        let m = l.formats().masks[0];
        for (a, b) in [m.left, m.right, m.top, m.bottom].into_iter().zip(expected) {
            assert!((a - b).abs() < 1e-9);
        }
        assert_eq!(l.formats().advanced[0], before.layers[0].advanced_crop);
        assert_eq!(l.layers()[0], before.layers[0].bounds);
        l.undo();
        assert_eq!(Project::from_layout(&l), before);
    }
}

#[test]
fn source_mask_reference_tracks_crop_fit_and_processed_source_dimensions() {
    use canvas_pilot::formats::MaskOrigin;
    let mask = Mask {
        origin: MaskOrigin::Source,
        left: 0.2,
        right: 0.1,
        top: 0.1,
        bottom: 0.2,
    };
    let layer = Geometry {
        x: 100.0,
        y: 200.0,
        width: 400.0,
        height: 200.0,
    };
    let crop = Crop {
        left: 100.0,
        right: 100.0,
        top: 50.0,
        bottom: 50.0,
    };
    let full = mask.reference([1000.0, 500.0], crop, None, layer).unwrap();
    assert_eq!(
        full,
        Geometry {
            x: 50.0,
            y: 175.0,
            width: 500.0,
            height: 250.0
        }
    );
    let reveal = mask.bounds(full);
    assert_eq!([reveal.x, reveal.y, reveal.height], [150.0, 200.0, 175.0]);
    assert!((reveal.width - 350.0).abs() < 1e-8);
    let tall = Geometry {
        height: 300.0,
        ..layer
    };
    for (fit, expected) in [
        (
            LayerFit::OneToOne,
            Geometry {
                x: -200.0,
                y: 100.0,
                width: 1000.0,
                height: 500.0,
            },
        ),
        (
            LayerFit::Centered,
            Geometry {
                x: 50.0,
                y: 225.0,
                width: 500.0,
                height: 250.0,
            },
        ),
        (
            LayerFit::Cropped,
            Geometry {
                x: -75.0,
                y: 162.5,
                width: 750.0,
                height: 375.0,
            },
        ),
        (
            LayerFit::Fullscreen,
            Geometry {
                x: 50.0,
                y: 162.5,
                width: 500.0,
                height: 375.0,
            },
        ),
    ] {
        assert_eq!(
            mask.reference([1000.0, 500.0], crop, Some(fit), tall)
                .unwrap(),
            expected
        );
    }
    let mut layout = Layout::default();
    layout.select(Some(0));
    layout
        .set_framing(
            0,
            canvas_pilot::formats::Framing {
                zoom: 2.0,
                offset: [0.0; 2],
            },
        )
        .unwrap();
    layout.set_mask(0, mask).unwrap();
    let before = Project::from_layout(&layout);
    let reference = layout
        .formats()
        .mask_reference(0, layout.layers()[0])
        .unwrap();
    assert!(reference.width > layout.layers()[0].width);
    layout.start_mask_drag(0, [0.0; 2], None);
    layout.drag_to([20.0, 10.0]);
    layout.finish_drag();
    assert_eq!(
        layout.formats().masks[0],
        mask.translated([20.0, 10.0], reference).unwrap()
    );
    layout.undo();
    assert_eq!(Project::from_layout(&layout), before);
    let source = canvas_pilot::formats::Source {
        native: [1820, 1080],
        aspect: canvas_pilot::formats::Aspect::Custom,
    };
    assert_eq!(source.input().unwrap(), [1820.0, 1080.0]);
    let reference = mask
        .reference(
            source.input().unwrap(),
            Crop::default(),
            Some(LayerFit::Fullscreen),
            layer,
        )
        .unwrap();
    assert_eq!(reference, layer);
}

#[test]
fn mask_origin_migrates_strictly_and_survives_project_save() {
    use canvas_pilot::formats::MaskOrigin;
    canvas_pilot::project_recovery::from_json(include_str!(
        "../../../docs/interfaces/canvas-pilot-v17.example.json"
    ))
    .unwrap();
    let clipboard = include_str!("../../../docs/interfaces/canvas-pilot-clipboard-v3.example.txt");
    let (_, json) = clipboard
        .rsplit_once(canvas_pilot::clipboard::MARKER)
        .unwrap();
    assert_eq!(
        canvas_pilot::clipboard::parse(json).unwrap().shapes[0]
            .appearance
            .unwrap()
            .mask
            .origin,
        MaskOrigin::Source
    );
    let old = include_str!("../../../docs/interfaces/canvas-pilot-v16.example.json");
    let migrated = canvas_pilot::project_recovery::from_json(old)
        .unwrap()
        .project;
    assert!(
        migrated
            .layers
            .iter()
            .all(|l| l.mask.origin == MaskOrigin::Layer)
    );
    let mut value = serde_json::to_value(&migrated).unwrap();
    value["layers"][0]["mask"]["origin"] = "source".into();
    let project = Project::from_json(&value.to_string()).unwrap();
    assert_eq!(project.layers[0].mask.origin, MaskOrigin::Source);
    assert_eq!(
        Project::from_json(&project.to_json().unwrap()).unwrap(),
        project
    );
    for bad in [serde_json::json!("unknown"), serde_json::Value::Null] {
        let mut invalid = value.clone();
        invalid["layers"][0]["mask"]["origin"] = bad;
        assert!(Project::from_json(&invalid.to_string()).is_err());
    }
    let mut missing = value.clone();
    missing["layers"][0]["mask"]
        .as_object_mut()
        .unwrap()
        .remove("origin");
    assert!(Project::from_json(&missing.to_string()).is_err());
    value["version"] = 16.into();
    assert!(Project::from_json(&value.to_string()).is_err());
}
