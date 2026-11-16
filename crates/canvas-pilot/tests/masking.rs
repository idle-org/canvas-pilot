use canvas_pilot::{
    Corner, Geometry, Layout, ResizeHandle,
    formats::{Crop, LayerAppearance, LayerFit, Mask},
    project::Project,
};
fn mask() -> Mask {
    Mask {
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
    let migrated = Project::from_json(old).unwrap();
    assert_eq!(migrated.version, 15);
    assert!(migrated.layers.iter().all(|l| l.mask == Mask::default()));
    let mut json = serde_json::to_value(&p).unwrap();
    json["layers"][0].as_object_mut().unwrap().remove("mask");
    assert!(Project::from_json(&json.to_string()).is_err());
    let mut json = serde_json::to_value(&p).unwrap();
    json["version"] = 14.into();
    assert!(Project::from_json(&json.to_string()).is_err());
    for invalid in [
        Mask {
            left: -0.1,
            ..mask()
        },
        Mask {
            left: f64::INFINITY,
            ..mask()
        },
        Mask {
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
