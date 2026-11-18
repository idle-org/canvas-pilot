//! Explicit WebRCS layer writes, plus separately opted-in advanced input edits. No routing or TAKE.
use crate::{
    Layout,
    device::{self, Snapshot},
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, net::TcpStream, time::Duration};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Target {
    #[default]
    Preview,
    Program,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Sizing {
    #[default]
    OutputBox,
    SourceAspect,
}
#[derive(Clone, Debug)]
pub struct Change {
    pub path: Vec<String>,
    pub before: Value,
    pub after: Value,
}
#[derive(Clone, Debug)]
pub struct Plan {
    pub profile: device::Profile,
    /// Native values that affect conversion, checked again before the first write.
    pub observed: Vec<(String, Value)>,
    pub screen: String,
    pub bank: String,
    pub target: Target,
    pub changes: Vec<Change>,
}
fn field<'a>(raw: &'a Value, path: &str) -> Result<&'a Value, String> {
    raw.pointer(path)
        .ok_or_else(|| format!("Missing device field {path}. Refresh the connection."))
}
fn integer(v: f64, min: f64, max: f64, name: &str) -> Result<i64, String> {
    let v = v.round();
    if !v.is_finite() || v < min || v > max {
        return Err(format!("{name} is outside the device range {min}–{max}."));
    }
    Ok(v as i64)
}
impl Plan {
    /// Map stable editor IDs to device live-layer IDs, in one reviewed screen/bank.
    pub fn prepare(
        snapshot: &Snapshot,
        layout: &Layout,
        screen_index: usize,
        mappings: &[(usize, String)],
        target: Target,
        sizing: Sizing,
    ) -> Result<Self, String> {
        let mut plan =
            snapshot
                .profile
                .prepare(snapshot, layout, screen_index, mappings, target, sizing)?;
        for path in snapshot.profile.screen_context_paths(&plan.screen) {
            plan.observed
                .push((path.clone(), field(&snapshot.raw, &path)?.clone()));
        }
        for (_, destination) in mappings {
            let path = snapshot
                .profile
                .layer_path(&plan.screen, &plan.bank, destination);
            let layer = field(&snapshot.raw, &path)?;
            plan.observed.push((path, layer.clone()));
            if let Some(source) = layer
                .pointer(snapshot.profile.source_field())
                .and_then(Value::as_str)
                && let Some(input) = snapshot.profile.input_from_source(source)
            {
                let path = format!(
                    "/device/inputList/items/{}",
                    snapshot.profile.input_key(input)
                );
                if let Some(value) = snapshot.raw.pointer(&path)
                    && !plan.observed.iter().any(|(p, _)| p == &path)
                {
                    plan.observed.push((path, value.clone()));
                }
            }
        }
        Ok(plan)
    }
    pub(crate) fn prepare_midra(
        snapshot: &Snapshot,
        layout: &Layout,
        screen_index: usize,
        mappings: &[(usize, String)],
        target: Target,
        sizing: Sizing,
    ) -> Result<Self, String> {
        let screen = snapshot
            .screens
            .get(screen_index)
            .ok_or("Choose a device screen.")?;
        if mappings.is_empty() {
            return Err("Choose editor layers and their device destinations.".into());
        }
        if screen.resolution != layout.formats().output {
            return Err("Match the editor and device screen resolutions before sending.".into());
        }
        let bank = match target {
            Target::Preview => screen.preview_bank.as_str(),
            Target::Program => screen.program_bank.as_str(),
        };
        let base = format!("/device/screenList/items/{}", screen.id);
        let mut plan = Self {
            profile: snapshot.profile,
            observed: vec![],
            screen: screen.id.clone(),
            bank: bank.into(),
            target,
            changes: vec![],
        };
        let mut destinations = BTreeSet::new();
        for (id, destination) in mappings {
            if !destinations.insert(destination.clone()) {
                return Err("Map each device destination only once.".into());
            }
            if !screen.layers.iter().any(|l| l.id == *destination) {
                return Err("Choose a device destination for every editor layer.".into());
            }
            let i = (0..layout.layers().len())
                .find(|&i| layout.layer_info(i).id == *id)
                .ok_or("An editor layer was removed.")?;
            let g = layout.layers()[i].validate()?;
            let source = &layout.formats().sources[layout.formats().source_for[i]];
            source.input()?;
            layout.formats().mapping(i, g)?;
            let advanced = layout.formats().advanced[i];
            let crop = [advanced.left, advanced.right, advanced.top, advanced.bottom];
            let native = source.native.map(f64::from);
            let layer = format!("{base}/presetList/items/{bank}/liveLayerList/items/{destination}");
            let device_layer = field(&snapshot.raw, &layer)?;
            let content = [
                device_layer
                    .pointer("/status/pp/contentWidth")
                    .and_then(Value::as_f64),
                device_layer
                    .pointer("/status/pp/contentHeight")
                    .and_then(Value::as_f64),
            ];
            if crate::device::has_content_size(device_layer)
                && crate::device::content_resolution(device_layer)? != source.native
            {
                return Err(format!(
                    "Layer {destination}: editor native source {} × {} differs from device content {} × {}. Match source resolution before sending.",
                    native[0],
                    native[1],
                    content[0].unwrap_or(0.0),
                    content[1].unwrap_or(0.0)
                ));
            }
            let quantized: Vec<_> = crop
                .iter()
                .enumerate()
                .map(|(j, v)| integer(v / native[j / 2] * 65535.0, 0.0, 65535.0, "Crop"))
                .collect::<Result<_, _>>()?;
            if quantized[0] + quantized[1] >= 65535 || quantized[2] + quantized[3] >= 65535 {
                return Err("Crop quantization removes all content.".into());
            }
            let fit = layout.formats().layer_fit[i];
            let size = match (fit, sizing) {
                (Some(_), _) => [g.width, g.height],
                (None, Sizing::OutputBox) => [g.width, g.height],
                (None, Sizing::SourceAspect) => {
                    let scale = (g.width / native[0]).max(g.height / native[1]);
                    [native[0] * scale, native[1] * scale]
                }
            };
            let size = [
                integer(size[0], 1.0, 65535.0, "Width")?,
                integer(size[1], 1.0, 65535.0, "Height")?,
            ];
            let retained = [
                native[0] * (1.0 - (quantized[0] + quantized[1]) as f64 / 65535.0),
                native[1] * (1.0 - (quantized[2] + quantized[3]) as f64 / 65535.0),
            ];
            let scale = (size[0] as f64 / retained[0]).min(size[1] as f64 / retained[1]);
            if fit.is_none()
                && ((retained[0] * scale - g.width).abs() > 1.0
                    || (retained[1] * scale - g.height).abs() > 1.0)
            {
                return Err("Rounded device crop/size differs from the requested visible box by more than one pixel. Adjust the box or zoom.".into());
            }
            let center = g.center();
            let mut requested = vec![
                ("size/pp/sizeH", json!(size[0])),
                ("size/pp/sizeV", json!(size[1])),
                (
                    "position/pp/posH",
                    json!(integer(center[0], -67268.0, 67268.0, "Center X")?),
                ),
                (
                    "position/pp/posV",
                    json!(integer(center[1], -67268.0, 67268.0, "Center Y")?),
                ),
            ];
            for (j, path) in [
                "crop/pp/left",
                "crop/pp/right",
                "crop/pp/top",
                "crop/pp/bottom",
            ]
            .iter()
            .enumerate()
            {
                requested.push((path, json!(quantized[j])));
            }
            let mask = layout.formats().masks[i].validate()?;
            let reveal = layout.formats().mask_bounds(i, g)?;
            let device_left = center[0] - size[0] as f64 / 2.0;
            let device_top = center[1] - size[1] as f64 / 2.0;
            // Normal source-aspect sizing can use a larger container than the
            // editor visible box. Convert the reveal window into that container.
            let edges = if mask.is_empty() {
                [0.0; 4]
            } else {
                [
                    (reveal.x - device_left) / size[0] as f64,
                    (device_left + size[0] as f64 - reveal.x - reveal.width) / size[0] as f64,
                    (reveal.y - device_top) / size[1] as f64,
                    (device_top + size[1] as f64 - reveal.y - reveal.height) / size[1] as f64,
                ]
            };
            let mut units = [0i64; 4];
            for (j, v) in edges.into_iter().enumerate() {
                units[j] = integer(v.clamp(0.0, 1.0) * 65535.0, 0.0, 65535.0, "Mask")?;
            }
            units[1] = units[1].min(65535 - units[0]);
            units[3] = units[3].min(65535 - units[2]);
            for (j, path) in [
                "mask/pp/left",
                "mask/pp/right",
                "mask/pp/top",
                "mask/pp/bottom",
            ]
            .into_iter()
            .enumerate()
            {
                requested.push((path, json!(units[j])));
            }
            requested.push((
                "crop/pp/aspectOverride",
                json!(fit.map_or("CENTERED", crate::formats::LayerFit::midra)),
            ));
            for (suffix, after) in requested {
                let path = format!("{layer}/{suffix}");
                let before = field(&snapshot.raw, &path)?.clone();
                if before != after {
                    plan.changes.push(Change {
                        path: path
                            .trim_start_matches('/')
                            .split('/')
                            .map(str::to_owned)
                            .collect(),
                        before,
                        after,
                    });
                }
            }
        }
        // Release crop first so moving a heavily cropped region never temporarily
        // exhausts an axis while its opposite edge still has the old value.
        plan.changes.sort_by_key(|change| {
            !(change
                .path
                .get(10)
                .is_some_and(|field| field == "crop" || field == "mask")
                && change
                    .after
                    .as_i64()
                    .zip(change.before.as_i64())
                    .is_some_and(|(after, before)| after < before))
        });
        Ok(plan)
    }
    #[cfg(test)]
    fn verified(&self, snapshot: &Snapshot) -> bool {
        self.changes.iter().all(|c| {
            snapshot
                .raw
                .pointer(&format!("/{}", c.path.join("/")))
                .is_some_and(|v| values_equal(v, &c.after))
        })
    }
}
fn values_equal(a: &Value, b: &Value) -> bool {
    a == b
        || a.as_f64()
            .zip(b.as_f64())
            .is_some_and(|(a, b)| (a - b).abs() < 1e-7)
}
trait Transport {
    fn read(&mut self) -> Result<Snapshot, String>;
    fn write(&mut self, change: &Change) -> Result<(), String>;
    fn pause(&self) {}
}
fn execute(plan: &Plan, transport: &mut impl Transport) -> Result<Snapshot, String> {
    execute_changes(plan.profile, &plan.changes, transport, true)
}
fn execute_changes(
    profile: device::Profile,
    changes: &[Change],
    transport: &mut impl Transport,
    apply_preset: bool,
) -> Result<Snapshot, String> {
    for (i, change) in changes.iter().enumerate() {
        transport.write(change).map_err(|e| format!("Send stopped after {i}/{} confirmed socket writes. Device may be partially changed; refresh before retrying. {e}", changes.len()))?;
    }
    // Input crop is staged independently of preset geometry, as in WebRCS.
    let input_crops: BTreeSet<_> = changes
        .iter()
        .filter_map(|change| {
            let p = &change.path;
            (p.get(1).is_some_and(|v| v == "inputList")
                && p.get(8).is_some_and(|v| v == "cropping")
                && p.len() == 12)
                .then(|| {
                    if profile == device::Profile::Aquilon6 {
                        let mut path = p[..8].to_vec();
                        path.push("pp".into());
                        path
                    } else {
                        p[..11].to_vec()
                    }
                })
        })
        .collect();
    for mut path in input_crops {
        path.push("xUpdate".into());
        for value in [false, true] {
            transport
                .write(&Change {
                    path: path.clone(),
                    before: json!(!value),
                    after: json!(value),
                })
                .map_err(|e| format!("Input values sent, but applying input crop failed: {e}"))?;
        }
    }
    // WebRCS commits staged preset edits with a false -> true edge.
    // Always pulse, even if values already match the server's staging store.
    if apply_preset {
        for value in [false, true] {
            let apply = Change {
                path: profile.apply_path(),
                before: json!(!value),
                after: json!(value),
            };
            transport.write(&apply).map_err(|e| {
                format!("Layer values sent, but applying preset updates failed: {e}")
            })?;
        }
    }
    let mut mismatches = Vec::new();
    for _ in 0..3 {
        transport.pause();
        let snapshot = transport.read().map_err(|e| format!("Commands sent, but readback failed; outcome uncertain. Refresh before retrying. {e}"))?;
        if changes.iter().all(|c| {
            snapshot
                .raw
                .pointer(&format!("/{}", c.path.join("/")))
                .is_some_and(|v| values_equal(v, &c.after))
        }) {
            return Ok(snapshot);
        }
        mismatches = changes
            .iter()
            .filter_map(|change| {
                let path = format!("/{}", change.path.join("/"));
                let actual = snapshot.raw.pointer(&path);
                (!actual.is_some_and(|v| values_equal(v, &change.after))).then(|| {
                    format!(
                        "{path}: expected {}, read {}",
                        change.after,
                        actual.unwrap_or(&Value::Null)
                    )
                })
            })
            .take(4)
            .collect();
    }
    Err(format!(
        "Commands sent but readback did not match. Device may have clamped values or another controller changed them. Refresh and review; no automatic resend or rollback was performed. {}",
        mismatches.join("; ")
    ))
}
struct Connection {
    host: String,
    awj_port: u16,
    input: Option<u32>,
    socket: Option<tungstenite::WebSocket<TcpStream>>,
}
impl Transport for Connection {
    fn read(&mut self) -> Result<Snapshot, String> {
        let mut snapshot = device::fetch_with_awj_port(
            &self.host,
            &std::sync::atomic::AtomicBool::new(false),
            self.awj_port,
        )?;
        if let Some(input) = self.input {
            device::enrich_input(&self.host, &mut snapshot, input, self.awj_port)?;
        }
        Ok(snapshot)
    }
    fn write(&mut self, change: &Change) -> Result<(), String> {
        if self.socket.is_none() {
            let address = device::address(&self.host)?;
            let stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))
                .map_err(|e| e.to_string())?;
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .map_err(|e| e.to_string())?;
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .map_err(|e| e.to_string())?;
            let (socket, _) = tungstenite::client(format!("ws://{address}/"), stream)
                .map_err(|e| e.to_string())?;
            self.socket = Some(socket);
        }
        let message = json!({"channel":"DEVICE", "data":{"path":change.path,"value":change.after}})
            .to_string();
        self.socket
            .as_mut()
            .unwrap()
            .send(tungstenite::Message::Text(message.into()))
            .map_err(|e| e.to_string())
    }
    fn pause(&self) {
        std::thread::sleep(Duration::from_millis(200));
    }
}
/// Explicit send: write fields, pulse preset xUpdate, then verify geometry readback.
/// Banks are pinned; another controller's TAKE can change their on-air role.
pub fn send(host: &str, plan: &Plan) -> Result<Snapshot, String> {
    send_with_awj_port(host, plan, 10606)
}
pub(crate) fn send_with_awj_port(
    host: &str,
    plan: &Plan,
    awj_port: u16,
) -> Result<Snapshot, String> {
    let mut connection = Connection {
        host: host.trim().into(),
        awj_port,
        socket: None,
        input: None,
    };
    let fresh = connection.read()?;
    validate_fresh(plan, &fresh)?;
    execute(plan, &mut connection)
}

fn validate_fresh(plan: &Plan, fresh: &Snapshot) -> Result<(), String> {
    if fresh.profile != plan.profile {
        return Err("Device profile changed; reconnect and review mapping.".into());
    }
    let screen = fresh
        .screens
        .iter()
        .find(|s| s.id == plan.screen)
        .ok_or("Destination screen is no longer available.")?;
    let bank = match plan.target {
        Target::Preview => &screen.preview_bank,
        Target::Program => &screen.program_bank,
    };
    if bank != &plan.bank {
        return Err("Preview/Program bank changed since review. Refresh and send again; no writes performed.".into());
    }
    for (path, value) in &plan.observed {
        if fresh.raw.pointer(path) != Some(value) {
            return Err("Device source, framing or destination changed since review. Refresh before sending; no writes performed.".into());
        }
    }
    for c in &plan.changes {
        let path = format!("/{}", c.path.join("/"));
        if !fresh
            .raw
            .pointer(&path)
            .is_some_and(|v| values_equal(v, &c.before))
        {
            return Err(
                "Device values changed since review. Refresh before sending; no writes performed."
                    .into(),
            );
        }
    }
    Ok(())
}

/// Input-only edits: fresh preflight and readback, without preset updates or layer writes.
pub fn send_inputs(
    host: &str,
    draft: &crate::device_inputs::InputDraft,
) -> Result<Snapshot, String> {
    let mut connection = Connection {
        host: host.trim().into(),
        awj_port: 10606,
        socket: None,
        input: Some(draft.input),
    };
    let fresh = connection.read()?;
    let changes = draft.changes(&fresh)?;
    if changes.is_empty() {
        return Ok(fresh);
    }
    execute_changes(fresh.profile, &changes, &mut connection, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn websocket_writes_use_the_configured_port() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let host = listener.local_addr().unwrap().to_string();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            let message = socket.read().unwrap();
            serde_json::from_str::<Value>(message.to_text().unwrap()).unwrap()
        });
        let mut connection = Connection {
            host,
            awj_port: 10606,
            socket: None,
            input: None,
        };
        connection
            .write(&Change {
                path: vec!["test".into()],
                before: json!(0),
                after: json!(123),
            })
            .unwrap();
        assert_eq!(
            server.join().unwrap(),
            json!({"channel":"DEVICE","data":{"path":["test"],"value":123}})
        );
    }

    fn snapshot() -> Snapshot {
        let layer = json!({"position":{"pp":{"posH":500,"posV":300}},"size":{"pp":{"sizeH":640,"sizeV":360}},"source":{"pp":{"input":"INPUT_4"}},"status":{"pp":{"contentWidth":1920,"contentHeight":1080}},"crop":{"pp":{"left":0,"right":0,"top":0,"bottom":0,"aspectOverride":"GLOBAL_SETTING"}},"mask":{"pp":{"left":0,"right":0,"top":0,"bottom":0}}});
        let raw = json!({"device": {"preconfig":{"status":{"stateList":{"items":{"CURRENT":{"screenList":{"items":{"1":{"pp":{"layerCount":2}}}}}}}}},"preset":{"control":{"pp":{"xUpdate":true,"enableAutoUpdate":false}}},"transition":{"screenList":{"items":{"1":{"status":{"pp":{"transition":"AT_DOWN"}}}}}},"screenList":{"items":{"1":{"canvas":{"status":{"size":{"pp":{"sizeH":1920,"sizeV":1080}}}},"presetList":{"items":{"UP":{"liveLayerList":{"items":{"1":layer,"2":layer}}},"DOWN":{"liveLayerList":{"items":{"1":layer,"2":layer}}}}}}}}}});
        Snapshot::parse(
            &json!({"type":"Eikos 4K","version":{"major":3,"minor":3,"patch":10}}),
            &raw,
        )
        .unwrap()
    }
    fn layout() -> Layout {
        let mut l = Layout::default();
        l.select(Some(0));
        l.apply(
            0,
            crate::Geometry {
                x: 12.0,
                y: 34.0,
                width: 300.0,
                height: 250.0,
            },
        )
        .unwrap();
        l.set_framing(
            0,
            crate::formats::Framing {
                zoom: 2.0,
                offset: [30.0, 0.0],
            },
        )
        .unwrap();
        l
    }
    #[test]
    fn sending_accepts_rounded_content_dimensions_but_rejects_real_mismatches() {
        let mut state = snapshot();
        let editor = layout();
        let path = "/device/screenList/items/1/presetList/items/UP/liveLayerList/items/2/status/pp";
        *state.raw.pointer_mut(path).unwrap() =
            json!({"contentWidth":1919.7,"contentHeight":1080.2});
        for sizing in [Sizing::OutputBox, Sizing::SourceAspect] {
            assert!(
                Plan::prepare(
                    &state,
                    &editor,
                    0,
                    &[(2, "2".into())],
                    Target::Preview,
                    sizing
                )
                .is_ok()
            );
        }
        state.raw.pointer_mut(path).unwrap()["contentWidth"] = json!(1800.2);
        assert!(
            Plan::prepare(
                &state,
                &editor,
                0,
                &[(2, "2".into())],
                Target::Preview,
                Sizing::OutputBox
            )
            .is_err()
        );
        state.raw.pointer_mut(path).unwrap()["contentWidth"] = json!(16384.1);
        assert!(
            Plan::prepare(
                &state,
                &editor,
                0,
                &[(2, "2".into())],
                Target::Preview,
                Sizing::OutputBox
            )
            .is_err()
        );
    }

    #[test]
    fn allocated_layers_only_are_valid_send_destinations() {
        let mut raw = snapshot().raw;
        for bank in ["UP", "DOWN"] {
            let entries = raw
                .pointer_mut(&format!(
                    "/device/screenList/items/1/presetList/items/{bank}/liveLayerList/items"
                ))
                .unwrap();
            entries["8"] = entries["1"].clone();
        }
        let state = Snapshot::parse(
            &json!({"type":"Eikos 4K","version":{"major":3,"minor":3,"patch":10}}),
            &raw,
        )
        .unwrap();
        let editor = layout();
        for target in [Target::Preview, Target::Program] {
            assert!(
                Plan::prepare(
                    &state,
                    &editor,
                    0,
                    &[(2, "2".into())],
                    target,
                    Sizing::OutputBox
                )
                .is_ok()
            );
            assert!(
                Plan::prepare(
                    &state,
                    &editor,
                    0,
                    &[(2, "8".into())],
                    target,
                    Sizing::OutputBox
                )
                .is_err()
            );
        }
    }
    #[test]
    fn unassigned_existing_layers_accept_geometry_without_routing_changes() {
        let identity = json!({"type":"Eikos 4K","version":{"major":3,"minor":3,"patch":10}});
        for input in ["NONE", ""] {
            for size in [json!(0), Value::Null] {
                let mut raw = snapshot().raw;
                for bank in ["UP", "DOWN"] {
                    let layer = raw.pointer_mut(&format!("/device/screenList/items/1/presetList/items/{bank}/liveLayerList/items/1")).unwrap();
                    layer["source"]["pp"]["input"] = json!(input);
                    layer["status"]["pp"]["contentWidth"] = size.clone();
                    layer["status"]["pp"]["contentHeight"] = size.clone();
                }
                let state = Snapshot::parse(&identity, &raw).unwrap();
                let editor = layout();
                for target in [Target::Preview, Target::Program] {
                    for sizing in [Sizing::OutputBox, Sizing::SourceAspect] {
                        let plan =
                            Plan::prepare(&state, &editor, 0, &[(2, "1".into())], target, sizing)
                                .unwrap();
                        assert!(!plan.changes.is_empty());
                        assert!(
                            plan.changes
                                .iter()
                                .all(|c| !c.path.iter().any(|key| key == "source"))
                        );
                        assert!(
                            Plan::prepare(&state, &editor, 0, &[(2, "99".into())], target, sizing)
                                .is_err()
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn both_strategies_include_crop_aspect_size_and_center_only() {
        let s = snapshot();
        let l = layout();
        for (strategy, expected_width) in [(Sizing::OutputBox, 300), (Sizing::SourceAspect, 444)] {
            let plan =
                Plan::prepare(&s, &l, 0, &[(2, "1".into())], Target::Preview, strategy).unwrap();
            assert_eq!(plan.bank, "UP");
            assert!(
                plan.changes
                    .iter()
                    .any(|c| c.path.last().unwrap() == "sizeH" && c.after == json!(expected_width))
            );
            assert!(
                plan.changes
                    .iter()
                    .any(|c| c.path.last().unwrap() == "posH" && c.after == json!(162))
            );
            for c in plan.changes {
                assert!(matches!(c.path[10].as_str(), "size" | "position" | "crop"));
                assert_eq!(c.path[5], "items");
            }
        }
        let program = Plan::prepare(
            &s,
            &l,
            0,
            &[(2, "1".into())],
            Target::Program,
            Sizing::OutputBox,
        )
        .unwrap();
        assert_eq!(program.bank, "DOWN");
        assert!(
            Plan::prepare(
                &s,
                &l,
                0,
                &[(1, "1".into()), (2, "1".into())],
                Target::Preview,
                Sizing::OutputBox
            )
            .is_err()
        );
    }
    struct Mock {
        state: Snapshot,
        writes: usize,
        fail_at: Option<usize>,
        ignore: bool,
        writes_log: Vec<(Vec<String>, Value)>,
    }
    impl Transport for Mock {
        fn read(&mut self) -> Result<Snapshot, String> {
            Ok(self.state.clone())
        }
        fn write(&mut self, c: &Change) -> Result<(), String> {
            if self.fail_at == Some(self.writes) {
                return Err("Connection lost".into());
            }
            self.writes += 1;
            self.writes_log.push((c.path.clone(), c.after.clone()));
            if !self.ignore {
                *self
                    .state
                    .raw
                    .pointer_mut(&format!("/{}", c.path.join("/")))
                    .unwrap() = c.after.clone();
            }
            Ok(())
        }
    }
    #[test]
    fn explicit_input_crop_changes_pulse_only_that_input_before_preset_apply() {
        let mut state = snapshot();
        state.raw["device"]["inputList"] = json!({"items":{"INPUT_4":{"plugList":{"items":{"2":{"settings":{"cropping":{"control":{"pp":{"left":0,"xUpdate":true}}}}}}}}}});
        let path =
            "device/inputList/items/INPUT_4/plugList/items/2/settings/cropping/control/pp/left";
        let plan = Plan {
            profile: Default::default(),
            observed: vec![],
            screen: "1".into(),
            bank: "UP".into(),
            target: Target::Preview,
            changes: vec![Change {
                path: path.split('/').map(str::to_owned).collect(),
                before: json!(0),
                after: json!(40),
            }],
        };
        let mut transport = Mock {
            state,
            writes: 0,
            fail_at: None,
            ignore: false,
            writes_log: vec![],
        };
        let mut isolated = Mock {
            state: transport.state.clone(),
            writes: 0,
            fail_at: None,
            ignore: false,
            writes_log: vec![],
        };
        execute_changes(plan.profile, &plan.changes, &mut isolated, false).unwrap();
        assert_eq!(isolated.writes_log.len(), 3);
        assert!(
            isolated
                .writes_log
                .iter()
                .all(|(path, _)| path[1] == "inputList")
        );
        execute(&plan, &mut transport).unwrap();
        assert_eq!(transport.writes_log.len(), 5);
        for (i, value) in [(1, false), (2, true)] {
            assert_eq!(
                transport.writes_log[i].0.join("/"),
                path.replace("/left", "/xUpdate")
            );
            assert_eq!(transport.writes_log[i].1, json!(value));
        }
        assert_eq!(
            transport.writes_log[3].0.join("/"),
            "device/preset/control/pp/xUpdate"
        );
    }

    #[test]
    fn masks_roundtrip_with_every_fit_and_release_opposite_edges_before_increasing() {
        use crate::formats::{LayerFit, Mask};
        for origin in [
            crate::formats::MaskOrigin::Layer,
            crate::formats::MaskOrigin::Source,
        ] {
            for fit in [
                None,
                Some(LayerFit::OneToOne),
                Some(LayerFit::Centered),
                Some(LayerFit::Cropped),
                Some(LayerFit::Fullscreen),
            ] {
                for sizing in [Sizing::OutputBox, Sizing::SourceAspect] {
                    let mut state = snapshot();
                    let mut editor = layout();
                    if let Some(fit) = fit {
                        editor
                            .set_layer_fit(0, Some(fit), Default::default())
                            .unwrap();
                    }
                    editor
                        .set_mask(
                            0,
                            Mask {
                                origin,
                                left: 0.2,
                                right: 0.1,
                                top: 0.3,
                                bottom: 0.2,
                            },
                        )
                        .unwrap();
                    let path =
                        "/device/screenList/items/1/presetList/items/UP/liveLayerList/items/1";
                    *state
                        .raw
                        .pointer_mut(&format!("{path}/mask/pp/right"))
                        .unwrap() = json!(50000);
                    let plan = Plan::prepare(
                        &state,
                        &editor,
                        0,
                        &[(2, "1".into())],
                        Target::Preview,
                        sizing,
                    )
                    .unwrap();
                    let mask_changes = plan
                        .changes
                        .iter()
                        .filter(|c| c.path.get(10).is_some_and(|p| p == "mask"))
                        .collect::<Vec<_>>();
                    assert_eq!(mask_changes[0].path.last().unwrap(), "right");
                    for change in &plan.changes {
                        *state
                            .raw
                            .pointer_mut(&format!("/{}", change.path.join("/")))
                            .unwrap() = change.after.clone();
                    }
                    let identity = serde_json::json!({"type":"Eikos 4K","version":{"major":3,"minor":3,"patch":10}});
                    let read = Snapshot::parse(&identity, &state.raw).unwrap();
                    let layer = &read.screens[0].layers[0];
                    assert!(layer.unsupported.is_none());
                    let expected = editor.formats().mask_bounds(0, editor.layers()[0]).unwrap();
                    let clip = |r: crate::Geometry| {
                        let g = layer.geometry;
                        let x = r.x.clamp(g.x, g.x + g.width);
                        let y = r.y.clamp(g.y, g.y + g.height);
                        crate::Geometry {
                            x,
                            y,
                            width: (r.x + r.width).clamp(g.x, g.x + g.width) - x,
                            height: (r.y + r.height).clamp(g.y, g.y + g.height) - y,
                        }
                    };
                    let expected = clip(expected);
                    let actual = clip(layer.mask.bounds(layer.geometry));
                    assert!(
                        (expected.x - actual.x).abs() < 1.0 && (expected.y - actual.y).abs() < 1.0
                    );
                    assert!(
                        (expected.width - actual.width).abs() < 1.0
                            && (expected.height - actual.height).abs() < 1.0
                    );
                    assert!(
                        plan.changes
                            .iter()
                            .all(|c| !c.path.iter().any(|p| p == "source" || p == "opacity"))
                    );
                }
            }
        }
    }

    #[test]
    fn crop_decreases_precede_increases_when_reframing() {
        let mut state = snapshot();
        *state.raw.pointer_mut("/device/screenList/items/1/presetList/items/UP/liveLayerList/items/1/crop/pp/right").unwrap()=json!(50000);
        let plan = Plan::prepare(
            &state,
            &layout(),
            0,
            &[(2, "1".into())],
            Target::Preview,
            Sizing::OutputBox,
        )
        .unwrap();
        let right = plan
            .changes
            .iter()
            .position(|c| c.path.last().unwrap() == "right")
            .unwrap();
        let left = plan
            .changes
            .iter()
            .position(|c| c.path.last().unwrap() == "left")
            .unwrap();
        assert!(right < left);
    }
    #[test]
    fn layer_crop_is_sent_once_clears_mask_and_rejects_source_mismatch() {
        let mut state = snapshot();
        let mut l = layout();
        let source = l.formats().sources[0];
        l.configure(0, 0, source, l.layers()[0], true).unwrap();
        l.set_framing(
            0,
            crate::formats::Framing {
                zoom: 2.0,
                offset: [30.0, 0.0],
            },
        )
        .unwrap();
        let plan = Plan::prepare(
            &state,
            &l,
            0,
            &[(2, "1".into())],
            Target::Preview,
            Sizing::OutputBox,
        )
        .unwrap();
        let crop = l.formats().advanced[0];
        let top = plan
            .changes
            .iter()
            .find(|c| c.path.last().unwrap() == "top")
            .unwrap();
        assert_eq!(
            top.after,
            json!((crop.top / 1080.0 * 65535.0).round() as i64)
        );
        let path = "/device/screenList/items/1/presetList/items/UP/liveLayerList/items/1";
        *state
            .raw
            .pointer_mut(&format!("{path}/mask/pp/top"))
            .unwrap() = json!(1);
        let plan = Plan::prepare(
            &state,
            &l,
            0,
            &[(2, "1".into())],
            Target::Preview,
            Sizing::OutputBox,
        )
        .unwrap();
        assert!(plan.changes.iter().any(|c| c.path.ends_with(&[
            "mask".into(),
            "pp".into(),
            "top".into()
        ]) && c.after == json!(0)));
        *state
            .raw
            .pointer_mut(&format!("{path}/mask/pp/top"))
            .unwrap() = json!(0);
        *state
            .raw
            .pointer_mut(&format!("{path}/status/pp/contentWidth"))
            .unwrap() = json!(1280);
        assert!(
            Plan::prepare(
                &state,
                &l,
                0,
                &[(2, "1".into())],
                Target::Preview,
                Sizing::OutputBox
            )
            .unwrap_err()
            .contains("differs")
        );
    }
    #[test]
    fn explicit_modes_roundtrip_container_crop_and_bank_without_routing_writes() {
        use crate::formats::{Crop, LayerFit};
        for mode in LayerFit::ALL {
            for target in [Target::Preview, Target::Program] {
                for sizing in [Sizing::OutputBox, Sizing::SourceAspect] {
                    let mut editor = layout();
                    let crop = Crop {
                        left: 123.0,
                        right: 45.0,
                        top: 67.0,
                        bottom: 21.0,
                    };
                    editor.set_layer_fit(0, Some(mode), crop).unwrap();
                    let plan =
                        Plan::prepare(&snapshot(), &editor, 0, &[(2, "1".into())], target, sizing)
                            .unwrap();
                    assert!(
                        plan.changes
                            .iter()
                            .all(|c| !c.path.iter().any(|p| p == "source" || p == "inputList"))
                    );
                    let mut mock = Mock {
                        state: snapshot(),
                        writes: 0,
                        fail_at: None,
                        ignore: false,
                        writes_log: vec![],
                    };
                    let result = execute(&plan, &mut mock).unwrap();
                    let root = format!(
                        "/device/screenList/items/1/presetList/items/{}/liveLayerList/items/1",
                        plan.bank
                    );
                    assert_eq!(
                        result
                            .raw
                            .pointer(&format!("{root}/crop/pp/aspectOverride"))
                            .unwrap(),
                        mode.midra()
                    );
                    assert_eq!(
                        result
                            .raw
                            .pointer(&format!("{root}/size/pp/sizeH"))
                            .unwrap(),
                        300
                    );
                    assert_eq!(
                        result
                            .raw
                            .pointer(&format!("{root}/size/pp/sizeV"))
                            .unwrap(),
                        250
                    );
                    let mut raw = result.raw.clone();
                    // Parse the written bank through the Preview importer.
                    raw["device"]["transition"]["screenList"]["items"]["1"]["status"]["pp"]["transition"] =
                        json!(if plan.bank == "UP" {
                            "AT_DOWN"
                        } else {
                            "AT_UP"
                        });
                    let parsed = Snapshot::parse(
                        &json!({"type":"Eikos 4K","version":{"major":3,"minor":3,"patch":10}}),
                        &raw,
                    )
                    .unwrap();
                    let imported = &parsed.screens[0].layers[0];
                    assert_eq!(imported.fit, Some(mode));
                    assert!(imported.unsupported.is_none());
                    assert_eq!(imported.geometry.width, 300.0);
                    let mut restored = Layout::default();
                    restored
                        .import_device_layers_with_fit(
                            &[(Some(2), imported.geometry, imported.framing)],
                            &[imported.fit],
                        )
                        .unwrap();
                    assert_eq!(restored.formats().layer_fit[0], Some(mode));
                    for (actual, wanted) in [
                        restored.formats().advanced[0].left,
                        restored.formats().advanced[0].right,
                        restored.formats().advanced[0].top,
                        restored.formats().advanced[0].bottom,
                    ]
                    .into_iter()
                    .zip([123.0, 45.0, 67.0, 21.0])
                    {
                        assert!((actual - wanted).abs() < 0.02);
                    }
                }
            }
        }
    }
    #[test]
    fn sent_centered_crop_imports_visible_box_and_framing_in_both_modes() {
        for sizing in [Sizing::OutputBox, Sizing::SourceAspect] {
            let state = snapshot();
            let l = layout();
            let plan =
                Plan::prepare(&state, &l, 0, &[(2, "1".into())], Target::Preview, sizing).unwrap();
            let mut mock = Mock {
                state,
                writes: 0,
                fail_at: None,
                ignore: false,
                writes_log: vec![],
            };
            let result = execute(&plan, &mut mock).unwrap();
            let imported = Snapshot::parse(
                &json!({"type":"Eikos 4K","version":{"major":3,"minor":3,"patch":10}}),
                &result.raw,
            )
            .unwrap();
            let layer = &imported.screens[0].layers[0];
            assert!(layer.unsupported.is_none());
            let (source, crop) = layer.framing.unwrap();
            let image = layer
                .fit
                .unwrap()
                .image(
                    crop.remaining(source.input().unwrap()).unwrap(),
                    layer.geometry,
                )
                .unwrap();
            assert!((image.width - 300.0).abs() < 1.0);
            assert!((image.height - 250.0).abs() < 1.0);
            let mut restored = Layout::default();
            let before = restored.layers().clone();
            let formats = restored.formats().clone();
            restored
                .import_device_layers_with_fit(
                    &[(Some(2), layer.geometry, layer.framing)],
                    &[layer.fit],
                )
                .unwrap();
            assert_eq!(restored.formats().layer_fit[0], layer.fit);
            assert_eq!(restored.formats().advanced[0], crop);
            let framing =
                crate::formats::mapping(source.input().unwrap(), [image.width, image.height], crop)
                    .unwrap()
                    .framing;
            assert!((framing.zoom - 2.0).abs() < 0.001);
            assert!((framing.offset[0] - 30.0).abs() < 0.01);
            restored.undo();
            assert_eq!(*restored.layers(), before);
            assert_eq!(*restored.formats(), formats);
        }
    }
    #[test]
    fn changed_device_state_does_not_block_send_and_partial_failure_is_reported() {
        let state = snapshot();
        let plan = Plan::prepare(
            &state,
            &layout(),
            0,
            &[(2, "1".into())],
            Target::Preview,
            Sizing::OutputBox,
        )
        .unwrap();
        let mut mock = Mock {
            state: state.clone(),
            writes: 0,
            fail_at: None,
            ignore: false,
            writes_log: vec![],
        };
        *mock
            .state
            .raw
            .pointer_mut("/device/screenList/items/1/canvas/status/size/pp")
            .unwrap() = json!(null);
        assert!(plan.verified(&execute(&plan, &mut mock).unwrap()));
        assert_eq!(mock.writes, plan.changes.len() + 2);
        mock.state = state;
        mock.writes = 0;
        mock.fail_at = Some(2);
        assert!(
            execute(&plan, &mut mock)
                .unwrap_err()
                .contains("partially changed")
        );
        assert_eq!(mock.writes, 2);
        mock.writes = 0;
        mock.fail_at = Some(plan.changes.len());
        assert!(
            execute(&plan, &mut mock)
                .unwrap_err()
                .contains("applying preset updates failed")
        );
    }
    #[test]
    fn success_requires_readback_and_unchanged_values_still_apply() {
        let state = snapshot();
        let mut plan = Plan::prepare(
            &state,
            &layout(),
            0,
            &[(2, "1".into())],
            Target::Preview,
            Sizing::OutputBox,
        )
        .unwrap();
        let mut mock = Mock {
            state: state.clone(),
            writes: 0,
            fail_at: None,
            ignore: false,
            writes_log: vec![],
        };
        assert!(plan.verified(&execute(&plan, &mut mock).unwrap()));
        assert_eq!(mock.writes, plan.changes.len() + 2);
        let pulse = ["device", "preset", "control", "pp", "xUpdate"]
            .map(str::to_owned)
            .to_vec();
        assert_eq!(
            &mock.writes_log[plan.changes.len()..],
            &[(pulse.clone(), json!(false)), (pulse, json!(true))]
        );
        assert!(
            mock.writes_log[..plan.changes.len()]
                .iter()
                .all(|(path, _)| path.last().unwrap() != "xUpdate")
        );
        mock.state = state;
        mock.writes = 0;
        mock.ignore = true;
        assert!(
            execute(&plan, &mut mock)
                .unwrap_err()
                .contains("did not match")
        );
        assert_eq!(mock.writes, plan.changes.len() + 2);
        plan.changes.clear();
        mock.writes = 0;
        execute(&plan, &mut mock).unwrap();
        assert_eq!(mock.writes, 2);
    }
    #[test]
    fn fresh_validation_refuses_changed_bank_source_allocation_and_canvas() {
        let state = snapshot();
        let plan = Plan::prepare(
            &state,
            &layout(),
            0,
            &[(2, "1".into())],
            Target::Preview,
            Sizing::OutputBox,
        )
        .unwrap();
        validate_fresh(&plan, &state).unwrap();
        let mut changed = state.clone();
        changed.screens[0].preview_bank = "DOWN".into();
        assert!(
            validate_fresh(&plan, &changed)
                .unwrap_err()
                .contains("bank changed")
        );
        for (path, value) in [
            (
                "/device/screenList/items/1/presetList/items/UP/liveLayerList/items/1/source/pp/input",
                json!("INPUT_9"),
            ),
            (
                "/device/screenList/items/1/canvas/status/size/pp/sizeH",
                json!(3840),
            ),
            (
                "/device/preconfig/status/stateList/items/CURRENT/screenList/items/1/pp/layerCount",
                json!(1),
            ),
        ] {
            let mut changed = state.clone();
            *changed.raw.pointer_mut(path).unwrap() = value;
            assert!(validate_fresh(&plan, &changed).is_err());
        }
        let mut changed = state;
        changed.profile = device::Profile::Aquilon6;
        assert!(validate_fresh(&plan, &changed).is_err());
    }
}
