//! LivePremier 6.2 adapter. Native identifiers and paths never escape into editor geometry math.
use super::*;
use crate::{
    Layout,
    device_send::{Change, Plan, Sizing, Target},
    formats::{Aspect, Crop, Mask, Source},
};
use serde_json::json;
use std::collections::BTreeSet;

fn input_settings<'a>(raw: &'a Value, source: &str) -> Option<&'a Value> {
    let id = Profile::Aquilon6.input_from_source(source)?;
    let input = raw.pointer(&format!("/device/inputList/items/IN_{id}"))?;
    let plug = input.pointer("/status/pp/plug")?.as_str()?;
    input.pointer(&format!("/plugList/items/{plug}"))
}
fn source_info(raw: &Value, source: &str) -> Result<Option<(Source, String)>, String> {
    if source == "NONE" || source == "COLOR" {
        return Ok(None);
    }
    let plug = input_settings(raw, source)
        .ok_or("Source dimensions unavailable; only discovered live inputs support framing.")?;
    // Shared input processing is not silently reinterpreted as layer crop.
    for edge in ["left", "right", "top", "bottom"] {
        if number(plug, &format!("/settings/cropping/status/pp/{edge}"))? != 0.0 {
            return Err("Shared input crop is active; visible-bound conversion requires an uncropped input.".into());
        }
    }
    if text(plug, "/settings/aspect/pp/signal")? != "NATIVE"
        || text(plug, "/settings/aspect/pp/transformTo")? != "KEEP_INPUT_RATIO"
    {
        return Err("Shared input aspect correction is active; reset it in WebRCS before transferring framing.".into());
    }
    let native = crate::formats::resolution_from_values([
        number(plug, "/status/signal/pp/imageWidth")?,
        number(plug, "/status/signal/pp/imageHeight")?,
    ])?;
    Ok(Some((
        Source {
            native,
            aspect: Aspect::Custom,
        },
        text(plug, "/settings/aspect/pp/layerFill")?,
    )))
}
fn crop(layer: &Value, native: [u32; 2]) -> Result<Crop, String> {
    let edge = |key: &str, axis: usize| -> Result<f64, String> {
        let n = number(layer, &format!("/cropping/classic/pp/{key}"))?;
        if n.fract() != 0.0 || !(0.0..=65535.0).contains(&n) {
            return Err("Invalid Aquilon crop units.".into());
        }
        Ok(n / 65535.0 * native[axis] as f64)
    };
    Ok(Crop {
        left: edge("left", 0)?,
        right: edge("right", 0)?,
        top: edge("top", 1)?,
        bottom: edge("bottom", 1)?,
    })
}
fn native_mask(layer: &Value) -> Result<Mask, String> {
    let edge = |key: &str| -> Result<f64, String> {
        let n = number(layer, &format!("/cropping/mask/pp/{key}"))?;
        if n.fract() != 0.0 || !(0.0..=65535.0).contains(&n) {
            return Err("Invalid Aquilon mask units.".into());
        }
        Ok(n / 65535.0)
    };
    Mask {
        left: edge("left")?,
        right: edge("right")?,
        top: edge("top")?,
        bottom: edge("bottom")?,
        ..Default::default()
    }
    .validate()
}
// Aquilon blanking is relative to its container; the normal editor workflow
// uses the visible centered image. Preserve their intersection without resizing.
fn relative_mask(reveal: Geometry, reference: Geometry) -> Mask {
    let left = ((reveal.x - reference.x) / reference.width).clamp(0.0, 1.0);
    let top = ((reveal.y - reference.y) / reference.height).clamp(0.0, 1.0);
    Mask {
        left,
        right: ((reference.x + reference.width - reveal.x - reveal.width) / reference.width)
            .clamp(0.0, 1.0 - left),
        top,
        bottom: ((reference.y + reference.height - reveal.y - reveal.height) / reference.height)
            .clamp(0.0, 1.0 - top),
        ..Default::default()
    }
}
fn anchor(layer: &Value) -> Result<(), String> {
    if text(layer, "/position/pp/anchor")? != "MIDDLE_CENTER" {
        return Err("Only middle-center layer anchors are currently supported.".into());
    }
    Ok(())
}
fn framing(
    raw: &Value,
    layer: &Value,
    source: &str,
    container: Geometry,
) -> Result<(Geometry, Option<(Source, Crop)>), String> {
    anchor(layer)?;
    let Some((source, inherited)) = source_info(raw, source)? else {
        if ["left", "right", "top", "bottom"]
            .iter()
            .any(|e| number(layer, &format!("/cropping/classic/pp/{e}")).ok() != Some(0.0))
        {
            return Err("Cannot interpret crop without source dimensions.".into());
        }
        return Ok((container, None));
    };
    let mode = text(layer, "/cropping/classic/pp/aspectOverride")?;
    let mode = if mode == "NONE" {
        inherited.as_str()
    } else {
        mode.as_str()
    };
    if mode != "CENTERED" {
        return Err(format!(
            "{mode} fit mode is not supported for framing; choose Centered in WebRCS."
        ));
    }
    let crop = crop(layer, source.native)?;
    let remaining = crop.remaining(source.native.map(f64::from))?;
    let scale = (container.width / remaining[0]).min(container.height / remaining[1]);
    let [cx, cy] = container.center();
    let visible = Geometry {
        x: cx - remaining[0] * scale / 2.0,
        y: cy - remaining[1] * scale / 2.0,
        width: remaining[0] * scale,
        height: remaining[1] * scale,
    }
    .validate()?;
    Ok((visible, Some((source, crop))))
}
pub(super) fn parse(identity: String, raw: &Value) -> Result<Snapshot, String> {
    let mut screens = Vec::new();
    for list in ["screenList", "auxiliaryList"] {
        let entries = value(raw, &format!("/device/{list}/items"))?
            .as_object()
            .ok_or("Invalid screen list")?;
        if entries.len() > 256 {
            return Err("Too many screens.".into());
        }
        for (id, s) in entries {
            if text(s, "/status/pp/mode")? == "DISABLED" {
                continue;
            }
            crate::DeviceLink {
                family: Family::Aquilon,
                screen: id.clone(),
                layer: "1".into(),
            }
            .validate()?;
            let resolution = crate::formats::resolution_from_values([
                number(s, "/status/size/pp/sizeH")?,
                number(s, "/status/size/pp/sizeV")?,
            ])?;
            let count = value(s, "/status/pp/layerCount")?
                .as_u64()
                .filter(|n| *n <= 128)
                .ok_or("Invalid allocated layer count")?;
            let group = text(s, "/status/pp/transitionGroup")?;
            let group = value(raw, &format!("/device/screenAuxGroupList/items/{group}"))?;
            let up = text(group, "/control/pp/presetUp")?;
            let down = text(group, "/control/pp/presetDown")?;
            if up == down
                || ![&up, &down]
                    .iter()
                    .all(|b| matches!(b.as_str(), "A" | "B" | "C"))
            {
                return Err("Invalid Aquilon bank mapping.".into());
            }
            let (preview_bank, program_bank) = match text(group, "/status/pp/transition")?.as_str()
            {
                "AT_UP" => (down, up),
                "AT_DOWN" => (up, down),
                _ => return Err(format!("Screen {id} is transitioning. Refresh when idle.")),
            };
            let mut layers = Vec::new();
            for n in 1..=count {
                let layer_id = n.to_string();
                let path = Profile::Aquilon6.layer_path(id, &preview_bank, &layer_id);
                let layer = value(raw, &path)?;
                let source = text(layer, "/source/pp/inputNum")?;
                let width = number(layer, "/position/pp/sizeH")?;
                let height = number(layer, "/position/pp/sizeV")?;
                let container = Geometry {
                    x: number(layer, "/position/pp/posH")? - width / 2.0,
                    y: number(layer, "/position/pp/posV")? - height / 2.0,
                    width,
                    height,
                }
                .validate()?;
                let appearance =
                    framing(raw, layer, &source, container).and_then(|(geometry, framing)| {
                        let mask = native_mask(layer)?;
                        let mask = if mask.is_empty() {
                            mask
                        } else {
                            relative_mask(mask.bounds(container), geometry)
                        };
                        Ok((geometry, framing, mask))
                    });
                let (geometry, framing, mask, unsupported) = match appearance {
                    Ok((geometry, framing, mask)) => (geometry, framing, mask, None),
                    Err(error) => (container, None, Mask::default(), Some(error)),
                };
                layers.push(Layer {
                    id: layer_id,
                    source,
                    geometry,
                    container,
                    fit: None,
                    mask,
                    framing,
                    unsupported,
                    warning: None,
                });
            }
            screens.push(Screen {
                id: id.clone(),
                resolution,
                preview_bank,
                program_bank,
                layers,
            });
        }
    }
    screens.sort_by_key(|s| (s.id.starts_with('A'), s.id[1..].parse::<u32>().unwrap_or(0)));
    if screens.is_empty() {
        return Err("No enabled screens returned by device.".into());
    }
    Ok(Snapshot {
        identity,
        profile: Profile::Aquilon6,
        screens,
        raw: raw.clone(),
    })
}
pub(super) fn prepare(
    snapshot: &Snapshot,
    layout: &Layout,
    index: usize,
    mappings: &[(usize, String)],
    target: Target,
    sizing: Sizing,
) -> Result<Plan, String> {
    let screen = snapshot
        .screens
        .get(index)
        .ok_or("Choose a device screen.")?;
    if mappings.is_empty() {
        return Err("Choose editor layers and device destinations.".into());
    }
    if screen.resolution != layout.formats().output {
        return Err("Match the editor and device screen resolutions before sending.".into());
    }
    let bank = match target {
        Target::Preview => &screen.preview_bank,
        Target::Program => &screen.program_bank,
    };
    let mut plan = Plan {
        profile: snapshot.profile,
        observed: vec![],
        screen: screen.id.clone(),
        bank: bank.clone(),
        target,
        changes: vec![],
    };
    let mut destinations = BTreeSet::new();
    for (id, destination) in mappings {
        if !destinations.insert(destination) {
            return Err("Map each device destination only once.".into());
        }
        if !screen.layers.iter().any(|l| &l.id == destination) {
            return Err("Destination is not allocated.".into());
        }
        let i = (0..layout.layers().len())
            .find(|&i| layout.layer_info(i).id == *id)
            .ok_or("An editor layer was removed.")?;
        if layout.formats().layer_fit[i].is_some() {
            return Err("Explicit layer fit is currently supported for Midra only. Disable advanced layer fit before sending to Aquilon.".into());
        }
        let g = layout.layers()[i].validate()?;
        layout.formats().mapping(i, g)?;
        let source = &layout.formats().sources[layout.formats().source_for[i]];
        let path = snapshot.profile.layer_path(&screen.id, bank, destination);
        let layer = value(&snapshot.raw, &path)?;
        anchor(layer)?;
        native_mask(layer)?;
        let routed = text(layer, "/source/pp/inputNum")?;
        if let Some((native, _)) = source_info(&snapshot.raw, &routed)?
            && native.native != source.native
        {
            return Err(format!(
                "Layer {destination}: match editor source resolution to device content {} × {}.",
                native.native[0], native.native[1]
            ));
        }
        let advanced = layout.formats().advanced[i];
        let crop = [advanced.left, advanced.right, advanced.top, advanced.bottom];
        let native = source.native.map(f64::from);
        let units = crop
            .iter()
            .enumerate()
            .map(|(i, n)| (n / native[i / 2] * 65535.0).round())
            .collect::<Vec<_>>();
        if units
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=65535.0).contains(v))
            || units[0] + units[1] >= 65535.0
            || units[2] + units[3] >= 65535.0
        {
            return Err("Crop quantization removes all content.".into());
        }
        let size = match sizing {
            Sizing::OutputBox => [g.width, g.height],
            Sizing::SourceAspect => {
                let scale = (g.width / native[0]).max(g.height / native[1]);
                native.map(|v| v * scale)
            }
        };
        // 6.2.79 stores floats, but WebRCS anchor conversion throws on fractional
        // positions/sizes. Preserve editor precision; quantize only the device plan.
        let size = size.map(f64::round);
        if size.iter().any(|v| !(1.0..=1_000_000.0).contains(v)) {
            return Err("Rounded Aquilon dimensions must be between 1 and 1000000 pixels.".into());
        }
        let retained = [
            native[0] * (1.0 - (units[0] + units[1]) / 65535.0),
            native[1] * (1.0 - (units[2] + units[3]) / 65535.0),
        ];
        let scale = (size[0] / retained[0]).min(size[1] / retained[1]);
        if (retained[0] * scale - g.width).abs() > 1.0
            || (retained[1] * scale - g.height).abs() > 1.0
        {
            return Err(
                "Quantized crop differs from the visible box by more than one pixel.".into(),
            );
        }
        let center = g.center().map(f64::round);
        let mut requested = vec![
            ("position/pp/sizeH", json!(size[0])),
            ("position/pp/sizeV", json!(size[1])),
            ("position/pp/posH", json!(center[0])),
            ("position/pp/posV", json!(center[1])),
            ("cropping/classic/pp/aspectOverride", json!("CENTERED")),
        ];
        for (j, suffix) in [
            "cropping/classic/pp/left",
            "cropping/classic/pp/right",
            "cropping/classic/pp/top",
            "cropping/classic/pp/bottom",
        ]
        .iter()
        .enumerate()
        {
            requested.push((suffix, json!(units[j] as u32)));
        }
        let editor_mask = layout.formats().masks[i].validate()?;
        let container = Geometry {
            x: center[0] - size[0] / 2.0,
            y: center[1] - size[1] / 2.0,
            width: size[0],
            height: size[1],
        };
        let reveal = layout.formats().mask_bounds(i, g)?;
        let mask = if editor_mask.is_empty() {
            Mask::default()
        } else {
            relative_mask(reveal, container)
        };
        let mut mask_units = [mask.left, mask.right, mask.top, mask.bottom]
            .map(|edge| (edge * 65535.0).round() as u32);
        mask_units[1] = mask_units[1].min(65535 - mask_units[0]);
        mask_units[3] = mask_units[3].min(65535 - mask_units[2]);
        if !editor_mask.is_empty() {
            let quantized = Mask {
                left: mask_units[0] as f64 / 65535.0,
                right: mask_units[1] as f64 / 65535.0,
                top: mask_units[2] as f64 / 65535.0,
                bottom: mask_units[3] as f64 / 65535.0,
                ..Default::default()
            };
            let visible = Geometry {
                x: center[0] - retained[0] * scale / 2.0,
                y: center[1] - retained[1] * scale / 2.0,
                width: retained[0] * scale,
                height: retained[1] * scale,
            };
            let expected = relative_mask(reveal, visible).bounds(visible);
            let actual = relative_mask(quantized.bounds(container), visible).bounds(visible);
            if [
                (expected.x, actual.x),
                (expected.y, actual.y),
                (expected.width, actual.width),
                (expected.height, actual.height),
            ]
            .iter()
            .any(|(a, b)| (a - b).abs() > 1.0)
            {
                return Err(
                    "Mask quantization differs from the reveal bounds by more than one pixel."
                        .into(),
                );
            }
        }
        for (edge, units) in ["left", "right", "top", "bottom"]
            .into_iter()
            .zip(mask_units)
        {
            let full = format!("{path}/cropping/mask/pp/{edge}");
            let before = value(&snapshot.raw, &full)?.clone();
            let after = json!(units);
            if before != after {
                plan.changes.push(Change {
                    path: full
                        .trim_start_matches('/')
                        .split('/')
                        .map(str::to_owned)
                        .collect(),
                    before,
                    after,
                });
            }
        }
        for (suffix, after) in requested {
            let full = format!("{path}/{suffix}");
            let before = value(&snapshot.raw, &full)?.clone();
            if before != after {
                plan.changes.push(Change {
                    path: full
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
    plan.changes.sort_by_key(|c| {
        !(c.path.iter().any(|p| p == "classic" || p == "mask")
            && c.after
                .as_f64()
                .zip(c.before.as_f64())
                .is_some_and(|(a, b)| a < b))
    });
    Ok(plan)
}
