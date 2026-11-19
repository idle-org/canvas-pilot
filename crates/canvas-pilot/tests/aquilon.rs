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
    id["version"]["major"] = json!("7");
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
fn invalid_masks_fit_modes_and_shared_input_corrections_are_explicit() {
    for (path, value) in [
        (
            "/device/screenList/items/S1/presetList/items/A/layerList/items/1/cropping/mask/pp/left",
            json!(-1),
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
    assert_eq!(p.version, 21);
    assert_eq!(
        p.layers[0].load_mapping.as_ref().unwrap().family,
        Family::Midra4k
    );
    assert_eq!(p.layers[0].load_mapping.as_ref().unwrap().screen, "1");
    let mut layout = p.layout().unwrap();
    let link = DeviceLink {
        family: Family::Aquilon,
        screen: "A1".into(),
        layer: "17".into(),
    };
    layout.set_device_link(0, Some(link.clone())).unwrap();
    let p = Project::from_layout(&layout);
    let text = p.to_json().unwrap();
    assert!(text.contains("load_mapping"));
    assert!(!text.contains("midra_link"));
    assert_eq!(
        Project::from_json(&text).unwrap().layers[0].load_mapping,
        Some(link)
    );
    layout.undo();
    assert_eq!(
        layout.layer_info(0).load_mapping.as_ref().unwrap().family,
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
    assert_eq!(p.version, 21);
    let link = p.layers[0].load_mapping.as_ref().unwrap();
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

fn clipped(reveal: Geometry, g: Geometry) -> Geometry {
    let x = reveal.x.clamp(g.x, g.x + g.width);
    let y = reveal.y.clamp(g.y, g.y + g.height);
    Geometry {
        x,
        y,
        width: (reveal.x + reveal.width).clamp(g.x, g.x + g.width) - x,
        height: (reveal.y + reveal.height).clamp(g.y, g.y + g.height) - y,
    }
}

#[test]
fn masks_roundtrip_both_origins_sizing_modes_and_banks_without_changing_crop() {
    use canvas_pilot::formats::{Mask, MaskOrigin};
    for origin in [MaskOrigin::Layer, MaskOrigin::Source] {
        for sizing in [Sizing::OutputBox, Sizing::SourceAspect] {
            for target in [Target::Preview, Target::Program] {
                for (width, height, zoom, offset) in [
                    (640.5, 360.25, 1.0, [0.0, 0.0]),
                    (301.25, 251.5, 2.0, [30.0, -10.0]),
                    (100.0, 200.0, 4.0, [-15.0, 20.0]),
                ] {
                    let mut layout = Layout::default();
                    layout
                        .apply(
                            0,
                            Geometry {
                                x: 12.125,
                                y: 34.375,
                                width,
                                height,
                            },
                        )
                        .unwrap();
                    layout.set_framing(0, Framing { zoom, offset }).unwrap();
                    let mut raw = routed();
                    let mut unmasked_fields = None;
                    for mask in [
                        Mask {
                            origin,
                            left: 0.2,
                            right: 0.1,
                            top: 0.3,
                            bottom: 0.2,
                        },
                        Mask {
                            origin,
                            left: 0.8,
                            right: 0.2,
                            top: 0.0,
                            bottom: 0.0,
                        },
                        Mask {
                            origin,
                            left: 0.0,
                            right: 0.0,
                            top: 0.4,
                            bottom: 0.6,
                        },
                        Mask::default(),
                    ] {
                        layout.set_mask(0, mask).unwrap();
                        let snapshot = Snapshot::parse(&identity(), &raw).unwrap();
                        let plan = Plan::prepare(
                            &snapshot,
                            &layout,
                            0,
                            &[(layout.layer_info(0).id, "1".into())],
                            target,
                            sizing,
                        )
                        .unwrap();
                        let mut increasing_mask_seen = false;
                        for change in &plan.changes {
                            let path = format!("/{}", change.path.join("/"));
                            assert!(
                                !path.contains("/source/")
                                    && !path.contains("/inputList/")
                                    && !path.contains("xTake")
                            );
                            if path.contains("/mask/") {
                                let decrease = change.after.as_u64().unwrap()
                                    < change.before.as_u64().unwrap();
                                if decrease {
                                    assert!(!increasing_mask_seen);
                                } else {
                                    increasing_mask_seen = true;
                                }
                            }
                            *raw.pointer_mut(&path).unwrap() = change.after.clone();
                        }
                        let base = Profile::Aquilon6.layer_path("S1", &plan.bank, "1");
                        let unchanged = (
                            raw.pointer(&format!("{base}/position")).unwrap().clone(),
                            raw.pointer(&format!("{base}/cropping/classic"))
                                .unwrap()
                                .clone(),
                        );
                        if let Some(previous) = &unmasked_fields {
                            assert_eq!(
                                previous, &unchanged,
                                "Masking must not resize or recrop the image"
                            );
                        }
                        unmasked_fields = Some(unchanged);
                        let mut decoded_raw = raw.clone();
                        let group = &mut decoded_raw["device"]["screenAuxGroupList"]["items"]["S1"];
                        group["status"]["pp"]["transition"] =
                            json!(if group["control"]["pp"]["presetDown"] == plan.bank {
                                "AT_UP"
                            } else {
                                "AT_DOWN"
                            });
                        let decoded = Snapshot::parse(&identity(), &decoded_raw).unwrap();
                        let layer = &decoded.screens[0].layers[0];
                        assert!(layer.unsupported.is_none(), "{:?}", layer.unsupported);
                        decoded.screens[0].import(&[0]).unwrap();
                        let expected = clipped(
                            layout.formats().mask_bounds(0, layout.layers()[0]).unwrap(),
                            layer.geometry,
                        );
                        let actual = layer.mask.bounds(layer.geometry);
                        for (a, b) in [
                            (expected.x, actual.x),
                            (expected.y, actual.y),
                            (expected.width, actual.width),
                            (expected.height, actual.height),
                        ] {
                            assert!(
                                (a - b).abs() < 1.0,
                                "{origin:?} {sizing:?} {target:?}: {expected:?} vs {actual:?}"
                            );
                        }
                        if mask.is_empty() {
                            let base = Profile::Aquilon6.layer_path("S1", &plan.bank, "1");
                            for edge in ["left", "right", "top", "bottom"] {
                                assert_eq!(
                                    raw.pointer(&format!("{base}/cropping/mask/pp/{edge}")),
                                    Some(&json!(0))
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn malformed_masks_block_import_and_send_but_valid_unrouted_masks_import() {
    let base = Profile::Aquilon6.layer_path("S1", "A", "1");
    for (left, right) in [
        (json!(-1), json!(0)),
        (json!(65536), json!(0)),
        (json!(0.5), json!(0)),
        (json!(40000), json!(30000)),
        (json!(null), json!(0)),
    ] {
        let mut r = raw();
        *r.pointer_mut(&format!("{base}/cropping/mask/pp/left"))
            .unwrap() = left;
        *r.pointer_mut(&format!("{base}/cropping/mask/pp/right"))
            .unwrap() = right;
        let s = Snapshot::parse(&identity(), &r).unwrap();
        assert!(s.screens[0].import(&[0]).is_err());
        let layout = Layout::default();
        assert!(
            Plan::prepare(
                &s,
                &layout,
                0,
                &[(layout.layer_info(0).id, "1".into())],
                Target::Preview,
                Sizing::OutputBox
            )
            .is_err()
        );
    }
    let mut r = raw();
    *r.pointer_mut(&format!("{base}/cropping/mask/pp/left"))
        .unwrap() = json!(16384);
    let s = Snapshot::parse(&identity(), &r).unwrap();
    s.screens[0].import(&[0]).unwrap();
    assert!((s.screens[0].layers[0].mask.left - 16384.0 / 65535.0).abs() < 1e-9);
}

#[test]
fn all_models_and_firmware_versions_use_checked_geometry_with_compatibility_warning() {
    for model in [
        "RS alpha", "RS Alpha", "RS1", "RS2", "RS3", "RS4", "RS5", "RS6", "Cmini", "C", "C+",
        "Cmax",
    ] {
        for (major, minor) in [
            (1, 0),
            (2, 2),
            (3, 0),
            (4, 0),
            (5, 0),
            (6, 0),
            (6, 2),
            (7, 0),
        ] {
            let mut id = identity();
            id["type"] = json!(format!("AQL {model}"));
            id["version"]["major"] = json!(major);
            id["version"]["minor"] = json!(minor);
            let snapshot = Snapshot::parse(&id, &routed()).unwrap();
            assert_eq!(
                snapshot
                    .identity
                    .contains("backward compatibility is not guaranteed"),
                (major, minor) != (6, 2)
            );
            let layout = Layout::default();
            for target in [Target::Preview, Target::Program] {
                let plan = Plan::prepare(
                    &snapshot,
                    &layout,
                    0,
                    &[(layout.layer_info(0).id, "1".into())],
                    target,
                    Sizing::OutputBox,
                )
                .unwrap();
                assert_eq!(plan.profile, Profile::Aquilon6);
                assert!(plan.changes.iter().all(|change| {
                    !change
                        .path
                        .iter()
                        .any(|part| part == "source" || part == "inputList")
                }));
            }
        }
    }
    for model in ["AQL Unknown", "AQL Cmax extra", "AQL"] {
        let mut id = identity();
        id["type"] = json!(model);
        assert!(Snapshot::parse(&id, &raw()).is_err());
    }
}

#[test]
fn excessive_mask_quantization_is_recorded_without_blocking_send() {
    let snapshot = Snapshot::parse(&identity(), &routed()).unwrap();
    let mut layout = Layout::default();
    layout
        .apply(
            0,
            Geometry {
                x: 0.0,
                y: 0.0,
                width: 900000.0,
                height: 506250.0,
            },
        )
        .unwrap();
    layout
        .set_mask(
            0,
            canvas_pilot::formats::Mask {
                left: 0.123456,
                ..Default::default()
            },
        )
        .unwrap();
    let plan = Plan::prepare(
        &snapshot,
        &layout,
        0,
        &[(layout.layer_info(0).id, "1".into())],
        Target::Preview,
        Sizing::OutputBox,
    )
    .unwrap();
    assert!(
        plan.discrepancies
            .iter()
            .any(|entry| entry["field"] == "mask_bounds")
    );
}

#[test]
fn captured_rs1_mask_import_preserves_visible_geometry_and_reveal_bounds() {
    let capture: Value =
        serde_json::from_str(include_str!("fixtures/aquilon-rs1-mask-6.2.json")).unwrap();
    let mut state = raw();
    let path = Profile::Aquilon6.layer_path("S1", "A", "1");
    *state.pointer_mut(&path).unwrap() = capture["layer"].clone();
    let snapshot = Snapshot::parse(&capture["identity"], &state).unwrap();
    let layer = &snapshot.screens[0].layers[0];
    snapshot.screens[0].import(&[0]).unwrap();
    let expected: Geometry = serde_json::from_value(capture["editor_geometry"].clone()).unwrap();
    let mask: canvas_pilot::formats::Mask =
        serde_json::from_value(capture["editor_mask"].clone()).unwrap();
    let expected_reveal = clipped(mask.bounds(expected), layer.geometry);
    let actual_reveal = layer.mask.bounds(layer.geometry);
    for (want, got) in [(expected, layer.geometry), (expected_reveal, actual_reveal)] {
        for (a, b) in [
            (want.x, got.x),
            (want.y, got.y),
            (want.width, got.width),
            (want.height, got.height),
        ] {
            assert!((a - b).abs() < 1.0, "{want:?} vs {got:?}");
        }
    }
    assert_eq!(layer.source, "LIVE_1");
    assert_eq!(layer.mask.origin, canvas_pilot::formats::MaskOrigin::Layer);
}

#[test]
fn one_editor_layer_can_prepare_multiple_distinct_aquilon_destinations() {
    let snapshot = Snapshot::parse(&identity(), &routed()).unwrap();
    let layout = Layout::default();
    let mappings = vec![(1, "1".into()), (1, "2".into())];
    let plan = Plan::prepare(
        &snapshot,
        &layout,
        0,
        &mappings,
        Target::Preview,
        Sizing::OutputBox,
    )
    .unwrap();
    for layer in ["1", "2"] {
        assert!(
            plan.observed
                .iter()
                .any(|(path, _)| path.ends_with(&format!("/layerList/items/{layer}")))
        );
    }
    assert!(
        Plan::prepare(
            &snapshot,
            &layout,
            0,
            &[(1, "1".into()), (2, "1".into())],
            Target::Preview,
            Sizing::OutputBox
        )
        .is_err()
    );
}

#[test]
fn explicit_source_selection_is_guarded_and_verified_in_the_plan() {
    let snapshot = Snapshot::parse(&identity(), &raw()).unwrap();
    let layout = Layout::default();
    let id = layout.layer_info(0).id;
    let plan = Plan::prepare_with_inputs(
        &snapshot,
        &layout,
        0,
        &[(id, "1".into())],
        Target::Preview,
        Sizing::OutputBox,
        &[(id, 1)].into(),
    )
    .unwrap();
    assert_eq!(plan.changes[0].after, json!("LIVE_1"));
    assert!(
        plan.changes[0]
            .path
            .join("/")
            .ends_with("source/pp/inputNum")
    );
    canvas_pilot::device_send::validate_fresh(&plan, &snapshot).unwrap();
    let mut changed = snapshot.clone();
    changed.raw["device"]["inputList"]["items"]["IN_1"]["status"]["pp"]["plug"] = json!("OTHER");
    assert!(canvas_pilot::device_send::validate_fresh(&plan, &changed).is_err());
}

#[test]
fn reviewed_aquilon_source_adaptation_is_explicit_and_preserves_input_processing_guards() {
    use canvas_pilot::device_send::{self, Conversion};
    let mut state = routed();
    state["device"]["inputList"]["items"]["IN_1"]["plugList"]["items"]["1"]["status"]["signal"]["pp"]
        ["imageWidth"] = json!(3840);
    state["device"]["inputList"]["items"]["IN_1"]["plugList"]["items"]["1"]["status"]["signal"]["pp"]
        ["imageHeight"] = json!(2160);
    let snapshot = Snapshot::parse(&identity(), &state).unwrap();
    let editor = Layout::default();
    let before = Project::from_layout(&editor);
    let mappings = [(editor.layer_info(0).id, "1".into())];
    assert!(
        Plan::prepare(
            &snapshot,
            &editor,
            0,
            &mappings,
            Target::Preview,
            Sizing::OutputBox
        )
        .is_err()
    );
    let options = Conversion {
        source: true,
        ..Default::default()
    };
    let plan = device_send::prepare_converted(
        &snapshot,
        &editor,
        0,
        &mappings,
        Target::Preview,
        Sizing::OutputBox,
        &Default::default(),
        options,
    )
    .unwrap();
    assert_eq!(plan.conversions.len(), 1);
    assert!(
        !plan
            .changes
            .iter()
            .any(|c| c.path.join("/").contains("source/"))
    );
    device_send::validate_fresh(&plan, &snapshot).unwrap();
    assert_eq!(Project::from_layout(&editor), before);
    state["device"]["inputList"]["items"]["IN_1"]["plugList"]["items"]["1"]["settings"]["aspect"]
        ["pp"]["transformTo"] = json!("CUSTOM");
    let corrected = Snapshot::parse(&identity(), &state).unwrap();
    assert!(
        device_send::prepare_converted(
            &corrected,
            &editor,
            0,
            &mappings,
            Target::Preview,
            Sizing::OutputBox,
            &Default::default(),
            options
        )
        .unwrap_err()
        .contains("aspect correction")
    );
}
