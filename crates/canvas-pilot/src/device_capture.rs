//! Explicit, bounded reads of WebRCS input snapshot PNGs.
use crate::device::Snapshot;
use std::{io::Read, time::Duration};

#[derive(Clone, Debug, PartialEq)]
pub struct Input {
    pub id: u32,
    pub signal: bool,
    pub enabled: bool,
}
pub fn inputs(snapshot: &Snapshot) -> Vec<Input> {
    let Some(items) = snapshot
        .raw
        .pointer("/device/inputList/items")
        .and_then(|v| v.as_object())
    else {
        return Vec::new();
    };
    let mut inputs: Vec<_> = items
        .iter()
        .filter_map(|(key, value)| {
            if !value.pointer("/status/pp/isAvailable")?.as_bool()? {
                return None;
            }
            let id = key
                .strip_prefix(match snapshot.profile {
                    crate::device::Profile::Midra4k => "INPUT_",
                    crate::device::Profile::Aquilon6 => "IN_",
                })?
                .parse::<u32>()
                .ok()?;
            if !(1..=snapshot.profile.input_limit()).contains(&id) {
                return None;
            }
            let plug = value.pointer("/status/pp/plug")?.as_str()?;
            Some(Input {
                id,
                signal: value
                    .pointer(&format!("/plugList/items/{plug}/status/signal/pp/isValid"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                enabled: value
                    .pointer("/snapshot/pp/enable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            })
        })
        .collect();
    inputs.sort_by_key(|input| input.id);
    inputs
}

/// A single on-demand GET; no routing, capture-library or device-setting writes.
pub fn capture(host: &str, input: u32) -> Result<Vec<u8>, String> {
    capture_for(host, crate::device::Profile::Midra4k, input)
}
pub fn capture_for(
    host: &str,
    profile: crate::device::Profile,
    input: u32,
) -> Result<Vec<u8>, String> {
    if !(1..=profile.input_limit()).contains(&input) {
        return Err(format!(
            "Select an input between 1 and {}.",
            profile.input_limit()
        ));
    }
    let authority = crate::device::address(host)?;
    read_image(&format!(
        "http://{authority}/api/device/snapshots/inputs/{input}"
    ))
}
fn read_image(url: &str) -> Result<Vec<u8>, String> {
    const LIMIT: u64 = 16 * 1024 * 1024;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(5))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(url)
        .header("Cache-Control", "no-cache")
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Cannot capture input preview: {e}"))?;
    if !response.status().is_success() {
        return Err("The snapshot endpoint redirected instead of returning an image.".into());
    }
    if response.content_length().is_some_and(|size| size > LIMIT) {
        return Err("Input preview exceeds 16 MiB.".into());
    }
    let mut bytes = Vec::new();
    response
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("Input preview exceeds 16 MiB.".into());
    }
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Device did not return a PNG input preview. Check input signal and WebRCS snapshot availability.".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    fn response(status: &str, headers: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let host = listener.local_addr().unwrap().to_string();
        let reply = format!("HTTP/1.1 {status}\r\nConnection: close\r\n{headers}\r\n");
        let bytes = bytes.to_vec();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert_eq!(line, "GET /api/device/snapshots/inputs/3 HTTP/1.1\r\n");
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
            }
            stream.write_all(reply.as_bytes()).unwrap();
            let _ = stream.write_all(&bytes);
        });
        let result = capture(&host, 3);
        server.join().unwrap();
        result
    }
    #[test]
    fn snapshot_reads_are_bounded_and_reject_errors_redirects_and_non_images() {
        let png = b"\x89PNG\r\n\x1a\n";
        assert_eq!(
            response("200 OK", "Content-Length: 8\r\n", png).unwrap(),
            png
        );
        assert!(response("200 OK", "Content-Length: 3\r\n", b"bad").is_err());
        assert!(response("503 Unavailable", "Content-Length: 0\r\n", b"").is_err());
        assert!(
            response(
                "302 Found",
                "Location: http://127.0.0.1/other\r\nContent-Length: 0\r\n",
                b""
            )
            .is_err()
        );
        assert!(response("200 OK", "Content-Length: 16777217\r\n", b"").is_err());
        assert!(capture("not a host", 1).is_err());
        assert!(capture("127.0.0.1", 0).is_err());
    }
    #[test]
    fn input_choices_use_available_hardware_and_current_plug_signal() {
        let snapshot = Snapshot {
            profile: Default::default(),
            identity: String::new(),
            screens: vec![],
            raw: serde_json::json!({"device":{"inputList":{"items":{
                "INPUT_3":{"status":{"pp":{"isAvailable":true,"plug":"2"}},"snapshot":{"pp":{"enable":true}},"plugList":{"items":{"2":{"status":{"signal":{"pp":{"isValid":true}}}}}}},
                "INPUT_2":{"status":{"pp":{"isAvailable":true,"plug":"1"}}},
                "INPUT_11":{"status":{"pp":{"isAvailable":false,"plug":"1"}}}
            }}}}),
        };
        assert_eq!(
            inputs(&snapshot),
            vec![
                Input {
                    id: 2,
                    signal: false,
                    enabled: false
                },
                Input {
                    id: 3,
                    signal: true,
                    enabled: true
                }
            ]
        );
    }
}
