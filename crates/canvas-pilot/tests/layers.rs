use canvas_pilot::{AlignTarget, Alignment, Layout, ResizeHandle, ResizeMode, project::Project};
#[test]
fn add_remove_undo_redo_preserve_ids_names_and_crops() {
    let mut layout = Layout::default();
    let index = layout.add_layer().unwrap();
    assert_eq!(index, 2);
    let id = layout.layer_info(index).id;
    layout.rename_layer(index, "Camera", "Cameras").unwrap();
    let before = Project::from_layout(&layout);
    layout.remove_selected().unwrap();
    assert_eq!(layout.layers().len(), 2);
    layout.undo();
    assert_eq!(Project::from_layout(&layout), before);
    layout.redo();
    assert_eq!(layout.layers().len(), 2);
    let new = layout.add_layer().unwrap();
    assert!(layout.layer_info(new).id > id);
    let saved = Project::from_layout(&layout);
    assert_eq!(
        Project::from_json(&saved.to_json().unwrap()).unwrap(),
        saved
    );
}
#[test]
fn many_layers_move_align_and_resize_with_one_undo() {
    let mut layout = Layout::default();
    for _ in 0..8 {
        layout.add_layer().unwrap();
    }
    layout.set_selection(vec![true; 10]);
    layout.nudge(21.0, 34.0).unwrap();
    let before = layout.layers().clone();
    layout.align(Alignment::Top, AlignTarget::Screen).unwrap();
    assert!(layout.layers().iter().all(|g| g.y == 0.0));
    layout.undo();
    assert_eq!(*layout.layers(), before);
    let g = layout.layers()[9];
    layout
        .start_selected_resize(
            9,
            ResizeHandle::Right,
            [g.x + g.width, g.y],
            false,
            ResizeMode::Same,
        )
        .unwrap();
    layout.drag_to([g.x + g.width * 2.0, g.y]);
    layout.finish_drag();
    for (a, b) in layout.layers().iter().zip(&before) {
        assert_eq!(a.width, b.width * 2.0);
    }
    layout.undo();
    assert_eq!(*layout.layers(), before);
    assert!(
        layout
            .start_selected_resize(9, ResizeHandle::Right, [0.0, 0.0], true, ResizeMode::Mirror)
            .is_err()
    );
}
#[test]
fn identities_selection_and_selected_svg_survive_deletion() {
    let mut layout = Layout::default();
    layout.add_layer().unwrap();
    layout.select(Some(0));
    layout.remove_selected().unwrap();
    layout.select(Some(1));
    let project = Project::from_layout(&layout);
    assert_eq!(project.layers[0].id, 1);
    assert_eq!(project.layers[1].id, 3);
    assert_eq!(project.editor.selected_layer_ids, vec![3]);
    let loaded = project.layout().unwrap();
    assert_eq!(loaded.selected(), Some(1));
    let svg = project.to_svg_ids(&[3]).unwrap();
    assert!(svg.contains("layer-3"));
    assert!(!svg.contains("layer-1"));
    assert!(project.to_svg_ids(&[2]).is_err());
    assert!(project.to_svg_ids(&[3, 3]).is_err());
    let mut invalid = project.clone();
    invalid.next_layer_id = 3;
    assert!(invalid.layout().is_err());
    let mut invalid = project.clone();
    invalid.layers[1].id = 1;
    assert!(invalid.layout().is_err());
}
#[test]
fn whole_list_rename_is_atomic_and_empty_document_is_refused() {
    let mut layout = Layout::default();
    let before = Project::from_layout(&layout);
    assert!(
        layout
            .rename_layers(&[(2, "Valid".into(), "".into()), (1, "".into(), "".into())])
            .is_err()
    );
    assert_eq!(Project::from_layout(&layout), before);
    layout.set_selection([true, true]);
    assert!(layout.remove_selected().is_err());
    layout
        .rename_layers(&[
            (2, "Left".into(), "Pair".into()),
            (1, "Right".into(), "Pair".into()),
        ])
        .unwrap();
    layout.undo();
    assert_eq!(layout.layer_info(0).name, "Slides");
    assert_eq!(layout.layer_info(1).name, "Presenter");
}
#[test]
fn documented_v3_example_keeps_original_labels() {
    let project = Project::from_json(include_str!(
        "../../../docs/interfaces/canvas-pilot-v3.example.json"
    ))
    .unwrap();
    assert_eq!(project.layers[0].id, 1);
    assert_eq!(project.layers[0].name, "Presenter");
    assert_eq!(project.layers[1].id, 2);
    assert_eq!(project.layers[1].name, "Slides");
}

#[test]
fn visibility_and_lock_changes_are_local_undoable_edits() {
    let mut layout = Layout::default();
    let before = layout.layers().clone();
    layout.set_visible(1, false);
    assert!(!layout.visible(1));
    layout.undo();
    assert!(layout.visible(1));
    layout.redo();
    assert!(!layout.visible(1));
    layout.set_locked(0, true);
    assert!(layout.locked(0));
    layout.undo();
    assert!(!layout.locked(0));
    assert_eq!(*layout.layers(), before);
}

#[test]
fn reorder_preserves_complete_layer_identity_history_and_file_order() {
    use canvas_pilot::{DeviceLink, formats::Framing};
    let mut layout = Layout::default();
    layout.add_layer().unwrap();
    layout.rename_layer(0, "Camera", "Video").unwrap();
    layout
        .set_device_link(
            0,
            Some(DeviceLink {
                family: Default::default(),
                screen: "2".into(),
                layer: "1".into(),
            }),
        )
        .unwrap();
    layout
        .set_framing(
            0,
            Framing {
                zoom: 2.0,
                offset: [20.0, -10.0],
            },
        )
        .unwrap();
    // Share an input while retaining different per-layer crops.
    layout
        .configure(2, 0, layout.formats().sources[0], layout.layers()[2], true)
        .unwrap();
    layout.set_locked(0, true);
    layout.set_visible(0, false);
    layout.set_selection([false, true, true]);
    let before = Project::from_layout(&layout);
    layout.move_layer(2, 2).unwrap();
    let after = Project::from_layout(&layout);
    assert_eq!(
        after.layers.iter().map(|l| l.id).collect::<Vec<_>>(),
        [1, 3, 2]
    );
    for layer in &before.layers {
        assert_eq!(
            after.layers.iter().find(|l| l.id == layer.id).unwrap(),
            layer
        );
    }
    assert_eq!(after.inputs, before.inputs);
    assert_eq!(after.next_layer_id, before.next_layer_id);
    assert_eq!(after.editor.selected_layer_ids, [1, 3]);
    let loaded = Project::from_json(&after.to_json().unwrap())
        .unwrap()
        .layout()
        .unwrap();
    assert_eq!(Project::from_layout(&loaded), after);
    let svg = after.to_svg(false).unwrap();
    assert!(svg.find("layer-1").unwrap() < svg.find("layer-3").unwrap());
    assert!(svg.find("layer-3").unwrap() < svg.find("layer-2").unwrap());
    layout.undo();
    assert_eq!(Project::from_layout(&layout), before);
    layout.redo();
    assert_eq!(Project::from_layout(&layout), after);
    layout.move_layer(2, 0).unwrap();
    assert_eq!(Project::from_layout(&layout), before);
}

#[test]
fn invalid_and_noop_reorders_leave_history_and_ids_unchanged() {
    let mut layout = Layout::default();
    layout.move_layer(2, 1).unwrap();
    let before = Project::from_layout(&layout);
    assert!(layout.move_layer(999, 0).is_err());
    assert!(layout.move_layer(2, 2).is_err());
    layout.move_layer(2, 1).unwrap();
    assert_eq!(Project::from_layout(&layout), before);
    layout.undo();
    assert_eq!(layout.layer_info(0).id, 2);
    assert_eq!(layout.layer_info(1).id, 1);
}

#[test]
fn reordered_stacking_and_hit_testing_follow_document_order() {
    let mut layout = Layout::default();
    layout.apply(1, layout.layers()[0]).unwrap();
    layout.select(None);
    assert_eq!(layout.layer_info(layout.hit([200.0, 200.0]).unwrap()).id, 1);
    layout.move_layer(2, 1).unwrap();
    assert_eq!(layout.layer_info(layout.hit([200.0, 200.0]).unwrap()).id, 2);
    layout.select(Some(0));
    // Selecting the lower layer does not raise it above the saved top layer.
    assert_eq!(layout.layer_info(layout.hit([200.0, 200.0]).unwrap()).id, 2);
}
