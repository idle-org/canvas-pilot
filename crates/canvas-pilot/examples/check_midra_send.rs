//! Explicit hardware round trip. Never run as part of automated tests.
use canvas_pilot::{
    Geometry, Layout, device,
    device_send::{self, Plan, Sizing, Target},
    formats::Framing,
};
use serde_json::json;
use std::{
    net::{IpAddr, SocketAddr, TcpStream},
    time::Duration,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 6 || args[1] != "--apply-and-restore" {
        return Err(
            "Usage: check_midra_send --apply-and-restore IP SCREEN LAYER preview|program".into(),
        );
    }
    let host = &args[2];
    let before = device::fetch(host)?;
    let screen = before
        .screens
        .iter()
        .position(|s| s.id == args[3])
        .ok_or("Screen missing")?;
    let target = match args[5].as_str() {
        "preview" => Target::Preview,
        "program" => Target::Program,
        _ => return Err("Invalid target".into()),
    };
    let mut layout = Layout::default();
    layout.select(Some(0));
    layout.apply(
        0,
        Geometry {
            x: 12.5,
            y: 34.5,
            width: 301.0,
            height: 251.0,
        },
    )?;
    let mut source = layout.formats().sources[0];
    source.input_crop.top = 40.0;
    source.input_crop.bottom = 40.0;
    layout.configure(0, 0, source, layout.layers()[0], true)?;
    layout.set_framing(
        0,
        Framing {
            zoom: 2.0,
            offset: [30.0, -10.0],
        },
    )?;
    for sizing in [Sizing::OutputBox, Sizing::SourceAspect] {
        let snapshot = device::fetch(host)?;
        let plan = Plan::prepare(
            &snapshot,
            &layout,
            screen,
            &[(layout.layer_info(0).id, args[4].clone())],
            target,
            sizing,
        )?;
        println!(
            "Testing {target:?}, {sizing:?}, screen {}, bank {}, {} fields",
            plan.screen,
            plan.bank,
            plan.changes.len()
        );
        let result = device_send::send(host, &plan);
        // Explicitly requested restoration to the captured values, not production retry logic.
        let ip: IpAddr = host.parse()?;
        let address = SocketAddr::new(ip, 80);
        let stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let (mut ws, _) = tungstenite::client(format!("ws://{address}/"), stream)?;
        for change in plan.changes.iter().rev() {
            ws.send(tungstenite::Message::Text(
                json!({"channel":"DEVICE","data":{"path":change.path,"value":change.before}})
                    .to_string()
                    .into(),
            ))?;
        }
        std::thread::sleep(Duration::from_millis(600));
        let restored = device::fetch(host)?;
        for change in &plan.changes {
            if restored.raw.pointer(&format!("/{}", change.path.join("/"))) != Some(&change.before)
            {
                return Err(format!("Restore mismatch: {}", change.path.join("/")).into());
            }
        }
        let applied = result?;
        let read_layer = applied.screens[screen]
            .layers
            .iter()
            .find(|l| l.id == args[4])
            .unwrap();
        // Snapshot imports Preview. For Program, decode the same physical bank as
        // Preview in an isolated copy solely to verify the import conversion.
        let mut raw = applied.raw.clone();
        raw["device"]["transition"]["screenList"]["items"][&args[3]]["status"]["pp"]["transition"] =
            json!(if plan.bank == "UP" {
                "AT_DOWN"
            } else {
                "AT_UP"
            });
        let decoded = device::Snapshot::parse(
            &json!({"type":"Eikos 4K","version":{"major":3,"minor":3,"patch":10}}),
            &raw,
        )?;
        let decoded = decoded.screens[screen]
            .layers
            .iter()
            .find(|l| l.id == read_layer.id)
            .unwrap();
        let g = decoded.geometry;
        let wanted = layout.layers()[0];
        if decoded.framing.is_none()
            || (g.width - wanted.width).abs() > 1.0
            || (g.height - wanted.height).abs() > 1.0
            || (g.x - wanted.x).abs() > 1.0
            || (g.y - wanted.y).abs() > 1.0
        {
            return Err("Imported visible geometry did not match within one pixel".into());
        }

        println!("Send readback and restoration verified.");
    }
    Ok(())
}
