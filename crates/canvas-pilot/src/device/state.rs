//! Targeted Aquilon reads over AWJ (TCP 10606). No full-store download or retained cache.
use super::*;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

const MAX_FRAME: usize = 16 * 1024;
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_FIELDS: usize = 65_536;
const BATCH: usize = 64;

fn awj_path(path: &str) -> String {
    path.trim_start_matches('/')
        .split('/')
        .map(|part| match part {
            "device" => "DeviceObject".to_owned(),
            "items" => "@items".to_owned(),
            "pp" => "@props".to_owned(),
            p if p.ends_with("List") => format!("${}", &p[..p.len() - 4]),
            p => p.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("/")
}
trait Reader {
    fn get(&mut self, paths: &[String]) -> Result<Vec<Value>, String>;
}
struct Connection<'a> {
    reader: BufReader<TcpStream>,
    cancelled: &'a AtomicBool,
    deadline: Instant,
    bytes: usize,
}
impl Connection<'_> {
    fn check(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err("Device read cancelled.".into())
        } else if Instant::now() >= self.deadline {
            Err("Aquilon targeted read timed out after 30 seconds.".into())
        } else {
            Ok(())
        }
    }
    fn frame(&mut self) -> Result<Value, String> {
        let mut frame = Vec::new();
        loop {
            self.check()?;
            let chunk = match self.reader.fill_buf() {
                Ok([]) => return Err("Aquilon AWJ connection closed during read.".into()),
                Ok(chunk) => chunk,
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    continue;
                }
                Err(e) => return Err(format!("Cannot read Aquilon AWJ response: {e}")),
            };
            let end = chunk.iter().position(|b| *b == 4);
            let size = end.map_or(chunk.len(), |n| n + 1);
            self.bytes += size;
            if frame.len() + size > MAX_FRAME || self.bytes > MAX_BYTES {
                return Err("Aquilon targeted response exceeds its size budget.".into());
            }
            frame.extend_from_slice(&chunk[..size]);
            self.reader.consume(size);
            if end.is_some() {
                frame.pop();
                return serde_json::from_slice(&frame)
                    .map_err(|e| format!("Invalid Aquilon AWJ response: {e}"));
            }
        }
    }
}
impl Reader for Connection<'_> {
    fn get(&mut self, paths: &[String]) -> Result<Vec<Value>, String> {
        let mut result = Vec::with_capacity(paths.len());
        for batch in paths.chunks(BATCH) {
            self.check()?;
            let canonical: Vec<_> = batch.iter().map(|p| awj_path(p)).collect();
            let mut pending: BTreeSet<_> = canonical.iter().cloned().collect();
            if pending.len() != batch.len() {
                return Err("Duplicate targeted read path.".into());
            }
            let mut request = Vec::new();
            for path in &canonical {
                serde_json::to_writer(&mut request, &json!({"op":"get","path":path}))
                    .map_err(|e| e.to_string())?;
                request.push(4);
            }
            self.reader
                .get_mut()
                .write_all(&request)
                .map_err(|e| format!("Cannot request Aquilon fields: {e}"))?;
            let mut values = BTreeMap::new();
            while !pending.is_empty() {
                let response = self.frame()?;
                if let Some(error) = response.get("error") {
                    return Err(format!("Aquilon rejected targeted read: {error}"));
                }
                let path = response
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or("AWJ response has no path")?;
                if !pending.remove(path) {
                    return Err(format!(
                        "Unexpected or duplicate Aquilon AWJ response: {path}"
                    ));
                }
                values.insert(
                    path.to_owned(),
                    response
                        .get("value")
                        .ok_or("AWJ response has no value")?
                        .clone(),
                );
            }
            result.extend(canonical.iter().map(|path| values.remove(path).unwrap()));
        }
        Ok(result)
    }
}

pub(super) fn fetch(
    address: SocketAddr,
    identity: &Value,
    cancelled: &AtomicBool,
) -> Result<Value, String> {
    let stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))
        .map_err(|e| format!("Cannot connect to Aquilon AWJ at {address}: {e}. Enable/reach TCP port {} for targeted reads; no full-store fallback is performed.", address.port()))?;
    stream
        .set_read_timeout(Some(Duration::from_millis(250)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    stream.set_nodelay(true).map_err(|e| e.to_string())?;
    let mut connection = Connection {
        reader: BufReader::new(stream),
        cancelled,
        deadline: Instant::now() + Duration::from_secs(30),
        bytes: 0,
    };
    collect(&mut connection, identity)
}
fn set(raw: &mut Value, path: &str, value: Value) {
    let mut node = raw;
    for key in path.trim_start_matches('/').split('/') {
        if !node.is_object() {
            *node = json!({});
        }
        node = node
            .as_object_mut()
            .unwrap()
            .entry(key)
            .or_insert(Value::Null);
    }
    *node = value;
}
struct State<'a, R> {
    reader: &'a mut R,
    raw: Value,
    paths: Vec<String>,
    values: Vec<Value>,
}
impl<R: Reader> State<'_, R> {
    fn read(&mut self, paths: Vec<String>) -> Result<(), String> {
        if self.paths.len() + paths.len() > MAX_FIELDS {
            return Err("Aquilon targeted read exceeds 65536 fields.".into());
        }
        let values = self.reader.get(&paths)?;
        if values.len() != paths.len() {
            return Err("Incomplete Aquilon targeted read.".into());
        }
        for (path, value) in paths.iter().zip(&values) {
            set(&mut self.raw, path, value.clone());
        }
        self.paths.extend(paths);
        self.values.extend(values);
        Ok(())
    }
    fn keys(&self, list: &str, prefix: &str) -> Result<Vec<String>, String> {
        let keys = value(&self.raw, &format!("/device/{list}/itemKeys"))?
            .as_array()
            .ok_or("Invalid Aquilon itemKeys")?;
        if keys.len() > 256 {
            return Err("Aquilon collection exceeds 256 entries.".into());
        }
        let mut unique = BTreeSet::new();
        for key in keys {
            let key = key.as_str().ok_or("Invalid Aquilon identifier")?;
            let id = key
                .strip_prefix(prefix)
                .ok_or("Invalid Aquilon identifier prefix")?;
            if id.is_empty()
                || id.len() > 16
                || id.starts_with('0')
                || !id.bytes().all(|b| b.is_ascii_digit())
                || !unique.insert(key.to_owned())
            {
                return Err("Invalid or duplicate Aquilon identifier.".into());
            }
        }
        Ok(unique.into_iter().collect())
    }
}
fn fields(paths: &mut Vec<String>, base: &str, suffixes: &[&str]) {
    paths.extend(suffixes.iter().map(|suffix| format!("{base}/{suffix}")));
}
fn collect(reader: &mut impl Reader, identity: &Value) -> Result<Value, String> {
    let mut state = State {
        reader,
        raw: json!({"device":{"screenList":{"items":{}},"auxiliaryList":{"items":{}},"inputList":{"items":{}}}}),
        paths: vec![],
        values: vec![],
    };
    state.read(vec![
        "/device/system/deviceList/items/1/pp/dev".into(),
        "/device/system/deviceList/items/1/version/pp/updater".into(),
        "/device/screenList/itemKeys".into(),
        "/device/auxiliaryList/itemKeys".into(),
        "/device/inputList/itemKeys".into(),
    ])?;
    let version = format!(
        "{}.{}.{}",
        identity["version"]["major"], identity["version"]["minor"], identity["version"]["patch"]
    );
    let expected_model =
        profile::aquilon_model(&text(identity, "/type")?).ok_or("Unsupported Aquilon model")?;
    if text(&state.raw, "/device/system/deviceList/items/1/pp/dev")? != expected_model
        || text(
            &state.raw,
            "/device/system/deviceList/items/1/version/pp/updater",
        )? != version
    {
        return Err("Aquilon AWJ model/firmware does not match the HTTP device identity.".into());
    }
    let mut screens = Vec::new();
    for (list, prefix) in [("screenList", "S"), ("auxiliaryList", "A")] {
        for id in state.keys(list, prefix)? {
            screens.push(format!("/device/{list}/items/{id}"));
        }
    }
    let inputs = state.keys("inputList", "IN_")?;
    let mut paths = screens
        .iter()
        .map(|s| format!("{s}/status/pp/mode"))
        .collect::<Vec<_>>();
    paths.extend(
        inputs
            .iter()
            .map(|i| format!("/device/inputList/items/{i}/status/pp/isAvailable")),
    );
    state.read(paths)?;
    let screens: Vec<_> = screens
        .into_iter()
        .map(|screen| {
            let mode = text(&state.raw, &format!("{screen}/status/pp/mode"))?;
            Ok((screen, mode != "DISABLED"))
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .filter_map(|(screen, enabled)| enabled.then_some(screen))
        .collect();
    let inputs: Vec<_> = inputs
        .into_iter()
        .map(|input| {
            let available = value(
                &state.raw,
                &format!("/device/inputList/items/{input}/status/pp/isAvailable"),
            )?
            .as_bool()
            .ok_or("Invalid Aquilon input availability")?;
            Ok((input, available))
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .filter_map(|(input, available)| available.then_some(input))
        .collect();
    let mut paths = Vec::new();
    for screen in &screens {
        fields(
            &mut paths,
            screen,
            &[
                "status/pp/layerCount",
                "status/pp/transitionGroup",
                "status/size/pp/sizeH",
                "status/size/pp/sizeV",
            ],
        );
    }
    for input in &inputs {
        fields(
            &mut paths,
            &format!("/device/inputList/items/{input}"),
            &["status/pp/plug", "snapshot/pp/enable"],
        );
    }
    state.read(paths)?;
    let mut groups = BTreeSet::new();
    let mut paths = Vec::new();
    for screen in &screens {
        let group = text(&state.raw, &format!("{screen}/status/pp/transitionGroup"))?;
        if group.is_empty() || group.len() > 16 || !group.bytes().all(|b| b.is_ascii_alphanumeric())
        {
            return Err("Invalid Aquilon transition group identifier.".into());
        }
        groups.insert(group);
    }
    for group in &groups {
        fields(
            &mut paths,
            &format!("/device/screenAuxGroupList/items/{group}"),
            &[
                "control/pp/presetUp",
                "control/pp/presetDown",
                "status/pp/transition",
            ],
        );
    }
    for input in &inputs {
        let base = format!("/device/inputList/items/{input}");
        let plug = text(&state.raw, &format!("{base}/status/pp/plug"))?;
        if plug.is_empty() || plug.len() > 16 || !plug.bytes().all(|b| b.is_ascii_digit()) {
            return Err("Invalid Aquilon plug identifier.".into());
        }
        fields(
            &mut paths,
            &format!("{base}/plugList/items/{plug}"),
            &[
                "status/signal/pp/imageWidth",
                "status/signal/pp/imageHeight",
                "status/signal/pp/isValid",
                "settings/aspect/pp/signal",
                "settings/aspect/pp/transformTo",
                "settings/aspect/pp/layerFill",
                "settings/cropping/status/pp/left",
                "settings/cropping/status/pp/right",
                "settings/cropping/status/pp/top",
                "settings/cropping/status/pp/bottom",
            ],
        );
    }
    state.read(paths)?;
    let mut paths = Vec::new();
    for screen in &screens {
        let count = value(&state.raw, &format!("{screen}/status/pp/layerCount"))?
            .as_u64()
            .filter(|v| *v <= 128)
            .ok_or("Invalid allocated layer count")?;
        let group = text(&state.raw, &format!("{screen}/status/pp/transitionGroup"))?;
        let base = format!("/device/screenAuxGroupList/items/{group}");
        let up = text(&state.raw, &format!("{base}/control/pp/presetUp"))?;
        let down = text(&state.raw, &format!("{base}/control/pp/presetDown"))?;
        if up == down
            || ![&up, &down]
                .iter()
                .all(|b| matches!(b.as_str(), "A" | "B" | "C"))
        {
            return Err("Invalid Aquilon bank mapping.".into());
        }
        if !matches!(
            text(&state.raw, &format!("{base}/status/pp/transition"))?.as_str(),
            "AT_UP" | "AT_DOWN"
        ) {
            return Err("Aquilon screen is transitioning; refresh when idle.".into());
        }
        for bank in [&up, &down] {
            for layer in 1..=count {
                fields(
                    &mut paths,
                    &format!("{screen}/presetList/items/{bank}/layerList/items/{layer}"),
                    &[
                        "position/pp/posH",
                        "position/pp/posV",
                        "position/pp/sizeH",
                        "position/pp/sizeV",
                        "position/pp/anchor",
                        "source/pp/inputNum",
                        "cropping/classic/pp/aspectOverride",
                        "cropping/classic/pp/left",
                        "cropping/classic/pp/right",
                        "cropping/classic/pp/top",
                        "cropping/classic/pp/bottom",
                        "cropping/mask/pp/left",
                        "cropping/mask/pp/right",
                        "cropping/mask/pp/top",
                        "cropping/mask/pp/bottom",
                    ],
                );
                if paths.len() > MAX_FIELDS {
                    return Err("Aquilon targeted read exceeds 65536 fields.".into());
                }
            }
        }
    }
    state.read(paths)?;
    // A second bounded pass catches edits, input changes and bank/allocation changes
    // while collecting the first pass. Never use a mixed snapshot to plan writes.
    if state.reader.get(&state.paths)? != state.values {
        return Err("Aquilon state changed during targeted read. Refresh when idle.".into());
    }
    Ok(state.raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> Value {
        json!({"type":"AQL Cmax","isLinked":false,"version":{"major":6,"minor":2,"patch":79}})
    }
    fn fixture() -> Value {
        let mut raw: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/aquilon-6.2.json")).unwrap();
        for list in ["screenList", "auxiliaryList", "inputList"] {
            let keys: Vec<_> = raw["device"][list]["items"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect();
            raw["device"][list]["itemKeys"] = json!(keys);
        }
        set(
            &mut raw,
            "/device/system/deviceList/items/1/pp/dev",
            json!("NLC_CMAX"),
        );
        set(
            &mut raw,
            "/device/system/deviceList/items/1/version/pp/updater",
            json!("6.2.79"),
        );
        raw["device"]["screenList"]["items"]["S1"]["presetList"]["items"]["A"]["layerList"]["items"]
            ["1"]["source"]["pp"]["inputNum"] = json!("LIVE_1");
        raw
    }
    struct Memory {
        raw: Value,
        calls: usize,
        change: bool,
    }
    impl Reader for Memory {
        fn get(&mut self, paths: &[String]) -> Result<Vec<Value>, String> {
            self.calls += 1;
            if self.change && self.calls == 6 {
                self.raw["device"]["screenAuxGroupList"]["items"]["S1"]["status"]["pp"]["transition"] =
                    json!("AT_DOWN");
            }
            paths
                .iter()
                .map(|path| {
                    self.raw
                        .pointer(path)
                        .cloned()
                        .ok_or_else(|| format!("Missing test field {path}"))
                })
                .collect()
        }
    }
    fn memory(raw: Value) -> Memory {
        Memory {
            raw,
            calls: 0,
            change: false,
        }
    }
    #[test]
    fn targeted_collection_reads_only_allocated_banks_and_detects_changes() {
        let mut reader = memory(fixture());
        let raw = collect(&mut reader, &identity()).unwrap();
        let snapshot = Snapshot::parse(&identity(), &raw).unwrap();
        assert_eq!(snapshot.screens.len(), 1);
        assert_eq!(snapshot.screens[0].layers.len(), 7);
        assert!(
            raw.pointer("/device/screenList/items/S1/presetList/items/C")
                .is_none()
        );
        assert!(
            raw.pointer("/device/screenList/items/S1/presetList/items/A/layerList/items/8")
                .is_none()
        );
        assert!(
            raw.pointer("/device/auxiliaryList/items/A1/presetList")
                .is_none()
        );
        assert!(
            crate::device_capture::inputs(&snapshot)
                .iter()
                .any(|i| i.id == 17)
        );
        reader = memory(fixture());
        reader.change = true;
        assert!(
            collect(&mut reader, &identity())
                .unwrap_err()
                .contains("changed during")
        );
    }
    #[test]
    fn every_model_checks_exact_awj_identity_and_firmware() {
        let models: Vec<(String, String)> =
            serde_json::from_str(include_str!("../../tests/fixtures/aquilon-models.json")).unwrap();
        for (http, awj) in models {
            for version in [(6, 2, 79), (4, 0, 1), (7, 1, 0)] {
                let mut id = identity();
                id["type"] = json!(http);
                id["version"] = json!({"major":version.0,"minor":version.1,"patch":version.2});
                let mut raw = fixture();
                set(
                    &mut raw,
                    "/device/system/deviceList/items/1/pp/dev",
                    json!(awj),
                );
                set(
                    &mut raw,
                    "/device/system/deviceList/items/1/version/pp/updater",
                    json!(format!("{}.{}.{}", version.0, version.1, version.2)),
                );
                let collected = collect(&mut memory(raw.clone()), &id).unwrap();
                assert_eq!(
                    Snapshot::parse(&id, &collected).unwrap().screens[0]
                        .layers
                        .len(),
                    7
                );
                set(
                    &mut raw,
                    "/device/system/deviceList/items/1/pp/dev",
                    json!(if awj == "NLC_CMAX" {
                        "NLC_C"
                    } else {
                        "NLC_CMAX"
                    }),
                );
                assert!(
                    collect(&mut memory(raw), &id)
                        .unwrap_err()
                        .contains("does not match")
                );
            }
        }
    }
    #[test]
    fn malformed_topology_and_mismatched_endpoint_fail() {
        for (path, bad) in [
            ("/device/system/deviceList/items/1/pp/dev", json!("OTHER")),
            (
                "/device/system/deviceList/items/1/version/pp/updater",
                json!("6.2.80"),
            ),
            ("/device/screenList/itemKeys", json!(["S1", "S1"])),
            ("/device/screenList/itemKeys", json!(["S1/other"])),
            ("/device/inputList/itemKeys", json!(["IN_01"])),
            (
                "/device/screenList/items/S1/status/pp/layerCount",
                json!(129),
            ),
            (
                "/device/screenList/items/S1/status/pp/transitionGroup",
                json!("../S1"),
            ),
            ("/device/inputList/items/IN_1/status/pp/plug", json!("../1")),
            (
                "/device/screenAuxGroupList/items/S1/control/pp/presetUp",
                json!("D"),
            ),
            (
                "/device/screenAuxGroupList/items/S1/status/pp/transition",
                json!("MOVING"),
            ),
        ] {
            let mut raw = fixture();
            set(&mut raw, path, bad);
            assert!(collect(&mut memory(raw), &identity()).is_err(), "{path}");
        }
    }
    fn connect(address: SocketAddr, cancelled: &AtomicBool) -> Connection<'_> {
        let stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        Connection {
            reader: BufReader::new(stream),
            cancelled,
            deadline: Instant::now() + Duration::from_secs(3),
            bytes: 0,
        }
    }
    #[test]
    fn canonical_paths_pipelining_fragmentation_and_out_of_order_replies() {
        assert_eq!(
            awj_path(
                "/device/screenList/items/S1/presetList/items/A/layerList/items/1/position/pp/sizeH"
            ),
            "DeviceObject/$screen/@items/S1/$preset/@items/A/$layer/@items/1/position/@props/sizeH"
        );
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let mut reader = BufReader::new(listener.accept().unwrap().0);
            let mut paths = vec![];
            for _ in 0..2 {
                let mut bytes = vec![];
                reader.read_until(4, &mut bytes).unwrap();
                bytes.pop();
                let message: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(message["op"], "get");
                paths.push(message["path"].clone());
            }
            for (i, path) in paths.into_iter().enumerate().rev() {
                let bytes = format!("{}\x04", json!({"path":path,"value":i}));
                for part in bytes.as_bytes().chunks(7) {
                    reader.get_mut().write_all(part).unwrap();
                }
            }
        });
        let cancel = AtomicBool::new(false);
        let mut reader = connect(address, &cancel);
        assert_eq!(
            reader
                .get(&["/device/first".into(), "/device/second".into()])
                .unwrap(),
            vec![json!(0), json!(1)]
        );
        server.join().unwrap();
    }
    #[test]
    fn wire_errors_oversized_frames_missing_values_and_cancellation_fail_closed() {
        for response in [
            b"not json\x04".to_vec(),
            b"{\"error\":{\"code\":\"E12\"}}\x04".to_vec(),
            b"{\"path\":\"DeviceObject/wrong\",\"value\":1}\x04".to_vec(),
            b"{\"path\":\"DeviceObject/test\"}\x04".to_vec(),
            vec![b' '; MAX_FRAME + 1],
            vec![],
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let mut reader = BufReader::new(listener.accept().unwrap().0);
                let mut request = vec![];
                reader.read_until(4, &mut request).unwrap();
                let _ = reader.get_mut().write_all(&response);
            });
            let cancel = AtomicBool::new(false);
            assert!(
                connect(address, &cancel)
                    .get(&["/device/test".into()])
                    .is_err()
            );
            server.join().unwrap();
        }
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let cancel = AtomicBool::new(true);
        let mut reader = connect(listener.local_addr().unwrap(), &cancel);
        assert!(
            reader
                .get(&["/device/test".into()])
                .unwrap_err()
                .contains("cancelled")
        );
        cancel.store(false, Ordering::Relaxed);
        reader.deadline = Instant::now();
        assert!(
            reader
                .get(&["/device/test".into()])
                .unwrap_err()
                .contains("timed out")
        );
    }

    #[test]
    fn http_identity_awj_preflight_websocket_writes_and_awj_readback() {
        use crate::{
            Geometry, Layout,
            device_send::{self, Plan, Sizing, Target},
        };
        use std::sync::{Arc, Mutex};
        let raw = fixture();
        let state = Snapshot::parse(
            &identity(),
            &collect(&mut memory(raw.clone()), &identity()).unwrap(),
        )
        .unwrap();
        let mut layout = Layout::default();
        layout
            .apply(
                0,
                Geometry {
                    x: 12.125,
                    y: 34.375,
                    width: 301.25,
                    height: 251.5,
                },
            )
            .unwrap();
        let plan = Plan::prepare(
            &state,
            &layout,
            0,
            &[(layout.layer_info(0).id, "1".into())],
            Target::Preview,
            Sizing::SourceAspect,
        )
        .unwrap();
        let changes = plan.changes.len();
        let raw = Arc::new(Mutex::new(raw));
        let awj = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = awj.local_addr().unwrap().port();
        let awj_raw = raw.clone();
        let awj_server = std::thread::spawn(move || {
            let mut count = 0;
            for _ in 0..2 {
                let mut reader = BufReader::new(awj.accept().unwrap().0);
                reader
                    .get_mut()
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                loop {
                    let mut bytes = vec![];
                    if reader.read_until(4, &mut bytes).unwrap() == 0 {
                        break;
                    }
                    bytes.pop();
                    let message: Value = serde_json::from_slice(&bytes).unwrap();
                    assert_eq!(message["op"], "get");
                    let path = message["path"].as_str().unwrap();
                    let native = path
                        .split('/')
                        .map(|p| match p {
                            "DeviceObject" => "device".to_owned(),
                            "@items" => "items".to_owned(),
                            "@props" => "pp".to_owned(),
                            p if p.starts_with('$') => format!("{}List", &p[1..]),
                            p => p.to_owned(),
                        })
                        .collect::<Vec<_>>()
                        .join("/");
                    let value = awj_raw
                        .lock()
                        .unwrap()
                        .pointer(&format!("/{native}"))
                        .unwrap()
                        .clone();
                    write!(
                        reader.get_mut(),
                        "{}\x04",
                        json!({"path":path,"value":value})
                    )
                    .unwrap();
                    count += 1;
                }
            }
            count
        });
        let http = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let host = http.local_addr().unwrap().to_string();
        let server = std::thread::spawn(move || {
            fn identity_response(listener: &std::net::TcpListener) {
                let mut reader = BufReader::new(listener.accept().unwrap().0);
                reader
                    .get_mut()
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                assert_eq!(
                    line, "GET /api/tpp/v1/system HTTP/1.1\r\n",
                    "Full-store HTTP requests are forbidden"
                );
                loop {
                    line.clear();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                }
                let body = identity().to_string();
                write!(
                    reader.get_mut(),
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
            identity_response(&http);
            let stream = http.accept().unwrap().0;
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut ws = tungstenite::accept(stream).unwrap();
            for i in 0..changes + 2 {
                let message = ws.read().unwrap();
                let message: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
                assert_eq!(message["channel"], "DEVICE");
                let path = message["data"]["path"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join("/");
                let value = message["data"]["value"].clone();
                if i >= changes {
                    assert_eq!(path, "device/screenAuxGroupList/control/pp/xUpdate");
                    assert_eq!(value, json!(i == changes + 1));
                }
                assert!(
                    !path.contains("source/")
                        && !path.contains("inputList")
                        && !path.contains("xTake")
                );
                set(&mut raw.lock().unwrap(), &path, value);
            }
            identity_response(&http);
        });
        let result = device_send::send_with_awj_port(&host, &plan, port);
        assert!(result.is_ok(), "{result:?}");
        server.join().unwrap();
        assert!(awj_server.join().unwrap() < 2000);
    }
}
