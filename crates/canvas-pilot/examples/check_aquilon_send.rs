//! Explicit emulator acceptance with restoration. Never run by CI.
use canvas_pilot::{
    Geometry, Layout,
    device::{self, Profile},
    device_send::{self, Plan, Sizing, Target},
    formats::{Framing, Mask, MaskOrigin},
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};
fn write(host: &str, changes: &[(String, Value)]) -> Result<(), String> {
    let address = device::address(host)?;
    let stream =
        TcpStream::connect_timeout(&address, Duration::from_secs(5)).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    let (mut socket, _) =
        tungstenite::client(format!("ws://{address}/"), stream).map_err(|e| e.to_string())?;
    for (path, value) in changes {
        socket.send(tungstenite::Message::Text(json!({"channel":"DEVICE","data":{"path":path.trim_start_matches('/').split('/').collect::<Vec<_>>(),"value":value}}).to_string().into())).map_err(|e|e.to_string())?;
    }
    for value in [false, true] {
        socket.send(tungstenite::Message::Text(json!({"channel":"DEVICE","data":{"path":Profile::Aquilon6.apply_path(),"value":value}}).to_string().into())).map_err(|e|e.to_string())?;
    }
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if !(3..=4).contains(&args.len())
        || args
            .get(3)
            .is_some_and(|scope| !matches!(scope.as_str(), "preview" | "program"))
        || !matches!(
            args[1].as_str(),
            "--apply-and-restore" | "--visual-check-and-restore"
        )
    {
        return Err("Usage: check_aquilon_send --apply-and-restore|--visual-check-and-restore IP:PORT [preview|program] (temporarily routes S1 layer 1 to LIVE_1 in selected banks; default both)".into());
    }
    let visual = args[1] == "--visual-check-and-restore";
    let host = &args[2];
    let targets: &[Target] = match args.get(3).map(String::as_str) {
        Some("program") if visual => return Err("Visual check uses Preview only".into()),
        Some("program") => &[Target::Program],
        Some("preview") => &[Target::Preview],
        _ if visual => &[Target::Preview],
        _ => &[Target::Preview, Target::Program],
    };
    let baseline = device::fetch(host)?;
    println!("Connected: {}", baseline.identity);
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    client
        .get(format!(
            "http://{}/api/tpp/v1/system",
            device::address(host)?
        ))
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4096 {
        return Err("Oversized identity".into());
    }
    let identity: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if device::Snapshot::parse(&identity, &baseline.raw)?.identity != baseline.identity {
        return Err("Identity changed before diagnostic".into());
    }
    if baseline.profile != Profile::Aquilon6 {
        return Err("Expected Aquilon 6.2 emulator".into());
    }
    let screen = baseline
        .screens
        .iter()
        .position(|s| s.id == "S1")
        .ok_or("S1 unavailable")?;
    let s = &baseline.screens[screen];
    let mut restore = vec![];
    let mut setup = vec![];
    for target in targets {
        let bank = match target {
            Target::Preview => &s.preview_bank,
            Target::Program => &s.program_bank,
        };
        let base = baseline.profile.layer_path("S1", bank, "1");
        for suffix in [
            "source/pp/inputNum",
            "position/pp/sizeH",
            "position/pp/sizeV",
            "position/pp/posH",
            "position/pp/posV",
            "cropping/classic/pp/aspectOverride",
            "cropping/classic/pp/left",
            "cropping/classic/pp/right",
            "cropping/classic/pp/top",
            "cropping/classic/pp/bottom",
            "cropping/mask/pp/left",
            "cropping/mask/pp/right",
            "cropping/mask/pp/top",
            "cropping/mask/pp/bottom",
        ] {
            let path = format!("{base}/{suffix}");
            restore.push((
                path.clone(),
                baseline
                    .raw
                    .pointer(&path)
                    .ok_or("Missing original field")?
                    .clone(),
            ));
        }
        setup.push((format!("{base}/source/pp/inputNum"), json!("LIVE_1")));
    }
    // Keep a recovery record before the first write, including on interruption.
    let backup_path = std::env::temp_dir().join(format!(
        "canvas-pilot-aquilon-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis()
    ));
    let mut backup = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup_path)
        .map_err(|e| e.to_string())?;
    backup
        .write_all(
            serde_json::to_string_pretty(
                &json!({"identity":identity,"host":host,"fields":restore}),
            )
            .map_err(|e| e.to_string())?
            .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
    backup.sync_all().map_err(|e| e.to_string())?;
    println!("Recovery baseline: {}", backup_path.display());
    let result = (|| {
        write(host, &setup)?;
        let mut layout = Layout::default();
        layout.set_output(s.resolution)?;
        for &target in targets {
            let strategies: &[Sizing] = if visual {
                &[Sizing::OutputBox]
            } else {
                &[Sizing::OutputBox, Sizing::SourceAspect]
            };
            for &sizing in strategies {
                for (width, height, zoom, offset) in [
                    (640.5, 360.25, 1.0, [0.0, 0.0]),
                    (301.25, 251.5, 2.0, [30.0, -10.0]),
                    (100.0, 200.0, 4.0, [-15.0, 20.0]),
                ] {
                    layout.apply(
                        0,
                        Geometry {
                            x: 12.125,
                            y: 34.375,
                            width,
                            height,
                        },
                    )?;
                    layout.set_framing(0, Framing { zoom, offset })?;
                    let masks: Vec<_> = if visual {
                        vec![Mask {
                            left: 0.2,
                            right: 0.1,
                            top: 0.3,
                            bottom: 0.2,
                            ..Default::default()
                        }]
                    } else {
                        [MaskOrigin::Layer, MaskOrigin::Source]
                            .into_iter()
                            .flat_map(|origin| {
                                [
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
                                        ..Default::default()
                                    },
                                    Mask {
                                        origin,
                                        top: 0.4,
                                        bottom: 0.6,
                                        ..Default::default()
                                    },
                                    Mask::default(),
                                ]
                            })
                            .collect()
                    };
                    for mask in masks {
                        layout.set_mask(0, mask)?;
                        let current = device::fetch(host)?;
                        let plan = Plan::prepare(
                            &current,
                            &layout,
                            screen,
                            &[(layout.layer_info(0).id, "1".into())],
                            target,
                            sizing,
                        )?;
                        println!("Testing {target:?} {sizing:?}, {width} × {height}, zoom {zoom}");
                        let applied = device_send::send(host, &plan)?;
                        let mut raw = applied.raw.clone();
                        // Decode the written physical bank as Preview in a local copy only.
                        let group = &mut raw["device"]["screenAuxGroupList"]["items"]["S1"];
                        group["status"]["pp"]["transition"] =
                            json!(if group["control"]["pp"]["presetDown"] == plan.bank {
                                "AT_UP"
                            } else {
                                "AT_DOWN"
                            });
                        let decoded = device::Snapshot::parse(&identity, &raw)?;
                        let layer = &decoded.screens[screen].layers[0];
                        if let Some(reason) = &layer.unsupported {
                            return Err(reason.clone());
                        }
                        let g = layer.geometry;
                        let want = layout.layers()[0];
                        if (g.x - want.x).abs() > 1.0
                            || (g.y - want.y).abs() > 1.0
                            || (g.width - want.width).abs() > 1.0
                            || (g.height - want.height).abs() > 1.0
                        {
                            return Err(format!("Geometry mismatch: {g:?} vs {want:?}"));
                        }
                        let expected = layout.formats().mask_bounds(0, want)?;
                        let clip = |r: Geometry| {
                            let x = r.x.clamp(g.x, g.x + g.width);
                            let y = r.y.clamp(g.y, g.y + g.height);
                            Geometry {
                                x,
                                y,
                                width: (r.x + r.width).clamp(g.x, g.x + g.width) - x,
                                height: (r.y + r.height).clamp(g.y, g.y + g.height) - y,
                            }
                        };
                        let expected = clip(expected);
                        let actual = layer.mask.bounds(g);
                        for (a, b) in [
                            (expected.x, actual.x),
                            (expected.y, actual.y),
                            (expected.width, actual.width),
                            (expected.height, actual.height),
                        ] {
                            if (a - b).abs() > 1.0 {
                                return Err(format!(
                                    "Mask bounds mismatch: {expected:?} vs {actual:?}"
                                ));
                            }
                        }
                        if visual {
                            println!(
                                "Visual check ready: S1 Preview layer 1 at 12.125,34.375, size {width} × {height}; restoring in 60 seconds."
                            );
                            std::thread::sleep(Duration::from_secs(60));
                            return Ok(());
                        }
                        println!(
                            "Verified {target:?} {sizing:?}: {width} × {height}, zoom {zoom}, mask {mask:?}"
                        );
                    }
                }
            }
        }
        Ok(())
    })();
    // Release opposing edges before restoring the original crop and mask.
    let mut restoration: Vec<_> = restore
        .iter()
        .filter(|(p, _)| p.contains("/cropping/") && !p.ends_with("aspectOverride"))
        .map(|(p, _)| (p.clone(), json!(0)))
        .collect();
    restoration.extend(restore.clone());
    let restored = write(host, &restoration).and_then(|()| device::fetch(host));
    let restored =
        restored.map_err(|e| format!("RESTORE FAILED: {e}; diagnostic result: {result:?}"))?;
    for (path, value) in restore {
        if restored.raw.pointer(&path) != Some(&value) {
            return Err(format!("RESTORE MISMATCH {path}"));
        }
    }
    for list in ["screenList", "auxiliaryList"] {
        if restored.raw["device"][list] != baseline.raw["device"][list] {
            return Err(format!(
                "RESTORE MISMATCH: {list} differs from the full targeted baseline"
            ));
        }
    }
    println!(
        "Original routing, geometry, crop and masks restored; all targeted screens/auxiliaries match baseline."
    );
    result
}
