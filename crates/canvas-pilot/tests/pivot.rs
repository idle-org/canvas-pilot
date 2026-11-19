use canvas_pilot::{Geometry, Layout, ResizeHandle, ResizeMode, snapping::Snapping};

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
fn pivot(g: Geometry, relative: [f64; 2]) -> [f64; 2] {
    [g.x + g.width * relative[0], g.y + g.height * relative[1]]
}

#[test]
fn all_handles_keep_custom_pivots_fixed_in_each_resize_mode() {
    for mode in [
        ResizeMode::Individual,
        ResizeMode::Same,
        ResizeMode::Mirror,
        ResizeMode::Opposite,
    ] {
        for ratio in [false, true] {
            for relative in [[0.5, 0.5], [0.2, 0.7], [-0.25, 1.25]] {
                for index in [0, 1] {
                    let mut layout = Layout::default();
                    layout
                        .apply(
                            1,
                            Geometry {
                                width: 320.0,
                                height: 240.0,
                                ..layout.layers()[1]
                            },
                        )
                        .unwrap();
                    let before = layout.layers().clone();
                    for (handle, start) in ResizeHandle::positions(before[index]) {
                        layout.set_selection([true; 2]);
                        let formats = layout.formats().clone();
                        layout
                            .start_selected_resize(index, handle, start, ratio, mode)
                            .unwrap();
                        layout.set_resize_pivot(relative);
                        layout.use_resize_pivot(true);
                        layout.drag_to([start[0] + 17.0, start[1] + 11.0]);
                        for (i, original) in before.iter().enumerate() {
                            if mode == ResizeMode::Individual && i != index {
                                assert_eq!(layout.layers()[i], *original);
                                continue;
                            }
                            let mut rp = relative;
                            if i != index {
                                if matches!(mode, ResizeMode::Mirror | ResizeMode::Opposite) {
                                    rp[0] = 1.0 - rp[0];
                                }
                                if mode == ResizeMode::Opposite {
                                    rp[1] = 1.0 - rp[1];
                                }
                            }
                            let old = pivot(*original, rp);
                            let new = pivot(layout.layers()[i], rp);
                            near(old[0], new[0]);
                            near(old[1], new[1]);
                            if ratio {
                                near(
                                    layout.layers()[i].width / layout.layers()[i].height,
                                    original.width / original.height,
                                );
                            }
                        }
                        let resized = layout.layers().clone();
                        layout.finish_drag();
                        layout.undo();
                        assert_eq!(layout.layers(), &before);
                        assert_eq!(layout.formats(), &formats);
                        layout.redo();
                        assert_eq!(layout.layers(), &resized);
                        layout.undo();
                    }
                }
            }
        }
    }
}

#[test]
fn pivot_toggle_recomputes_without_drift_and_degenerate_handles_stay_valid() {
    let mut layout = Layout::default();
    let before = layout.layers().clone();
    layout.start_handle_resize(1, ResizeHandle::Right, [1800.0, 360.0], false);
    for _ in 0..3 {
        layout.use_resize_pivot(true);
        layout.drag_to([1900.0, 360.0]);
        near(layout.layers()[1].center()[0], before[1].center()[0]);
        near(layout.layers()[1].width, 840.0);
        layout.use_resize_pivot(false);
        layout.drag_to([1900.0, 360.0]);
        near(layout.layers()[1].x, before[1].x);
        near(layout.layers()[1].width, 740.0);
    }
    layout.set_resize_pivot([1.0, 0.5]);
    layout.use_resize_pivot(true);
    layout.drag_to([1900.0, 360.0]);
    assert_eq!(layout.layers(), &before);
    layout.cancel_drag();
    assert_eq!(layout.layers(), &before);
}

#[test]
fn pivot_resizing_preserves_screen_and_mirror_contact_snaps() {
    for ratio in [false, true] {
        let mut layout = Layout::default();
        let before = layout.layers().clone();
        layout.set_selection([true; 2]);
        layout
            .start_selected_resize(
                0,
                ResizeHandle::Right,
                [760.0, 360.0],
                ratio,
                ResizeMode::Mirror,
            )
            .unwrap();
        layout.set_resize_pivot([0.25, 0.6]);
        layout.use_resize_pivot(true);
        let guides = layout.drag_to_snapped(
            [958.0, 360.0],
            Snapping {
                enabled: true,
                ..Default::default()
            },
            6.0,
        );
        assert_eq!(guides[0], Some(960.0));
        near(
            layout.layers()[0].x + layout.layers()[0].width,
            layout.layers()[1].x,
        );
        near(
            pivot(layout.layers()[0], [0.25, 0.6])[0],
            pivot(before[0], [0.25, 0.6])[0],
        );
        layout.cancel_drag();
        layout.start_handle_resize(1, ResizeHandle::Right, [1800.0, 360.0], ratio);
        layout.use_resize_pivot(true);
        let guides = layout.drag_to_snapped(
            [1918.0, 360.0],
            Snapping {
                enabled: true,
                ..Default::default()
            },
            6.0,
        );
        assert_eq!(guides[0], Some(1920.0));
        near(layout.layers()[1].center()[0], before[1].center()[0]);
    }
}

#[test]
fn ratio_locked_corner_snaps_without_shifting_pivot() {
    use canvas_pilot::Corner;
    let mut layout = Layout::default();
    let g = layout.layers()[1];
    let relative = [0.2, 0.7];
    let fixed = pivot(g, relative);
    layout.start_handle_resize(
        1,
        ResizeHandle::Corner(Corner::BottomRight),
        [1800.0, 540.0],
        true,
    );
    layout.set_resize_pivot(relative);
    layout.use_resize_pivot(true);
    let point = [
        1918.0,
        540.0 + 118.0 * (540.0 - fixed[1]) / (1800.0 - fixed[0]),
    ];
    let guides = layout.drag_to_snapped(
        point,
        Snapping {
            enabled: true,
            ..Default::default()
        },
        6.0,
    );
    assert_eq!(guides[0], Some(1920.0));
    near(pivot(layout.layers()[1], relative)[0], fixed[0]);
    near(pivot(layout.layers()[1], relative)[1], fixed[1]);
    near(
        layout.layers()[1].width / layout.layers()[1].height,
        g.width / g.height,
    );
}

#[test]
fn pivot_placement_snaps_to_all_nine_anchors_even_with_snapping_off_or_grid_on() {
    let mut layout = Layout::default();
    let g = Geometry {
        x: 120.25,
        y: 180.75,
        width: 641.5,
        height: 361.25,
    };
    layout.apply(0, g).unwrap();
    let before = layout.layers().clone();
    for scale in [0.25, 1.0, 2.0] {
        for target in ResizeHandle::positions(g)
            .into_iter()
            .map(|(_, p)| p)
            .chain(std::iter::once(g.center()))
        {
            let point = [target[0] + 2.0 / scale, target[1] - 1.0 / scale];
            for grid in [None, Some(10.0), Some(100.0)] {
                assert_eq!(
                    layout.snap_pivot(0, point, Snapping::default(), 6.0 / scale, grid, true),
                    (target, target.map(Some))
                );
            }
            assert_eq!(
                layout.snap_pivot(0, point, Snapping::default(), 6.0 / scale, None, false),
                (point, [None; 2])
            );
        }
    }
    assert_eq!(layout.layers(), &before);
}

#[test]
fn pivot_placement_honors_screen_layer_pixel_and_grid_options() {
    let mut layout = Layout::default();
    let mut options = Snapping {
        enabled: true,
        pixels: false,
        screen_edges: true,
        layer_edges: false,
        ..Default::default()
    };
    assert_eq!(
        layout.snap_pivot(1, [3.0, 600.0], options, 6.0, None, true),
        ([0.0, 600.0], [Some(0.0), None])
    );
    options.screen_edges = false;
    options.layer_edges = true;
    layout.set_locked(0, true);
    assert_eq!(
        layout.snap_pivot(1, [763.0, 600.0], options, 6.0, None, true),
        ([760.0, 600.0], [Some(760.0), None])
    );
    layout.set_visible(0, false);
    assert_eq!(
        layout.snap_pivot(1, [763.0, 600.0], options, 6.0, None, true),
        ([763.0, 600.0], [None; 2])
    );
    options.layer_edges = false;
    options.pixels = true;
    assert_eq!(
        layout.snap_pivot(1, [833.2, 653.8], options, 6.0, None, true),
        ([833.0, 654.0], [None; 2])
    );
    assert_eq!(
        layout.snap_pivot(1, [833.3, 653.7], options, 6.0, None, true),
        ([833.3, 653.7], [None; 2])
    );
    for spacing in [10.0, 100.0] {
        let target = [833.3_f64, 653.7].map(|p| (p / spacing).round() * spacing);
        assert_eq!(
            layout.snap_pivot(
                1,
                [833.3, 653.7],
                Snapping::default(),
                6.0,
                Some(spacing),
                true
            ),
            (target, target.map(Some))
        );
    }
    // Being near an edge alone is not one of the nine local point targets.
    assert_eq!(
        layout.snap_pivot(1, [1162.0, 270.0], Snapping::default(), 6.0, None, true),
        ([1162.0, 270.0], [None; 2])
    );
}

#[test]
fn stored_pivots_follow_layers_and_edits_are_atomic() {
    let mut layout = Layout::default();
    let geometry = layout.layers().clone();
    let custom = [-0.25, 0.75];
    layout.set_layer_pivots(&[(0, custom)]).unwrap();
    layout.select(Some(1));
    assert_eq!(layout.layer_info(0).pivot, custom);
    assert_eq!(layout.layers(), &geometry);
    for invalid in [[f64::NAN, 0.0], [f64::INFINITY, 0.0], [1e7, 0.0]] {
        assert!(
            layout
                .set_layer_pivots(&[(1, [0.0; 2]), (0, invalid)])
                .is_err()
        );
        assert_eq!(layout.layer_info(1).pivot, [0.5; 2]);
    }
    layout.undo();
    assert_eq!(layout.layer_info(0).pivot, [0.5; 2]);
    layout.redo();
    let id = layout.layer_info(0).id;
    layout.move_layer(id, 1).unwrap();
    assert_eq!(layout.layer_info(1).id, id);
    assert_eq!(layout.layer_info(1).pivot, custom);
}

#[test]
fn grouped_resize_keeps_each_stored_pivot_fixed() {
    for mode in [
        ResizeMode::Individual,
        ResizeMode::Same,
        ResizeMode::Mirror,
        ResizeMode::Opposite,
    ] {
        let mut layout = Layout::default();
        let relative = [[0.2, 0.7], [0.6, 0.3]];
        layout
            .set_layer_pivots(&[(0, relative[0]), (1, relative[1])])
            .unwrap();
        let before = layout.layers().clone();
        layout.set_selection([true; 2]);
        let start = [
            before[0].x + before[0].width,
            before[0].y + before[0].height,
        ];
        layout
            .start_selected_resize(
                0,
                ResizeHandle::Corner(canvas_pilot::Corner::BottomRight),
                start,
                false,
                mode,
            )
            .unwrap();
        layout.use_resize_pivot(true);
        layout.drag_to([start[0] + 30.0, start[1] + 20.0]);
        for i in 0..2 {
            let fixed = pivot(before[i], relative[i]);
            let after = pivot(layout.layers()[i], relative[i]);
            near(fixed[0], after[0]);
            near(fixed[1], after[1]);
            assert_eq!(layout.layer_info(i).pivot, relative[i]);
        }
    }
}

#[test]
fn crop_zoom_keeps_source_pixel_under_saved_pivot_fixed_and_never_exposes_empty_space() {
    use canvas_pilot::formats::Framing;
    for relative in [[0.5, 0.5], [0.2, 0.7], [-0.25, 1.25], [0.0, 0.0]] {
        let mut layout = Layout::default();
        let i = 1;
        let g = layout.layers()[i];
        layout.set_layer_pivots(&[(i, relative)]).unwrap();
        layout
            .set_framing(
                i,
                Framing {
                    zoom: 2.0,
                    offset: [20.0, -10.0],
                },
            )
            .unwrap();
        let before = layout.formats().clone();
        let mapping = before.mapping(i, g).unwrap();
        let crop = before.advanced[i];
        let input = before.input(i).unwrap();
        let full = Geometry {
            x: g.x - crop.left * mapping.scale,
            y: g.y - crop.top * mapping.scale,
            width: input[0] * mapping.scale,
            height: input[1] * mapping.scale,
        };
        let fixed = pivot(g, relative);
        let source = [
            (fixed[0] - full.x) / mapping.scale,
            (fixed[1] - full.y) / mapping.scale,
        ];
        for (handle, start) in ResizeHandle::positions(full) {
            for factor in [0.001, 0.8, 1.2, 1000.0] {
                layout.start_content_resize(i, handle, start);
                layout.use_resize_pivot(true);
                let end = std::array::from_fn(|a| fixed[a] + (start[a] - fixed[a]) * factor);
                layout.drag_to(end);
                let m = layout.formats().mapping(i, g).unwrap();
                let c = layout.formats().advanced[i];
                near(g.x - c.left * m.scale + source[0] * m.scale, fixed[0]);
                near(g.y - c.top * m.scale + source[1] * m.scale, fixed[1]);
                assert!([c.left, c.right, c.top, c.bottom].iter().all(|v| *v >= 0.0));
                assert_eq!(layout.layers()[i], g);
                let resized = layout.formats().clone();
                // Modifier changes recompute from the gesture baseline without drift.
                layout.use_resize_pivot(false);
                layout.drag_to(end);
                layout.use_resize_pivot(true);
                layout.drag_to(end);
                assert_eq!(layout.formats(), &resized);
                layout.finish_drag();
                if resized != before {
                    layout.undo();
                    assert_eq!(layout.formats(), &before);
                    layout.redo();
                    assert_eq!(layout.formats(), &resized);
                }
                layout.start_content_resize(i, handle, start);
                layout.use_resize_pivot(true);
                layout.drag_to(end);
                layout.cancel_drag();
                assert_eq!(layout.formats(), &resized);
                layout.set_framing(i, mapping.framing).unwrap();
            }
        }
    }
}
