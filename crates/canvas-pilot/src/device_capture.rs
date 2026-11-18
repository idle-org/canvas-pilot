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

/// How the PNG represents the applied input crop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CaptureFraming {
    /// Aquilon retains full-source coordinates, blanking the cropped edges.
    FullFrame,
    /// Midra returns only the retained image, without the removed edges.
    Cropped,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptureGeometry {
    pub native: [f64; 2],
    pub crop: crate::formats::Crop,
    pub framing: CaptureFraming,
}

/// Interpret device readback, never an unsent editor draft. Callers must also
/// verify that input state stayed unchanged across the capture request.
pub fn capture_geometry(
    snapshot: &Snapshot,
    input: u32,
    image_size: [u32; 2],
) -> Result<CaptureGeometry, String> {
    use crate::device::Profile;
    let draft = crate::device_inputs::InputDraft::read(snapshot, input)?;
    let native = draft.native.ok_or("No valid input signal dimensions")?;
    let prefix = if snapshot.profile == Profile::Aquilon6 {
        "IN"
    } else {
        "INPUT"
    };
    let base = format!(
        "/device/inputList/items/{prefix}_{input}/plugList/items/{}",
        draft.plug
    );
    let plug = snapshot
        .raw
        .pointer(&base)
        .ok_or("Input plug unavailable")?;
    if plug
        .pointer("/control/pp/enableCropFinder")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
    {
        return Err("Disable Crop Finder to preview the applied input crop".into());
    }
    let (crop, framing) = match snapshot.profile {
        Profile::Aquilon6 => {
            let status = plug
                .pointer("/settings/cropping/status/pp")
                .ok_or("Applied input crop unavailable")?;
            let edge = |key: &str| {
                status
                    .get(key)
                    .and_then(serde_json::Value::as_f64)
                    .ok_or("Applied crop edge unavailable")
            };
            (
                crate::formats::Crop {
                    left: edge("left")?,
                    right: edge("right")?,
                    top: edge("top")?,
                    bottom: edge("bottom")?,
                },
                CaptureFraming::FullFrame,
            )
        }
        Profile::Midra4k => {
            // Midra exposes crop controls plus the resulting image dimensions,
            // not Aquilon's separate cropping/status object.
            let controls = plug
                .pointer("/settings/cropping/control/pp")
                .ok_or("Input crop unavailable")?;
            let edge = |key: &str| {
                controls
                    .get(key)
                    .and_then(serde_json::Value::as_f64)
                    .ok_or("Crop edge unavailable")
            };
            (
                crate::formats::Crop {
                    left: edge("left")?,
                    right: edge("right")?,
                    top: edge("top")?,
                    bottom: edge("bottom")?,
                },
                CaptureFraming::Cropped,
            )
        }
    };
    let retained = crop.remaining(native)?;
    let expected = match framing {
        CaptureFraming::FullFrame => native,
        CaptureFraming::Cropped => {
            for (key, expected) in [("imageWidth", retained[0]), ("imageHeight", retained[1])] {
                let actual = plug
                    .pointer(&format!("/status/signal/pp/{key}"))
                    .and_then(serde_json::Value::as_f64)
                    .ok_or("Applied image dimensions unavailable")?;
                if (actual - expected).abs() > 1.0 {
                    return Err(
                        "Input crop has not settled; waiting for matching image dimensions".into(),
                    );
                }
            }
            retained
        }
    };
    if image_size.contains(&0) {
        return Err("Empty capture image".into());
    }
    let longest = usize::from(expected[1] > expected[0]);
    let scale = image_size[longest] as f64 / expected[longest];
    if (0..2).any(|axis| (image_size[axis] as f64 - expected[axis] * scale).abs() > 1.1) {
        return Err("Capture dimensions do not match the applied input crop".into());
    }
    Ok(CaptureGeometry {
        native,
        crop,
        framing,
    })
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
    fn geometry_snapshot(profile: crate::device::Profile, crop: [f64; 4]) -> Snapshot {
        let [left, right, top, bottom] = crop;
        let prefix = if profile == crate::device::Profile::Aquilon6 {
            "IN"
        } else {
            "INPUT"
        };
        Snapshot {
            profile,
            identity: "capture geometry fixture".into(),
            screens: vec![],
            raw: serde_json::json!({"device":{"inputList":{"items":{format!("{prefix}_6"):{
                "status":{"pp":{"isAvailable":true,"plug":"1"}},
                "plugList":{"items":{"1":{
                    "control":{"pp":{"enableCropFinder":false}},
                    "status":{"signal":{"pp":{"isValid":true,"aspectWidth":1920,"aspectHeight":1080,"imageWidth":1920.-left-right,"imageHeight":1080.-top-bottom}}},
                    "settings":{"cropping":{
                        "control":{"pp":{"left":left,"right":right,"top":top,"bottom":bottom,"predefined":"NONE"}},
                        "status":{"pp":{"left":left,"right":right,"top":top,"bottom":bottom}}
                    }}
                }}}
            }}}}}),
        }
    }
    #[test]
    fn midra_capture_uses_retained_dimensions_including_portrait_and_pixel_rounding() {
        use crate::device::Profile;
        for (crop, size) in [
            ([0.; 4], [512, 288]),
            ([1157., 0., 0., 0.], [361, 512]),
            ([279., 0., 0., 338.], [512, 231]),
        ] {
            let mut snapshot = geometry_snapshot(Profile::Midra4k, crop);
            // Actual Eikos 4K state has no cropping/status subtree.
            snapshot
                .raw
                .pointer_mut("/device/inputList/items/INPUT_6/plugList/items/1/settings/cropping")
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove("status");
            let geometry = capture_geometry(&snapshot, 6, size).unwrap();
            assert_eq!(geometry.framing, CaptureFraming::Cropped);
            assert_eq!(geometry.native, [1920., 1080.]);
            assert_eq!(geometry.crop.left, crop[0]);
        }
    }
    #[test]
    fn aquilon_capture_keeps_full_frame_coordinates_and_applied_status() {
        let mut snapshot =
            geometry_snapshot(crate::device::Profile::Aquilon6, [480., 480., 270., 270.]);
        // Pending controls do not describe the captured pixels.
        snapshot.raw["device"]["inputList"]["items"]["IN_6"]["plugList"]["items"]["1"]["settings"]
            ["cropping"]["control"]["pp"]["left"] = serde_json::json!(0);
        let geometry = capture_geometry(&snapshot, 6, [512, 288]).unwrap();
        assert_eq!(geometry.framing, CaptureFraming::FullFrame);
        assert_eq!(geometry.crop.left, 480.);
        assert!(capture_geometry(&snapshot, 6, [361, 512]).is_err());
    }
    #[test]
    fn capture_geometry_rejects_unsettled_crop_missing_edges_finder_and_stale_image() {
        let snapshot = geometry_snapshot(crate::device::Profile::Midra4k, [1157., 0., 0., 0.]);
        assert!(capture_geometry(&snapshot, 6, [512, 288]).is_err());
        assert!(capture_geometry(&snapshot, 6, [0, 512]).is_err());
        for (path, value) in [
            ("status/signal/pp/imageWidth", serde_json::json!(1920)),
            ("status/signal/pp/isValid", serde_json::json!(false)),
            ("settings/cropping/control/pp/left", serde_json::Value::Null),
            ("control/pp/enableCropFinder", serde_json::json!(true)),
        ] {
            let mut changed = snapshot.clone();
            *changed
                .raw
                .pointer_mut(&format!(
                    "/device/inputList/items/INPUT_6/plugList/items/1/{path}"
                ))
                .unwrap() = value;
            assert!(capture_geometry(&changed, 6, [361, 512]).is_err(), "{path}");
        }
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
