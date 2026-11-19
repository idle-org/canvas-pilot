use canvas_pilot::{Layout, snapping::Snapping};

#[test]
fn screen_snapping_recomputes_from_original_and_is_one_undo_step() {
    let mut layout = Layout::default();
    layout.set_visible(0, false);
    let before = layout.layers().clone();
    let formats = layout.formats().clone();
    let options = Snapping {
        enabled: true,
        layer_edges: true,
        ..Default::default()
    };
    layout.start_drag([1200.0, 200.0]);
    assert_eq!(
        layout.drag_to_snapped([1317.0, 201.0], options, 6.0),
        [Some(1920.0), Some(540.0)]
    );
    assert_eq!(layout.layers()[1].x, 1280.0);
    layout.drag_to_snapped([1250.0, 200.0], options, 6.0);
    assert_eq!(layout.layers()[1].x, before[1].x + 50.0);
    layout.drag_to_snapped([1317.0, 201.0], options, 6.0);
    layout.finish_drag();
    assert_eq!(*layout.formats(), formats);
    layout.undo();
    assert_eq!(*layout.layers(), before);
    layout.redo();
    assert_eq!(layout.layers()[1].x, 1280.0);
}
#[test]
fn visible_layer_edges_include_locked_targets_but_not_hidden_targets() {
    let options = Snapping {
        enabled: true,
        screen_edges: false,
        pixels: false,
        ..Default::default()
    };
    for hidden in [false, true] {
        let mut layout = Layout::default();
        layout.set_locked(0, true);
        if hidden {
            layout.set_visible(0, false);
        }
        layout.start_drag([1160.0, 180.0]);
        let guides = layout.drag_to_snapped([763.0, 190.0], options, 6.0);
        assert_eq!(guides[0], if hidden { None } else { Some(760.0) });
        assert_eq!(layout.layers()[1].x, if hidden { 763.0 } else { 760.0 });
    }
}
#[test]
fn group_snapping_preserves_spacing_and_cancel_restores_geometry() {
    let mut layout = Layout::default();
    layout.set_selection([true; 2]);
    let before = layout.layers().clone();
    layout.start_drag([200.0, 200.0]);
    layout.drag_to_snapped(
        [83.0, 200.0],
        Snapping {
            enabled: true,
            ..Default::default()
        },
        6.0,
    );
    assert_eq!(layout.layers()[0].x, 0.0);
    assert_eq!(
        layout.layers()[1].x - layout.layers()[0].x,
        before[1].x - before[0].x
    );
    layout.cancel_drag();
    assert_eq!(*layout.layers(), before);
    layout.start_drag([200.0, 200.0]);
    assert_eq!(
        layout.drag_to_snapped([83.0, 200.0], Snapping::default(), 6.0),
        [None; 2]
    );
    assert_eq!(layout.layers()[0].x, 3.0);
}
#[test]
fn pixel_snap_is_soft_and_optional_while_numeric_entry_bypasses_it() {
    let mut layout = Layout::default();
    let mut g = layout.layers()[1];
    g.x = 1160.25;
    layout.apply(1, g).unwrap();
    let options = |pixels| Snapping {
        enabled: true,
        pixels,
        screen_edges: false,
        layer_edges: false,
        ..Default::default()
    };
    for (pixels, pointer, expected) in [
        (false, 1199.9, 1160.15),
        (true, 1199.9, 1160.0),
        (true, 1200.2, 1160.45),
    ] {
        layout.start_drag([1200.0, 200.0]);
        layout.drag_to_snapped([pointer, 200.0], options(pixels), 6.0);
        assert!((layout.layers()[1].x - expected).abs() < 1e-8);
        layout.cancel_drag();
        assert_eq!(layout.layers()[1], g);
    }
}
#[test]
fn resize_snaps_to_screen_keeps_anchor_and_undo() {
    use canvas_pilot::ResizeHandle;
    let mut layout = Layout::default();
    layout.select(Some(0));
    let before = layout.layers().clone();
    let g = before[0];
    layout.start_handle_resize(
        0,
        ResizeHandle::Right,
        [g.x + g.width, g.y + g.height / 2.0],
        true,
    );
    let guides = layout.drag_to_snapped(
        [1918.0, g.y + g.height / 2.0],
        Snapping {
            enabled: true,
            pixels: false,
            screen_edges: true,
            layer_edges: false,
            ..Default::default()
        },
        6.0,
    );
    let next = layout.layers()[0];
    assert_eq!(guides, [Some(1920.0), None]);
    assert_eq!(next.x, g.x);
    assert_eq!(next.x + next.width, 1920.0);
    assert!((next.width / next.height - g.width / g.height).abs() < 1e-9);
    layout.finish_drag();
    layout.undo();
    assert_eq!(*layout.layers(), before);
}
#[test]
fn resize_can_bypass_snapping_and_cancel() {
    use canvas_pilot::ResizeHandle;
    let mut layout = Layout::default();
    layout.select(Some(0));
    let g = layout.layers()[0];
    layout.start_handle_resize(0, ResizeHandle::Top, [g.x + g.width / 2.0, g.y], false);
    let options = Snapping {
        enabled: true,
        pixels: false,
        screen_edges: true,
        layer_edges: false,
        ..Default::default()
    };
    assert_eq!(
        layout.drag_to_snapped([g.x + g.width / 2.0, 2.0], options, 6.0),
        [None, Some(0.0)]
    );
    assert_eq!(
        layout.layers()[0].y + layout.layers()[0].height,
        g.y + g.height
    );
    layout.drag_to_snapped(
        [g.x + g.width / 2.0, 2.0],
        Snapping {
            enabled: false,
            ..options
        },
        6.0,
    );
    assert_eq!(layout.layers()[0].y, 2.0);
    layout.cancel_drag();
    assert_eq!(layout.layers()[0], g);
}

#[test]
fn ratio_locked_corners_reach_fractional_layer_edges() {
    use canvas_pilot::{Corner, ResizeHandle};
    for corner in [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ] {
        let mut layout = Layout::default();
        let mut target = layout.layers()[1];
        target.x = 900.25;
        layout.apply(1, target).unwrap();
        layout.select(Some(0));
        let g = layout.layers()[0];
        let handle = ResizeHandle::Corner(corner);
        let start = ResizeHandle::positions(g)
            .into_iter()
            .find(|(h, _)| *h == handle)
            .unwrap()
            .1;
        let left = matches!(corner, Corner::TopLeft | Corner::BottomLeft);
        let target_x = if left { 0.0 } else { target.x };
        layout.start_handle_resize(0, handle, start, true);
        // Move along the aspect-ratio diagonal to just inside the snap radius.
        let dx = target_x - start[0] + if left { 2.0 } else { -2.0 };
        let same_sign = matches!(corner, Corner::TopLeft | Corner::BottomRight);
        let point = [
            start[0] + dx,
            start[1] + dx * g.height / g.width * if same_sign { 1.0 } else { -1.0 },
        ];
        let guides = layout.drag_to_snapped(
            point,
            Snapping {
                enabled: true,
                pixels: false,
                screen_edges: true,
                layer_edges: true,
                ..Default::default()
            },
            6.0,
        );
        let next = layout.layers()[0];
        let edge = if left { next.x } else { next.x + next.width };
        assert!((edge - target_x).abs() < 1e-6, "{corner:?}: {edge}");
        assert_eq!(guides[0], Some(target_x));
        assert!((next.width / next.height - g.width / g.height).abs() < 1e-9);
    }
}

#[test]
fn grid_movement_uses_group_bounds_and_preserves_fractional_spacing() {
    for spacing in [10.0, 100.0] {
        let mut layout = Layout::default();
        let mut g = layout.layers()[0];
        g.x = -123.25;
        g.y = -17.5;
        layout.apply(0, g).unwrap();
        layout.set_selection([true; 2]);
        let before = layout.layers().clone();
        let formats = layout.formats().clone();
        layout.start_drag([1200.0, 200.0]);
        let guides = layout.drag_to_grid([1207.0, 203.0], spacing);
        let x = ((g.x + 7.0) / spacing).round() * spacing;
        let y = ((g.y + 3.0) / spacing).round() * spacing;
        assert_eq!(guides, [Some(x), Some(y)]);
        assert_eq!(layout.layers()[0].x, x);
        assert_eq!(
            layout.layers()[1].x - layout.layers()[0].x,
            before[1].x - before[0].x
        );
        assert_eq!(*layout.formats(), formats);
        layout.finish_drag();
        layout.undo();
        assert_eq!(*layout.layers(), before);
    }
}

#[test]
fn grid_resize_respects_anchors_ratios_and_free_corner_axes() {
    use canvas_pilot::ResizeHandle;
    for spacing in [10.0, 100.0] {
        for keep_ratio in [false, true] {
            let mut layout = Layout::default();
            layout.select(Some(0));
            let mut g = layout.layers()[0];
            g.x = 123.25;
            g.y = 181.5;
            layout.apply(0, g).unwrap();
            for (handle, start) in ResizeHandle::positions(g) {
                layout.start_handle_resize(0, handle, start, keep_ratio);
                let guides = layout.drag_to_grid([start[0] + 37.0, start[1] + 23.0], spacing);
                let next = layout.layers()[0];
                let positions = ResizeHandle::positions(next);
                let actual = positions.iter().find(|(h, _)| *h == handle).unwrap().1;
                let anchor = positions
                    .iter()
                    .find(|(h, _)| *h == handle.opposite())
                    .unwrap()
                    .1;
                let original_anchor = ResizeHandle::positions(g)
                    .into_iter()
                    .find(|(h, _)| *h == handle.opposite())
                    .unwrap()
                    .1;
                for axis in 0..2 {
                    assert!((anchor[axis] - original_anchor[axis]).abs() < 1e-8);
                }
                assert!(guides.iter().any(Option::is_some));
                for axis in 0..2 {
                    if let Some(target) = guides[axis] {
                        assert!((actual[axis] - target).abs() < 1e-8);
                        assert!((target / spacing - (target / spacing).round()).abs() < 1e-8);
                    }
                }
                if keep_ratio {
                    assert!((next.width / next.height - g.width / g.height).abs() < 1e-8);
                } else if matches!(handle, ResizeHandle::Corner(_)) {
                    assert!(guides.iter().all(Option::is_some));
                }
                layout.cancel_drag();
                assert_eq!(layout.layers()[0], g);
            }
        }
    }
}

#[test]
fn grid_group_resize_keeps_mirror_and_opposite_relationships() {
    use canvas_pilot::{ResizeHandle, ResizeMode};
    for mode in [ResizeMode::Same, ResizeMode::Mirror, ResizeMode::Opposite] {
        let mut layout = Layout::default();
        layout.set_selection([true; 2]);
        let before = layout.layers().clone();
        layout
            .start_selected_resize(0, ResizeHandle::Right, [760.0, 360.0], true, mode)
            .unwrap();
        assert_eq!(
            layout.drag_to_grid([837.0, 360.0], 100.0),
            [Some(800.0), None]
        );
        assert_eq!(layout.layers()[0].width, 680.0);
        assert_eq!(layout.layers()[1].width, 680.0);
        if mode == ResizeMode::Same {
            assert_eq!(layout.layers()[1].x, before[1].x);
        } else {
            assert_eq!(
                layout.layers()[1].x + layout.layers()[1].width,
                before[1].x + before[1].width
            );
        }
        layout.finish_drag();
        layout.undo();
        assert_eq!(*layout.layers(), before);
    }
}

#[test]
fn grid_ignores_crop_content_and_invalid_spacing_does_not_edit() {
    use canvas_pilot::formats::Framing;
    let mut layout = Layout::default();
    layout
        .set_framing(
            1,
            Framing {
                zoom: 2.0,
                offset: [0.0; 2],
            },
        )
        .unwrap();
    let geometry = layout.layers().clone();
    layout.start_content_drag(1, [1200.0, 200.0]);
    assert_eq!(layout.drag_to_grid([1207.5, 203.5], 100.0), [None; 2]);
    let framing = layout
        .formats()
        .mapping(1, layout.layers()[1])
        .unwrap()
        .framing;
    assert!((framing.offset[0] - 7.5).abs() < 1e-8);
    assert!((framing.offset[1] - 3.5).abs() < 1e-8);
    assert_eq!(*layout.layers(), geometry);
    layout.cancel_drag();
    layout.start_drag([1200.0, 200.0]);
    for spacing in [0.0, -10.0, f64::INFINITY, f64::NAN] {
        assert_eq!(layout.drag_to_grid([1300.0, 300.0], spacing), [None; 2]);
        assert_eq!(*layout.layers(), geometry);
    }
}

#[test]
fn mirrored_layers_snap_to_exact_contact_from_either_handle_and_keep_anchors() {
    use canvas_pilot::{Corner, Geometry, ResizeHandle, ResizeMode};
    for index in [0, 1] {
        for corner in [false, true] {
            for ratio in [false, true] {
                for offset in [-2.0, 2.0] {
                    let mut layout = Layout::default();
                    layout
                        .apply(
                            0,
                            Geometry {
                                x: 100.25,
                                y: 200.0,
                                width: 400.0,
                                height: 200.0,
                            },
                        )
                        .unwrap();
                    layout
                        .apply(
                            1,
                            Geometry {
                                x: 800.25,
                                y: 200.0,
                                width: 600.0,
                                height: 300.0,
                            },
                        )
                        .unwrap();
                    layout.set_selection([true; 2]);
                    let before = layout.layers().clone();
                    let formats = layout.formats().clone();
                    let handle = match (index, corner) {
                        (0, false) => ResizeHandle::Right,
                        (0, true) => ResizeHandle::Corner(Corner::BottomRight),
                        (_, false) => ResizeHandle::Left,
                        (_, true) => ResizeHandle::Corner(Corner::BottomLeft),
                    };
                    let start = ResizeHandle::positions(before[index])
                        .into_iter()
                        .find(|(h, _)| *h == handle)
                        .unwrap()
                        .1;
                    let contact = 620.25;
                    let factor = if index == 0 {
                        (contact + offset - 100.25) / 400.0
                    } else {
                        (1400.25 - contact - offset) / 600.0
                    };
                    let point = [
                        contact + offset,
                        if corner && ratio {
                            200.0 + before[index].height * factor
                        } else {
                            start[1]
                        },
                    ];
                    layout
                        .start_selected_resize(index, handle, start, ratio, ResizeMode::Mirror)
                        .unwrap();
                    let options = Snapping {
                        enabled: true,
                        pixels: false,
                        screen_edges: false,
                        layer_edges: true,
                        ..Default::default()
                    };
                    let guides = layout.drag_to_snapped(point, options, 6.0);
                    assert_eq!(
                        guides,
                        [Some(contact), None],
                        "index={index} corner={corner} ratio={ratio}"
                    );
                    let a = layout.layers()[0];
                    let b = layout.layers()[1];
                    assert!((a.x + a.width - b.x).abs() < 1e-8);
                    assert_eq!(a.x, before[0].x);
                    assert!((b.x + b.width - 1400.25).abs() < 1e-8);
                    assert!((a.width / 400.0 - b.width / 600.0).abs() < 1e-8);
                    if ratio {
                        assert!((a.width / a.height - 2.0).abs() < 1e-8);
                        assert!((b.width / b.height - 2.0).abs() < 1e-8);
                    }
                    let snapped = layout.layers().clone();
                    layout.drag_to_snapped(point, options, 6.0);
                    assert_eq!(layout.layers(), &snapped);
                    layout.drag_to_snapped([point[0] + 40.0, point[1]], options, 6.0);
                    assert!(
                        (layout.layers()[0].x + layout.layers()[0].width - layout.layers()[1].x)
                            .abs()
                            > 1.0
                    );
                    layout.drag_to_snapped(point, options, 6.0);
                    layout.finish_drag();
                    layout.undo();
                    assert_eq!(layout.layers(), &before);
                    assert_eq!(layout.formats(), &formats);
                    layout.redo();
                    assert_eq!(layout.layers(), &snapped);
                }
            }
        }
    }
}

#[test]
fn meeting_snap_respects_disabled_options_and_requires_actual_edge_contact() {
    use canvas_pilot::{ResizeHandle, ResizeMode};
    for (enabled, layer_edges, separated) in [
        (false, true, false),
        (true, false, false),
        (true, true, true),
    ] {
        let mut layout = Layout::default();
        if separated {
            let mut g = layout.layers()[1];
            g.y = 2000.0;
            layout.apply(1, g).unwrap();
        }
        layout.set_selection([true; 2]);
        let before = layout.layers().clone();
        layout
            .start_selected_resize(
                0,
                ResizeHandle::Right,
                [760.0, 360.0],
                false,
                ResizeMode::Mirror,
            )
            .unwrap();
        assert_eq!(
            layout.drag_to_snapped(
                [958.0, 360.0],
                Snapping {
                    enabled,
                    layer_edges,
                    pixels: false,
                    screen_edges: false,
                    ..Default::default()
                },
                6.0
            ),
            [None; 2]
        );
        assert_eq!(layout.layers()[0].x + layout.layers()[0].width, 958.0);
        layout.cancel_drag();
        assert_eq!(layout.layers(), &before);
    }
}

#[test]
fn opposite_vertical_edges_snap_to_meet() {
    use canvas_pilot::{Geometry, ResizeHandle, ResizeMode};
    let mut layout = Layout::default();
    layout
        .apply(
            0,
            Geometry {
                x: 100.0,
                y: 100.0,
                width: 300.0,
                height: 200.0,
            },
        )
        .unwrap();
    layout
        .apply(
            1,
            Geometry {
                x: 100.0,
                y: 500.0,
                width: 300.0,
                height: 400.0,
            },
        )
        .unwrap();
    layout.set_selection([true; 2]);
    layout
        .start_selected_resize(
            0,
            ResizeHandle::Bottom,
            [250.0, 300.0],
            false,
            ResizeMode::Opposite,
        )
        .unwrap();
    let target = 100.0 + 800.0 / 3.0;
    let guides = layout.drag_to_snapped(
        [250.0, target - 2.0],
        Snapping {
            enabled: true,
            pixels: false,
            screen_edges: false,
            layer_edges: true,
            ..Default::default()
        },
        6.0,
    );
    assert!((guides[1].unwrap() - target).abs() < 1e-8);
    assert!((layout.layers()[0].y + layout.layers()[0].height - layout.layers()[1].y).abs() < 1e-8);
}

#[test]
fn screen_quarters_snap_moving_edges_and_centers_and_survive_release() {
    let options = Snapping {
        enabled: true,
        pixels: false,
        layer_edges: true,
        ..Default::default()
    };
    for fraction in [0.25, 0.5, 0.75] {
        for anchor in [0.0, 0.5, 1.0] {
            let mut layout = Layout::default();
            layout.set_visible(0, false);
            let g = layout.layers()[1];
            let target = [1920.0 * fraction, 1080.0 * fraction];
            let point = [
                target[0] - g.width * anchor + 2.0,
                target[1] - g.height * anchor - 2.0,
            ];
            layout.start_drag([g.x, g.y]);
            assert_eq!(
                layout.drag_to_snapped(point, options, 6.0),
                target.map(Some)
            );
            let snapped = layout.layers()[1];
            assert_eq!(snapped.x + g.width * anchor, target[0]);
            assert_eq!(snapped.y + g.height * anchor, target[1]);
            layout.finish_drag();
            assert_eq!(layout.layers()[1], snapped);
            layout.undo();
            assert_eq!(layout.layers()[1], g);
        }
    }
}

#[test]
fn screen_quarters_snap_resizes_and_pivots_and_respect_screen_toggle() {
    use canvas_pilot::ResizeHandle;
    let options = Snapping {
        enabled: true,
        pixels: false,
        layer_edges: false,
        ..Default::default()
    };
    for fraction in [0.25, 0.5, 0.75] {
        for axis in 0..2 {
            for enabled in [false, true] {
                let mut layout = Layout::default();
                layout.select(Some(0));
                let g = layout.layers()[0];
                let handle = if axis == 0 {
                    ResizeHandle::Right
                } else {
                    ResizeHandle::Bottom
                };
                let start = ResizeHandle::positions(g)
                    .into_iter()
                    .find(|(h, _)| *h == handle)
                    .unwrap()
                    .1;
                let target = layout.formats().output[axis] as f64 * fraction;
                let mut point = start;
                point[axis] = target + 2.0;
                let options = Snapping {
                    screen_edges: enabled,
                    ..options
                };
                let (pivot, guides) = layout.snap_pivot(0, point, options, 6.0, None, false);
                assert_eq!(guides[axis], enabled.then_some(target));
                assert_eq!(pivot[axis], if enabled { target } else { point[axis] });
                layout.start_handle_resize(0, handle, start, false);
                let guides = layout.drag_to_snapped(point, options, 6.0);
                assert_eq!(guides[axis], enabled.then_some(target));
                let next = layout.layers()[0];
                assert_eq!(
                    [next.x + next.width, next.y + next.height][axis],
                    if enabled { target } else { point[axis] }
                );
                assert_eq!([next.x, next.y], [g.x, g.y]);
            }
        }
    }
}

#[test]
fn overlay_grid_snaps_moves_resizes_and_pivots_with_fractional_divisions() {
    use canvas_pilot::ResizeHandle;
    let options = Snapping {
        enabled: true,
        pixels: false,
        screen_edges: false,
        layer_edges: true,
        grid: true,
        show_grid: false,
        grid_divisions: [7, 5],
        mask_edges: false,
    };
    let mut layout = Layout::default();
    let x = 1920.0 / 7.0;
    let y = 1080.0 / 5.0;
    layout.select(Some(0));
    let g = layout.layers()[0];
    layout.start_drag([g.x, g.y]);
    let guides = layout.drag_to_snapped([x + 2.0, y - 2.0], options, 6.0);
    assert_eq!(guides, [Some(x), Some(y)]);
    assert!((layout.layers()[0].x - x).abs() < 1e-8);
    assert!((layout.layers()[0].y - y).abs() < 1e-8);
    layout.finish_drag();
    layout.undo();
    assert_eq!(layout.layers()[0], g);
    layout.start_handle_resize(
        0,
        ResizeHandle::Right,
        [g.x + g.width, g.center()[1]],
        false,
    );
    let target = 3.0 * 1920.0 / 7.0;
    assert_eq!(
        layout.drag_to_snapped([target - 2.0, g.center()[1]], options, 6.0),
        [Some(target), None]
    );
    assert!((layout.layers()[0].x + layout.layers()[0].width - target).abs() < 1e-8);
    layout.cancel_drag();
    assert_eq!(
        layout.snap_pivot(0, [x + 1.0, y - 1.0], options, 6.0, None, false),
        ([x, y], [Some(x), Some(y)])
    );
    let disabled = Snapping {
        grid: false,
        ..options
    };
    assert_eq!(
        layout
            .snap_pivot(0, [x + 1.0, y - 1.0], disabled, 6.0, None, false)
            .1,
        [None; 2]
    );
    assert_eq!(options.grid_guides(1920, 0).len(), 8);
    assert_eq!(
        Snapping {
            grid_divisions: [0, u32::MAX],
            ..options
        }
        .grid_guides(1080, 1)
        .len(),
        65
    );
}

fn mask_options() -> Snapping {
    Snapping {
        enabled: true,
        pixels: false,
        screen_edges: false,
        layer_edges: true,
        mask_edges: true,
        ..Default::default()
    }
}

#[test]
fn masked_edges_snap_movement_pivot_and_resize_for_both_origins() {
    use canvas_pilot::{
        ResizeHandle,
        formats::{Mask, MaskOrigin},
    };
    for origin in [MaskOrigin::Layer, MaskOrigin::Source] {
        let mut layout = Layout::default();
        layout
            .set_mask(
                0,
                Mask {
                    origin,
                    right: 0.25,
                    ..Default::default()
                },
            )
            .unwrap();
        layout.set_locked(0, true);
        // First layer ends at 760, its picture at 600.
        let target = 600.0;
        let options = mask_options();
        assert_eq!(
            layout
                .snap_pivot(1, [target + 3.0, 300.0], options, 6.0, None, false)
                .0[0],
            target
        );
        let original = layout.layers().clone();
        layout.start_drag([1160.0, 200.0]);
        assert_eq!(
            layout.drag_to_snapped([target + 3.0, 210.0], options, 6.0)[0],
            Some(target)
        );
        assert_eq!(layout.layers()[1].x, target);
        layout.finish_drag();
        layout.undo();
        assert_eq!(*layout.layers(), original);
        layout.select(Some(1));
        layout.start_handle_resize(1, ResizeHandle::Left, [1160.0, 360.0], false);
        assert_eq!(
            layout.drag_to_snapped([target + 3.0, 360.0], options, 6.0)[0],
            Some(target)
        );
        assert_eq!(layout.layers()[1].x, target);
        assert_eq!(layout.layers()[1].x + layout.layers()[1].width, 1800.0);
        layout.cancel_drag();
        for hidden in [false, true] {
            layout.set_visible(0, !hidden);
            let mut disabled = options;
            disabled.mask_edges = false;
            assert_eq!(
                layout
                    .snap_pivot(1, [603.0, 300.0], disabled, 6.0, None, false)
                    .1[0],
                None
            );
            assert_eq!(
                layout
                    .snap_pivot(1, [603.0, 300.0], options, 6.0, None, false)
                    .1[0],
                if hidden { None } else { Some(600.0) }
            );
        }
    }
}

#[test]
fn moving_mask_edges_align_without_changing_masks_and_groups_remain_rigid() {
    use canvas_pilot::formats::Mask;
    let mut layout = Layout::default();
    layout
        .set_mask(
            1,
            Mask {
                left: 0.25,
                ..Default::default()
            },
        )
        .unwrap();
    let formats = layout.formats().clone();
    layout.start_drag([1200.0, 200.0]);
    // Moving left mask edge is 160 pixels inside the layer.
    assert_eq!(
        layout.drag_to_snapped([643.0, 210.0], mask_options(), 6.0)[0],
        Some(760.0)
    );
    assert_eq!(layout.layers()[1].x, 600.0);
    assert_eq!(*layout.formats(), formats);
    layout.cancel_drag();
    layout
        .set_mask(
            1,
            Mask {
                left: 0.2,
                ..Default::default()
            },
        )
        .unwrap();
    layout.set_selection([true, true]);
    layout.start_drag([1200.0, 200.0]);
    let before = layout.layers().clone();
    let options = Snapping {
        screen_edges: true,
        ..mask_options()
    };
    // Selected layers cannot target one another; a mask edge can target the output.
    assert_eq!(
        layout.drag_to_snapped([875.0, 210.0], options, 6.0)[0],
        Some(960.0)
    );
    assert_eq!(
        layout.layers()[1].x - layout.layers()[0].x,
        before[1].x - before[0].x
    );
    layout.cancel_drag();
    assert_eq!(*layout.layers(), before);
}

#[test]
fn fully_masked_targets_are_ignored_and_source_masks_are_clipped_to_layer() {
    use canvas_pilot::formats::{Framing, Mask, MaskOrigin};
    let mut layout = Layout::default();
    layout
        .set_mask(
            0,
            Mask {
                left: 0.5,
                right: 0.5,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        layout
            .snap_pivot(1, [443.0, 300.0], mask_options(), 6.0, None, false)
            .1,
        [None; 2]
    );
    layout
        .set_mask(
            0,
            Mask {
                origin: MaskOrigin::Source,
                left: 0.3,
                ..Default::default()
            },
        )
        .unwrap();
    layout
        .set_framing(
            0,
            Framing {
                zoom: 2.0,
                ..Default::default()
            },
        )
        .unwrap();
    // Source is now twice the aperture: 120 + (0.30 - 0.25) * 1280 = 184.
    assert_eq!(
        layout
            .snap_pivot(1, [187.0, 300.0], mask_options(), 6.0, None, false)
            .1[0],
        Some(184.0)
    );
    assert_eq!(
        layout
            .snap_pivot(1, [763.0, 300.0], mask_options(), 6.0, None, false)
            .1[0],
        Some(760.0)
    );
    assert_eq!(
        layout
            .snap_pivot(1, [1083.0, 300.0], mask_options(), 6.0, None, false)
            .1[0],
        None
    );
}

#[test]
fn mask_targets_follow_all_fit_modes_and_skip_letterbox_space() {
    use canvas_pilot::formats::{LayerFit, Mask, MaskOrigin};
    for fit in LayerFit::ALL {
        for origin in [MaskOrigin::Layer, MaskOrigin::Source] {
            let mut layout = Layout::default();
            let mut g = layout.layers()[0];
            g.height = 600.0;
            layout.apply(0, g).unwrap();
            layout
                .set_layer_fit(0, Some(fit), Default::default())
                .unwrap();
            layout
                .set_mask(
                    0,
                    Mask {
                        origin,
                        left: 0.3,
                        top: 0.3,
                        ..Default::default()
                    },
                )
                .unwrap();
            let mask = layout.formats().mask_bounds(0, g).unwrap();
            let picture = fit.image([1920.0, 1080.0], g).unwrap();
            let expected = [
                g.x.max(mask.x).max(picture.x),
                g.y.max(mask.y).max(picture.y),
            ];
            let (actual, guides) = layout.snap_pivot(
                1,
                expected.map(|v| v + 2.0),
                mask_options(),
                6.0,
                None,
                false,
            );
            assert_eq!(actual, expected, "{fit:?} {origin:?}");
            assert_eq!(guides, expected.map(Some));
            let high = [
                (g.x + g.width)
                    .min(mask.x + mask.width)
                    .min(picture.x + picture.width),
                (g.y + g.height)
                    .min(mask.y + mask.height)
                    .min(picture.y + picture.height),
            ];
            let center = std::array::from_fn(|axis| (expected[axis] + high[axis]) / 2.0);
            assert_eq!(
                layout.snap_pivot(1, center.map(|v| v + 2.0), mask_options(), 6.0, None, false),
                (center, center.map(Some)),
                "{fit:?} {origin:?}"
            );
        }
    }
}

#[test]
fn mask_centers_snap_pivots_resizes_and_moves_and_respect_visibility() {
    use canvas_pilot::{
        ResizeHandle,
        formats::{Mask, MaskOrigin},
    };
    for origin in [MaskOrigin::Layer, MaskOrigin::Source] {
        let mut layout = Layout::default();
        layout
            .set_mask(
                0,
                Mask {
                    origin,
                    right: 0.25,
                    ..Default::default()
                },
            )
            .unwrap();
        layout.set_locked(0, true);
        // Visible mask spans x=120..600; its center is 360, not the layer's 440.
        let options = mask_options();
        assert_eq!(
            layout.snap_pivot(1, [362.0, 362.0], options, 6.0, None, false),
            ([360.0, 360.0], [Some(360.0), Some(360.0)])
        );
        layout.start_handle_resize(1, ResizeHandle::Left, [1160.0, 360.0], false);
        assert_eq!(
            layout.drag_to_snapped([362.0, 360.0], options, 6.0)[0],
            Some(360.0)
        );
        assert_eq!(layout.layers()[1].x, 360.0);
        layout.cancel_drag();
        // A whole layer's center can also align to the visible mask center.
        layout.start_drag([1200.0, 200.0]);
        assert_eq!(
            layout.drag_to_snapped([82.0, 200.0], options, 6.0)[0],
            Some(360.0)
        );
        assert_eq!(layout.layers()[1].center()[0], 360.0);
        layout.cancel_drag();
        for hidden in [false, true] {
            layout.set_visible(0, !hidden);
            for enabled in [false, true] {
                let options = Snapping {
                    mask_edges: enabled,
                    ..options
                };
                assert_eq!(
                    layout
                        .snap_pivot(1, [362.0, 362.0], options, 6.0, None, false)
                        .1,
                    if enabled && !hidden {
                        [Some(360.0); 2]
                    } else {
                        [None; 2]
                    }
                );
            }
        }
    }
}

#[test]
fn moving_mask_center_aligns_to_masks_screen_and_grid_with_rigid_groups_and_undo() {
    use canvas_pilot::formats::Mask;
    let mut layout = Layout::default();
    layout
        .set_mask(
            0,
            Mask {
                right: 0.375,
                ..Default::default()
            },
        )
        .unwrap();
    layout
        .set_mask(
            1,
            Mask {
                left: 0.25,
                ..Default::default()
            },
        )
        .unwrap();
    let before = layout.layers().clone();
    let formats = layout.formats().clone();
    layout.start_drag([1200.0, 200.0]);
    assert_eq!(
        layout.drag_to_snapped([-37.0, 200.0], mask_options(), 6.0)[0],
        Some(320.0)
    );
    assert_eq!(
        layout
            .formats()
            .mask_bounds(1, layout.layers()[1])
            .unwrap()
            .center()[0],
        320.0
    );
    layout.finish_drag();
    assert_eq!(*layout.formats(), formats);
    layout.undo();
    assert_eq!(*layout.layers(), before);
    layout.set_selection([true, true]);
    for grid in [false, true] {
        layout.start_drag([1200.0, 200.0]);
        let options = Snapping {
            screen_edges: !grid,
            grid,
            grid_divisions: [2, 2],
            ..mask_options()
        };
        assert_eq!(
            layout.drag_to_snapped([603.0, 200.0], options, 6.0)[0],
            Some(960.0)
        );
        assert_eq!(
            layout
                .formats()
                .mask_bounds(1, layout.layers()[1])
                .unwrap()
                .center()[0],
            960.0
        );
        assert_eq!(
            layout.layers()[1].x - layout.layers()[0].x,
            before[1].x - before[0].x
        );
        layout.cancel_drag();
        assert_eq!(*layout.layers(), before);
        assert_eq!(*layout.formats(), formats);
    }
}

#[test]
fn layer_borders_and_mask_anchors_can_be_disabled_independently() {
    use canvas_pilot::formats::Mask;
    for borders in [false, true] {
        for masks in [false, true] {
            let mut layout = Layout::default();
            layout.select(Some(1));
            layout
                .set_mask(
                    1,
                    Mask {
                        left: 0.25,
                        right: 0.1,
                        ..Default::default()
                    },
                )
                .unwrap();
            let g = layout.layers()[1];
            let formats = layout.formats().clone();
            let options = Snapping {
                enabled: true,
                pixels: false,
                screen_edges: true,
                layer_edges: borders,
                mask_edges: masks,
                ..Default::default()
            };
            // Outer border near screen edge; masked edges and both centers are far away.
            layout.start_drag([g.x, g.y]);
            let guides = layout.drag_to_snapped([2.0, g.y + 17.0], options, 6.0);
            assert_eq!(guides[0], borders.then_some(0.0));
            assert_eq!(layout.layers()[1].x, if borders { 0.0 } else { 2.0 });
            layout.cancel_drag();
            // Mask's left edge near screen edge; outer border is far from guides.
            layout.start_drag([g.x, g.y]);
            let guides = layout.drag_to_snapped(
                [-158.0, g.y + 17.0],
                Snapping {
                    screen_edges: true,
                    layer_edges: false,
                    ..options
                },
                6.0,
            );
            assert_eq!(guides[0], masks.then_some(0.0));
            assert_eq!(layout.layers()[1].x, if masks { -160.0 } else { -158.0 });
            layout.cancel_drag();
            assert_eq!(layout.layers()[1], g);
            assert_eq!(*layout.formats(), formats);
        }
    }
}
