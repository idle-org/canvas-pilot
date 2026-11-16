use canvas_pilot::{Corner, Geometry, Layout, parse_drawio};
const SAMPLE: &str = include_str!("fixtures/drawio-rectangle.txt");
fn xml(body: &str) -> String {
    format!(
        r#"<mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/>{body}</root></mxGraphModel>"#
    )
}
fn vertex(geometry: &str, style: &str) -> String {
    xml(&format!(
        r#"<mxCell id="2" parent="1" vertex="1" style="{style}">{geometry}</mxCell>"#
    ))
}
fn g() -> Geometry {
    Geometry {
        x: 12.0,
        y: 271.0,
        width: 945.0,
        height: 531.58,
    }
}
#[test]
fn supplied_clipboard_preserves_decimals_and_converts_center() {
    let shapes = parse_drawio(SAMPLE).unwrap();
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0].id, "2");
    assert_eq!(shapes[0].geometry, g());
    assert_eq!(shapes[0].geometry.center(), [484.5, 536.79]);
    assert!((g().width / g().height - 16.0 / 9.0).abs() > 0.000001); // no preset snapping
}
#[test]
fn raw_xml_defaults_missing_origin_and_keeps_negative_and_fractional_coordinates() {
    let raw = vertex(r#"<mxGeometry width="20.5" height="30"/>"#, "rounded=0;");
    let a = parse_drawio(&raw).unwrap()[0].geometry;
    assert_eq!([a.x, a.y], [0.0, 0.0]);
    let raw = vertex(
        r#"<mxGeometry x="-12.25" y="2.5" width="20.5" height="30"/>"#,
        "",
    );
    let a = parse_drawio(&raw).unwrap()[0].geometry;
    assert_eq!([a.x, a.y], [-12.25, 2.5]);
}
#[test]
fn multiple_rectangles_keep_source_order() {
    let data = xml(
        r#"<mxCell id="9" vertex="1" parent="1"><mxGeometry width="50" height="30"/></mxCell><mxCell id="2" vertex="1" parent="1"><mxGeometry x="100" width="10" height="10"/></mxCell>"#,
    );
    let list = parse_drawio(&data).unwrap();
    assert_eq!(
        list.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        vec!["9", "2"]
    );
}
#[test]
fn invalid_clipboard_is_rejected_without_panicking() {
    for s in [
        "",
        "%",
        "%3",
        "%GG",
        "%ff",
        "not geometry",
        "<mxGraphModel>",
        "<mxfile/>",
        "<mxGraphModel/>",
        "<mxGraphModel><root/></mxGraphModel>",
    ] {
        assert!(parse_drawio(s).is_err(), "{s}");
    }
    assert!(parse_drawio(&"x".repeat(1_048_577)).is_err());
    assert!(
        parse_drawio(
            r#"<!DOCTYPE a [<!ENTITY e "hello">]><mxGraphModel><root>&e;</root></mxGraphModel>"#
        )
        .is_err()
    );
}
#[test]
fn invalid_sizes_or_nonfinite_numbers_cannot_enter_document() {
    for attr in [
        r#"width="0" height="2""#,
        r#"width="-1" height="2""#,
        r#"width="NaN" height="2""#,
        r#"width="2" height="inf""#,
        r#"x="1e300" width="2" height="2""#,
        r#"height="20""#,
    ] {
        assert!(parse_drawio(&vertex(&format!("<mxGeometry {attr}/>"), "")).is_err());
    }
    let mut layout = Layout::default();
    let before = layout.layers().clone();
    assert!(
        layout
            .apply(
                1,
                Geometry {
                    width: f64::NAN,
                    ..g()
                }
            )
            .is_err()
    );
    assert_eq!(*layout.layers(), before);
    assert!(layout.apply(7, g()).is_err());
    assert_eq!(*layout.layers(), before);
}
#[test]
fn transforms_groups_connections_and_ambiguous_geometry_are_refused() {
    for style in [
        "rotation=90;",
        "flipH=1;",
        "shape=ellipse;",
        "ellipse;",
        "group;",
    ] {
        assert!(parse_drawio(&vertex(r#"<mxGeometry width="10" height="10"/>"#, style)).is_err());
    }
    for geometry in [
        r#"<mxGeometry relative="1" width="10" height="10"/>"#,
        r#"<mxGeometry width="10" height="10"><mxPoint x="2" as="offset"/></mxGeometry>"#,
        r#"<mxGeometry width="10" height="10"/><mxGeometry width="20" height="20"/>"#,
    ] {
        assert!(parse_drawio(&vertex(geometry, "")).is_err());
    }
    for body in [
        r#"<mxCell id="2" parent="1" edge="1"/>"#,
        r#"<object><mxCell id="2"/></object>"#,
        r#"<mxCell id="1"/>"#,
        r#"<mxCell id="2" parent="missing" vertex="1"><mxGeometry width="10" height="10"/></mxCell>"#,
        r#"<mxCell id="2" parent="1" vertex="1"><mxGeometry width="10" height="10"/></mxCell><mxCell id="3" parent="2" vertex="1"><mxGeometry width="5" height="5"/></mxCell>"#,
    ] {
        assert!(parse_drawio(&xml(body)).is_err());
    }
}
#[test]
fn percent_decoding_is_once_and_does_not_treat_plus_as_space() {
    let raw = vertex(r#"<mxGeometry x="+12" width="10" height="10"/>"#, "");
    let encoded = raw.replace('<', "%3C").replace('>', "%3E");
    assert_eq!(parse_drawio(&encoded).unwrap()[0].geometry.x, 12.0);
    assert!(parse_drawio(&encoded.replace('%', "%25")).is_err());
}
#[test]
fn corners_roundtrip_without_moving_or_rounding_the_box() {
    for c in [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ] {
        let [x, y] = c.offsets(g());
        let result = c.geometry(x, y, g().width, g().height).unwrap();
        assert!((result.x - g().x).abs() < 1e-10);
        assert!((result.y - g().y).abs() < 1e-10);
        assert_eq!(result.width, g().width);
    }
    let a = Corner::TopRight
        .geometry(120.0, 180.0, 640.0, 360.0)
        .unwrap();
    assert_eq!(a.center(), [1480.0, 360.0]);
}
#[test]
fn paste_and_undo_change_only_the_target() {
    let mut l = Layout::default();
    let original = l.layers().clone();
    l.apply(1, parse_drawio(SAMPLE).unwrap()[0].geometry)
        .unwrap();
    assert_eq!(l.layers()[0], original[0]);
    assert_eq!(l.layers()[1], g());
    l.undo();
    assert_eq!(*l.layers(), original);
    l.redo();
    assert_eq!(l.layers()[1], g());
    l.undo();
    l.nudge(1.0, 0.0).unwrap();
    let nudged = l.layers().clone();
    l.redo();
    assert_eq!(*l.layers(), nudged);
}
#[test]
fn drag_uses_original_position_and_is_one_undo_record() {
    let mut l = Layout::default();
    l.apply(1, g()).unwrap();
    let original = l.layers().clone();
    l.start_drag([20.0, 280.0]);
    for n in 1..10 {
        l.drag_to([20.0 + n as f64, 280.0 + n as f64]);
    }
    l.finish_drag();
    assert_eq!(
        l.layers()[1],
        Geometry {
            x: 21.0,
            y: 280.0,
            ..g()
        }
    );
    l.undo();
    assert_eq!(*l.layers(), original);
    l.redo();
    assert_eq!(l.layers()[1].x, 21.0);
}
#[test]
fn cancel_drag_restores_geometry_and_retains_fractional_origin() {
    let mut l = Layout::default();
    l.apply(1, Geometry { x: 12.25, ..g() }).unwrap();
    let original = l.layers().clone();
    l.start_drag([20.0, 280.0]);
    l.drag_to([22.8, 282.2]);
    assert_eq!(l.layers()[1].x, 15.25);
    l.cancel_drag();
    assert_eq!(*l.layers(), original);
    assert!(!l.dragging());
}
#[test]
fn hit_testing_matches_stacking_and_background_deselects() {
    let mut l = Layout::default();
    l.apply(0, g()).unwrap();
    l.apply(1, g()).unwrap();
    assert_eq!(l.hit([20.0, 280.0]), Some(1));
    l.start_drag([-100.0, -100.0]);
    assert_eq!(l.selected(), None);
    l.nudge(10.0, 10.0).unwrap();
    assert_eq!(l.layers()[0], g());
    l.select(Some(77));
    assert_eq!(l.selected(), None);
}
#[test]
fn offscreen_placement_is_preserved_and_nudging_keeps_size() {
    let mut l = Layout::default();
    l.apply(
        1,
        Geometry {
            x: -20.25,
            y: 1200.0,
            ..g()
        },
    )
    .unwrap();
    l.nudge(1.0, -10.0).unwrap();
    let a = l.layers()[1];
    assert_eq!(a.x, -19.25);
    assert_eq!(a.y, 1190.0);
    assert_eq!(a.width, 945.0);
    assert!(a.outside());
}

#[test]
fn outline_expands_sample_without_moving_center_and_position_only_is_unadjusted() {
    let shape = parse_drawio(SAMPLE).unwrap().remove(0);
    assert_eq!(shape.stroke_width, 5.0);
    let before = Geometry {
        x: 100.0,
        y: 200.0,
        width: 300.0,
        height: 250.0,
    };
    assert_eq!(shape.paste_geometry(before, true, false).unwrap(), g());
    let outlined = shape.paste_geometry(before, true, true).unwrap();
    assert_eq!(
        outlined,
        Geometry {
            x: 9.5,
            y: 268.5,
            width: 950.0,
            height: 536.58
        }
    );
    assert_eq!(outlined.center(), g().center());
    assert_eq!(
        shape.paste_geometry(before, false, true).unwrap(),
        Geometry {
            x: 12.0,
            y: 271.0,
            ..before
        }
    );
}
#[test]
fn default_fractional_and_hidden_outlines_are_handled() {
    for (style, width) in [
        ("", 1.0),
        ("strokeWidth=0.5;", 0.5),
        ("strokeWidth=0;", 0.0),
        ("strokeWidth=5;strokeColor=none;", 0.0),
        ("strokeWidth=5;strokeColor=transparent;", 0.0),
        ("strokeWidth=5;strokeOpacity=0;", 0.0),
        ("strokeWidth=5;opacity=0;", 0.0),
        ("strokeWidth=2;strokeWidth=6;", 6.0),
    ] {
        let shape = parse_drawio(&vertex(r#"<mxGeometry width="10" height="20"/>"#, style))
            .unwrap()
            .remove(0);
        assert_eq!(shape.stroke_width, width, "{style}");
        let result = shape.paste_geometry(g(), true, true).unwrap();
        assert_eq!(result.x, -width / 2.0);
        assert_eq!(result.width, 10.0 + width);
        assert_eq!(result.center(), [5.0, 10.0]);
    }
}
#[test]
fn invalid_outline_settings_and_expansion_beyond_limits_are_refused() {
    for style in [
        "strokeWidth=NaN;",
        "strokeWidth=-1;",
        "strokeWidth=inf;",
        "strokeWidth=huge;",
        "strokeOpacity=101;",
        "opacity=-1;",
    ] {
        assert!(parse_drawio(&vertex(r#"<mxGeometry width="10" height="20"/>"#, style)).is_err());
    }
    let shape = parse_drawio(&vertex(
        r#"<mxGeometry width="1000000" height="20"/>"#,
        "strokeWidth=5;",
    ))
    .unwrap()
    .remove(0);
    assert!(shape.paste_geometry(g(), true, false).is_ok());
    assert!(shape.paste_geometry(g(), true, true).is_err());
}
#[test]
fn each_pasted_rectangle_keeps_its_own_outline_width() {
    let data = xml(
        r#"<mxCell id="2" vertex="1" parent="1" style="strokeWidth=2;"><mxGeometry width="10" height="20"/></mxCell><mxCell id="3" vertex="1" parent="1" style="strokeWidth=8;"><mxGeometry width="30" height="40"/></mxCell>"#,
    );
    let shapes = parse_drawio(&data).unwrap();
    assert_eq!(
        shapes[0].paste_geometry(g(), true, true).unwrap().width,
        12.0
    );
    assert_eq!(
        shapes[1].paste_geometry(g(), true, true).unwrap().width,
        38.0
    );
}

#[test]
fn selection_toggle_and_marquee_are_independent_of_geometry_history() {
    let mut l = Layout::default();
    let original = l.layers().clone();
    l.toggle(0);
    assert_eq!(l.selected_indices(), vec![0, 1]);
    assert_eq!(l.selected(), None);
    l.toggle(1);
    assert_eq!(l.selected(), Some(0));
    l.toggle(0);
    assert!(l.selected_indices().is_empty());
    l.select_region([1800.0, 600.0], [100.0, 100.0], [false; 2]);
    assert_eq!(l.selected_indices(), vec![0, 1]);
    l.select_region([100.0, 100.0], [125.0, 185.0], [false, true]);
    assert_eq!(l.selected_indices(), vec![0, 1]);
    assert_eq!(*l.layers(), original);
    l.undo();
    assert_eq!(*l.layers(), original);
}
#[test]
fn selected_layers_drag_and_nudge_together_with_atomic_undo() {
    let mut l = Layout::default();
    l.set_selection([true; 2]);
    let before = l.layers().clone();
    l.start_drag([130.0, 190.0]);
    l.drag_to([155.0, 200.0]);
    l.finish_drag();
    for (i, g) in before.iter().enumerate() {
        assert_eq!(
            l.layers()[i],
            Geometry {
                x: g.x + 25.0,
                y: g.y + 10.0,
                ..*g
            }
        );
    }
    l.undo();
    assert_eq!(*l.layers(), before);
    l.redo();
    l.nudge(1.0, -10.0).unwrap();
    for (i, g) in before.iter().enumerate() {
        assert_eq!(
            l.layers()[i],
            Geometry {
                x: g.x + 26.0,
                y: g.y,
                ..*g
            }
        );
    }
    l.undo();
    l.undo();
    assert_eq!(*l.layers(), before);
    l.start_drag([130.0, 190.0]);
    l.drag_to([150.0, 220.0]);
    l.cancel_drag();
    assert_eq!(*l.layers(), before);
}
#[test]
fn align_edges_and_centers_to_explicit_targets_without_resizing() {
    use canvas_pilot::{AlignTarget as T, Alignment as A};
    let initial = [
        Geometry {
            x: 100.5,
            y: 70.25,
            width: 300.0,
            height: 210.0,
        },
        Geometry {
            x: 700.25,
            y: 350.5,
            width: 100.0,
            height: 80.0,
        },
    ];
    for target in [T::Selection, T::Screen, T::Layer(0), T::Layer(1)] {
        for alignment in [
            A::Left,
            A::Right,
            A::Top,
            A::Bottom,
            A::HorizontalCenter,
            A::VerticalCenter,
        ] {
            let mut l = Layout::default();
            for (i, g) in initial.iter().enumerate() {
                l.apply(i, *g).unwrap();
            }
            l.set_selection([true; 2]);
            let reference = match target {
                T::Screen => Geometry {
                    x: 0.0,
                    y: 0.0,
                    width: 1920.0,
                    height: 1080.0,
                },
                T::Selection => l.selection_bounds().unwrap(),
                T::Layer(i) => initial[i],
            };
            l.align(alignment, target).unwrap();
            for (i, g) in l.layers().iter().enumerate() {
                assert_eq!([g.width, g.height], [initial[i].width, initial[i].height]);
                let (actual, wanted) = match alignment {
                    A::Left => (g.x, reference.x),
                    A::Right => (g.x + g.width, reference.x + reference.width),
                    A::Top => (g.y, reference.y),
                    A::Bottom => (g.y + g.height, reference.y + reference.height),
                    A::HorizontalCenter => (g.center()[0], reference.center()[0]),
                    A::VerticalCenter => (g.center()[1], reference.center()[1]),
                };
                assert!((actual - wanted).abs() < 1e-9);
                if matches!(alignment, A::Left | A::Right | A::HorizontalCenter) {
                    assert_eq!(g.y, initial[i].y);
                } else {
                    assert_eq!(g.x, initial[i].x);
                }
            }
            if let T::Layer(i) = target {
                assert_eq!(l.layers()[i], initial[i]);
            }
            let aligned = l.layers().clone();
            l.undo();
            assert_eq!(*l.layers(), initial);
            l.redo();
            assert_eq!(*l.layers(), aligned);
        }
    }
}
#[test]
fn alignment_refuses_invalid_reference_and_failed_group_moves_are_atomic() {
    use canvas_pilot::{AlignTarget as T, Alignment as A};
    let mut l = Layout::default();
    let before = l.layers().clone();
    assert!(l.align(A::Top, T::Layer(99)).is_err());
    assert_eq!(*l.layers(), before);
    l.select(None);
    assert!(l.align(A::Left, T::Screen).is_err());
    l.apply(
        1,
        Geometry {
            x: 1_000_000.0,
            ..before[1]
        },
    )
    .unwrap();
    l.set_selection([true; 2]);
    let before = l.layers().clone();
    assert!(l.nudge(1.0, 0.0).is_err());
    assert_eq!(*l.layers(), before);
    l.start_drag([130.0, 190.0]);
    l.drag_to([140.0, 190.0]);
    l.finish_drag();
    assert_eq!(*l.layers(), before);
}
#[test]
fn alignment_moves_only_selected_layers_and_is_order_independent() {
    use canvas_pilot::{AlignTarget as T, Alignment as A};
    let mut l = Layout::default();
    let before = l.layers().clone();
    l.align(A::Left, T::Screen).unwrap();
    assert_eq!(l.layers()[0], before[0]);
    assert_eq!(l.layers()[1].x, 0.0);
    l.undo();
    l.select(Some(0));
    l.toggle(1);
    l.align(A::Left, T::Selection).unwrap();
    let first = l.layers().clone();
    l.undo();
    l.select(Some(1));
    l.toggle(0);
    l.align(A::Left, T::Selection).unwrap();
    assert_eq!(*l.layers(), first);
}

#[test]
fn protected_layers_are_excluded_from_selection_and_edits() {
    for hidden in [false, true] {
        let mut layout = Layout::default();
        let before = layout.layers().clone();
        layout.set_selection([true; 2]);
        if hidden {
            layout.set_visible(1, false);
        } else {
            layout.set_locked(1, true);
        }
        assert_eq!(layout.selected(), Some(0));
        assert_eq!(layout.hit(before[1].center()), None);
        layout.select(Some(1));
        layout.toggle(1);
        assert_eq!(layout.selected(), None);
        layout.select_region([0.0, 0.0], [1920.0, 1080.0], [true; 2]);
        assert_eq!(layout.selected(), Some(0));
        layout.nudge(10.0, 20.0).unwrap();
        layout
            .align(
                canvas_pilot::Alignment::Left,
                canvas_pilot::AlignTarget::Screen,
            )
            .unwrap();
        assert_eq!(layout.layers()[1], before[1]);
        assert!(layout.apply(1, before[0]).is_err());
        layout.start_drag(before[1].center());
        assert!(!layout.dragging());
        layout.set_visible(1, true);
        layout.set_locked(1, false);
        layout.select(Some(1));
        assert_eq!(layout.selected(), Some(1));
        assert_eq!(layout.layers()[1], before[1]);
    }
}

#[test]
fn locking_during_a_group_drag_restores_geometry_and_preserves_visibility() {
    let mut layout = Layout::default();
    let before = layout.layers().clone();
    layout.set_selection([true; 2]);
    layout.start_drag(before[0].center());
    layout.drag_to([500.0, 500.0]);
    layout.set_locked(0, true);
    assert_eq!(*layout.layers(), before);
    assert!(!layout.dragging());
    assert!(layout.visible(0));
    layout.set_visible(0, false);
    layout.set_visible(0, true);
    assert!(layout.locked(0));
    layout.select(Some(0));
    assert_eq!(layout.selected(), None);
}

#[test]
fn corner_resize_preserves_opposite_anchor_ratio_and_atomic_history() {
    for corner in [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ] {
        for keep_ratio in [true, false] {
            let mut layout = Layout::default();
            let before = layout.layers().clone();
            let left = matches!(corner, Corner::TopLeft | Corner::BottomLeft);
            let top = matches!(corner, Corner::TopLeft | Corner::TopRight);
            let anchor = |g: Geometry| {
                [
                    if left { g.x + g.width } else { g.x },
                    if top { g.y + g.height } else { g.y },
                ]
            };
            let start = [
                if left {
                    before[1].x
                } else {
                    before[1].x + before[1].width
                },
                if top {
                    before[1].y
                } else {
                    before[1].y + before[1].height
                },
            ];
            let end = [
                start[0] + if left { -160.0 } else { 160.0 },
                start[1] + if top { -90.0 } else { 90.0 },
            ];
            layout.start_resize(1, corner, start, keep_ratio);
            layout.drag_to(end);
            layout.drag_to(end); // no incremental drift
            layout.finish_drag();
            let after = layout.layers().clone();
            assert_eq!(after[0], before[0]);
            assert_eq!(anchor(after[1]), anchor(before[1]));
            assert_eq!([after[1].width, after[1].height], [800.0, 450.0]);
            layout.undo();
            assert_eq!(*layout.layers(), before);
            layout.redo();
            assert_eq!(*layout.layers(), after);
        }
    }
}

#[test]
fn free_resize_changes_ratio_and_cancels_without_flipping_or_moving_locked_layers() {
    let mut layout = Layout::default();
    let before = layout.layers().clone();
    layout.start_resize(1, Corner::BottomRight, [0.0, 0.0], false);
    layout.drag_to([100.0, 20.0]);
    assert_eq!(
        [layout.layers()[1].width, layout.layers()[1].height],
        [740.0, 380.0]
    );
    layout.drag_to([-2000.0, -2000.0]);
    assert_eq!(
        [layout.layers()[1].width, layout.layers()[1].height],
        [1.0, 1.0]
    );
    layout.cancel_drag();
    assert_eq!(*layout.layers(), before);
    layout.start_resize(1, Corner::BottomRight, [0.0, 0.0], true);
    layout.drag_to([100.0, 0.0]);
    assert!((layout.layers()[1].width / layout.layers()[1].height - 16.0 / 9.0).abs() < 1e-10);
    layout.drag_to([f64::NAN, 0.0]);
    assert!(layout.layers()[1].validate().is_ok());
    layout.set_locked(1, true);
    assert_eq!(*layout.layers(), before);
    layout.start_resize(1, Corner::BottomRight, [0.0, 0.0], false);
    assert!(!layout.dragging());
}

#[test]
fn midpoint_handles_fix_opposite_edge_and_preserve_undo_and_ratio() {
    use canvas_pilot::ResizeHandle as H;
    for handle in [H::Left, H::Right, H::Top, H::Bottom] {
        for keep_ratio in [true, false] {
            let mut layout = Layout::default();
            let before = layout.layers().clone();
            let center = before[1].center();
            let (handle, start) = H::positions(before[1])
                .into_iter()
                .find(|(h, _)| *h == handle)
                .unwrap();
            let end = match handle {
                H::Left => [start[0] - 20.0, start[1]],
                H::Right => [start[0] + 20.0, start[1]],
                H::Top => [start[0], start[1] - 20.0],
                H::Bottom => [start[0], start[1] + 20.0],
                _ => unreachable!(),
            };
            layout.start_handle_resize(1, handle, start, keep_ratio);
            layout.drag_to(end);
            layout.finish_drag();
            let after = layout.layers()[1];
            match handle {
                H::Left => assert_eq!(after.x + after.width, before[1].x + before[1].width),
                H::Right => assert_eq!(after.x, before[1].x),
                H::Top => assert_eq!(after.y + after.height, before[1].y + before[1].height),
                H::Bottom => assert_eq!(after.y, before[1].y),
                _ => unreachable!(),
            }
            assert_eq!(layout.layers()[0], before[0]);
            if matches!(handle, H::Left | H::Right) {
                assert_eq!(after.width, 660.0);
                assert_eq!(after.center()[1], center[1]);
                if !keep_ratio {
                    assert_eq!(after.height, 360.0);
                }
            } else {
                assert_eq!(after.height, 380.0);
                assert_eq!(after.center()[0], center[0]);
                if !keep_ratio {
                    assert_eq!(after.width, 640.0);
                }
            }
            if keep_ratio {
                assert!((after.width / after.height - 16.0 / 9.0).abs() < 1e-10);
            }
            layout.undo();
            assert_eq!(*layout.layers(), before);
            layout.redo();
            assert_eq!(layout.layers()[1], after);
            layout.start_handle_resize(1, handle, start, false);
            layout.drag_to(end);
            layout.cancel_drag();
            assert_eq!(layout.layers()[1], after);
        }
    }
}

#[test]
fn corner_alignment_to_unselected_reference_is_one_undo_step() {
    use canvas_pilot::{AlignTarget as T, Alignment as A};
    let mut l = Layout::default();
    let before = l.layers().clone();
    l.align_axes(&[A::Right, A::Bottom], T::Layer(0)).unwrap();
    assert_eq!(l.layers()[0], before[0]);
    assert_eq!(
        l.layers()[1].x + l.layers()[1].width,
        before[0].x + before[0].width
    );
    assert_eq!(
        l.layers()[1].y + l.layers()[1].height,
        before[0].y + before[0].height
    );
    l.undo();
    assert_eq!(*l.layers(), before);
}

#[test]
fn selected_resize_scales_proportionally_with_same_or_opposite_anchors() {
    use canvas_pilot::{ResizeHandle as H, ResizeMode as M};
    for mode in [M::Same, M::Mirror, M::Opposite] {
        for keep_ratio in [false, true] {
            for initiator in 0..2 {
                for (handle, _) in H::positions(Layout::default().layers()[0]) {
                    let mut l = Layout::default();
                    l.apply(
                        0,
                        Geometry {
                            x: 100.0,
                            y: 80.0,
                            width: 300.0,
                            height: 200.0,
                        },
                    )
                    .unwrap();
                    l.set_selection([true; 2]);
                    let before = l.layers().clone();
                    let formats = l.formats().clone();
                    let start = H::positions(before[initiator])
                        .into_iter()
                        .find(|(h, _)| *h == handle)
                        .unwrap()
                        .1;
                    let dx = if matches!(
                        handle,
                        H::Left | H::Corner(Corner::TopLeft | Corner::BottomLeft)
                    ) {
                        -30.0
                    } else {
                        30.0
                    };
                    let dy = if matches!(
                        handle,
                        H::Top | H::Corner(Corner::TopLeft | Corner::TopRight)
                    ) {
                        -20.0
                    } else {
                        20.0
                    };
                    l.start_selected_resize(initiator, handle, start, keep_ratio, mode)
                        .unwrap();
                    l.drag_to([start[0] + dx, start[1] + dy]);
                    l.finish_drag();
                    assert_eq!(l.selection_mask(), [true; 2]);
                    let after = l.layers().clone();
                    assert!(
                        (after[0].width / before[0].width - after[1].width / before[1].width).abs()
                            < 1e-10
                    );
                    assert!(
                        (after[0].height / before[0].height - after[1].height / before[1].height)
                            .abs()
                            < 1e-10
                    );
                    for i in 0..2 {
                        let actual = if i != initiator {
                            match mode {
                                M::Mirror => handle.mirrored_horizontally(),
                                M::Opposite => handle.opposite(),
                                _ => handle,
                            }
                        } else {
                            handle
                        };
                        let anchor = |g| {
                            H::positions(g)
                                .into_iter()
                                .find(|(h, _)| *h == actual.opposite())
                                .unwrap()
                                .1
                        };
                        for (a, b) in anchor(before[i]).into_iter().zip(anchor(after[i])) {
                            assert!((a - b).abs() < 1e-8);
                        }
                        if keep_ratio {
                            assert!(
                                (after[i].width / after[i].height
                                    - before[i].width / before[i].height)
                                    .abs()
                                    < 1e-10
                            );
                        }
                    }
                    l.undo();
                    assert_eq!(*l.layers(), before);
                    assert_eq!(*l.formats(), formats);
                    l.redo();
                    assert_eq!(*l.layers(), after);
                }
            }
        }
    }
}

#[test]
fn selected_resize_limits_cancel_and_protection_are_atomic() {
    use canvas_pilot::{ResizeHandle as H, ResizeMode as M};
    let mut l = Layout::default();
    assert!(
        l.start_selected_resize(1, H::Right, [0.0; 2], false, M::Mirror)
            .is_err()
    );
    l.apply(
        0,
        Geometry {
            x: 100.0,
            y: 80.0,
            width: 300.0,
            height: 200.0,
        },
    )
    .unwrap();
    l.set_selection([true; 2]);
    let before = l.layers().clone();
    let formats = l.formats().clone();
    l.start_selected_resize(0, H::Right, [0.0; 2], false, M::Same)
        .unwrap();
    l.drag_to([800_000.0, 0.0]);
    assert!(l.gesture_error.is_some());
    assert_eq!(*l.layers(), before);
    assert_eq!(*l.formats(), formats);
    l.drag_to([30.0, 0.0]);
    assert_ne!(*l.layers(), before);
    l.cancel_drag();
    assert_eq!(*l.layers(), before);
    assert_eq!(*l.formats(), formats);
    l.start_selected_resize(0, H::Right, [0.0; 2], false, M::Mirror)
        .unwrap();
    l.drag_to([30.0, 0.0]);
    l.set_locked(1, true);
    assert_eq!(*l.layers(), before);
    assert_eq!(*l.formats(), formats);
    assert!(!l.dragging());
    assert!(
        l.start_selected_resize(0, H::Right, [0.0; 2], false, M::Mirror)
            .is_err()
    );
}

#[test]
fn resize_handle_modes_distinguish_horizontal_mirror_from_opposite() {
    use canvas_pilot::ResizeHandle as H;
    for (handle, mirrored, opposite) in [
        (
            H::Corner(Corner::BottomRight),
            H::Corner(Corner::BottomLeft),
            H::Corner(Corner::TopLeft),
        ),
        (
            H::Corner(Corner::TopRight),
            H::Corner(Corner::TopLeft),
            H::Corner(Corner::BottomLeft),
        ),
        (H::Right, H::Left, H::Left),
        (H::Top, H::Top, H::Bottom),
        (H::Bottom, H::Bottom, H::Top),
    ] {
        assert_eq!(handle.mirrored_horizontally(), mirrored);
        assert_eq!(mirrored.mirrored_horizontally(), handle);
        assert_eq!(handle.opposite(), opposite);
        assert_eq!(opposite.opposite(), handle);
    }
}

#[test]
fn selection_never_changes_viewport_stacking_or_hit_order() {
    let mut l = Layout::default();
    l.apply(0, g()).unwrap();
    l.apply(1, g()).unwrap();
    let before = l.layers().clone();
    for (selection, order, hit) in [
        ([false, false], [0, 1], Some(1)),
        ([true, false], [0, 1], Some(1)),
        ([false, true], [0, 1], Some(1)),
        ([true, true], [0, 1], Some(1)),
    ] {
        l.set_selection(selection);
        assert_eq!(l.viewport_order(), order);
        assert_eq!(l.hit([20.0, 280.0]), hit);
        assert_eq!(*l.layers(), before);
    }
    l.select(Some(0));
    l.set_locked(0, true);
    assert_eq!(l.hit([20.0, 280.0]), Some(1));
    l.set_visible(1, false);
    assert_eq!(l.hit([20.0, 280.0]), None);
}

#[test]
fn crop_corner_zoom_keeps_layer_fixed_and_supports_limits_history_and_cancel() {
    use canvas_pilot::ResizeHandle;
    for handle in [
        ResizeHandle::Corner(Corner::TopLeft),
        ResizeHandle::Corner(Corner::TopRight),
        ResizeHandle::Corner(Corner::BottomLeft),
        ResizeHandle::Corner(Corner::BottomRight),
        ResizeHandle::Top,
        ResizeHandle::Bottom,
        ResizeHandle::Left,
        ResizeHandle::Right,
    ] {
        let mut l = Layout::default();
        let i = 1;
        let before = l.layers().clone();
        let formats = l.formats().clone();
        let g = before[i];
        let start = ResizeHandle::positions(g)
            .into_iter()
            .find(|(h, _)| *h == handle)
            .unwrap()
            .1;
        let opposite = ResizeHandle::positions(g)
            .into_iter()
            .find(|(h, _)| *h == handle.opposite())
            .unwrap()
            .1;
        let end = std::array::from_fn(|axis| opposite[axis] + 2.0 * (start[axis] - opposite[axis]));
        l.start_content_resize(i, handle, start);
        l.drag_to(end);
        l.finish_drag();
        assert_eq!(*l.layers(), before);
        let after = l.formats().clone();
        let m = after.mapping(i, g).unwrap();
        assert!((m.framing.zoom - 2.0).abs() < 1e-8);
        let crop = after.advanced[i];
        let input = after.input(i).unwrap();
        let full = Geometry {
            x: g.x - crop.left * m.scale,
            y: g.y - crop.top * m.scale,
            width: input[0] * m.scale,
            height: input[1] * m.scale,
        };
        let fixed = ResizeHandle::positions(full)
            .into_iter()
            .find(|(h, _)| *h == handle.opposite())
            .unwrap()
            .1;
        for axis in 0..2 {
            assert!((fixed[axis] - opposite[axis]).abs() < 1e-7);
        }
        l.undo();
        assert_eq!(*l.formats(), formats);
        l.redo();
        assert_eq!(*l.formats(), after);
        l.start_content_resize(i, handle, end);
        l.drag_to(opposite);
        let m = l.formats().mapping(i, g).unwrap();
        assert!((m.framing.zoom - 1.0).abs() < 1e-8);
        l.cancel_drag();
        assert_eq!(*l.formats(), after);
        l.start_content_resize(i, handle, end);
        l.drag_to(std::array::from_fn(|axis| {
            opposite[axis] + 1000.0 * (start[axis] - opposite[axis])
        }));
        assert!((l.formats().mapping(i, g).unwrap().framing.zoom - 100.0).abs() < 1e-6);
        l.set_locked(i, true);
        assert_eq!(*l.formats(), after);
        l.start_content_resize(i, handle, start);
        assert!(!l.dragging());
    }
}

#[test]
fn independent_measurement_points_roundtrip_and_resize_about_the_layer_endpoint() {
    use canvas_pilot::{PositionPoint, PositionReference};
    let points = [
        PositionPoint::TopLeft,
        PositionPoint::TopRight,
        PositionPoint::BottomLeft,
        PositionPoint::BottomRight,
        PositionPoint::Center,
    ];
    let output = [1921, 1081];
    let geometry = Geometry {
        x: -120.25,
        y: 183.5,
        width: 641.5,
        height: 361.25,
    };
    for screen in points {
        for layer in points {
            let reference = PositionReference { screen, layer };
            let [x, y] = reference.offsets_on(geometry, output);
            assert_eq!(
                reference
                    .geometry_on(x, y, geometry.width, geometry.height, output)
                    .unwrap(),
                geometry
            );
            let resized = reference.geometry_on(x, y, 200.25, 400.5, output).unwrap();
            assert_eq!(layer.point_on(resized), layer.point_on(geometry));
            assert_eq!(reference.offsets_on(resized, output), [x, y]);
        }
    }
    let reference = PositionReference {
        screen: PositionPoint::TopLeft,
        layer: PositionPoint::Center,
    };
    assert_eq!(reference.offsets_on(geometry, output), geometry.center());
    assert_eq!(
        reference
            .geometry_on(960.5, 540.5, 640.0, 360.0, output)
            .unwrap()
            .center(),
        [960.5, 540.5]
    );
}
