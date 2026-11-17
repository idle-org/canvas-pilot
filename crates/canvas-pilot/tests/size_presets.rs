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
