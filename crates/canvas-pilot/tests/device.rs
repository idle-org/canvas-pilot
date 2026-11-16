use canvas_pilot::device::Snapshot;
use serde_json::{Value, json};
fn fixture() -> (Value, Value) {
    let layer = json!({"position":{"pp":{"posH":1436,"posV":540}},"size":{"pp":{"sizeH":967,"sizeV":544}},"source":{"pp":{"input":"INPUT_4"}},"crop":{"pp":{"left":0,"right":0,"top":0,"bottom":0}},"mask":{"pp":{"left":0,"right":0,"top":0,"bottom":0}}});
    let screen = json!({
        "canvas": {"status": {"size": {"pp": {"sizeH": 1920, "sizeV": 1080}}}},
        "presetList": {"items": {"UP": {"liveLayerList": {"items": {"1": layer}}}}}
    });
    let raw = json!({"device": {
        "preconfig":{"status":{"stateList":{"items":{"CURRENT":{"screenList":{"items":{"2":{"pp":{"layerCount":2}}}}}}}}},
        "transition": {"screenList": {"items": {"2": {"status": {"pp": {"transition": "AT_DOWN"}}}}}},
        "screenList": {"items": {"2": screen}}
    }});
    (
        json!({"type":"Eikos 4K","version":{"major":3,"minor":3,"patch":10}}),
        raw,
    )
}
#[test]
fn preview_read_retains_half_pixels_and_maps_source_label() {
    let (identity, raw) = fixture();
    let snapshot = Snapshot::parse(&identity, &raw).unwrap();
    assert_eq!(snapshot.identity, "Eikos 4K · firmware 3.3.10");
    let screen = &snapshot.screens[0];
    assert_eq!(screen.preview_bank, "UP");
    let doc = screen.import(&[0]).unwrap();
    assert_eq!(doc.shapes[0].geometry.x, 952.5);
    assert_eq!(doc.shapes[0].geometry.y, 268.0);
    assert!(doc.shapes[0].id.contains("INPUT_4"));
    assert!(screen.import(&[]).is_err());
    assert!(screen.import(&[0, 0]).is_err());
    assert!(screen.import(&[9]).is_err());
}
#[test]
fn reject_transition_and_missing_preview_without_falling_back_to_program() {
    let (identity, mut raw) = fixture();
    let state = "/device/transition/screenList/items/2/status/pp/transition";
    *raw.pointer_mut(state).unwrap() = json!("EFFECT_FROM_DOWN");
    assert!(
        Snapshot::parse(&identity, &raw)
            .unwrap_err()
            .contains("transitioning")
    );
    *raw.pointer_mut(state).unwrap() = json!("AT_UP");
    assert!(Snapshot::parse(&identity, &raw).is_err());
    let up = raw
        .pointer("/device/screenList/items/2/presetList/items/UP")
        .unwrap()
        .clone();
    raw["device"]["screenList"]["items"]["2"]["presetList"]["items"]["DOWN"] = up;
    assert_eq!(
        Snapshot::parse(&identity, &raw).unwrap().screens[0].preview_bank,
        "DOWN"
    );
}
#[test]
fn unsupported_framing_is_visible_but_not_importable() {
    let (identity, mut raw) = fixture();
    raw["device"]["screenList"]["items"]["2"]["presetList"]["items"]["UP"]["liveLayerList"]["items"]
        ["1"]["crop"]["pp"]["left"] = json!(18350);
    let s = Snapshot::parse(&identity, &raw).unwrap();
    assert_eq!(s.screens[0].layers.len(), 1);
    assert!(s.screens[0].import(&[0]).unwrap_err().contains("crop"));
}
#[test]
fn malformed_device_state_fails_before_import() {
    let (mut identity, mut raw) = fixture();
    identity["type"] = json!("different device");
    assert!(Snapshot::parse(&identity, &raw).is_err());
    identity["type"] = json!("Eikos 4K");
    raw["device"]["screenList"]["items"]["2"]["canvas"]["status"]["size"]["pp"]["sizeH"] =
        json!(1920.5);
    assert!(Snapshot::parse(&identity, &raw).is_err());
    assert!(canvas_pilot::device::fetch("http://localhost/").is_err());
}
#[test]
fn disabled_screen_slots_are_ignored() {
    let (identity, mut raw) = fixture();
    let mut disabled = raw["device"]["screenList"]["items"]["2"].clone();
    disabled["canvas"]["status"]["size"]["pp"] = json!({"sizeH":0,"sizeV":0});
    raw["device"]["screenList"]["items"]["3"] = disabled;
    raw["device"]["transition"]["screenList"]["items"]["3"] =
        json!({"status":{"pp":{"transition":"AT_DOWN"}}});
    assert_eq!(Snapshot::parse(&identity, &raw).unwrap().screens.len(), 1);
}

#[test]
fn mapped_import_ignores_selection_and_undoes_new_layers_together() {
    let mut layout = canvas_pilot::Layout::default();
    layout.select(None);
    let before = layout.layers().clone();
    let id = layout.layer_info(0).id;
    let geometry = canvas_pilot::Geometry {
        x: 12.5,
        y: 27.0,
        width: 945.0,
        height: 531.58,
    };
    layout
        .import_mapped_layers(&[(Some(id), geometry), (None, geometry), (None, geometry)])
        .unwrap();
    assert_eq!(layout.layers().len(), 4);
    assert_eq!(layout.layers()[0], geometry);
    let new_id = layout.layer_info(3).id;
    layout.undo();
    assert_eq!(*layout.layers(), before);
    layout.redo();
    assert_eq!(layout.layers().len(), 4);
    assert_eq!(layout.layer_info(3).id, new_id);
    let added = layout.add_layer().unwrap();
    assert!(layout.layer_info(added).id > new_id);
}

#[test]
fn invalid_mapping_never_partially_imports() {
    let mut layout = canvas_pilot::Layout::default();
    let before = layout.layers().clone();
    let geometry = before[1];
    let id = layout.layer_info(0).id;
    for values in [
        vec![],
        vec![(None, geometry), (Some(999), geometry)],
        vec![(Some(id), geometry), (Some(id), geometry)],
    ] {
        assert!(layout.import_mapped_layers(&values).is_err());
        assert_eq!(*layout.layers(), before);
    }
    layout.set_locked(0, true);
    assert!(
        layout
            .import_mapped_layers(&[(None, geometry), (Some(id), geometry)])
            .is_err()
    );
    assert_eq!(*layout.layers(), before);
    layout.set_locked(0, false);
    layout.set_visible(0, false);
    assert!(
        layout
            .import_mapped_layers(&[(Some(id), geometry)])
            .is_err()
    );
}

#[test]
fn existing_layers_remain_available_without_source_assignment() {
    let (identity, mut raw) = fixture();
    let entries = raw
        .pointer_mut("/device/screenList/items/2/presetList/items/UP/liveLayerList/items")
        .unwrap();
    entries["1"]["status"] = json!({"pp": {"state": "OFF"}});
    entries["2"] = entries["1"].clone();
    entries["2"]["crop"]["pp"]["aspectOverride"] = json!("CENTERED");
    entries["2"]["status"]["pp"]["contentWidth"] = json!(0);
    entries["2"]["status"]["pp"]["contentHeight"] = json!(0);
    for input in [json!("NONE"), json!(""), Value::Null] {
        raw["device"]["screenList"]["items"]["2"]["presetList"]["items"]["UP"]["liveLayerList"]["items"]
            ["2"]["source"]["pp"]["input"] = input;
        let snapshot = Snapshot::parse(&identity, &raw).unwrap();
        let screen = &snapshot.screens[0];
        assert_eq!(screen.layers.len(), 2);
        assert!(screen.layers.iter().all(|l| l.unsupported.is_none()));
        assert_eq!(screen.import(&[0, 1]).unwrap().shapes.len(), 2);
    }
}

#[test]
fn allocation_filters_firmware_slots_independently_of_input_routing() {
    let (identity, mut raw) = fixture();
    let entries = raw
        .pointer_mut("/device/screenList/items/2/presetList/items/UP/liveLayerList/items")
        .unwrap();
    for id in 2..=8 {
        entries[id.to_string()] = entries["1"].clone();
    }
    // Ignore staged NEW configuration and use only the active CURRENT allocation.
    raw["device"]["preconfig"]["status"]["stateList"]["items"]["NEW"] =
        json!({"screenList":{"items":{"2":{"pp":{"layerCount":8}}}}});
    for count in [0, 1, 2] {
        raw["device"]["preconfig"]["status"]["stateList"]["items"]["CURRENT"]["screenList"]["items"]
            ["2"]["pp"]["layerCount"] = json!(count);
        let snapshot = Snapshot::parse(&identity, &raw).unwrap();
        assert_eq!(snapshot.screens[0].layers.len(), count);
        assert!(
            snapshot.screens[0]
                .layers
                .iter()
                .all(|l| l.id.parse::<usize>().unwrap() <= count)
        );
    }
    for invalid in [Value::Null, json!(-1), json!(2.5), json!(9)] {
        raw["device"]["preconfig"]["status"]["stateList"]["items"]["CURRENT"]["screenList"]["items"]
            ["2"]["pp"]["layerCount"] = invalid;
        assert!(Snapshot::parse(&identity, &raw).is_err());
    }
}

#[test]
fn all_midra_4k_models_use_reported_layer_allocations() {
    let (mut identity, raw) = fixture();
    let expected = Snapshot::parse(&identity, &raw).unwrap();
    for model in ["Eikos 4K", "Pulse 4K", "QuickVu 4K", "QuickMatrix 4K"] {
        identity["type"] = json!(model);
        let parsed = Snapshot::parse(&identity, &raw).unwrap();
        assert!(parsed.identity.starts_with(model));
        assert_eq!(parsed.screens.len(), expected.screens.len());
        for (actual, expected) in parsed.screens.iter().zip(&expected.screens) {
            assert_eq!(actual.layers.len(), expected.layers.len());
            assert_eq!(actual.resolution, expected.resolution);
        }
    }
}

#[test]
fn device_addresses_support_ports_and_ipv6() {
    use canvas_pilot::device::address;
    for (input, expected) in [
        ("192.168.10.146", "192.168.10.146:80"),
        (" 192.168.10.146:3000 ", "192.168.10.146:3000"),
        ("::1", "[::1]:80"),
        ("[::1]:3000", "[::1]:3000"),
    ] {
        assert_eq!(address(input).unwrap().to_string(), expected);
    }
    for input in [
        "",
        "localhost",
        "http://127.0.0.1:3000",
        "127.0.0.1:0",
        "127.0.0.1:65536",
        "127.0.0.1:abc",
        "127.0.0.1:3000/path",
    ] {
        assert!(address(input).is_err(), "{input}");
    }
}

#[test]
fn fetch_uses_custom_port_for_identity_and_state() {
    use std::io::{BufRead, BufReader, Write};
    let (identity, raw) = fixture();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let host = listener.local_addr().unwrap().to_string();
    let server = std::thread::spawn(move || {
        for (path, payload) in [
            ("/api/tpp/v1/system", identity),
            ("/api/stores/device", raw),
        ] {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert_eq!(line, format!("GET {path} HTTP/1.1\r\n"));
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
            }
            let body = payload.to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    let result = canvas_pilot::device::fetch(&host);
    server.join().unwrap();
    assert!(result.unwrap().identity.starts_with("Eikos 4K"));
}

#[test]
fn fractional_device_content_rounds_with_a_nonblocking_warning() {
    let (identity, mut raw) = fixture();
    let path = "/device/screenList/items/2/presetList/items/UP/liveLayerList/items/1";
    let layer = raw.pointer_mut(path).unwrap();
    layer["crop"]["pp"]["aspectOverride"] = json!("CENTERED");
    layer["crop"]["pp"]["left"] = json!(500);
    layer["status"] = json!({"pp":{"contentWidth":1919.7,"contentHeight":1080.2}});
    let state = Snapshot::parse(&identity, &raw).unwrap();
    let layer = &state.screens[0].layers[0];
    assert!(layer.unsupported.is_none());
    assert_eq!(layer.framing.unwrap().0.native, [1920, 1080]);
    assert!(
        layer
            .warning
            .as_ref()
            .unwrap()
            .contains("1919.7 × 1080.2 to 1920 × 1080")
    );
    assert!(state.screens[0].import(&[0]).is_ok());
    assert_eq!(state.raw, raw, "Keep the original device snapshot intact");
    for width in [0.5, 16384.1, 20000.0] {
        raw.pointer_mut(path).unwrap()["status"]["pp"]["contentWidth"] = json!(width);
        let invalid = Snapshot::parse(&identity, &raw).unwrap();
        assert!(invalid.screens[0].layers[0].unsupported.is_some());
        assert!(invalid.screens[0].import(&[0]).is_err());
    }
    raw.pointer_mut(path).unwrap()["status"]["pp"] =
        json!({"contentWidth":1920,"contentHeight":1080});
    assert!(
        Snapshot::parse(&identity, &raw).unwrap().screens[0].layers[0]
            .warning
            .is_none()
    );
}

#[test]
fn unsupported_aquilon_profile_is_rejected_before_requesting_large_state() {
    use std::io::{BufRead, BufReader, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let host = listener.local_addr().unwrap().to_string();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line, "GET /api/tpp/v1/system HTTP/1.1\r\n");
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
        }
        let body =
            r#"{"type":"AQL Cmax","isLinked":false,"version":{"major":5,"minor":2,"patch":79}}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        listener
    });
    let error = canvas_pilot::device::fetch(&host).unwrap_err();
    let listener = server.join().unwrap();
    assert!(error.contains("Unsupported Aquilon profile"), "{error}");
    assert!(error.contains("6.2.x"), "{error}");
    listener.set_nonblocking(true).unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(
        Snapshot::parse(&json!({"type":"AQL Cmax"}), &json!({}))
            .unwrap_err()
            .contains("Missing device field")
    );
}
