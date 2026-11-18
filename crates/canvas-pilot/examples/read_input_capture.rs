//! Explicit read-only check of input capture framing; never changes device settings.
use canvas_pilot::{device, device_capture, device_inputs::InputDraft};
fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let host = args
        .next()
        .ok_or("Usage: read_input_capture DEVICE_IP INPUT_NUMBER")?;
    let input: u32 = args
        .next()
        .ok_or("Input number required")?
        .parse()
        .map_err(|_| "Invalid input number")?;
    let before = device::fetch_input(&host, input)?;
    let draft = InputDraft::read(&before, input)?;
    let bytes = device_capture::capture_for(&host, before.profile, input)?;
    let image = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let size = [image.width(), image.height()];
    let after = device::fetch_input(&host, input)?;
    draft.changes(&after)?;
    let geometry = device_capture::capture_geometry(&before, input, size)?;
    if geometry != device_capture::capture_geometry(&after, input, size)? {
        return Err("Capture geometry changed during the request".into());
    }
    println!(
        "{} · input {input} · PNG {} × {} · {geometry:?}",
        before.identity, size[0], size[1]
    );
    Ok(())
}
