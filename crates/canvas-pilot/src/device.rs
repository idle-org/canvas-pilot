//! Device-neutral screen/layer snapshot and shared transport. Native semantics live in adapters.
mod aquilon;
mod midra;
mod profile;
mod state;
use crate::{
    Geometry,
    import::{DetectedShape, ImportDocument},
};
pub use profile::{Family, Profile};
use serde_json::Value;
use std::{
    io::Read,
    net::{IpAddr, SocketAddr},
    time::Duration,
};

const MAX_RESPONSE: u64 = 32 * 1024 * 1024;

/// A locally captured device snapshot, not an editor project or writable payload.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub identity: String,
    pub profile: Profile,
    pub screens: Vec<Screen>,
    /// In-memory firmware state used for reviewed sends; never saved to projects.
    pub raw: Value,
}
#[derive(Clone, Debug)]
pub struct Screen {
    pub id: String,
    pub resolution: [u32; 2],
    pub preview_bank: String,
    pub program_bank: String,
    pub layers: Vec<Layer>,
}
#[derive(Clone, Debug)]
pub struct Layer {
    pub id: String,
    pub source: String,
    pub geometry: Geometry,
    pub container: Geometry,
    pub fit: Option<crate::formats::LayerFit>,
    pub mask: crate::formats::Mask,
    pub framing: Option<(crate::formats::Source, crate::formats::Crop)>,
    pub unsupported: Option<String>,
    pub warning: Option<String>,
}
fn value<'a>(v: &'a Value, path: &str) -> Result<&'a Value, String> {
    v.pointer(path)
        .ok_or_else(|| format!("Missing device field {path}"))
}
fn text(v: &Value, path: &str) -> Result<String, String> {
    value(v, path)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("Invalid device text {path}"))
}
fn number(v: &Value, path: &str) -> Result<f64, String> {
    value(v, path)?
        .as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("Invalid device number {path}"))
}
impl Snapshot {
    pub fn parse(identity: &Value, raw: &Value) -> Result<Self, String> {
        let (profile, label) = Profile::detect(identity)?;
        match profile {
            Profile::Midra4k => midra::parse(label, raw),
            Profile::Aquilon6 => aquilon::parse(label, raw),
        }
    }
}
/// Reported content dimensions are optional for unrouted/no-signal layers.
pub(crate) fn has_content_size(layer: &Value) -> bool {
    ["contentWidth", "contentHeight"].iter().all(|key| {
        layer
            .pointer(&format!("/status/pp/{key}"))
            .and_then(Value::as_f64)
            .is_some_and(|value| value.is_finite() && value > 0.0)
    })
}
/// Quantize reported native content dimensions without accepting invalid ranges.
pub(crate) fn content_resolution(layer: &Value) -> Result<[u32; 2], String> {
    let reported = [
        number(layer, "/status/pp/contentWidth")?,
        number(layer, "/status/pp/contentHeight")?,
    ];
    if reported.iter().any(|v| !(1.0..=16384.0).contains(v)) {
        return Err("Device content resolution must be from 1 to 16384 pixels per side.".into());
    }
    crate::formats::resolution_from_values(reported.map(f64::round))
}

impl Screen {
    pub fn import(&self, indices: &[usize]) -> Result<ImportDocument, String> {
        if indices.is_empty() {
            return Err("Choose device layers to import.".into());
        }
        let mut shapes = Vec::new();
        for &index in indices {
            let layer = self
                .layers
                .get(index)
                .ok_or("Device layer no longer exists")?;
            if let Some(reason) = &layer.unsupported {
                return Err(format!("Layer {}: {reason}", layer.id));
            }
            let id = format!("Screen {} · Layer {} · {}", self.id, layer.id, layer.source);
            if shapes.iter().any(|s: &DetectedShape| s.id == id) {
                return Err("Duplicate device layer selection".into());
            }
            shapes.push(DetectedShape {
                name: None,
                id,
                geometry: layer.geometry,
                outline: [0.0; 2],
                background: false,
                background_only: false,
                zoom: None,
                appearance: None,
            });
        }
        Ok(ImportDocument {
            shapes,
            svg: false,
            note: format!(
                "Preview screen {} ({} × {}). Supported explicit Midra modes preserve container, crop and fit. Aquilon Centered framing uses visible bounds. Inherited or unsupported modes may provide container geometry only. Routing, shared input crop and effects are not imported. Only currently allocated live layers are listed. Review mapping before applying.",
                self.id, self.resolution[0], self.resolution[1]
            ),
        })
    }
}
fn read_json(
    client: &reqwest::blocking::Client,
    url: &str,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<Value, String> {
    let response = client
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Cannot read device: {e}"))?;
    let mut bytes = Vec::new();
    CancelReader {
        inner: response,
        cancelled,
    }
    .take(MAX_RESPONSE + 1)
    .read_to_end(&mut bytes)
    .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_RESPONSE {
        return Err("Device response exceeds 32 MiB".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("Invalid device JSON: {e}"))
}
/// Parse an IP address with an optional TCP port (default 80), or a root HTTP URL.
/// IPv6 with a port uses bracket notation: `[::1]:3000`.
pub fn address(host: &str) -> Result<SocketAddr, String> {
    let host = host.trim();
    let host = host.strip_prefix("http://").unwrap_or(host);
    let host = host.strip_suffix('/').unwrap_or(host);
    let address = host.parse::<IpAddr>().map(|ip| SocketAddr::new(ip, 80))
        .or_else(|_| host.parse::<SocketAddr>())
        .map_err(|_| "Enter a device IP address, optionally with a port, e.g. 192.168.10.146:3000 or [::1]:3000; HTTP root URLs are also accepted; omit credentials, queries and paths.")?;
    if address.port() == 0 {
        return Err("Device port must be between 1 and 65535.".into());
    }
    Ok(address)
}

/// Read identity first; fetch state only for supported devices. Call from a worker thread.
pub fn fetch(host: &str) -> Result<Snapshot, String> {
    fetch_cancellable(host, &std::sync::atomic::AtomicBool::new(false))
}
/// Cancellation is checked between bounded network reads; an in-progress socket read retains its timeout.
pub fn fetch_cancellable(
    host: &str,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<Snapshot, String> {
    fetch_with_awj_port(host, cancelled, 10606)
}

pub(crate) fn fetch_with_awj_port(
    host: &str,
    cancelled: &std::sync::atomic::AtomicBool,
    awj_port: u16,
) -> Result<Snapshot, String> {
    if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("Device read cancelled.".into());
    }
    let authority = address(host)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(45))
        .connect_timeout(Duration::from_secs(5))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let identity = read_json(
        &client,
        &format!("http://{authority}/api/tpp/v1/system"),
        cancelled,
    )?;
    let (profile, label) = Profile::detect(&identity)?;
    let state = match profile {
        Profile::Midra4k => read_json(
            &client,
            &format!("http://{authority}/api/stores/device"),
            cancelled,
        )?,
        Profile::Aquilon6 => state::fetch(
            SocketAddr::new(authority.ip(), awj_port),
            &identity,
            cancelled,
        )
        .map_err(|error| format!("{label}: Aquilon compatibility/read check failed: {error}"))?,
    };
    Snapshot::parse(&identity, &state)
}

pub(super) struct CancelReader<'a, R> {
    pub inner: R,
    pub cancelled: &'a std::sync::atomic::AtomicBool,
}
impl<R: Read> Read for CancelReader<'_, R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        if self.cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(std::io::Error::other("Device read cancelled"));
        }
        self.inner.read(bytes)
    }
}

/// Read one input's preprocessing controls without downloading the whole Aquilon store.
pub fn fetch_input(host: &str, input: u32) -> Result<Snapshot, String> {
    let mut snapshot = fetch(host)?;
    enrich_input(host, &mut snapshot, input, 10606)?;
    Ok(snapshot)
}
pub(crate) fn enrich_input(
    host: &str,
    snapshot: &mut Snapshot,
    input: u32,
    port: u16,
) -> Result<(), String> {
    if !(1..=snapshot.profile.input_limit()).contains(&input) {
        return Err("Invalid input".into());
    }
    if snapshot.profile == Profile::Aquilon6 {
        state::input_settings(SocketAddr::new(address(host)?.ip(), port), snapshot, input)?;
    }
    Ok(())
}

/// Dimensions of the selected input after device preprocessing. Midra input crop
/// is already included in imageWidth/imageHeight and must not be subtracted twice.
pub fn input_content_resolution(snapshot: &Snapshot, input: u32) -> Result<[u32; 2], String> {
    let raw = value(
        &snapshot.raw,
        &format!(
            "/device/inputList/items/{}",
            snapshot.profile.input_key(input)
        ),
    )?;
    let plug = text(raw, "/status/pp/plug")?;
    let settings = value(raw, &format!("/plugList/items/{plug}"))?;
    if settings
        .pointer("/status/signal/pp/isValid")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err("Input has no valid signal".into());
    }
    match snapshot.profile {
        Profile::Midra4k => midra::cropped_input_resolution(settings),
        Profile::Aquilon6 => crate::formats::resolution_from_values([
            number(settings, "/status/signal/pp/imageWidth")?,
            number(settings, "/status/signal/pp/imageHeight")?,
        ]),
    }
}
