//! Explicit emulator acceptance with restoration. Never run by CI.
use canvas_pilot::{
    Geometry, Layout,
    device::{self, Profile},
    device_send::{self, Plan, Sizing, Target},
    formats::Framing,
};
use serde_json::{Value, json};
use std::{net::TcpStream, time::Duration};
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
    if args.len() != 3
        || !matches!(
            args[1].as_str(),
            "--apply-and-restore" | "--visual-check-and-restore"
        )
    {
        return Err("Usage: check_aquilon_send --apply-and-restore|--visual-check-and-restore IP:PORT (temporarily routes S1 layer 1 to LIVE_1 in both banks)".into());
    }
    let visual = args[1] == "--visual-check-and-restore";
    let host = &args[2];
    let baseline = device::fetch(host)?;
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
    for bank in [&s.preview_bank, &s.program_bank] {
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
    let result = (|| {
        write(host, &setup)?;
        let mut layout = Layout::default();
        layout.set_output(s.resolution)?;
        let targets: &[Target] = if visual {
            &[Target::Preview]
        } else {
            &[Target::Preview, Target::Program]
        };
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
                    let decoded = device::Snapshot::parse(
                        &json!({"type":"AQL Cmax","isLinked":false,"version":{"major":6,"minor":2,"patch":79}}),
                        &raw,
                    )?;
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
                    if visual {
                        println!(
                            "Visual check ready: S1 Preview layer 1 at 12.125,34.375, size {width} × {height}; restoring in 60 seconds."
                        );
                        std::thread::sleep(Duration::from_secs(60));
                        return Ok(());
                    }
                    println!(
                        "Verified {target:?} {sizing:?}: {width} × {height}, zoom {zoom}, fractional center and asymmetric crop"
                    );
                }
            }
        }
        Ok(())
    })();
    let restored = write(host, &restore).and_then(|()| device::fetch(host));
    let restored =
        restored.map_err(|e| format!("RESTORE FAILED: {e}; diagnostic result: {result:?}"))?;
    for (path, value) in restore {
        if restored.raw.pointer(&path) != Some(&value) {
            return Err(format!("RESTORE MISMATCH {path}"));
        }
    }
    println!("Original routing, geometry and crop restored and read back.");
    result
}
