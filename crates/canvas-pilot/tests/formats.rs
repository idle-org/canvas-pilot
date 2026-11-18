use canvas_pilot::formats::*;
use canvas_pilot::{AlignTarget, Alignment, Corner, Geometry, Layout};
fn close(a: f64, b: f64) {
    assert!(
        (a - b).abs() < 1e-7 * (1.0 + a.abs().max(b.abs())),
        "{a} != {b}"
    );
}
fn box_size(w: f64, h: f64) -> Geometry {
    Geometry {
        x: 100.0,
        y: 200.0,
        width: w,
        height: h,
    }
}
#[test]
fn worked_examples_and_inverse_controls_match_without_rounding() {
    for (input, zoom, offset, expected, projected) in [
        (
            [1920.0, 1080.0],
            1.0,
            [0.0, 0.0],
            [312.0, 312.0, 0.0, 0.0],
            [444.4444444444444, 250.0],
        ),
        (
            [1920.0, 1000.0],
            1.0,
            [0.0, 0.0],
            [360.0, 360.0, 0.0, 0.0],
            [480.0, 250.0],
        ),
        (
            [1920.0, 1000.0],
            2.0,
            [0.0, 0.0],
            [660.0, 660.0, 250.0, 250.0],
            [960.0, 500.0],
        ),
        (
            [1920.0, 1000.0],
            2.0,
            [30.0, 0.0],
            [600.0, 720.0, 250.0, 250.0],
            [960.0, 500.0],
        ),
    ] {
        let crop = crop_from_framing(input, [300.0, 250.0], Framing { zoom, offset }).unwrap();
        for (actual, wanted) in [crop.left, crop.right, crop.top, crop.bottom]
            .into_iter()
            .zip(expected)
        {
            close(actual, wanted);
        }
        let m = mapping(input, [300.0, 250.0], crop).unwrap();
        close(m.framing.zoom, zoom);
        close(m.framing.offset[0], offset[0]);
        close(m.framing.offset[1], offset[1]);
        close(m.scale * input[0], projected[0]);
        close(m.scale * input[1], projected[1]);
        close(m.retained[0] * m.scale, 300.0);
        close(m.retained[1] * m.scale, 250.0);
    }
}
#[test]
fn ratios_extremes_fractional_bounds_and_repeat_cycles_roundtrip() {
    for input in [
        [1920.0, 1080.0],
        [1080.0, 1920.0],
        [1919.0, 999.0],
        [4000.0, 17.0],
    ] {
        for size in [
            [300.0, 250.0],
            [250.0, 300.0],
            [300.5, 300.5],
            [1.0, 4000.0],
        ] {
            for zoom in [1.0, 1.375, 2.0, 100.0] {
                let (_, limits) = limits(input, size, zoom).unwrap();
                let framing = Framing {
                    zoom,
                    offset: [limits[0] * 0.37, -limits[1] * 0.63],
                };
                let mut crop = crop_from_framing(input, size, framing).unwrap();
                for _ in 0..50 {
                    let m = mapping(input, size, crop).unwrap();
                    close(m.framing.zoom, zoom);
                    close(m.framing.offset[0], framing.offset[0]);
                    close(m.framing.offset[1], framing.offset[1]);
                    crop = crop_from_framing(input, size, m.framing).unwrap();
                }
                let baseline = crop_from_framing(input, size, Framing::default()).unwrap();
                close(mapping(input, size, baseline).unwrap().framing.zoom, 1.0);
            }
        }
    }
}
#[test]
fn invalid_formats_crops_and_offsets_are_rejected() {
    for size in [
        [1920.5, 1080.0],
        [f64::NAN, 1080.0],
        [-1.0, 100.0],
        [16385.0, 1080.0],
    ] {
        assert!(resolution_from_values(size).is_err());
    }
    for size in [[0, 1080], [1920, 0], [16385, 100]] {
        assert!(validate_resolution(size).is_err());
    }
    assert!(
        Source {
            native: [1920, 1200],
            ..Source::default()
        }
        .input()
        .is_err()
    );
    assert!(Crop::centered([1920.0, 1080.0], [1921.0, 1000.0]).is_err());
    assert!(
        Crop {
            left: -1.0,
            ..Crop::default()
        }
        .remaining([1920.0, 1080.0])
        .is_err()
    );
    assert!(
        Crop {
            left: 1920.0,
            ..Crop::default()
        }
        .remaining([1920.0, 1080.0])
        .is_err()
    );
    assert!(mapping([1920.0, 1080.0], [300.0, 250.0], Crop::default()).is_err());
    for framing in [
        Framing {
            zoom: 0.99,
            offset: [0.0; 2],
        },
        Framing {
            zoom: f64::NAN,
            offset: [0.0; 2],
        },
        Framing {
            zoom: 101.0,
            offset: [0.0; 2],
        },
        Framing {
            zoom: 1.0,
            offset: [0.0, 1.0],
        },
        Framing {
            zoom: 2.0,
            offset: [f64::INFINITY, 0.0],
        },
    ] {
        assert!(crop_from_framing([1920.0, 1080.0], [300.0, 250.0], framing).is_err());
    }
}
#[test]
fn processed_source_dimensions_are_used_without_extra_input_crop() {
    let mut layout = Layout::default();
    let source = Source {
        native: [1920, 1000],
        aspect: Aspect::Custom,
    };
    layout
        .configure(1, 1, source, box_size(300.0, 250.0), false)
        .unwrap();
    layout
        .set_framing(
            1,
            Framing {
                zoom: 2.0,
                offset: [0.0; 2],
            },
        )
        .unwrap();
    assert_eq!(layout.formats().sources[1].aspect, Aspect::Custom);
    let c = layout.formats().advanced[1];
    close(c.top, 250.0);
    close(c.left, 660.0);
    let report = layout.calculation_report(1).unwrap();
    assert!(report.contains("not a verified device recipe"));
    assert!(report.contains("x 660.0, y 250.0, w 600.0, h 500.0"));
}
#[test]
fn shared_sources_update_atomically_and_layer_framing_stays_independent() {
    let mut layout = Layout::default();
    layout
        .configure(1, 0, Source::default(), box_size(300.0, 250.0), false)
        .unwrap();
    layout
        .set_framing(
            1,
            Framing {
                zoom: 2.0,
                offset: [20.0, 0.0],
            },
        )
        .unwrap();
    let old = layout.formats().clone();
    let boxes = layout.layers().clone();
    let source = Source {
        native: [1920, 1000],
        aspect: Aspect::Custom,
    };
    layout.configure(1, 0, source, boxes[1], false).unwrap();
    assert_eq!(layout.formats().input(0).unwrap(), [1920.0, 1000.0]);
    assert_eq!(layout.formats().input(1).unwrap(), [1920.0, 1000.0]);
    close(
        layout.formats().mapping(0, boxes[0]).unwrap().framing.zoom,
        1.0,
    );
    close(
        layout.formats().mapping(1, boxes[1]).unwrap().framing.zoom,
        2.0,
    );
    assert_eq!(*layout.layers(), boxes);
    let after = layout.formats().clone();
    layout.undo();
    assert_eq!(*layout.formats(), old);
    layout.redo();
    assert_eq!(*layout.formats(), after);
    layout.set_locked(0, true);
    assert!(
        layout
            .configure(1, 0, Source::default(), boxes[1], true)
            .is_err()
    );
    assert_eq!(*layout.formats(), after);
}
#[test]
fn resize_adjusts_crop_automatically_and_undo_restores_framing() {
    let mut layout = Layout::default();
    let g = box_size(300.0, 250.0);
    layout.apply(1, g).unwrap();
    layout
        .set_framing(
            1,
            Framing {
                zoom: 1.0,
                offset: [70.0, 0.0],
            },
        )
        .unwrap();
    let before = layout.formats().clone();
    let next = box_size(640.0, 360.0);
    layout.apply(1, next).unwrap();
    assert_eq!(layout.layers()[1], next);
    close(
        layout.formats().mapping(1, next).unwrap().framing.offset[0],
        0.0,
    );
    layout.undo();
    assert_eq!(layout.layers()[1], g);
    assert_eq!(*layout.formats(), before);
    layout.start_resize(1, Corner::BottomRight, [400.0, 450.0], false);
    layout.drag_to([740.0, 560.0]);
    layout.finish_drag();
    assert_eq!(layout.layers()[1], next);
    close(
        layout.formats().mapping(1, next).unwrap().framing.offset[0],
        0.0,
    );
    layout.undo();
    assert_eq!(layout.layers()[1], g);
    assert_eq!(*layout.formats(), before);
}
#[test]
fn moving_and_output_changes_do_not_mutate_crops_and_screen_alignment_is_dynamic() {
    let mut layout = Layout::default();
    layout.apply(1, box_size(300.0, 250.0)).unwrap();
    layout
        .set_framing(
            1,
            Framing {
                zoom: 2.0,
                offset: [30.0, 20.0],
            },
        )
        .unwrap();
    let crops = layout.formats().advanced.clone();
    let original = layout.layers().clone();
    layout.set_output([2560, 1600]).unwrap();
    assert_eq!(*layout.layers(), original);
    assert_eq!(layout.formats().advanced, crops);
    layout
        .align(Alignment::Bottom, AlignTarget::Screen)
        .unwrap();
    close(layout.layers()[1].y + layout.layers()[1].height, 1600.0);
    layout.nudge(1.0, 2.0).unwrap();
    assert_eq!(layout.formats().advanced, crops);
    let g = layout.layers()[1];
    let offset = Corner::BottomRight.offsets_on(g, [2560, 1600]);
    assert_eq!(
        Corner::BottomRight
            .geometry_on(offset[0], offset[1], g.width, g.height, [2560, 1600])
            .unwrap(),
        g
    );
    assert!(
        !Geometry {
            x: 2000.0,
            y: 100.0,
            width: 200.0,
            height: 200.0
        }
        .outside_on([2560, 1600])
    );
}
#[test]
fn content_drag_clamps_crops_and_undo_and_escape_restore_full_state() {
    let mut layout = Layout::default();
    layout.apply(1, box_size(300.0, 250.0)).unwrap();
    layout
        .set_framing(
            1,
            Framing {
                zoom: 2.0,
                offset: [0.0; 2],
            },
        )
        .unwrap();
    let before = layout.formats().clone();
    let boxes = layout.layers().clone();
    layout.start_content_drag(1, [0.0; 2]);
    layout.drag_to([1e6, -1e6]);
    let m = layout.formats().mapping(1, boxes[1]).unwrap();
    close(m.framing.offset[0], m.offset_limits[0]);
    close(m.framing.offset[1], -m.offset_limits[1]);
    layout.finish_drag();
    assert_eq!(*layout.layers(), boxes);
    layout.undo();
    assert_eq!(*layout.formats(), before);
    layout.start_content_drag(1, [0.0; 2]);
    layout.drag_to([20.0, 30.0]);
    layout.cancel_drag();
    assert_eq!(*layout.formats(), before);
    assert_eq!(*layout.layers(), boxes);
}

#[test]
fn view_and_export_rounding_never_changes_crop_geometry() {
    use canvas_pilot::{export_pixels, view_pixels};
    assert_eq!(view_pixels(12.36), "12.4");
    assert_eq!(export_pixels(12.36), "12");
    assert_eq!(export_pixels(12.5), "13");
    assert_eq!(export_pixels(-12.5), "-13");
    assert_eq!(view_pixels(-0.01), "0.0");
    assert_eq!(export_pixels(-0.1), "0");
    let mut layout = Layout::default();
    let g = Geometry {
        x: 12.36,
        y: 20.51,
        width: 300.33,
        height: 250.58,
    };
    layout.apply(1, g).unwrap();
    let before = layout.formats().clone();
    assert!(
        layout
            .calculation_report(1)
            .unwrap()
            .contains("300.3 x 250.6")
    );
    assert!(layout.calculation_export(1).unwrap().contains("300 x 251"));
    assert_eq!(layout.layers()[1], g);
    assert_eq!(*layout.formats(), before);
}

#[test]
fn crop_alignment_handles_cover_cropping_and_protected_layers() {
    for zoom in [1.0, 3.0] {
        let mut layout = Layout::default();
        layout.apply(1, box_size(300.0, 300.0)).unwrap();
        layout
            .set_framing(
                1,
                Framing {
                    zoom,
                    offset: [0.0; 2],
                },
            )
            .unwrap();
        let boxes = layout.layers().clone();
        layout
            .align_content(1, &[Alignment::Left, Alignment::Top])
            .unwrap();
        let crop = layout.formats().advanced[1];
        close(crop.left, 0.0);
        close(crop.top, 0.0);
        layout
            .align_content(1, &[Alignment::Right, Alignment::Bottom])
            .unwrap();
        let crop = layout.formats().advanced[1];
        close(crop.right, 0.0);
        close(crop.bottom, 0.0);
        layout
            .align_content(1, &[Alignment::HorizontalCenter, Alignment::VerticalCenter])
            .unwrap();
        let actual = layout.formats().mapping(1, boxes[1]).unwrap().framing;
        close(actual.zoom, zoom);
        close(actual.offset[0], 0.0);
        close(actual.offset[1], 0.0);
        assert_eq!(layout.layers(), &boxes);
        let before = layout.formats().clone();
        layout.set_locked(1, true);
        assert!(layout.align_content(1, &[Alignment::Left]).is_err());
        layout.set_locked(1, false);
        layout.set_visible(1, false);
        assert!(layout.align_content(1, &[Alignment::Left]).is_err());
        assert!(layout.align_content(1000, &[Alignment::Left]).is_err());
        assert_eq!(layout.formats(), &before);
    }
}
