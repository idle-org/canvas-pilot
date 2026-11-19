//! Versioned offline interchange. Contract: docs/interfaces/canvas-pilot.md.
use crate::{
    Corner, Geometry, Layout, PositionPoint,
    formats::{self, Aspect, Crop, Source},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};

pub const MAX_PROJECT_BYTES: u64 = 96 * 1024 * 1024;
pub const DEFAULT_BACKDROP_TITLE: &str = "Example Conference Name";
pub const FORMAT: &str = "beffroi.midra-layout";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}
impl From<[u32; 2]> for Resolution {
    fn from([width, height]: [u32; 2]) -> Self {
        Self { width, height }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub id: usize,
    pub source_format: Aspect,
    pub native_resolution: Resolution,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub pivot: [f64; 2],
    pub keep_aspect_ratio: bool,
    #[serde(deserialize_with = "deserialize_link")]
    pub load_mapping: Option<crate::DeviceLink>,
    pub send_mapping: crate::SendMapping,
    pub name: String,
    pub folder: String,
    pub id: usize,
    pub source_id: usize,
    pub bounds: Geometry,
    pub advanced_crop: Crop,
    pub mask: formats::Mask,
    #[serde(deserialize_with = "deserialize_fit")]
    pub layer_fit: Option<formats::LayerFit>,
    pub visible: bool,
    pub locked: bool,
}
fn deserialize_fit<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<formats::LayerFit>, D::Error> {
    Option::<formats::LayerFit>::deserialize(deserializer)
}
fn deserialize_link<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<crate::DeviceLink>, D::Error> {
    Option::<crate::DeviceLink>::deserialize(deserializer)
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub name: String,
    pub image_base64: String,
    pub visible: bool,
    pub opacity: f32,
    pub preserve_ratio: bool,
    pub native_size: bool,
    pub offset: [f64; 2],
}
impl Reference {
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        if self
            .offset
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
        {
            return Err("Reference offsets must be finite and within ±1,000,000 pixels.".into());
        }
        if !self.opacity.is_finite() || !(0.0..=1.0).contains(&self.opacity) {
            return Err("Reference opacity must be between 0 and 1.".into());
        }
        if self.image_base64.len() > 90 * 1024 * 1024 {
            return Err("Reference image is too large.".into());
        }
        let bytes = STANDARD
            .decode(&self.image_base64)
            .map_err(|e| format!("Invalid embedded reference: {e}"))?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err("Reference image exceeds 64 MiB.".into());
        }
        Ok(bytes)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorSettings {
    pub backdrop_title: String,
    pub selected_layer_ids: Vec<usize>,
    pub position_point: PositionPoint,
    #[serde(deserialize_with = "deserialize_layer_point")]
    pub position_layer_point: Option<PositionPoint>,
    pub viewport_zoom: f32,
    pub viewport_pan: [f32; 2],
    pub show_source_pattern: bool,
    pub show_toolbar_icon_names: bool,
    pub outline_layers: bool,
}
fn deserialize_layer_point<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<PositionPoint>, D::Error> {
    Option::<PositionPoint>::deserialize(deserializer)
}
/// Project-specific working setup; network jobs and connections are never restored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub snapping: crate::snapping::Snapping,
    pub resize_mode: crate::ResizeMode,
    pub grid_spacing: Option<u32>,
    pub collapsed_folders: Vec<String>,
    pub capture_interval_seconds: u64,
    pub show_empty_inputs: bool,
    pub previews: Vec<Preview>,
}
impl Default for Workspace {
    fn default() -> Self {
        Self {
            snapping: Default::default(),
            resize_mode: Default::default(),
            grid_spacing: None,
            collapsed_folders: Vec::new(),
            capture_interval_seconds: 10,
            show_empty_inputs: false,
            previews: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    pub layer_id: usize,
    pub source_id: usize,
    pub input: u32,
    pub host: String,
    pub enabled: bool,
    pub automatic: bool,
    pub guide_opacity: f32,
    pub name: String,
    pub image_base64: String,
}
impl Preview {
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        STANDARD
            .decode(&self.image_base64)
            .map_err(|e| format!("Invalid preview image: {e}"))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub folders: Vec<String>,
    pub workspace: Workspace,
    pub format: String,
    pub version: u32,
    pub next_layer_id: usize,
    pub output_resolution: Resolution,
    pub inputs: Vec<Input>,
    pub layers: Vec<Layer>,
    pub references: Vec<Reference>,
    pub editor: EditorSettings,
}
impl Project {
    pub fn from_layout(layout: &Layout) -> Self {
        Self {
            folders: layout.folders.clone(),
            workspace: Workspace::default(),
            format: FORMAT.into(),
            version: 21,
            next_layer_id: layout.next_id,
            output_resolution: layout.formats.output.into(),
            inputs: (0..layout.formats.sources.len())
                .map(|i| {
                    let source = layout.formats.sources[i];
                    Input {
                        id: i + 1,
                        source_format: source.aspect,
                        native_resolution: source.native.into(),
                    }
                })
                .collect(),
            layers: (0..layout.layers.len())
                .map(|i| Layer {
                    pivot: layout.info[i].pivot,
                    keep_aspect_ratio: layout.info[i].keep_aspect_ratio,
                    load_mapping: layout.info[i].load_mapping.clone(),
                    send_mapping: layout.info[i].send_mapping.clone(),
                    name: layout.info[i].name.clone(),
                    folder: layout.info[i].folder.clone(),
                    id: layout.info[i].id,
                    source_id: layout.formats.source_for[i] + 1,
                    bounds: layout.layers[i],
                    advanced_crop: layout.formats.advanced[i],
                    mask: layout.formats.masks[i],
                    layer_fit: layout.formats.layer_fit[i],
                    visible: layout.visible[i],
                    locked: layout.locked[i],
                })
                .collect(),
            references: Vec::new(),
            editor: EditorSettings {
                backdrop_title: DEFAULT_BACKDROP_TITLE.into(),
                selected_layer_ids: layout
                    .selected_indices()
                    .into_iter()
                    .map(|i| layout.info[i].id)
                    .collect(),
                position_point: PositionPoint::TopLeft,
                position_layer_point: None,
                viewport_zoom: 1.0,
                viewport_pan: [0.0; 2],
                show_source_pattern: false,
                show_toolbar_icon_names: false,
                outline_layers: false,
            },
        }
    }
    /// Build a fresh document only after validating the entire file. No history is restored.
    pub fn layout(&self) -> Result<Layout, String> {
        if self.format != FORMAT || self.version != 21 {
            return Err(
                "Unsupported project format/version; expected beffroi.midra-layout version 21."
                    .into(),
            );
        }
        if self.editor.backdrop_title.chars().count() > 128
            || self.editor.backdrop_title.chars().any(char::is_control)
        {
            return Err("Backdrop title must be a single line of at most 128 characters.".into());
        }
        let output = [self.output_resolution.width, self.output_resolution.height];
        formats::validate_resolution(output)?;
        let mut layout = Layout::default();
        if self.layers.is_empty()
            || self.layers.len() > 256
            || self.inputs.is_empty()
            || self.inputs.len() > 1024
        {
            return Err("Projects require 1–256 layers and 1–1024 input slots.".into());
        }
        let count = self.layers.len();
        layout.layers = vec![
            Geometry {
                x: 0.0,
                y: 0.0,
                width: 640.0,
                height: 360.0
            };
            count
        ];
        if self.folders.len() > 256
            || self
                .folders
                .iter()
                .any(|f| f.trim().is_empty() || f != f.trim() || f.chars().count() > 128)
            || self
                .folders
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != self.folders.len()
        {
            return Err(
                "Invalid explicit folder names; use up to 256 unique names of 1–128 characters."
                    .into(),
            );
        }
        layout.folders = self.folders.clone();
        layout.info.clear();
        layout.next_id = self.next_layer_id;
        layout.selected = vec![false; count];
        layout.visible = vec![true; count];
        layout.locked = vec![false; count];
        layout.formats.sources = vec![Source::default(); self.inputs.len()];
        layout.formats.source_for = vec![0; count];
        layout.formats.advanced = vec![Crop::default(); count];
        layout.formats.layer_fit = vec![None; count];
        layout.formats.masks = vec![Default::default(); count];
        layout.formats.output = output;
        for (i, input) in self.inputs.iter().enumerate() {
            if input.id != i + 1 {
                return Err("Input IDs must be consecutive starting at 1.".into());
            }
            let source = Source {
                aspect: input.source_format,
                native: [
                    input.native_resolution.width,
                    input.native_resolution.height,
                ],
            };
            source.input()?;
            layout.formats.sources[i] = source;
        }
        for (i, layer) in self.layers.iter().enumerate() {
            if layer.id == 0
                || layer.id >= self.next_layer_id
                || layout.info.iter().any(|old| old.id == layer.id)
                || !(1..=self.inputs.len()).contains(&layer.source_id)
                || layer.name.trim().is_empty()
                || layer.name.chars().count() > 128
                || layer.folder.chars().count() > 128
            {
                return Err("Invalid layer identity, name, folder or source ID.".into());
            }
            layer.send_mapping.validate()?;
            if let Some(link) = &layer.load_mapping {
                link.validate()?;
            }
            crate::validate_pivot(layer.pivot)?;
            layout.info.push(crate::LayerInfo {
                pivot: layer.pivot,
                keep_aspect_ratio: layer.keep_aspect_ratio,
                load_mapping: layer.load_mapping.clone(),
                send_mapping: layer.send_mapping.clone(),
                id: layer.id,
                name: layer.name.clone(),
                folder: layer.folder.clone(),
            });
            layer.bounds.validate()?;
            layout.layers[i] = layer.bounds;
            layout.formats.source_for[i] = layer.source_id - 1;
            layout.formats.advanced[i] = layer.advanced_crop;
            layout.formats.layer_fit[i] = layer.layer_fit;
            layout.formats.masks[i] = layer.mask.validate()?;
            layout.formats.mapping(i, layer.bounds)?;
            layout.visible[i] = layer.visible;
            layout.locked[i] = layer.locked;
        }
        let e = &self.editor;
        if !e.viewport_zoom.is_finite()
            || !(0.0001..=256.0).contains(&e.viewport_zoom)
            || e.viewport_pan
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 1e9)
        {
            return Err("Invalid viewport camera settings.".into());
        }
        layout.selected = vec![false; count];
        for &id in &e.selected_layer_ids {
            let index = layout
                .info
                .iter()
                .position(|info| info.id == id)
                .ok_or("Unknown selected layer ID")?;
            if layout.selected[index] || !layout.selectable(index) {
                return Err("Selected layer IDs must be unique, visible, unlocked layers.".into());
            }
            layout.selected[index] = true;
        }

        if self.references.len() > 32 {
            return Err("A project supports up to 32 reference images.".into());
        }
        let mut reference_bytes = 0;
        for r in &self.references {
            reference_bytes += r.bytes()?.len();
        }
        let w = &self.workspace;
        if !(5..=300).contains(&w.capture_interval_seconds)
            || w.snapping
                .grid_divisions
                .iter()
                .any(|n| !(1..=64).contains(n))
            || w.grid_spacing.is_some_and(|n| ![10, 100].contains(&n))
        {
            return Err("Invalid workspace grid or capture interval.".into());
        }
        let mut folders = std::collections::BTreeSet::new();
        for folder in &w.collapsed_folders {
            if folder.is_empty()
                || !folders.insert(folder)
                || (!self.folders.contains(folder)
                    && !self.layers.iter().any(|l| &l.folder == folder))
            {
                return Err("Collapsed folders must be unique existing folders.".into());
            }
        }
        let mut ids = std::collections::BTreeSet::new();
        for preview in &w.previews {
            if !ids.insert(preview.layer_id)
                || !self
                    .layers
                    .iter()
                    .any(|l| l.id == preview.layer_id && l.source_id == preview.source_id)
                || preview.input > 1024
                || (!preview.host.is_empty() && crate::device::address(&preview.host).is_err())
                || !preview.guide_opacity.is_finite()
                || !(0.0..=1.0).contains(&preview.guide_opacity)
                || preview.name.len() > 1024
            {
                return Err("Invalid layer preview assignment or settings.".into());
            }
            reference_bytes += preview.bytes()?.len();
        }
        if reference_bytes > 64 * 1024 * 1024 {
            return Err("Reference and preview images exceed the combined 64 MiB limit.".into());
        }
        Ok(layout)
    }
    pub fn to_json(&self) -> Result<String, String> {
        self.layout()?;
        serde_json::to_string_pretty(self)
            .map(|s| s + "\n")
            .map_err(|e| e.to_string())
    }
    pub fn from_json(text: &str) -> Result<Self, String> {
        if text.len() as u64 > MAX_PROJECT_BYTES {
            return Err("Project exceeds 96 MiB.".into());
        }
        // Version 1 predates toolbar captions. Migrate only that known schema;
        // current files still require every field and reject unknown fields.
        #[derive(Deserialize)]
        struct Header {
            version: u32,
        }
        let header: Header =
            serde_json::from_str(text).map_err(|e| format!("Invalid project JSON: {e}"))?;
        if !(1..=21).contains(&header.version) {
            return Err("Unsupported project version; expected versions 1–21.".into());
        }
        let mut migrated: serde_json::Value = if header.version == 1 || header.version == 2 {
            let mut value: serde_json::Value =
                serde_json::from_str(text).map_err(|e| format!("Invalid project JSON: {e}"))?;
            if header.version == 1 {
                let editor = value
                    .get_mut("editor")
                    .and_then(|v| v.as_object_mut())
                    .ok_or("Invalid version 1 editor settings.")?;
                if editor.contains_key("show_toolbar_icon_names") {
                    return Err("Unexpected toolbar captions setting in version 1 project.".into());
                }
                editor.insert("show_toolbar_icon_names".into(), false.into());
            }
            if value.get("next_layer_id").is_some() {
                return Err("Unexpected identity counter in old project.".into());
            }
            if value
                .get("inputs")
                .and_then(|v| v.as_array())
                .is_none_or(|v| v.len() != 2)
            {
                return Err("Legacy projects must have two inputs.".into());
            }
            let layers = value
                .get_mut("layers")
                .and_then(|v| v.as_array_mut())
                .ok_or("Missing layers")?;
            if layers.len() != 2 {
                return Err("Legacy projects must have two layers.".into());
            }
            for (i, layer) in layers.iter_mut().enumerate() {
                if layer.get("name").is_some()
                    || layer.get("folder").is_some()
                    || layer.get("id").and_then(|v| v.as_u64()) != Some((i + 1) as u64)
                {
                    return Err("Invalid legacy layer fields".into());
                }
                layer["name"] = if i == 0 { "Presenter" } else { "Slides" }.into();
                layer["folder"] = "".into();
            }
            value["next_layer_id"] = 3.into();
            value["version"] = 3.into();
            value
        } else {
            serde_json::from_str(text).map_err(|e| format!("Invalid project JSON: {e}"))?
        };
        if (1..=3).contains(&header.version) {
            let editor = migrated
                .get_mut("editor")
                .and_then(|v| v.as_object_mut())
                .ok_or("Missing editor settings")?;
            if editor.contains_key("outline_layers") {
                return Err("Unexpected outline setting in old project".into());
            }
            editor.insert("outline_layers".into(), false.into());
            if let Some(reference) = migrated
                .get_mut("reference")
                .and_then(|v| v.as_object_mut())
            {
                if reference.contains_key("native_size") || reference.contains_key("offset") {
                    return Err("Unexpected placement fields in old project".into());
                }
                reference.insert("native_size".into(), false.into());
                reference.insert("offset".into(), serde_json::json!([0.0, 0.0]));
            }
            migrated["version"] = 4.into();
        }
        if (1..=4).contains(&header.version) {
            for layer in migrated["layers"].as_array_mut().ok_or("Missing layers")? {
                let layer = layer.as_object_mut().ok_or("Invalid layer")?;
                if layer.contains_key("midra_link") {
                    return Err("Unexpected Midra link in old project".into());
                }
                layer.insert("midra_link".into(), serde_json::Value::Null);
            }
            migrated["version"] = 5.into();
        }
        if (1..=5).contains(&header.version) {
            let editor = migrated["editor"]
                .as_object_mut()
                .ok_or("Missing editor settings")?;
            if editor.contains_key("position_point") {
                return Err("Unexpected position point in old project".into());
            }
            let corner = editor
                .remove("position_corner")
                .ok_or("Missing position corner")?;
            serde_json::from_value::<Corner>(corner.clone())
                .map_err(|e| format!("Invalid position corner: {e}"))?;
            editor.insert("position_point".into(), corner);
            migrated["version"] = 6.into();
        }
        if (1..=6).contains(&header.version) {
            let editor = migrated["editor"]
                .as_object_mut()
                .ok_or("Missing editor settings")?;
            if editor.contains_key("backdrop_title") {
                return Err("Unexpected backdrop title in old project".into());
            }
            editor.insert("backdrop_title".into(), DEFAULT_BACKDROP_TITLE.into());
            migrated["version"] = 7.into();
        }
        if (1..=7).contains(&header.version) {
            let editor = migrated["editor"]
                .as_object_mut()
                .ok_or("Missing editor settings")?;
            if editor.contains_key("position_layer_point") {
                return Err("Unexpected layer position point in old project".into());
            }
            editor.insert("position_layer_point".into(), serde_json::Value::Null);
            migrated["version"] = 8.into();
        }
        if (1..=8).contains(&header.version) {
            for layer in migrated["layers"].as_array_mut().ok_or("Missing layers")? {
                let layer = layer.as_object_mut().ok_or("Invalid layer")?;
                if layer.contains_key("pivot") {
                    return Err("Unexpected layer pivot in old project".into());
                }
                layer.insert("pivot".into(), serde_json::json!([0.5, 0.5]));
            }
            migrated["version"] = 9.into();
        }
        if (1..=9).contains(&header.version) {
            let object = migrated.as_object_mut().ok_or("Invalid project")?;
            if object.contains_key("references") {
                return Err("Unexpected reference collection in old project".into());
            }
            let reference = object
                .remove("reference")
                .ok_or("Missing reference field")?;
            object.insert(
                "references".into(),
                if reference.is_null() {
                    serde_json::json!([])
                } else {
                    serde_json::json!([reference])
                },
            );
            object.insert("version".into(), 10.into());
        }
        if header.version <= 10 {
            let object = migrated.as_object_mut().ok_or("Invalid project")?;
            if object.contains_key("workspace") {
                return Err("Unexpected workspace in old project".into());
            }
            object.insert(
                "workspace".into(),
                serde_json::to_value(Workspace::default()).unwrap(),
            );
            object.insert("version".into(), 11.into());
        }
        if header.version <= 11 {
            let object = migrated.as_object_mut().ok_or("Invalid project")?;
            if object.contains_key("folders") {
                return Err("Unexpected folders in old project".into());
            }
            object.insert("folders".into(), serde_json::json!([]));
            object.insert("version".into(), 12.into());
        }
        if header.version <= 12 {
            for layer in migrated["layers"]
                .as_array_mut()
                .ok_or("Invalid project layers")?
            {
                let layer = layer.as_object_mut().ok_or("Invalid project layer")?;
                if layer.contains_key("device_link") {
                    return Err("Unexpected device_link in legacy project".into());
                }
                let mut link = layer
                    .remove("midra_link")
                    .ok_or("Missing legacy midra_link")?;
                if let Some(fields) = link.as_object_mut() {
                    if fields.len() != 2
                        || !fields.contains_key("screen")
                        || !fields.contains_key("layer")
                    {
                        return Err("Invalid legacy Midra link".into());
                    }
                    for key in ["screen", "layer"] {
                        let id = fields[key]
                            .as_str()
                            .ok_or("Invalid legacy Midra identifier")?;
                        if id.is_empty() || id.len() > 16 || !id.bytes().all(|b| b.is_ascii_digit())
                        {
                            return Err("Invalid legacy Midra identifier".into());
                        }
                        let number = id
                            .parse::<u64>()
                            .map_err(|_| "Invalid legacy Midra identifier")?;
                        if number == 0 {
                            return Err("Invalid legacy Midra identifier".into());
                        }
                        fields[key] = number.to_string().into();
                    }
                    fields.insert("family".into(), "midra4k".into());
                }
                layer.insert("device_link".into(), link);
            }
            migrated["version"] = 13.into();
        }
        if header.version <= 13 {
            for layer in migrated["layers"]
                .as_array_mut()
                .ok_or("Invalid project layers")?
            {
                let object = layer.as_object_mut().ok_or("Invalid project layer")?;
                if object.contains_key("layer_fit") {
                    return Err("Unexpected layer_fit in legacy project.".into());
                }
                object.insert("layer_fit".into(), serde_json::Value::Null);
            }
            migrated["version"] = 14.into();
        }
        if header.version <= 14 {
            for layer in migrated["layers"].as_array_mut().ok_or("Missing layers")? {
                if layer.get("mask").is_some() {
                    return Err("Unexpected mask in old project.".into());
                }
                layer["mask"] = serde_json::to_value(formats::Mask::default()).unwrap();
            }
            migrated["version"] = 15.into();
        }
        if header.version <= 15 {
            if migrated.get("input_proposals").is_some() {
                return Err("Unexpected input_proposals in legacy project.".into());
            }
            migrated["input_proposals"] = serde_json::json!([]);
            migrated["version"] = 16.into();
        }
        if header.version <= 16 {
            for layer in migrated["layers"].as_array_mut().ok_or("Missing layers")? {
                let mask = layer["mask"].as_object_mut().ok_or("Missing mask")?;
                if header.version >= 15 && mask.contains_key("origin") {
                    return Err("Unexpected mask origin in old project.".into());
                }
                mask.insert("origin".into(), "layer".into());
            }
            migrated["version"] = 17.into();
        }
        if header.version <= 17 {
            let snapping = migrated["workspace"]["snapping"]
                .as_object_mut()
                .ok_or("Missing snapping")?;
            if header.version >= 11 && snapping.contains_key("mask_edges") {
                return Err("Unexpected mask_edges in old project.".into());
            }
            snapping.insert("mask_edges".into(), false.into());
            migrated["version"] = 18.into();
        }
        if header.version <= 18 {
            for layer in migrated["layers"].as_array_mut().ok_or("Missing layers")? {
                let layer = layer.as_object_mut().ok_or("Invalid project layer")?;
                if layer.contains_key("load_mapping") || layer.contains_key("send_mapping") {
                    return Err("Unexpected directional mapping in legacy project.".into());
                }
                let link = layer.remove("device_link").ok_or("Missing device_link")?;
                let destinations = if link.is_null() {
                    serde_json::json!([])
                } else {
                    serde_json::json!([{ "connection": "current", "layer": link.clone() }])
                };
                layer.insert("load_mapping".into(), link);
                layer.insert(
                    "send_mapping".into(),
                    serde_json::json!({ "destinations": destinations }),
                );
            }
            migrated["version"] = 19.into();
        }
        if header.version <= 19 {
            if migrated["input_proposals"] != serde_json::json!([]) {
                return Err(
                    "Retired input proposals: open the project with best-effort recovery.".into(),
                );
            }
            migrated
                .as_object_mut()
                .ok_or("Invalid project")?
                .remove("input_proposals");
            for input in migrated["inputs"].as_array_mut().ok_or("Missing inputs")? {
                if let Some(crop) = input.get("input_crop") {
                    let crop: Crop =
                        serde_json::from_value(crop.clone()).map_err(|e| e.to_string())?;
                    if crop != Crop::default() {
                        return Err(
                            "Retired local input crop: open the project with best-effort recovery."
                                .into(),
                        );
                    }
                }
                input
                    .as_object_mut()
                    .ok_or("Invalid input")?
                    .remove("input_crop");
            }
            migrated["version"] = 20.into();
        }
        if header.version <= 20 {
            let keep = migrated["editor"]
                .as_object_mut()
                .ok_or("Missing editor settings")?
                .remove("keep_aspect_ratio")
                .ok_or("Missing aspect-ratio setting")?;
            let keep: bool = serde_json::from_value(keep)
                .map_err(|_| "Invalid aspect-ratio setting".to_string())?;
            for layer in migrated["layers"].as_array_mut().ok_or("Missing layers")? {
                let layer = layer.as_object_mut().ok_or("Invalid project layer")?;
                if layer.contains_key("keep_aspect_ratio") {
                    return Err("Unexpected per-layer aspect-ratio setting in old project.".into());
                }
                layer.insert("keep_aspect_ratio".into(), keep.into());
            }
            migrated["version"] = 21.into();
        }
        let p: Self =
            serde_json::from_value(migrated).map_err(|e| format!("Invalid project JSON: {e}"))?;
        p.layout()?;
        Ok(p)
    }
    pub fn read(path: &Path) -> Result<Self, String> {
        let mut text = String::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(MAX_PROJECT_BYTES + 1)
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        Self::from_json(&text)
    }
    /// Geometry exchange only: pixel-rounded rectangles, no device commands or hidden metadata.
    pub fn to_svg(&self, visible_only: bool) -> Result<String, String> {
        self.svg_filtered(visible_only, None)
    }
    /// Export exactly the chosen stable IDs, preserving document stacking order.
    pub fn to_svg_ids(&self, ids: &[usize]) -> Result<String, String> {
        if ids.is_empty()
            || ids.iter().enumerate().any(|(i, id)| {
                ids[..i].contains(id) || !self.layers.iter().any(|layer| layer.id == *id)
            })
        {
            return Err("Choose distinct existing layer IDs to export.".into());
        }
        self.svg_filtered(false, Some(ids))
    }
    fn svg_filtered(&self, visible_only: bool, ids: Option<&[usize]>) -> Result<String, String> {
        self.layout()?;
        if visible_only && self.layers.iter().all(|l| !l.visible) {
            return Err("No visible layers to export.".into());
        }
        let w = self.output_resolution.width;
        let h = self.output_resolution.height;
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" data-midra-layout-geometry=\"1\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n<title>Canvas Pilot geometry</title>\n"
        );
        for layer in &self.layers {
            if (visible_only && !layer.visible) || ids.is_some_and(|ids| !ids.contains(&layer.id)) {
                continue;
            }
            let g = layer.bounds;
            if g.width.round() < 1.0 || g.height.round() < 1.0 {
                return Err("A layer rounds to zero pixels; enlarge it before SVG export.".into());
            }
            svg.push_str(&format!("<rect id=\"layer-{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#173b50\"/>\n",layer.id,crate::export_pixels(g.x),crate::export_pixels(g.y),crate::export_pixels(g.width),crate::export_pixels(g.height)));
        }
        svg.push_str("</svg>\n");
        Ok(svg)
    }
}
/// Same-directory temporary write and atomic publication. Existing files require explicit consent.
pub fn write_file(path: &Path, contents: &[u8], overwrite: bool) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(contents)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    if overwrite {
        file.persist(path).map_err(|e| e.to_string())?;
    } else {
        file.persist_noclobber(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
