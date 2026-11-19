use canvas_pilot::{
    ConnectionTarget, DeviceLink, Layout, SendDestination, SendMapping, project::Project,
};

fn link(screen: &str, layer: &str) -> DeviceLink {
    DeviceLink {
        family: Default::default(),
        screen: screen.into(),
        layer: layer.into(),
    }
}
fn sends() -> SendMapping {
    SendMapping {
        destinations: vec![link("1", "2"), link("2", "3")]
            .into_iter()
            .map(|layer| SendDestination {
                connection: ConnectionTarget::Current,
                layer,
            })
            .collect(),
    }
}

#[test]
fn independent_directions_roundtrip_and_undo_together() {
    let mut layout = Layout::default();
    let before = Project::from_layout(&layout);
    layout
        .set_device_mappings(0, Some(link("1", "1")), sends())
        .unwrap();
    let project = Project::from_layout(&layout);
    assert_eq!(project.layers[0].load_mapping, Some(link("1", "1")));
    assert_eq!(project.layers[0].send_mapping, sends());
    let reopened = Project::from_json(&project.to_json().unwrap()).unwrap();
    assert_eq!(Project::from_layout(&reopened.layout().unwrap()), project);
    layout.undo();
    assert_eq!(Project::from_layout(&layout), before);
    layout.redo();
    assert_eq!(Project::from_layout(&layout), project);
    layout.set_device_link(0, Some(link("3", "4"))).unwrap();
    assert_eq!(layout.layer_info(0).load_mapping, Some(link("3", "4")));
    assert_eq!(
        layout.layer_info(0).send_mapping,
        SendMapping::linked(Some(link("3", "4")))
    );
    layout.set_device_link(0, None).unwrap();
    assert!(layout.layer_info(0).load_mapping.is_none());
    assert!(layout.layer_info(0).send_mapping.destinations.is_empty());
}

#[test]
fn v18_migration_synchronizes_links_and_v19_requires_both_directions() {
    let old = include_str!("../../../docs/interfaces/canvas-pilot-v18.example.json");
    let project = canvas_pilot::project_recovery::from_json(old)
        .unwrap()
        .project;
    assert_eq!(project.version, 22);
    for layer in &project.layers {
        assert_eq!(
            layer.send_mapping,
            SendMapping::linked(layer.load_mapping.clone())
        );
    }
    assert_eq!(
        project,
        Project::from_json(include_str!(
            "../../../docs/interfaces/canvas-pilot-v22.example.json"
        ))
        .unwrap()
    );
    let json = serde_json::to_value(&project).unwrap();
    for key in ["load_mapping", "send_mapping"] {
        let mut bad = json.clone();
        bad["layers"][0].as_object_mut().unwrap().remove(key);
        assert!(Project::from_json(&bad.to_string()).is_err());
    }
    let mut bad = json.clone();
    bad["version"] = 18.into();
    assert!(Project::from_json(&bad.to_string()).is_err());
    let mut bad = json;
    bad["layers"][0]["send_mapping"]["destinations"][0]["connection"] = "other-machine".into();
    assert!(Project::from_json(&bad.to_string()).is_err());
}

#[test]
fn invalid_destinations_are_atomic_and_repeated_load_sources_are_allowed() {
    let mut layout = Layout::default();
    layout.set_device_link(0, Some(link("1", "1"))).unwrap();
    layout.set_device_link(1, Some(link("1", "1"))).unwrap();
    let before = Project::from_layout(&layout);
    let mut duplicate = sends();
    duplicate
        .destinations
        .push(duplicate.destinations[0].clone());
    assert!(layout.set_device_mappings(0, None, duplicate).is_err());
    let mut invalid = sends();
    invalid.destinations[0].layer.screen = "../1".into();
    assert!(layout.set_device_mappings(0, None, invalid).is_err());
    assert!(layout.set_device_mappings(99, None, sends()).is_err());
    assert_eq!(Project::from_layout(&layout), before);
}

#[test]
fn every_snapshot_codec_preserves_independent_directions_and_fanout() {
    use canvas_pilot::{
        exchange::Options,
        exchange_metadata as metadata,
        interchange::{self, Codec},
    };
    let mut layout = Layout::default();
    layout
        .set_device_mappings(0, Some(link("3", "1")), sends())
        .unwrap();
    let project = Project::from_layout(&layout);
    let ids = [project.layers[0].id];
    let expected = metadata::snapshot(&project, &ids).unwrap();
    for codec in Codec::available()
        .into_iter()
        .filter(|&c| metadata::supported(c))
    {
        let bytes = interchange::export(
            &project,
            &ids,
            codec,
            Options {
                extra_parameters: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            metadata::extract(&bytes, codec).unwrap(),
            Some(expected.clone()),
            "{codec:?}"
        );
    }
}
