use canvas_pilot::{
    DeviceLink, Geometry, Layout,
    device::{self, Family, Profile, Snapshot},
    device_capture,
    device_send::{Plan, Sizing, Target},
    formats::Framing,
    project::Project,
};
use serde_json::{Value, json};
fn identity() -> Value {
    json!({"type":"AQL Cmax","isLinked":false,"version":{"major":6,"minor":2,"patch":79}})
}
fn raw() -> Value {
    serde_json::from_str(include_str!("fixtures/aquilon-6.2.json")).unwrap()
}
fn routed() -> Value {
    let mut raw = raw();
    raw["device"]["screenList"]["items"]["S1"]["presetList"]["items"]["A"]["layerList"]["items"]
        ["1"]["source"]["pp"]["inputNum"] = json!("LIVE_1");
    raw
}
#[test]
fn discovered_slots_bank_roles_sources_and_disabled_screens() {
    let s = Snapshot::parse(&identity(), &raw()).unwrap();
    assert_eq!(s.profile, Profile::Aquilon6);
    assert_eq!(s.screens.len(), 1);
    let screen = &s.screens[0];
    assert_eq!(
        (&screen.id, screen.resolution, screen.layers.len()),
        (&"S1".to_owned(), [1920, 1080], 7)
    );
    assert_eq!(
        (&screen.preview_bank, &screen.program_bank),
        (&"A".to_owned(), &"B".to_owned())
    );
    assert!(
        screen
            .layers
            .iter()
            .all(|l| l.source == "NONE" && l.unsupported.is_none())
    );
    assert_eq!(screen.import(&[0, 6]).unwrap().shapes.len(), 2);
    let mut raw = raw();
    raw["device"]["screenAuxGroupList"]["items"]["S1"]["status"]["pp"]["transition"] =
        json!("AT_DOWN");
    let s = Snapshot::parse(&identity(), &raw).unwrap();
    assert_eq!(s.screens[0].preview_bank, "B");
    assert_eq!(s.screens[0].program_bank, "A");
    assert_eq!(s.screens[0].layers[0].source, "LIVE_1");
    assert!(s.screens[0].layers[0].framing.is_some());
}
#[test]
fn auxiliary_identity_and_per_group_bank_resolution_are_independent() {
    let mut raw = raw();
    let aux = &mut raw["device"]["auxiliaryList"]["items"]["A1"];
    aux["status"]["pp"]["mode"] = json!("FREESTYLE");
    aux["status"]["pp"]["layerCount"] = json!(1);
    aux["status"]["size"]["pp"]["sizeH"] = json!(3840);
    raw["device"]["screenAuxGroupList"]["items"]["A1"]["status"]["pp"]["transition"] =
        json!("AT_DOWN");
    let s = Snapshot::parse(&identity(), &raw).unwrap();
    assert_eq!(s.screens.len(), 2);
    assert_eq!(s.screens[1].id, "A1");
    assert_eq!(s.screens[1].resolution, [3840, 1080]);
    assert_eq!(s.screens[1].preview_bank, "B");
    assert!(
        Profile::Aquilon6
            .layer_path("A1", "B", "1")
            .contains("/auxiliaryList/")
    );
    assert_ne!(
        DeviceLink {
            family: Family::Aquilon,
            screen: "S1".into(),
            layer: "1".into()
        },
        DeviceLink {
            family: Family::Aquilon,
            screen: "A1".into(),
            layer: "1".into()
        }
    );
}
#[test]
fn invalid_profiles_transitions_allocations_and_missing_fields_fail() {
    for field in [
        "/device/screenList/items/S1/status/size/pp/sizeH",
        "/device/screenList/items/S1/status/pp/layerCount",
        "/device/screenAuxGroupList/items/S1/status/pp/transition",
    ] {
        let mut raw = raw();
        *raw.pointer_mut(field).unwrap() = json!("bad");
        assert!(Snapshot::parse(&identity(), &raw).is_err());
    }
    let mut id = identity();
    id["isLinked"] = json!(true);
    assert!(Snapshot::parse(&id, &raw()).is_err());
    id = identity();
    id["version"]["major"] = json!(7);
    assert!(Snapshot::parse(&id, &raw()).is_err());
    let mut r = raw();
    r["device"]["screenAuxGroupList"]["items"]["S1"]["control"]["pp"]["presetUp"] = json!("A");
    assert!(Snapshot::parse(&identity(), &r).is_err());
    let mut r = raw();
    r.pointer_mut("/device/screenList/items/S1/presetList/items/A/layerList/items")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("7");
    assert!(Snapshot::parse(&identity(), &r).is_err());
}
#[test]
fn capture_discovers_inputs_above_midra_limit_and_native_source_ids() {
    let s = Snapshot::parse(&identity(), &raw()).unwrap();
    let inputs = device_capture::inputs(&s);
    assert!(inputs.iter().any(|i| i.id == 17 && i.signal && i.enabled));
    assert_eq!(s.profile.input_from_source("LIVE_17"), Some(17));
    assert_eq!(s.profile.input_from_source("STILL_17"), None);
    assert_eq!(Profile::Midra4k.input_from_source("INPUT_17"), None);
}
#[test]
fn plans_round_trip_fractional_geometry_crop_and_both_sizing_modes() {
    for sizing in [Sizing::OutputBox, Sizing::SourceAspect] {
        let mut raw = routed();
        let mut layout = Layout::default();
        layout
            .apply(
                0,
                Geometry {
                    x: 12.125,
                    y: -34.375,
                    width: 301.25,
                    height: 251.5,
                },
            )
            .unwrap();
        layout
            .set_framing(
                0,
                Framing {
                    zoom: 2.0,
                    offset: [30.0, -10.0],
                },
            )
            .unwrap();
        let s = Snapshot::parse(&identity(), &raw).unwrap();
        let plan = Plan::prepare(
            &s,
            &layout,
            0,
            &[(layout.layer_info(0).id, "1".into())],
            Target::Preview,
            sizing,
        )
        .unwrap();
        assert_eq!(plan.bank, "A");
        assert_eq!(
            plan.profile.apply_path().join("/"),
            "device/screenAuxGroupList/control/pp/xUpdate"
        );
        for c in &plan.changes {
            let path = format!("/{}", c.path.join("/"));
            assert!(
                !path.contains("source/") && !path.contains("inputList") && !path.contains("xTake")
            );
            if path.contains("/position/pp/") {
                assert_eq!(
                    c.after.as_f64().unwrap().fract(),
                    0.0,
                    "WebRCS cannot render fractional native geometry"
                );
            }
            *raw.pointer_mut(&path).unwrap() = c.after.clone();
        }
        let decoded = Snapshot::parse(&identity(), &raw).unwrap();
        let layer = &decoded.screens[0].layers[0];
        assert!(layer.unsupported.is_none(), "{:?}", layer.unsupported);
        let g = layer.geometry;
        let want = layout.layers()[0];
        for (a, b) in [
            (g.x, want.x),
            (g.y, want.y),
            (g.width, want.width),
            (g.height, want.height),
        ] {
            assert!((a - b).abs() < 1.0);
        }
        let program = Plan::prepare(
            &s,
            &layout,
            0,
            &[(layout.layer_info(0).id, "1".into())],
            Target::Program,
            sizing,
        )
        .unwrap();
        assert_eq!(program.bank, "B");
        assert!(
            Plan::prepare(
                &s,
                &layout,
                0,
                &[(1, "1".into()), (2, "1".into())],
                Target::Preview,
                sizing
            )
            .is_err()
        );
        assert!(
            Plan::prepare(&s, &layout, 0, &[(1, "8".into())], Target::Preview, sizing).is_err()
        );
    }
}
#[test]
fn masks_fit_modes_and_shared_input_corrections_are_explicit() {
    for (path, value) in [
        (
            "/device/screenList/items/S1/presetList/items/A/layerList/items/1/cropping/mask/pp/left",
            json!(100),
        ),
        (
            "/device/screenList/items/S1/presetList/items/A/layerList/items/1/cropping/classic/pp/aspectOverride",
            json!("CROPPED"),
        ),
        (
            "/device/inputList/items/IN_1/plugList/items/1/settings/aspect/pp/transformTo",
            json!("CUSTOM"),
        ),
    ] {
        let mut raw = routed();
        *raw.pointer_mut(path).unwrap() = value;
        let s = Snapshot::parse(&identity(), &raw).unwrap();
        assert!(s.screens[0].layers[0].unsupported.is_some());
        assert!(s.screens[0].import(&[0]).is_err());
    }
}
#[test]
fn family_links_save_migrate_recover_and_validate() {
    let old = include_str!("../../../docs/interfaces/canvas-pilot-v12.example.json");
    let mut old: Value = serde_json::from_str(old).unwrap();
    old["layers"][0]["midra_link"] = json!({"screen":"001","layer":"02"});
    let p = Project::from_json(&old.to_string()).unwrap();
    assert_eq!(p.version, 15);
    assert_eq!(
        p.layers[0].device_link.as_ref().unwrap().family,
        Family::Midra4k
    );
    assert_eq!(p.layers[0].device_link.as_ref().unwrap().screen, "1");
    let mut layout = p.layout().unwrap();
    let link = DeviceLink {
        family: Family::Aquilon,
        screen: "A1".into(),
        layer: "17".into(),
    };
    layout.set_device_link(0, Some(link.clone())).unwrap();
    let p = Project::from_layout(&layout);
    let text = p.to_json().unwrap();
    assert!(text.contains("device_link"));
    assert!(!text.contains("midra_link"));
    assert_eq!(
        Project::from_json(&text).unwrap().layers[0].device_link,
        Some(link)
    );
    layout.undo();
    assert_eq!(
        layout.layer_info(0).device_link.as_ref().unwrap().family,
        Family::Midra4k
    );
    for (family, screen) in [
        (Family::Midra4k, "S1"),
        (Family::Aquilon, "1"),
        (Family::Aquilon, "S01"),
        (Family::Aquilon, "S1/../A1"),
    ] {
        assert!(
            DeviceLink {
                family,
                screen: screen.into(),
                layer: "1".into()
            }
            .validate()
            .is_err()
        );
    }
    old["layers"][0]["midra_link"]["family"] = json!("aquilon");
    assert!(Project::from_json(&old.to_string()).is_err());
}
#[test]
fn cancellation_does_not_contact_device() {
    assert!(
        device::fetch_cancellable("127.0.0.1:1", &std::sync::atomic::AtomicBool::new(true))
            .unwrap_err()
            .contains("cancelled")
    );
}

#[test]
fn documented_v13_example_contains_native_aquilon_link() {
    let p = Project::from_json(include_str!(
        "../../../docs/interfaces/canvas-pilot-v13.example.json"
    ))
    .unwrap();
    assert_eq!(p.version, 15);
    let link = p.layers[0].device_link.as_ref().unwrap();
    assert_eq!(link.family, Family::Aquilon);
    assert_eq!(link.screen, "S1");
    assert_eq!(Project::from_json(&p.to_json().unwrap()).unwrap(), p);
}

#[test]
fn explicit_midra_fit_is_not_silently_sent_to_aquilon() {
    use canvas_pilot::formats::{Crop, LayerFit};
    let s = Snapshot::parse(&identity(), &routed()).unwrap();
    for mode in LayerFit::ALL {
        let mut layout = Layout::default();
        layout
            .set_layer_fit(0, Some(mode), Crop::default())
            .unwrap();
        let err = Plan::prepare(
            &s,
            &layout,
            0,
            &[(layout.layer_info(0).id, "1".into())],
            Target::Preview,
            Sizing::OutputBox,
        )
        .unwrap_err();
        assert!(err.contains("Midra"), "{err}");
    }
}

#[test]
fn masked_editor_layers_are_rejected_before_aquilon_send() {
    let snapshot = Snapshot::parse(&identity(), &routed()).unwrap();
    let mut layout = Layout::default();
    layout
        .set_mask(
            0,
            canvas_pilot::formats::Mask {
                left: 0.1,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        Plan::prepare(
            &snapshot,
            &layout,
            0,
            &[(layout.layer_info(0).id, "1".into())],
            Target::Preview,
            Sizing::OutputBox
        )
        .unwrap_err()
        .contains("masking")
    );
}
