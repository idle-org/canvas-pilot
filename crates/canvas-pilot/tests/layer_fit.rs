use canvas_pilot::{
    Geometry, Layout,
    formats::{Crop, LayerFit},
    project::Project,
};

fn box_geometry() -> Geometry {
    Geometry {
        x: 560.0,
        y: 240.0,
        width: 800.0,
        height: 600.0,
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}

#[test]
fn four_modes_apply_after_crop_and_center_the_image() {
    let g = box_geometry();
    for (mode, size, offset) in [
        (LayerFit::OneToOne, [1920.0, 1080.0], [-560.0, -240.0]),
        (LayerFit::Centered, [800.0, 450.0], [0.0, 75.0]),
        (LayerFit::Fullscreen, [800.0, 600.0], [0.0, 0.0]),
        (
            LayerFit::Cropped,
            [1066.6666666666667, 600.0],
            [-133.33333333333334, 0.0],
        ),
    ] {
        let image = mode.image([1920.0, 1080.0], g).unwrap();
        near(image.width, size[0]);
        near(image.height, size[1]);
        near(image.x, g.x + offset[0]);
        near(image.y, g.y + offset[1]);
        // Removing 480 pixels produces 4:3 content, so fit/fill/stretch converge.
        let retained = Crop {
            left: 480.0,
            ..Default::default()
        }
        .remaining([1920.0, 1080.0])
        .unwrap();
        if mode != LayerFit::OneToOne {
            assert_eq!(mode.image(retained, g).unwrap(), g);
        }
    }
    let small = LayerFit::OneToOne.image([640.0, 360.0], g).unwrap();
    assert_eq!(
        small,
        Geometry {
            x: 640.0,
            y: 360.0,
            width: 640.0,
            height: 360.0
        }
    );
    let odd = LayerFit::Centered
        .image(
            [1920.0, 1080.0],
            Geometry {
                width: 801.0,
                height: 601.0,
                ..g
            },
        )
        .unwrap();
    near(odd.height, 450.5625);
    near(odd.y, g.y + 75.21875);
    assert!(LayerFit::Centered.image([0.0, 1080.0], g).is_err());
}

#[test]
fn explicit_fit_survives_resize_undo_reordering_and_project_roundtrip() {
    let mut layout = Layout::default();
    let crop = Crop {
        left: 120.0,
        top: 60.0,
        ..Default::default()
    };
    layout
        .set_layer_fit(0, Some(LayerFit::OneToOne), crop)
        .unwrap();
    layout.apply(0, box_geometry()).unwrap();
    assert_eq!(layout.formats().advanced[0], crop);
    let before = Project::from_layout(&layout);
    assert_eq!(before.version, 17);
    let loaded = Project::from_json(&before.to_json().unwrap()).unwrap();
    assert_eq!(loaded, before);
    assert_eq!(
        loaded.layout().unwrap().formats().layer_fit[0],
        Some(LayerFit::OneToOne)
    );
    let id = layout.layer_info(0).id;
    layout.move_layer(id, 1).unwrap();
    assert_eq!(layout.formats().layer_fit[1], Some(LayerFit::OneToOne));
    layout.undo();
    layout.set_layer_fit(0, None, crop).unwrap();
    assert_eq!(layout.formats().layer_fit[0], None);
    assert!(layout.formats().mapping(0, layout.layers()[0]).is_ok());
    layout.undo();
    assert_eq!(layout.formats().layer_fit[0], Some(LayerFit::OneToOne));
    assert_eq!(layout.formats().advanced[0], crop);
}

#[test]
fn v13_migration_defaults_to_normal_crop_and_v14_requires_valid_fit() {
    let text = include_str!("../../../docs/interfaces/canvas-pilot-v13.example.json");
    let migrated = Project::from_json(text).unwrap();
    assert_eq!(migrated.version, 17);
    assert!(migrated.layers.iter().all(|l| l.layer_fit.is_none()));
    let mut json = serde_json::to_value(&migrated).unwrap();
    json["layers"][0]
        .as_object_mut()
        .unwrap()
        .remove("layer_fit");
    assert!(Project::from_json(&json.to_string()).is_err());
    json["layers"][0]["layer_fit"] = "input_setting".into();
    assert!(Project::from_json(&json.to_string()).is_err());
}

#[test]
fn current_documented_example_is_valid() {
    let p = Project::from_json(include_str!(
        "../../../docs/interfaces/canvas-pilot-v14.example.json"
    ))
    .unwrap();
    assert_eq!(p.version, 17);
    p.layout().unwrap();
}

#[test]
fn one_to_one_handles_trim_source_edges_without_zoom_and_undo_atomically() {
    use canvas_pilot::{Corner, ResizeHandle};
    let g = box_geometry();
    for (handle, delta, changes) in [
        (ResizeHandle::Left, [25.0, 99.0], [50.0, 0.0, 0.0, 0.0]),
        (ResizeHandle::Right, [-25.0, 99.0], [0.0, 50.0, 0.0, 0.0]),
        (ResizeHandle::Top, [99.0, 15.0], [0.0, 0.0, 30.0, 0.0]),
        (ResizeHandle::Bottom, [99.0, -15.0], [0.0, 0.0, 0.0, 30.0]),
        (
            ResizeHandle::Corner(Corner::TopLeft),
            [25.0, 15.0],
            [50.0, 0.0, 30.0, 0.0],
        ),
        (
            ResizeHandle::Corner(Corner::TopRight),
            [-25.0, 15.0],
            [0.0, 50.0, 30.0, 0.0],
        ),
        (
            ResizeHandle::Corner(Corner::BottomLeft),
            [25.0, -15.0],
            [50.0, 0.0, 0.0, 30.0],
        ),
        (
            ResizeHandle::Corner(Corner::BottomRight),
            [-25.0, -15.0],
            [0.0, 50.0, 0.0, 30.0],
        ),
    ] {
        let mut layout = Layout::default();
        layout.apply(0, g).unwrap();
        layout.select(Some(0));
        let initial = Crop {
            left: 100.0,
            right: 80.0,
            top: 60.0,
            bottom: 40.0,
        };
        layout
            .set_layer_fit(0, Some(LayerFit::OneToOne), initial)
            .unwrap();
        let before = Project::from_layout(&layout);
        let input = layout.formats().input(0).unwrap();
        let image = LayerFit::OneToOne
            .image(initial.remaining(input).unwrap(), g)
            .unwrap();
        let start = ResizeHandle::positions(image)
            .into_iter()
            .find(|(h, _)| *h == handle)
            .unwrap()
            .1;
        layout.start_content_resize(0, handle, start);
        layout.use_resize_pivot(true); // 1:1 never scales about a pivot.
        layout.drag_to([start[0] + delta[0] / 2.0, start[1] + delta[1] / 2.0]);
        layout.drag_to([start[0] + delta[0], start[1] + delta[1]]);
        layout.finish_drag();
        let crop = layout.formats().advanced[0];
        assert_eq!(
            [crop.left, crop.right, crop.top, crop.bottom],
            [
                100.0 + changes[0],
                80.0 + changes[1],
                60.0 + changes[2],
                40.0 + changes[3]
            ]
        );
        assert_eq!(layout.layers()[0], g);
        let retained = crop.remaining(input).unwrap();
        let fitted = LayerFit::OneToOne.image(retained, g).unwrap();
        assert_eq!([fitted.width, fitted.height], retained);
        assert_eq!(fitted.center(), g.center());
        let after = Project::from_layout(&layout);
        layout.undo();
        assert_eq!(Project::from_layout(&layout), before);
        layout.redo();
        assert_eq!(Project::from_layout(&layout), after);
        layout.start_content_resize(0, handle, start);
        layout.drag_to([start[0] + 100.0, start[1] + 100.0]);
        layout.cancel_drag();
        assert_eq!(Project::from_layout(&layout), after);
    }
}

#[test]
fn one_to_one_crop_stops_at_source_limits_and_uncropped_image_cannot_move() {
    use canvas_pilot::ResizeHandle;
    let mut layout = Layout::default();
    layout.select(Some(0));
    layout
        .set_layer_fit(0, Some(LayerFit::OneToOne), Crop::default())
        .unwrap();
    let input = layout.formats().input(0).unwrap();
    layout.start_content_resize(0, ResizeHandle::Left, [0.0, 0.0]);
    layout.drag_to([100000.0, 0.0]);
    let crop = layout.formats().advanced[0];
    near(crop.remaining(input).unwrap()[0], 0.001);
    layout.drag_to([-100000.0, 0.0]);
    assert_eq!(layout.formats().advanced[0], Crop::default());
    layout.finish_drag();
    layout.start_content_drag(0, [0.0, 0.0]);
    layout.drag_to([50.0, -50.0]);
    layout.finish_drag();
    assert_eq!(layout.formats().advanced[0], Crop::default());
    layout
        .set_layer_fit(0, Some(LayerFit::Centered), Crop::default())
        .unwrap();
    layout.start_content_resize(0, ResizeHandle::Left, [0.0, 0.0]);
    assert!(!layout.dragging());
}

#[test]
fn one_to_one_pan_preserves_crop_size_and_clamps_each_axis_with_undo_cancel() {
    let mut layout = Layout::default();
    layout.select(Some(0));
    let crop = Crop {
        left: 100.0,
        right: 80.0,
        top: 60.0,
        bottom: 40.0,
    };
    layout
        .set_layer_fit(0, Some(LayerFit::OneToOne), crop)
        .unwrap();
    let before = Project::from_layout(&layout);
    let input = layout.formats().input(0).unwrap();
    let g = layout.layers()[0];
    layout.start_content_drag(0, [500.0, 400.0]);
    layout.drag_to([510.0, 395.0]);
    layout.drag_to([520.0, 390.0]);
    layout.finish_drag();
    assert_eq!(
        layout.formats().advanced[0],
        Crop {
            left: 80.0,
            right: 100.0,
            top: 70.0,
            bottom: 30.0
        }
    );
    assert_eq!(
        layout.formats().advanced[0].remaining(input).unwrap(),
        crop.remaining(input).unwrap()
    );
    assert_eq!(layout.layers()[0], g);
    assert_eq!(layout.formats().mapping(0, g).unwrap().scale, 1.0);
    let after = Project::from_layout(&layout);
    layout.undo();
    assert_eq!(Project::from_layout(&layout), before);
    layout.redo();
    assert_eq!(Project::from_layout(&layout), after);
    layout.start_content_drag(0, [0.0, 0.0]);
    layout.drag_to([100000.0, -100000.0]);
    assert_eq!(
        layout.formats().advanced[0],
        Crop {
            left: 0.0,
            right: 180.0,
            top: 100.0,
            bottom: 0.0
        }
    );
    // Moving back uses the gesture's starting crop, so clamping cannot accumulate drift.
    layout.drag_to([-100000.0, 100000.0]);
    assert_eq!(
        layout.formats().advanced[0],
        Crop {
            left: 180.0,
            right: 0.0,
            top: 0.0,
            bottom: 100.0
        }
    );
    layout.cancel_drag();
    assert_eq!(Project::from_layout(&layout), after);
    assert!(crop.pan_image([f64::NAN, 0.0]).is_err());
}
