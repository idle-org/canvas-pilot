use canvas_pilot::{Layout, project::Project, project_recovery::from_json};
use serde_json::json;
fn value() -> serde_json::Value {
    serde_json::to_value(Project::from_layout(&Layout::default())).unwrap()
}

#[test]
fn valid_project_round_trip_is_exact_without_warnings_or_retired_fields() {
    let project = Project::from_layout(&Layout::default());
    let text = project.to_json().unwrap();
    assert!(!text.contains("input_crop"));
    assert!(!text.contains("input_proposals"));
    let loaded = from_json(&text).unwrap();
    assert!(loaded.warnings.is_empty());
    assert_eq!(loaded.project, project);
    assert_eq!(loaded.project.version, 20);
}

#[test]
fn bad_layer_does_not_discard_valid_geometry_and_invalid_selection_is_reset() {
    let mut v = value();
    let expected = v["layers"][1].clone();
    v["layers"][0]["bounds"]["width"] = json!(-1);
    v["editor"]["selected_layer_ids"] = json!([v["layers"][0]["id"]]);
    let loaded = from_json(&v.to_string()).unwrap();
    assert_eq!(loaded.project.layers.len(), 1);
    assert_eq!(
        serde_json::to_value(&loaded.project.layers[0]).unwrap(),
        expected
    );
    assert!(loaded.project.editor.selected_layer_ids.is_empty());
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| w.contains("Layer row 1 skipped"))
    );
    loaded.project.to_json().unwrap();
}

#[test]
fn invalid_optional_layer_settings_preserve_geometry_and_valid_names() {
    let mut v = value();
    v["layers"][0]["name"] = json!("Keep this name");
    v["layers"][0]["mask"]["left"] = json!(99);
    v["layers"][0]["load_mapping"] = json!({"invalid":true});
    v["layers"][1]["advanced_crop"]["left"] = json!(-1);
    v["editor"]["viewport_zoom"] = json!(0);
    v["workspace"]["capture_interval_seconds"] = json!(0);
    let loaded = from_json(&v.to_string()).unwrap();
    assert_eq!(loaded.project.layers.len(), 2);
    assert_eq!(loaded.project.layers[0].name, "Keep this name");
    assert_eq!(loaded.project.layers[0].mask, Default::default());
    assert!(loaded.project.layers[0].load_mapping.is_none());
    assert!(loaded.warnings.iter().any(|w| w.contains("mask")));
    for (actual, original) in loaded
        .project
        .layers
        .iter()
        .zip(v["layers"].as_array().unwrap())
    {
        assert_eq!(
            serde_json::to_value(actual.bounds).unwrap(),
            original["bounds"]
        );
    }
    loaded.project.to_json().unwrap();
}

#[test]
fn missing_source_skips_its_consumers_and_sparse_ids_are_remapped() {
    let mut v = value();
    v["inputs"][0]["native_resolution"]["width"] = json!(0);
    v["inputs"][1]["id"] = json!(800);
    v["layers"][1]["source_id"] = json!(800);
    let loaded = from_json(&v.to_string()).unwrap();
    assert_eq!(loaded.project.inputs.len(), 1);
    assert_eq!(loaded.project.layers.len(), 1);
    assert_eq!(loaded.project.layers[0].source_id, 1);
    assert_eq!(loaded.project.inputs[0].id, 1);
    loaded.project.layout().unwrap();
}

#[test]
fn removed_precrop_and_proposals_are_reported_and_never_resaved() {
    let mut v = value();
    v["version"] = json!(19);
    v["input_proposals"] = json!([{"arbitrary":"old proposal"}]);
    for input in v["inputs"].as_array_mut().unwrap() {
        input["input_crop"] = json!({"left":0,"right":0,"top":40,"bottom":40});
    }
    assert!(Project::from_json(&v.to_string()).is_err());
    let loaded = from_json(&v.to_string()).unwrap();
    assert!(loaded.warnings.iter().any(|w| w.contains("proposals")));
    assert!(loaded.warnings.iter().any(|w| w.contains("pre-crop")));
    let text = loaded.project.to_json().unwrap();
    assert!(!text.contains("input_crop"));
    assert!(!text.contains("input_proposals"));
}

#[test]
fn unsupported_unparseable_or_empty_projects_are_not_invented() {
    assert!(from_json("{broken").is_err());
    for (key, bad) in [
        ("version", json!(999)),
        ("format", json!("other")),
        ("layers", json!([])),
        ("inputs", json!([])),
    ] {
        let mut v = value();
        v[key] = bad;
        assert!(from_json(&v.to_string()).is_err(), "{key}");
    }
    let mut v = value();
    v["layers"][0]["id"] = json!(u64::MAX);
    v["layers"][1]["id"] = json!(0);
    assert!(from_json(&v.to_string()).is_err());
}

#[test]
fn invalid_embedded_images_do_not_prevent_geometry_recovery() {
    let mut v = value();
    v["references"] = json!([{"name":"bad", "image_base64":"invalid!", "visible":true, "opacity":1.0, "preserve_ratio":true, "native_size":false, "offset":[0.0,0.0]}]);
    let loaded = from_json(&v.to_string()).unwrap();
    assert_eq!(loaded.project.layers.len(), 2);
    assert!(loaded.project.references.is_empty());
    assert!(loaded.warnings.iter().any(|w| w.contains("references")));
}

#[test]
fn strict_legacy_reader_does_not_silently_drop_malformed_retired_settings() {
    let mut v = value();
    v["version"] = json!(19);
    v["input_proposals"] = json!({"unexpected":"object"});
    assert!(Project::from_json(&v.to_string()).is_err());
    let loaded = from_json(&v.to_string()).unwrap();
    assert!(loaded.warnings.iter().any(|w| w.contains("proposals")));
    loaded.project.layout().unwrap();
}
