//! Explicit Midra/Aquilon emulator diagnostic. Changes Input 1, verifies readback, then restores it.
//! Never run automatically or against a live show.
use canvas_pilot::{device, device_capture, device_inputs::InputDraft, device_send};
fn main() -> Result<(), String> {
    let host = std::env::args()
        .nth(1)
        .ok_or("Specify an authorized emulator address")?;
    let state = device::fetch_input(&host, 1)?;
    let mut draft = InputDraft::read(&state, 1)?;
    println!("{}: {} editable fields", state.identity, draft.fields.len());
    let original = draft.values.clone();
    let before = device_capture::capture_for(&host, state.profile, 1);
    let mut crop = draft.crop();
    crop.left += 24.;
    crop.top += 12.;
    draft.set_crop(crop)?;
    let paths: Vec<_> = draft
        .changes(&state)?
        .iter()
        .map(|c| format!("/{}", c.path.join("/")))
        .collect();
    let result = device_send::send_inputs(&host, &draft);
    let check = (|| {
        let after = result?;
        println!("Crop readback verified");
        let capture = device_capture::capture_for(&host, state.profile, 1);
        if let Some(dir) = std::env::args().nth(2) {
            if let Ok(bytes) = &before {
                let _ = std::fs::write(std::path::Path::new(&dir).join("input-before.png"), bytes);
            }
            if let Ok(bytes) = &capture {
                let _ = std::fs::write(std::path::Path::new(&dir).join("input-after.png"), bytes);
            }
        }
        let input_key = if state.profile == device::Profile::Aquilon6 {
            "IN_1"
        } else {
            "INPUT_1"
        };
        for (label, snapshot) in [("before", &state), ("after", &after)] {
            println!(
                "{label} dimensions: {}",
                snapshot
                    .raw
                    .pointer(&format!(
                        "/device/inputList/items/{input_key}/plugList/items/{}/status/signal/pp",
                        draft.plug
                    ))
                    .unwrap()
            );
        }
        match (&before, &capture) {
            (Ok(a), Ok(b)) => println!(
                "Capture bytes: before {}, after {}, identical {} (does not establish physical processing order)",
                a.len(),
                b.len(),
                a == b
            ),
            _ => println!(
                "Capture unavailable: before {:?}, after {:?}",
                before.as_ref().err(),
                capture.as_ref().err()
            ),
        }
        // Compare editable screen state, excluding computed status fields.
        fn remove_status(v: &mut serde_json::Value) {
            match v {
                serde_json::Value::Object(o) => {
                    o.remove("status");
                    for v in o.values_mut() {
                        remove_status(v);
                    }
                }
                serde_json::Value::Array(a) => {
                    for v in a {
                        remove_status(v)
                    }
                }
                _ => {}
            }
        }
        let mut a = state.raw["device"]["screenList"].clone();
        let mut b = after.raw["device"]["screenList"].clone();
        remove_status(&mut a);
        remove_status(&mut b);
        if a != b {
            return Err("Screen editable state changed during input-only test".into());
        }
        println!("Screen editable state unchanged");
        Ok(())
    })();
    // Always attempt restoration, including after uncertain write outcomes.
    let fresh = device::fetch_input(&host, 1)?;
    let mut restore = InputDraft::read(&fresh, 1)?;
    for path in paths {
        restore.values.insert(path.clone(), original[&path].clone());
    }
    device_send::send_inputs(&host, &restore)?;
    println!("Original input settings restored and verified");
    check
}
