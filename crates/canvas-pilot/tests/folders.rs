use canvas_pilot::{Layout, project::Project};

fn document() -> Layout {
    let mut layout = Layout::default();
    for _ in 0..3 {
        layout.add_layer().unwrap();
    }
    layout.assign_folder(&[2, 3], "Cameras").unwrap();
    layout.assign_folder(&[1, 4], "Graphics").unwrap();
    layout
}

#[test]
fn folder_edits_are_atomic_and_preserve_saved_members() {
    let mut layout = document();
    let before = Project::from_layout(&layout);
    layout.rename_folder("Cameras", "Video").unwrap();
    assert_eq!(layout.folder_members("Video"), vec![0, 2]);
    assert!(layout.rename_folder("Video", "Graphics").is_err());
    assert!(layout.rename_folder("Video", " ").is_err());
    layout.undo();
    assert_eq!(
        Project::from_layout(&layout).to_json().unwrap(),
        before.to_json().unwrap()
    );
    layout.redo();
    let saved = Project::from_layout(&layout).to_json().unwrap();
    let loaded = Project::from_json(&saved).unwrap().layout().unwrap();
    assert_eq!(loaded.folder_members("Video"), vec![0, 2]);
    layout.assign_folder(&[2, 3], "").unwrap();
    assert!(!layout.folders().contains(&"Video".into()));
    layout.undo();
    assert_eq!(layout.folder_members("Video"), vec![0, 2]);
}

#[test]
fn folder_selection_and_state_respect_protected_layers() {
    let mut layout = document();
    layout.set_locked(0, true);
    layout.select_folder("Cameras", false);
    assert_eq!(layout.selected_indices(), vec![2]);
    layout.select_folder("Graphics", true);
    assert_eq!(layout.selected_indices(), vec![1, 2, 3]);
    layout.select_folder("Graphics", true);
    assert_eq!(layout.selected_indices(), vec![2]);
    layout.set_folder_state("Cameras", Some(false), None);
    assert!(!layout.visible(0) && !layout.visible(2));
    assert!(layout.selected_indices().is_empty());
    assert!(layout.visible(1));
    layout.undo();
    assert!(layout.visible(0) && layout.visible(2));
    layout.set_folder_state("Cameras", None, Some(false));
    assert!(!layout.locked(0) && !layout.locked(2));
    layout.undo();
    assert!(layout.locked(0) && !layout.locked(2));
}

#[test]
fn moving_folders_and_layer_membership_preserves_complete_layer_records() {
    let mut layout = document();
    let before = Project::from_layout(&layout);
    layout.move_folder("Cameras", 4, true).unwrap();
    let after = Project::from_layout(&layout);
    assert_eq!(
        after.layers.iter().map(|l| l.id).collect::<Vec<_>>(),
        vec![1, 4, 2, 3, 5]
    );
    for layer in &after.layers {
        assert_eq!(
            serde_json::to_value(layer).unwrap(),
            serde_json::to_value(before.layers.iter().find(|old| old.id == layer.id).unwrap())
                .unwrap()
        );
    }
    layout.undo();
    assert_eq!(
        Project::from_layout(&layout).to_json().unwrap(),
        before.to_json().unwrap()
    );
    layout.move_layer_to_folder(5, "Cameras", 1).unwrap();
    assert_eq!(layout.layer_info(1).id, 5);
    assert_eq!(layout.layer_info(1).folder, "Cameras");
    layout.undo();
    assert_eq!(
        Project::from_layout(&layout).to_json().unwrap(),
        before.to_json().unwrap()
    );
    assert!(layout.assign_folder(&[999], "Bad").is_err());
    assert!(layout.move_folder("Missing", 1, true).is_err());
    assert!(layout.move_layer_to_folder(1, "Bad", 99).is_err());
    assert_eq!(
        Project::from_layout(&layout).to_json().unwrap(),
        before.to_json().unwrap()
    );
}

#[test]
fn explicit_empty_folders_survive_history_save_rename_and_membership_changes() {
    let mut layout = Layout::default();
    let before = Project::from_layout(&layout);
    layout.create_folder("Memory A").unwrap();
    assert!(layout.folder_members("Memory A").is_empty());
    assert_eq!(Project::from_layout(&layout).layers, before.layers);
    layout.undo();
    assert!(layout.folders().is_empty());
    layout.redo();
    let mut p = Project::from_layout(&layout);
    p.workspace.collapsed_folders.push("Memory A".into());
    let mut loaded = Project::from_json(&p.to_json().unwrap())
        .unwrap()
        .layout()
        .unwrap();
    assert_eq!(loaded.folders(), ["Memory A"]);
    loaded.rename_folder("Memory A", "Memory B").unwrap();
    loaded.assign_folder(&[1, 2], "Memory B").unwrap();
    loaded.assign_folder(&[1, 2], "").unwrap();
    assert_eq!(loaded.folders(), ["Memory B"]);
    assert!(loaded.create_folder("Memory B").is_err());
    assert!(loaded.create_folder(" ").is_err());
    loaded.ungroup_folder("Memory B").unwrap();
    assert!(loaded.folders().is_empty());
    loaded.undo();
    assert_eq!(loaded.folders(), ["Memory B"]);
    loaded
        .create_folder_with_layers("Together", &[1, 2])
        .unwrap();
    assert_eq!(loaded.folder_members("Together"), [0, 1]);
    loaded.undo();
    assert!(!loaded.folders().contains(&"Together".into()));
    assert!(loaded.folder_members("").len() == 2);
}

#[test]
fn version_12_explicit_folder_validation_and_v11_migration() {
    let old = include_str!("../../../docs/interfaces/canvas-pilot-v11.example.json");
    let p = Project::from_json(old).unwrap();
    assert_eq!(p.version, 20);
    assert!(p.folders.is_empty());
    let example = include_str!("../../../docs/interfaces/canvas-pilot-v12.example.json");
    assert_eq!(
        Project::from_json(example)
            .unwrap()
            .layout()
            .unwrap()
            .folders(),
        ["Memory A"]
    );
    for names in [
        vec!["".into()],
        vec!["Same".into(), "Same".into()],
        vec![" padded ".into()],
        vec!["x".repeat(129)],
    ] {
        let mut invalid = p.clone();
        invalid.folders = names;
        assert!(invalid.to_json().is_err());
    }
    let mut old: serde_json::Value = serde_json::from_str(old).unwrap();
    old["folders"] = serde_json::json!([]);
    assert!(Project::from_json(&old.to_string()).is_err());
}
