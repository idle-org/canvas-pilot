//! Midra 4K native-state adapter.
use super::*;
pub(super) fn parse(identity: String, raw: &Value) -> Result<Snapshot, String> {
    let device = value(raw, "/device")?;
    let items = value(device, "/screenList/items")?
        .as_object()
        .ok_or("Invalid screen list")?;
    let mut screens = Vec::new();
    for (id, s) in items {
        let transition = text(
            device,
            &format!("/transition/screenList/items/{id}/status/pp/transition"),
        )?;
        let bank = match transition.as_str() {
            "AT_DOWN" => "UP",
            "AT_UP" => "DOWN",
            _ => return Err(format!("Screen {id} is transitioning. Refresh when idle.")),
        };
        let w = number(s, "/canvas/status/size/pp/sizeH")?;
        let h = number(s, "/canvas/status/size/pp/sizeV")?;
        // The firmware includes disabled screen slots with a zero canvas.
        if w == 0.0 && h == 0.0 {
            continue;
        }
        if w.fract() != 0.0
            || h.fract() != 0.0
            || !(1.0..=16384.0).contains(&w)
            || !(1.0..=16384.0).contains(&h)
        {
            return Err(format!("Invalid canvas for screen {id}"));
        }
        let count_path = format!(
            "/preconfig/status/stateList/items/CURRENT/screenList/items/{id}/pp/layerCount"
        );
        let count = value(device, &count_path)?
            .as_u64()
            .filter(|count| *count <= 8)
            .ok_or_else(|| format!("Invalid allocated layer count for screen {id}"))?;
        let entries = value(s, &format!("/presetList/items/{bank}/liveLayerList/items"))?
            .as_object()
            .ok_or("Invalid layer list")?;
        let mut layers = Vec::new();
        for (id, layer) in entries {
            // Firmware stores eight slots even when only one or two are allocated.
            if !id.parse::<u64>().is_ok_and(|id| id >= 1 && id <= count) {
                continue;
            }
            // Routing does not determine whether a live-layer slot exists.
            let source = layer
                .pointer("/source/pp/input")
                .and_then(Value::as_str)
                .unwrap_or("NONE")
                .to_owned();
            let width = number(layer, "/size/pp/sizeH")?;
            let height = number(layer, "/size/pp/sizeV")?;
            let geometry = Geometry {
                x: number(layer, "/position/pp/posH")? - width / 2.0,
                y: number(layer, "/position/pp/posV")? - height / 2.0,
                width,
                height,
            }
            .validate()?;
            let warning = content_resolution(layer).ok().and_then(|native| {
                let reported = [
                    number(layer, "/status/pp/contentWidth").ok()?,
                    number(layer, "/status/pp/contentHeight").ok()?,
                ];
                (reported != native.map(f64::from)).then(|| {
                    format!(
                        "Device content resolution rounded from {} × {} to {} × {} pixels.",
                        reported[0], reported[1], native[0], native[1]
                    )
                })
            });
            let mut unsupported = None;
            for edge in ["left", "right", "top", "bottom"] {
                if number(layer, &format!("/crop/pp/{edge}"))? != 0.0 {
                    unsupported =
                        Some("Nonzero crop: visible-bound conversion is not verified.".into());
                }
            }
            let container = geometry;
            let mut geometry = geometry;
            let mut framing = None;
            let mut fit = None;
            if let Some(mode) = layer
                .pointer("/crop/pp/aspectOverride")
                .and_then(Value::as_str)
                .and_then(crate::formats::LayerFit::from_midra)
            {
                let result = (|| -> Result<_, String> {
                    let native = content_resolution(layer)?;
                    let edge = |name: &str, axis: usize| -> Result<f64, String> {
                        let v = number(layer, &format!("/crop/pp/{name}"))?;
                        if v.fract() != 0.0 || !(0.0..=65535.0).contains(&v) {
                            return Err("Invalid device crop units".into());
                        }
                        Ok(v / 65535.0 * f64::from(native[axis]))
                    };
                    let crop = crate::formats::Crop {
                        left: edge("left", 0)?,
                        right: edge("right", 0)?,
                        top: edge("top", 1)?,
                        bottom: edge("bottom", 1)?,
                    };
                    let source = crate::formats::Source {
                        aspect: crate::formats::Aspect::Custom,
                        native,
                    };
                    mode.image(crop.remaining(source.input()?)?, container)?;
                    Ok((source, crop))
                })();
                match result {
                    Ok(value) => {
                        geometry = container;
                        framing = Some(value);
                        fit = Some(mode);
                        unsupported = None;
                    }
                    Err(_) if unsupported.is_none() && !has_content_size(layer) => {}
                    Err(error) => unsupported = Some(error),
                }
            }
            let mask_edge = |edge: &str| -> Result<f64, String> {
                let v = number(layer, &format!("/mask/pp/{edge}"))?;
                if v.fract() != 0.0 || !(0.0..=65535.0).contains(&v) {
                    return Err("Invalid device mask units".into());
                }
                Ok(v / 65535.0)
            };
            let mask = crate::formats::Mask {
                origin: Default::default(),
                left: mask_edge("left")?,
                right: mask_edge("right")?,
                top: mask_edge("top")?,
                bottom: mask_edge("bottom")?,
            }
            .validate()?;
            layers.push(Layer {
                id: id.clone(),
                source,
                geometry,
                container,
                fit,
                mask,
                framing,
                unsupported,
                warning,
            });
        }
        screens.push(Screen {
            id: id.clone(),
            resolution: [w as u32, h as u32],
            preview_bank: bank.into(),
            program_bank: if bank == "UP" { "DOWN" } else { "UP" }.into(),
            layers,
        });
    }
    if screens.is_empty() {
        return Err("No screens returned by device".into());
    }
    Ok(Snapshot {
        identity,
        profile: Profile::Midra4k,
        screens,
        raw: raw.clone(),
    })
}

/// Midra image dimensions already exclude the shared input crop. Verify that
/// controls and signal readback agree before using that processed pixel space.
pub(super) fn cropped_input_resolution(plug: &Value) -> Result<[u32; 2], String> {
    let native = crate::formats::resolution_from_values([
        number(plug, "/status/signal/pp/imageWidth")?,
        number(plug, "/status/signal/pp/imageHeight")?,
    ])?;
    let edge = |key| {
        plug.pointer(&format!("/settings/cropping/control/pp/{key}"))
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v >= 0.)
            .ok_or_else(|| {
                "Input crop status unavailable; refresh before switching input.".to_owned()
            })
    };
    let crop = crate::formats::Crop {
        left: edge("left")?,
        right: edge("right")?,
        top: edge("top")?,
        bottom: edge("bottom")?,
    };
    if crop != crate::formats::Crop::default() {
        let full = [
            number(plug, "/status/signal/pp/aspectWidth")?,
            number(plug, "/status/signal/pp/aspectHeight")?,
        ];
        let retained = crop.remaining(full)?;
        if retained
            .iter()
            .zip(native)
            .any(|(expected, actual)| (*expected - actual as f64).abs() > 1.)
        {
            return Err("Input crop has not settled; refresh before switching input.".into());
        }
    }
    Ok(native)
}
