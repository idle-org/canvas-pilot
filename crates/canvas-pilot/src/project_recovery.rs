//! Explicit, reported recovery for opening native projects. Writers and exchange
//! metadata continue to use strict Project validation.
use crate::{
    Layout,
    formats::{Crop, Source},
    project::{FORMAT, MAX_PROJECT_BYTES, Project},
};
use serde_json::Value;

pub struct LoadedProject {
    pub project: Project,
    pub warnings: Vec<String>,
}

pub fn from_json(text: &str) -> Result<LoadedProject, String> {
    if text.len() as u64 > MAX_PROJECT_BYTES {
        return Err("Project exceeds 96 MiB.".into());
    }
    let mut raw: Value =
        serde_json::from_str(text).map_err(|e| format!("Invalid project JSON: {e}"))?;
    if raw["format"] != FORMAT {
        return Err("Not a Canvas Pilot project.".into());
    }
    let version = raw["version"]
        .as_u64()
        .filter(|v| (1..=22).contains(v))
        .ok_or("Unsupported project version; expected versions 1–22.")?;
    let mut warnings = Vec::new();
    if let Some(proposals) = raw.as_object_mut().unwrap().remove("input_proposals") {
        if proposals != serde_json::json!([]) {
            warnings.push(
                "Discarded retired device-input proposals; use shared Input settings.".into(),
            );
        }
        // The strict historical migration expects this field from version 16.
        if (16..20).contains(&version) {
            raw["input_proposals"] = serde_json::json!([]);
        }
    }
    if let Some(inputs) = raw["inputs"].as_array_mut() {
        for (i, input) in inputs.iter_mut().enumerate() {
            if let Some(obj) = input.as_object_mut() {
                if let Some(crop) = obj.remove("input_crop")
                    && crop != serde_json::to_value(Crop::default()).unwrap()
                {
                    warnings.push(format!("Input {}: removed retired local pre-crop; appearance may change. Reload from the device to use its processed dimensions.", i + 1));
                }
                if version < 20 {
                    obj.insert(
                        "input_crop".into(),
                        serde_json::to_value(Crop::default()).unwrap(),
                    );
                }
            }
        }
    }
    if let Ok(project) = Project::from_json(&raw.to_string()) {
        return Ok(LoadedProject { project, warnings });
    }
    warnings.push(
        "Recovered supported fields; missing, invalid or unknown settings were not restored."
            .into(),
    );
    let defaults = Project::from_layout(&Layout::default());
    let mut p = defaults.clone();
    p.editor.selected_layer_ids.clear();
    p.inputs.clear();
    p.layers.clear();
    // Dimensions are safe to default; geometry and identity must be present.
    if let Ok(output) =
        serde_json::from_value::<crate::project::Resolution>(raw["output_resolution"].clone())
        && crate::formats::validate_resolution([output.width, output.height]).is_ok()
    {
        p.output_resolution = output;
    } else {
        warnings.push("Output resolution reset to 1920 × 1080.".into());
    }
    let inputs = raw["inputs"]
        .as_array()
        .ok_or("No recoverable input definitions.")?;
    let mut source_ids = std::collections::BTreeMap::new();
    if inputs.len() > 1024 {
        warnings.push("Inputs beyond the 1024-slot limit were skipped.".into());
    }
    for (i, input) in inputs.iter().take(1024).enumerate() {
        let mut value = input.clone();
        if let Some(obj) = value.as_object_mut() {
            obj.retain(|key, _| {
                ["id", "source_format", "native_resolution"].contains(&key.as_str())
            });
            obj.entry("source_format")
                .or_insert(serde_json::to_value(crate::formats::Aspect::Custom).unwrap());
        }
        let parsed = serde_json::from_value::<crate::project::Input>(value);
        if let Ok(mut input) = parsed
            && input.id > 0
            && !source_ids.contains_key(&input.id)
            && (Source {
                aspect: input.source_format,
                native: [
                    input.native_resolution.width,
                    input.native_resolution.height,
                ],
            })
            .input()
            .is_ok()
        {
            source_ids.insert(input.id, p.inputs.len() + 1);
            input.id = p.inputs.len() + 1;
            p.inputs.push(input);
        } else {
            warnings.push(format!(
                "Input row {} skipped: invalid or duplicate definition.",
                i + 1
            ));
        }
    }
    if p.inputs.is_empty() {
        return Err("No usable input definitions; current project kept.".into());
    }
    let layers = raw["layers"].as_array().ok_or("No recoverable layers.")?;
    let mut ids = std::collections::BTreeSet::new();
    if layers.len() > 256 {
        warnings.push("Layers beyond the 256-layer limit were skipped.".into());
    }
    for (i, layer) in layers.iter().take(256).enumerate() {
        let id = layer["id"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .filter(|v| *v > 0 && *v < usize::MAX);
        let source = layer["source_id"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .and_then(|v| source_ids.get(&v).copied());
        let bounds = serde_json::from_value::<crate::Geometry>(layer["bounds"].clone())
            .ok()
            .and_then(|v| v.validate().ok());
        let (Some(id), Some(source_id), Some(bounds)) = (id, source, bounds) else {
            warnings.push(format!(
                "Layer row {} skipped: missing/invalid identity, source or geometry.",
                i + 1
            ));
            continue;
        };
        if !ids.insert(id) {
            warnings.push(format!("Layer {id} skipped: duplicate identity."));
            continue;
        }
        let mut recovered = defaults.layers[0].clone();
        recovered.id = id;
        recovered.source_id = source_id;
        recovered.bounds = bounds;
        recovered.name = format!("Layer {id}");
        recovered.load_mapping = None;
        recovered.send_mapping = Default::default();
        let input = &p.inputs[source_id - 1];
        let size = [
            input.native_resolution.width as f64,
            input.native_resolution.height as f64,
        ];
        let Ok(crop) = crate::formats::crop_from_framing(
            size,
            [bounds.width, bounds.height],
            Default::default(),
        ) else {
            warnings.push(format!(
                "Layer {id} skipped: unsupported geometry/source scale."
            ));
            continue;
        };
        recovered.advanced_crop = crop;
        p.next_layer_id = p.next_layer_id.max(id + 1);
        p.layers.push(recovered);
        if let Err(error) = p.layout() {
            p.layers.pop();
            warnings.push(format!("Layer {id} skipped: {error}"));
            continue;
        }
        let index = p.layers.len() - 1;
        // Apply dependent fit/crop before masks; validate each change against the
        // complete recovered document so a bad optional field cannot lose geometry.
        for key in [
            "name",
            "folder",
            "pivot",
            "keep_aspect_ratio",
            "visible",
            "locked",
            "load_mapping",
            "send_mapping",
            "layer_fit",
            "advanced_crop",
            "mask",
        ] {
            let Some(value) = layer.get(key) else {
                warnings.push(format!("Layer {id}: {key} defaulted."));
                continue;
            };
            let mut candidate = p.clone();
            let mut data = serde_json::to_value(&candidate.layers[index]).unwrap();
            data[key] = value.clone();
            let result = serde_json::from_value(data)
                .map_err(|e| e.to_string())
                .and_then(|l| {
                    candidate.layers[index] = l;
                    candidate.layout().map(|_| ())
                });
            if result.is_ok() {
                p = candidate;
            } else {
                warnings.push(format!("Layer {id}: invalid {key} reset."));
            }
        }
    }
    if p.layers.is_empty() {
        return Err("No usable layers; current project kept.".into());
    }
    if let Some(next) = raw["next_layer_id"]
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
    {
        p.next_layer_id = p.next_layer_id.max(next);
    }
    // Recover independent document settings individually, including selection,
    // references and previews only when their references and bounds remain valid.
    let baseline = serde_json::to_value(&p).unwrap();
    for section in ["editor", "workspace"] {
        if let Some(fields) = raw[section].as_object() {
            for (key, value) in fields {
                if ["previews", "collapsed_folders", "selected_layer_ids"].contains(&key.as_str()) {
                    continue;
                }
                let mut candidate = serde_json::to_value(&p).unwrap();
                candidate[section][key] = value.clone();
                match serde_json::from_value::<Project>(candidate)
                    .ok()
                    .filter(|p| p.layout().is_ok())
                {
                    Some(recovered) => p = recovered,
                    None => warnings.push(format!("{section}.{key} reset or skipped.")),
                }
            }
        } else if raw[section] != baseline[section] {
            warnings.push(format!("{section} reset to defaults."));
        }
    }
    for (section, limit) in [("folders", 256), ("references", 32)] {
        if let Some(items) = raw[section].as_array() {
            if items.len() > limit {
                warnings.push(format!("{section}: entries beyond {limit} skipped."));
            }
            for (i, item) in items.iter().take(limit).enumerate() {
                let mut candidate = serde_json::to_value(&p).unwrap();
                candidate[section]
                    .as_array_mut()
                    .unwrap()
                    .push(item.clone());
                match serde_json::from_value::<Project>(candidate)
                    .ok()
                    .filter(|p| p.layout().is_ok())
                {
                    Some(recovered) => p = recovered,
                    None => warnings.push(format!("{section} row {} skipped.", i + 1)),
                }
            }
        }
    }
    for (section, key, limit) in [
        ("editor", "selected_layer_ids", 256),
        ("workspace", "collapsed_folders", 256),
        ("workspace", "previews", 256),
    ] {
        if let Some(items) = raw[section][key].as_array() {
            if items.len() > limit {
                warnings.push(format!("{section}.{key}: entries beyond {limit} skipped."));
            }
            for (i, item) in items.iter().take(limit).enumerate() {
                let mut candidate = serde_json::to_value(&p).unwrap();
                let mut item = item.clone();
                if key == "previews"
                    && let Some(id) = item["source_id"]
                        .as_u64()
                        .and_then(|v| usize::try_from(v).ok())
                    && let Some(mapped) = source_ids.get(&id)
                {
                    item["source_id"] = (*mapped).into();
                }
                candidate[section][key].as_array_mut().unwrap().push(item);
                match serde_json::from_value::<Project>(candidate)
                    .ok()
                    .filter(|p| p.layout().is_ok())
                {
                    Some(recovered) => p = recovered,
                    None => warnings.push(format!("{section}.{key} row {} skipped.", i + 1)),
                }
            }
        } else if !raw[section][key].is_null() {
            warnings.push(format!("{section}.{key} reset."));
        }
    }
    p.layout()?;
    Ok(LoadedProject {
        project: p,
        warnings,
    })
}
