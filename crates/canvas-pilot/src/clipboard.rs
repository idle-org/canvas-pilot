//! Versioned clipboard appearance, separate from geometry-only external imports.
use crate::{
    Geometry, Layout,
    formats::{self, Crop, LayerFit, Mask},
    import::{DetectedShape, ImportDocument},
};
use serde::{Deserialize, Serialize};

pub const MARKER: &str = "\n\nCanvas Pilot layer data:\n";
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Appearance {
    pub input_size: [f64; 2],
    pub crop: Crop,
    #[serde(deserialize_with = "required_fit")]
    pub fit: Option<LayerFit>,
    pub mask: Mask,
}
fn required_fit<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<LayerFit>, D::Error> {
    Option::<LayerFit>::deserialize(d)
}
impl Appearance {
    fn validate(self, bounds: Geometry) -> Result<(), String> {
        formats::validate_size(self.input_size)?;
        self.mask.validate()?;
        let retained = self.crop.remaining(self.input_size)?;
        if let Some(fit) = self.fit {
            fit.image(retained, bounds)?;
        } else {
            formats::mapping(self.input_size, [bounds.width, bounds.height], self.crop)?;
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Layer {
    name: String,
    bounds: Geometry,
    appearance: Appearance,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    format: String,
    version: u32,
    layers: Vec<Layer>,
}
/// Append exact, locale-independent values to the readable clipboard summary.
pub fn encode(layout: &Layout, indices: &[usize], summary: &str) -> Result<String, String> {
    if indices.is_empty() || indices.len() > 256 {
        return Err("Copy between 1 and 256 layers.".into());
    }
    let mut layers = Vec::new();
    for &i in indices {
        let bounds = *layout.layers().get(i).ok_or("Invalid copied layer")?;
        let appearance = Appearance {
            input_size: layout.formats().input(i)?,
            crop: layout.formats().advanced[i],
            fit: layout.formats().layer_fit[i],
            mask: layout.formats().masks[i],
        };
        bounds.validate()?;
        appearance.validate(bounds)?;
        layers.push(Layer {
            name: layout.layer_info(i).name.clone(),
            bounds,
            appearance,
        });
    }
    let json = serde_json::to_string_pretty(&Payload {
        format: "canvas-pilot.layers".into(),
        version: 3,
        layers,
    })
    .map_err(|e| e.to_string())?;
    let text = format!("{summary}{MARKER}{json}");
    if text.len() > crate::exchange::MAX_EXCHANGE_BYTES {
        return Err("Clipboard exceeds 16 MiB.".into());
    }
    Ok(text)
}
pub fn parse(json: &str) -> Result<ImportDocument, String> {
    if json.len() > crate::exchange::MAX_EXCHANGE_BYTES {
        return Err("Clipboard exceeds 16 MiB.".into());
    }
    let mut value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("Invalid Canvas Pilot clipboard: {e}"))?;
    if value["version"] == 1 || value["version"] == 2 {
        for layer in value["layers"]
            .as_array_mut()
            .ok_or("Missing clipboard layers")?
        {
            let mask = layer["appearance"]["mask"]
                .as_object_mut()
                .ok_or("Missing clipboard mask")?;
            if mask.contains_key("origin") {
                return Err("Unexpected mask origin in old clipboard.".into());
            }
            mask.insert("origin".into(), "layer".into());
        }
    }
    let payload: Payload = serde_json::from_value(value)
        .map_err(|e| format!("Invalid Canvas Pilot clipboard: {e}"))?;
    if payload.format != "canvas-pilot.layers" || !matches!(payload.version, 1..=3) {
        return Err(
            "Unsupported Canvas Pilot clipboard format/version; expected version 1, 2 or 3.".into(),
        );
    }
    if payload.layers.is_empty() || payload.layers.len() > 256 {
        return Err("Clipboard must contain 1–256 layers.".into());
    }
    let mut shapes = Vec::new();
    for layer in payload.layers {
        if layer.name.trim().is_empty() || layer.name.chars().count() > 256 {
            return Err("Invalid clipboard layer name.".into());
        }
        layer.bounds.validate()?;
        layer.appearance.validate(layer.bounds)?;
        shapes.push(DetectedShape {
            name: (payload.version >= 2).then(|| layer.name.clone()),
            id: layer.name,
            geometry: layer.bounds,
            outline: [0.; 2],
            background: false,
            background_only: false,
            zoom: None,
            appearance: Some(layer.appearance),
        });
    }
    Ok(ImportDocument { shapes, svg: false, note: "Canvas Pilot layer values include exact crop, fit and blanking. Names can be applied separately; source assignments and device links stay unchanged.".into() })
}
impl Layout {
    /// Apply geometry and optional appearance atomically, preserving selection and source identity.
    pub fn apply_mapped_clipboard(
        &mut self,
        values: &[(usize, Geometry)],
        zooms: &[(usize, f64)],
        appearances: &[(usize, Appearance)],
        crop_fit: bool,
        masks: bool,
    ) -> Result<(), String> {
        self.apply_mapped_clipboard_with_names(values, zooms, appearances, crop_fit, masks, &[])
    }
    /// Apply optional explicit names in the same atomic edit as geometry/appearance.
    pub fn apply_mapped_clipboard_with_names(
        &mut self,
        values: &[(usize, Geometry)],
        zooms: &[(usize, f64)],
        appearances: &[(usize, Appearance)],
        crop_fit: bool,
        masks: bool,
        names: &[(usize, String)],
    ) -> Result<(), String> {
        let mut named = std::collections::BTreeSet::new();
        for (i, name) in names {
            if *i >= self.layers.len()
                || !self.selectable(*i)
                || !values.iter().any(|(target, _)| target == i)
                || !named.insert(*i)
                || name.trim().is_empty()
                || name.chars().count() > 128
            {
                return Err(
                    "Each name needs one editable destination and 1–128 characters.".into(),
                );
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for &(i, _) in appearances {
            if i >= self.layers.len() || !self.selectable(i) {
                return Err("Clipboard destination must be an editable layer.".into());
            }
            if !values.iter().any(|(target, _)| *target == i) || !seen.insert(i) {
                return Err("Each clipboard appearance must map to one imported layer.".into());
            }
            if zooms.iter().any(|(target, _)| *target == i) {
                return Err("Choose full crop and fit or legacy zoom, not both.".into());
            }
        }
        let mut draft = self.clone();
        draft.apply_mapped_import_with_zoom(values, zooms)?;
        for &(i, appearance) in appearances {
            if crop_fit {
                formats::validate_size(appearance.input_size)?;
                appearance.crop.remaining(appearance.input_size)?;
                let input = draft.formats.input(i)?;
                let scale = [
                    input[0] / appearance.input_size[0],
                    input[1] / appearance.input_size[1],
                ];
                let crop = Crop {
                    left: appearance.crop.left * scale[0],
                    right: appearance.crop.right * scale[0],
                    top: appearance.crop.top * scale[1],
                    bottom: appearance.crop.bottom * scale[1],
                };
                let g = draft.layers[i];
                let fit = draft.formats.layer_fit[i].or_else(|| {
                    formats::mapping(input, [g.width, g.height], crop)
                        .is_err()
                        .then_some(LayerFit::Centered)
                });
                let adapted = Appearance {
                    input_size: input,
                    crop,
                    fit,
                    mask: appearance.mask,
                };
                adapted.validate(g)?;
                draft.formats.advanced[i] = crop;
                draft.formats.layer_fit[i] = fit;
            }
            if masks {
                draft.formats.masks[i] = appearance.mask.validate()?;
            }
        }
        for (i, name) in names {
            draft.info[*i].name = name.trim().to_owned();
        }
        self.finish_drag();
        let before = self.snapshot();
        self.restore(draft.snapshot());
        self.remember(before);
        Ok(())
    }
}
