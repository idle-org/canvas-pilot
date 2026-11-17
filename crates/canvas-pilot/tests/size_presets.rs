use canvas_pilot::{DeviceLink, Geometry, Layout, project::Project, size_presets::SizePreset};

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

#[test]
fn full_screen_and_nine_quarter_positions_use_exact_output_dimensions() {
    let mut layout = Layout::default();
    layout.set_output([1919, 1079]).unwrap();
    layout.select(Some(0));
    let unselected = layout.layers()[1];
    for row in 0..3 {
        for column in 0..3 {
            let before = Project::from_layout(&layout);
            layout
                .apply_size_preset(SizePreset::Quarter { column, row })
                .unwrap();
            let g = layout.layers()[0];
            assert_eq!(
                g,
                Geometry {
                    x: column as f64 * 479.75,
                    y: row as f64 * 269.75,
                    width: 959.5,
                    height: 539.5
                }
            );
            assert_eq!(layout.layers()[1], unselected);
            layout.undo();
            assert_eq!(Project::from_layout(&layout), before);
        }
    }
    layout.set_selection([true, true]);
    let before = Project::from_layout(&layout);
    layout.apply_size_preset(SizePreset::Fullscreen).unwrap();
    for g in layout.layers() {
        assert_eq!(
            *g,
            Geometry {
                x: 0.0,
                y: 0.0,
                width: 1919.0,
                height: 1079.0
            }
        );
    }
    layout.undo();
    assert_eq!(Project::from_layout(&layout), before);
    layout.redo();
    assert_eq!(layout.layers()[0], layout.layers()[1]);
}

#[test]
fn tiling_follows_folder_list_order_and_repeats_cells_without_reordering() {
    let mut layout = Layout::default();
    for _ in 0..4 {
        layout.add_layer().unwrap();
    }
    layout.assign_folder(&[1, 4], "Cameras").unwrap();
    layout.set_selection([true; 6]);
    layout
        .set_device_link(
            0,
            Some(DeviceLink {
                family: Default::default(),
                screen: "1".into(),
                layer: "1".into(),
            }),
        )
        .unwrap();
    layout
        .set_mask(
            0,
            canvas_pilot::formats::Mask {
                left: 0.1,
                ..Default::default()
            },
        )
        .unwrap();
    let before = Project::from_layout(&layout);
    let formats = layout.formats().clone();
    let identities: Vec<_> = (0..6).map(|i| layout.layer_info(i).clone()).collect();
    layout
        .apply_size_preset(SizePreset::Tiles {
            columns: 2,
            rows: 2,
        })
        .unwrap();
    for (slot, id) in [6, 5, 4, 1, 3, 2].into_iter().enumerate() {
        let i = (0..6).find(|&i| layout.layer_info(i).id == id).unwrap();
        assert_eq!(
            layout.layers()[i],
            Geometry {
                x: (slot % 2) as f64 * 960.0,
                y: ((slot % 4) / 2) as f64 * 540.0,
                width: 960.0,
                height: 540.0
            }
        );
    }
    assert_eq!(layout.formats().sources, formats.sources);
    assert_eq!(layout.formats().source_for, formats.source_for);
    assert_eq!(layout.formats().masks, formats.masks);
    assert_eq!(layout.formats().layer_fit, formats.layer_fit);
    assert_eq!(
        (0..6)
            .map(|i| layout.layer_info(i).clone())
            .collect::<Vec<_>>(),
        identities
    );
    layout.undo();
    assert_eq!(Project::from_layout(&layout), before);
}

#[test]
fn halves_and_fractional_tiling_preserve_grid_edges() {
    let mut layout = Layout::default();
    layout.set_selection([true, true]);
    layout
        .apply_size_preset(SizePreset::Tiles {
            columns: 1,
            rows: 2,
        })
        .unwrap();
    assert_eq!(
        layout.layers()[1],
        Geometry {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 540.0
        }
    );
    assert_eq!(layout.layers()[0].y, 540.0);
    layout
        .apply_size_preset(SizePreset::Tiles {
            columns: 2,
            rows: 1,
        })
        .unwrap();
    assert_eq!(
        layout.layers()[1],
        Geometry {
            x: 0.0,
            y: 0.0,
            width: 960.0,
            height: 1080.0
        }
    );
    assert_eq!(layout.layers()[0].x, 960.0);
    layout.add_layer().unwrap();
    layout.set_selection([true; 3]);
    layout.set_output([1001, 777]).unwrap();
    layout
        .apply_size_preset(SizePreset::Tiles {
            columns: 3,
            rows: 1,
        })
        .unwrap();
    close(layout.layers()[0].x + layout.layers()[0].width, 1001.0);
    close(
        layout.layers()[1].x + layout.layers()[1].width,
        layout.layers()[0].x,
    );
    close(layout.layers()[2].width, 1001.0 / 3.0);
}

#[test]
fn orientation_changes_only_dimensions_and_repeated_orientation_is_noop() {
    let mut layout = Layout::default();
    layout.set_selection([true, true]);
    let original = layout.layers().clone();
    let formats = layout.formats().clone();
    layout.apply_size_preset(SizePreset::Portrait).unwrap();
    for (i, g) in layout.layers().iter().enumerate() {
        assert_eq!([g.x, g.y], [original[i].x, original[i].y]);
        assert_eq!([g.width, g.height], [original[i].height, original[i].width]);
    }
    layout.apply_size_preset(SizePreset::Portrait).unwrap();
    layout.undo();
    assert_eq!(*layout.layers(), original);
    assert_eq!(*layout.formats(), formats);
    layout.redo();
    layout.apply_size_preset(SizePreset::Landscape).unwrap();
    assert_eq!(*layout.layers(), original);
}

#[test]
fn preset_validation_and_hidden_locked_layers_cannot_partially_change_document() {
    let mut layout = Layout::default();
    layout.set_selection([true, true]);
    let before = Project::from_layout(&layout);
    for preset in [
        SizePreset::Tiles {
            columns: 0,
            rows: 2,
        },
        SizePreset::Tiles {
            columns: 257,
            rows: 1,
        },
        SizePreset::Quarter { column: 1, row: 1 },
    ] {
        assert!(layout.apply_size_preset(preset).is_err());
        assert_eq!(Project::from_layout(&layout), before);
    }
    layout.set_locked(0, true);
    let protected = layout.layers()[0];
    layout.apply_size_preset(SizePreset::Fullscreen).unwrap();
    assert_eq!(layout.layers()[0], protected);
    layout.set_visible(1, false);
    assert!(layout.apply_size_preset(SizePreset::Fullscreen).is_err());
    layout.set_visible(1, true);
    layout.select(Some(1));
    assert!(
        layout
            .apply_size_preset(SizePreset::Quarter { column: 3, row: 0 })
            .is_err()
    );
}

#[test]
fn crop_presets_trim_current_picture_without_scaling_and_round_trip() {
    use canvas_pilot::formats::{Aspect, Framing, LayerFit};
    for fit in [
        None,
        Some(LayerFit::Fullscreen),
        Some(LayerFit::Centered),
        Some(LayerFit::Cropped),
        Some(LayerFit::OneToOne),
    ] {
        for (aspect, width, edge) in [
            (Aspect::FourThree, 1440.0, 240.0),
            (Aspect::SixteenTen, 1728.0, 96.0),
        ] {
            let mut layout = Layout::default();
            layout.select(Some(0));
            layout.apply_size_preset(SizePreset::Fullscreen).unwrap();
            layout.set_layer_fit(0, fit, Default::default()).unwrap();
            let before = Project::from_layout(&layout);
            layout.apply_crop_preset(aspect).unwrap();
            let g = layout.layers()[0];
            close(g.x, edge);
            close(g.y, 0.0);
            close(g.width, width);
            close(g.height, 1080.0);
            let f = layout.formats();
            close(f.advanced[0].left, edge);
            close(f.advanced[0].right, edge);
            close(f.advanced[0].top, 0.0);
            assert_eq!(f.layer_fit[0], fit);
            let after = Project::from_layout(&layout);
            assert_eq!(
                Project::from_json(&after.to_json().unwrap()).unwrap(),
                after
            );
            layout.apply_crop_preset(aspect).unwrap();
            assert_eq!(Project::from_layout(&layout), after);
            layout.undo();
            assert_eq!(Project::from_layout(&layout), before);
            layout.redo();
            assert_eq!(Project::from_layout(&layout), after);
        }
    }
    // Existing off-center framing is trimmed in place, not reset or recentered.
    let mut layout = Layout::default();
    layout.select(Some(0));
    layout
        .set_framing(
            0,
            Framing {
                zoom: 2.0,
                offset: [30.0, -10.0],
            },
        )
        .unwrap();
    let old = layout.layers()[0];
    let crop = layout.formats().advanced[0];
    let scale = layout.formats().mapping(0, old).unwrap().scale;
    layout.apply_crop_preset(Aspect::FourThree).unwrap();
    let new = layout.layers()[0];
    close(layout.formats().mapping(0, new).unwrap().scale, scale);
    close(
        layout.formats().advanced[0].left,
        crop.left + (old.width - new.width) / 2.0 / scale,
    );
    close(layout.formats().advanced[0].top, crop.top);
}

#[test]
fn crop_presets_letterbox_multi_selection_matching_ratio_and_errors() {
    use canvas_pilot::formats::Aspect;
    let mut layout = Layout::default();
    layout.select(Some(0));
    layout
        .apply(
            0,
            Geometry {
                x: 13.25,
                y: 27.5,
                width: 800.0,
                height: 600.0,
            },
        )
        .unwrap();
    layout.set_selection([true, true]);
    let before = Project::from_layout(&layout);
    let same = layout.layers()[1];
    let same_crop = layout.formats().advanced[1];
    let old_crop = layout.formats().advanced[0];
    let old_scale = layout
        .formats()
        .mapping(0, layout.layers()[0])
        .unwrap()
        .scale;
    layout.apply_crop_preset(Aspect::SixteenNine).unwrap();
    assert_eq!(
        layout.layers()[0],
        Geometry {
            x: 13.25,
            y: 102.5,
            width: 800.0,
            height: 450.0
        }
    );
    close(
        layout.formats().advanced[0].top,
        old_crop.top + 75.0 / old_scale,
    );
    assert_eq!(layout.layers()[1], same);
    assert_eq!(layout.formats().advanced[1], same_crop);
    layout.undo();
    assert_eq!(Project::from_layout(&layout), before);
    assert!(layout.apply_crop_preset(Aspect::Custom).is_err());
    assert_eq!(Project::from_layout(&layout), before);
    layout.select(None);
    assert!(layout.apply_crop_preset(Aspect::FourThree).is_err());
}

#[test]
fn proportional_presets_fit_each_layer_and_undo_as_one_edit() {
    for (preset, columns, rows) in [
        (SizePreset::Fullscreen, 1, 1),
        (
            SizePreset::Tiles {
                columns: 1,
                rows: 2,
            },
            1,
            2,
        ),
        (
            SizePreset::Tiles {
                columns: 2,
                rows: 1,
            },
            2,
            1,
        ),
        (
            SizePreset::Tiles {
                columns: 3,
                rows: 2,
            },
            3,
            2,
        ),
    ] {
        let mut layout = Layout::default();
        layout.set_output([1919, 1079]).unwrap();
        layout.select(Some(0));
        layout
            .apply(
                0,
                Geometry {
                    x: 10.0,
                    y: 20.0,
                    width: 300.0,
                    height: 600.0,
                },
            )
            .unwrap();
        layout.set_selection([true, true]);
        let before = Project::from_layout(&layout);
        let original = layout.layers().clone();
        layout
            .apply_size_preset_with_proportions(preset, true)
            .unwrap();
        for (position, i) in [1, 0].into_iter().enumerate() {
            let g = layout.layers()[i];
            let w = 1919.0 / columns as f64;
            let h = 1079.0 / rows as f64;
            let cell = position % (columns * rows);
            close(g.width / g.height, original[i].width / original[i].height);
            close(g.x + g.width / 2.0, (cell % columns) as f64 * w + w / 2.0);
            close(g.y + g.height / 2.0, (cell / columns) as f64 * h + h / 2.0);
            assert!(g.width <= w + 1e-8 && g.height <= h + 1e-8);
            assert!((g.width - w).abs() < 1e-8 || (g.height - h).abs() < 1e-8);
        }
        let after = Project::from_layout(&layout);
        layout.undo();
        assert_eq!(Project::from_layout(&layout), before);
        layout.redo();
        assert_eq!(Project::from_layout(&layout), after);
    }
}

#[test]
fn proportional_quarters_center_in_all_nine_regions_and_orientation_stays_explicit() {
    let mut layout = Layout::default();
    layout.select(Some(0));
    layout
        .apply(
            0,
            Geometry {
                x: 0.0,
                y: 0.0,
                width: 600.0,
                height: 600.0,
            },
        )
        .unwrap();
    for row in 0..3 {
        for column in 0..3 {
            layout
                .apply_size_preset_with_proportions(SizePreset::Quarter { column, row }, true)
                .unwrap();
            assert_eq!(
                layout.layers()[0],
                Geometry {
                    x: column as f64 * 480.0 + 210.0,
                    y: row as f64 * 270.0,
                    width: 540.0,
                    height: 540.0,
                }
            );
        }
    }
    layout.apply_size_preset(SizePreset::Fullscreen).unwrap();
    layout
        .apply_size_preset_with_proportions(SizePreset::Portrait, true)
        .unwrap();
    assert_eq!(
        [layout.layers()[0].width, layout.layers()[0].height],
        [1080.0, 1920.0]
    );
}
