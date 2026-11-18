//! Layer geometry, reversible edits and bounded imports.
mod device_mapping;
pub use device_mapping::{ConnectionTarget, SendDestination, SendMapping};
pub mod clipboard;
pub mod device;
pub mod device_capture;
pub mod device_send;
use std::collections::HashMap;
pub mod exchange;
pub mod exchange_metadata;
mod folders;
pub mod formats;
mod geometry_formats;
pub mod import;
pub mod input_proposal;
pub mod interchange;
pub mod project;
pub mod reference_document;
pub mod size_presets;
pub mod snapping;
pub mod software;
#[cfg(all(feature = "vendor_formats", not(feature = "no_proprietary")))]
mod vendor_formats;
use formats::{Formats, Framing, Source};

/// Presentation only: retain full precision in the document.
pub fn view_pixels(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    format!("{:.1}", if rounded == 0.0 { 0.0 } else { rounded })
}
pub fn export_pixels(value: f64) -> String {
    let rounded = value.round();
    format!("{:.0}", if rounded == 0.0 { 0.0 } else { rounded })
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResizeHandle {
    Corner(Corner),
    Top,
    Bottom,
    Left,
    Right,
}
/// How a handle drag affects selected layers.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResizeMode {
    #[default]
    Individual,
    Same,
    Mirror,
    Opposite,
}
impl ResizeHandle {
    pub fn mirrored_horizontally(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top | Self::Bottom => self,
            Self::Corner(corner) => Self::Corner(match corner {
                Corner::TopLeft => Corner::TopRight,
                Corner::TopRight => Corner::TopLeft,
                Corner::BottomLeft => Corner::BottomRight,
                Corner::BottomRight => Corner::BottomLeft,
            }),
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Corner(corner) => Self::Corner(match corner {
                Corner::TopLeft => Corner::BottomRight,
                Corner::TopRight => Corner::BottomLeft,
                Corner::BottomLeft => Corner::TopRight,
                Corner::BottomRight => Corner::TopLeft,
            }),
        }
    }

    pub fn positions(g: Geometry) -> [(Self, [f64; 2]); 8] {
        let [cx, cy] = g.center();
        [
            (Self::Corner(Corner::TopLeft), [g.x, g.y]),
            (Self::Corner(Corner::TopRight), [g.x + g.width, g.y]),
            (Self::Corner(Corner::BottomLeft), [g.x, g.y + g.height]),
            (
                Self::Corner(Corner::BottomRight),
                [g.x + g.width, g.y + g.height],
            ),
            (Self::Top, [cx, g.y]),
            (Self::Bottom, [cx, g.y + g.height]),
            (Self::Left, [g.x, cy]),
            (Self::Right, [g.x + g.width, cy]),
        ]
    }
}
fn resized_geometry(
    original: Geometry,
    handle: ResizeHandle,
    delta: [f64; 2],
    keep_ratio: bool,
) -> Geometry {
    resized_geometry_exact(original, handle, delta.map(f64::round), keep_ratio)
}
fn resized_geometry_exact(
    original: Geometry,
    handle: ResizeHandle,
    delta: [f64; 2],
    keep_ratio: bool,
) -> Geometry {
    let left = matches!(
        handle,
        ResizeHandle::Corner(Corner::TopLeft | Corner::BottomLeft) | ResizeHandle::Left
    );
    let top = matches!(
        handle,
        ResizeHandle::Corner(Corner::TopLeft | Corner::TopRight) | ResizeHandle::Top
    );
    let dx = delta[0] * if left { -1.0 } else { 1.0 };
    let dy = delta[1] * if top { -1.0 } else { 1.0 };
    let side = !matches!(handle, ResizeHandle::Corner(_));
    let horizontal = matches!(handle, ResizeHandle::Left | ResizeHandle::Right);
    let (width, height) = if side {
        if horizontal {
            let width = (original.width + dx).max(if keep_ratio {
                (original.width / original.height).max(1.0)
            } else {
                1.0
            });
            (
                width,
                if keep_ratio {
                    width * original.height / original.width
                } else {
                    original.height
                },
            )
        } else {
            let height = (original.height + dy).max(if keep_ratio {
                (original.height / original.width).max(1.0)
            } else {
                1.0
            });
            (
                if keep_ratio {
                    height * original.width / original.height
                } else {
                    original.width
                },
                height,
            )
        }
    } else if keep_ratio {
        let factor = ((original.width * (original.width + dx)
            + original.height * (original.height + dy))
            / (original.width * original.width + original.height * original.height))
            .max((1.0 / original.width).max(1.0 / original.height));
        (original.width * factor, original.height * factor)
    } else {
        (
            (original.width + dx).max(1.0),
            (original.height + dy).max(1.0),
        )
    };
    anchored_resize(original, handle, width, height)
}
fn anchored_resize(original: Geometry, handle: ResizeHandle, width: f64, height: f64) -> Geometry {
    let side = !matches!(handle, ResizeHandle::Corner(_));
    let horizontal = matches!(handle, ResizeHandle::Left | ResizeHandle::Right);
    let left = matches!(
        handle,
        ResizeHandle::Left | ResizeHandle::Corner(Corner::TopLeft | Corner::BottomLeft)
    );
    let top = matches!(
        handle,
        ResizeHandle::Top | ResizeHandle::Corner(Corner::TopLeft | Corner::TopRight)
    );
    Geometry {
        x: if side && !horizontal {
            original.center()[0] - width / 2.0
        } else if left {
            original.x + original.width - width
        } else {
            original.x
        },
        y: if side && horizontal {
            original.center()[1] - height / 2.0
        } else if top {
            original.y + original.height - height
        } else {
            original.y
        },
        width,
        height,
    }
}

/// Visible layer bounds, in output screen pixels. No crop or camera transform.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Geometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Geometry {
    pub fn validate(self) -> Result<Self, String> {
        if [self.x, self.y, self.width, self.height]
            .iter()
            .any(|n| !n.is_finite() || n.abs() > 1_000_000.0)
            || self.width <= 0.0
            || self.height <= 0.0
        {
            return Err("Use finite coordinates within ±1,000,000 px and positive sizes up to 1,000,000 px.".into());
        }
        Ok(self)
    }
    pub fn center(self) -> [f64; 2] {
        [self.x + self.width / 2.0, self.y + self.height / 2.0]
    }
    pub fn contains(self, p: [f64; 2]) -> bool {
        p[0] >= self.x
            && p[0] <= self.x + self.width
            && p[1] >= self.y
            && p[1] <= self.y + self.height
    }
    pub fn outside(self) -> bool {
        self.outside_on([1920, 1080])
    }
    pub fn outside_on(self, output: [u32; 2]) -> bool {
        self.x < 0.0
            || self.y < 0.0
            || self.x + self.width > output[0] as f64
            || self.y + self.height > output[1] as f64
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
impl Corner {
    pub fn offsets(self, g: Geometry) -> [f64; 2] {
        self.offsets_on(g, [1920, 1080])
    }
    pub fn offsets_on(self, g: Geometry, output: [u32; 2]) -> [f64; 2] {
        [
            if matches!(self, Self::TopRight | Self::BottomRight) {
                output[0] as f64 - g.x - g.width
            } else {
                g.x
            },
            if matches!(self, Self::BottomLeft | Self::BottomRight) {
                output[1] as f64 - g.y - g.height
            } else {
                g.y
            },
        ]
    }
    pub fn geometry(self, dx: f64, dy: f64, width: f64, height: f64) -> Result<Geometry, String> {
        self.geometry_on(dx, dy, width, height, [1920, 1080])
    }
    pub fn geometry_on(
        self,
        dx: f64,
        dy: f64,
        width: f64,
        height: f64,
        output: [u32; 2],
    ) -> Result<Geometry, String> {
        Geometry {
            x: if matches!(self, Self::TopRight | Self::BottomRight) {
                output[0] as f64 - dx - width
            } else {
                dx
            },
            y: if matches!(self, Self::BottomLeft | Self::BottomRight) {
                output[1] as f64 - dy - height
            } else {
                dy
            },
            width,
            height,
        }
        .validate()
    }
}

/// Matching screen/layer point used for numeric positioning.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionPoint {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}
impl PositionPoint {
    fn corner(self) -> Option<Corner> {
        match self {
            Self::TopLeft => Some(Corner::TopLeft),
            Self::TopRight => Some(Corner::TopRight),
            Self::BottomLeft => Some(Corner::BottomLeft),
            Self::BottomRight => Some(Corner::BottomRight),
            Self::Center => None,
        }
    }
    pub fn offsets_on(self, g: Geometry, output: [u32; 2]) -> [f64; 2] {
        if let Some(corner) = self.corner() {
            corner.offsets_on(g, output)
        } else {
            let center = g.center();
            [
                center[0] - output[0] as f64 / 2.0,
                center[1] - output[1] as f64 / 2.0,
            ]
        }
    }
    pub fn geometry_on(
        self,
        dx: f64,
        dy: f64,
        width: f64,
        height: f64,
        output: [u32; 2],
    ) -> Result<Geometry, String> {
        if let Some(corner) = self.corner() {
            corner.geometry_on(dx, dy, width, height, output)
        } else {
            Geometry {
                x: output[0] as f64 / 2.0 + dx - width / 2.0,
                y: output[1] as f64 / 2.0 + dy - height / 2.0,
                width,
                height,
            }
            .validate()
        }
    }
}

/// Independent screen origin and layer endpoint for numeric positioning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionReference {
    pub screen: PositionPoint,
    pub layer: PositionPoint,
}
impl From<PositionPoint> for PositionReference {
    fn from(point: PositionPoint) -> Self {
        Self {
            screen: point,
            layer: point,
        }
    }
}
impl PositionPoint {
    fn fractions(self) -> [f64; 2] {
        match self {
            Self::TopLeft => [0.0, 0.0],
            Self::TopRight => [1.0, 0.0],
            Self::BottomLeft => [0.0, 1.0],
            Self::BottomRight => [1.0, 1.0],
            Self::Center => [0.5, 0.5],
        }
    }
    pub fn point_on(self, g: Geometry) -> [f64; 2] {
        let f = self.fractions();
        [g.x + f[0] * g.width, g.y + f[1] * g.height]
    }
}
impl PositionReference {
    pub fn screen_point(self, output: [u32; 2]) -> [f64; 2] {
        let f = self.screen.fractions();
        [f[0] * output[0] as f64, f[1] * output[1] as f64]
    }
    fn signs(self) -> [f64; 2] {
        self.screen
            .fractions()
            .map(|f| if f == 1.0 { -1.0 } else { 1.0 })
    }
    pub fn offsets_on(self, g: Geometry, output: [u32; 2]) -> [f64; 2] {
        if self.screen == self.layer {
            return self.screen.offsets_on(g, output);
        }
        let from = self.screen_point(output);
        let to = self.layer.point_on(g);
        let signs = self.signs();
        [(to[0] - from[0]) * signs[0], (to[1] - from[1]) * signs[1]]
    }
    pub fn geometry_on(
        self,
        dx: f64,
        dy: f64,
        width: f64,
        height: f64,
        output: [u32; 2],
    ) -> Result<Geometry, String> {
        if self.screen == self.layer {
            return self.screen.geometry_on(dx, dy, width, height, output);
        }
        let from = self.screen_point(output);
        let f = self.layer.fractions();
        let signs = self.signs();
        Geometry {
            x: from[0] + dx * signs[0] - f[0] * width,
            y: from[1] + dy * signs[1] - f[1] * height,
            width,
            height,
        }
        .validate()
    }
}

/// Edge or center line to align, without resizing layers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alignment {
    Left,
    Right,
    Top,
    Bottom,
    HorizontalCenter,
    VerticalCenter,
}
/// Explicit alignment reference, independent of selection order.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AlignTarget {
    #[default]
    Selection,
    Screen,
    Layer(usize),
}

/// Persistent identity independent of list order or device layer numbering.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceLink {
    pub family: device::Family,
    pub screen: String,
    pub layer: String,
}
impl DeviceLink {
    pub fn validate(&self) -> Result<(), String> {
        let decimal = |id: &str| {
            !id.is_empty()
                && id.len() <= 16
                && id.bytes().all(|b| b.is_ascii_digit())
                && !id.starts_with('0')
                && id.parse::<u64>().is_ok_and(|n| n > 0)
        };
        let screen = match self.family {
            device::Family::Midra4k => self.screen.as_str(),
            device::Family::Aquilon => self
                .screen
                .strip_prefix('S')
                .or_else(|| self.screen.strip_prefix('A'))
                .ok_or("Aquilon screen IDs must start with S or A.")?,
        };
        if !decimal(screen) || !decimal(&self.layer) {
            return Err("Device screen and layer IDs must use canonical positive decimal identifiers (S/A prefix for Aquilon screens).".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerInfo {
    pub pivot: [f64; 2],
    pub load_mapping: Option<DeviceLink>,
    pub send_mapping: SendMapping,
    pub id: usize,
    pub name: String,
    pub folder: String,
}
pub(crate) fn validate_pivot(pivot: [f64; 2]) -> Result<(), String> {
    if pivot.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
        return Err(
            "Layer pivot coordinates must be finite relative values within ±1,000,000.".into(),
        );
    }
    Ok(())
}
/// Multi-layer document with one undo record per complete gesture/edit.
#[derive(Clone)]
pub struct Layout {
    folders: Vec<String>,
    info: Vec<LayerInfo>,
    next_id: usize,
    layers: Vec<Geometry>,
    selected: Vec<bool>,
    locked: Vec<bool>,
    visible: Vec<bool>,
    formats: Formats,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    drag: Option<(Vec<bool>, Snapshot, [f64; 2])>,
    content_drag: Option<(usize, Snapshot, [f64; 2])>,
    content_resize: Option<Resize>,
    mask_drag: Option<(usize, Snapshot, [f64; 2], Option<ResizeHandle>)>,
    pub gesture_error: Option<String>,
    resize: Option<Resize>,
}
#[derive(Clone)]
struct Resize {
    pivot: Option<[f64; 2]>,
    use_pivot: bool,
    mode: ResizeMode,
    targets: Vec<bool>,
    index: usize,
    handle: ResizeHandle,
    before: Snapshot,
    start: [f64; 2],
    keep_ratio: bool,
}
impl Resize {
    fn handle_for(&self, i: usize) -> ResizeHandle {
        if i == self.index {
            return self.handle;
        }
        match self.mode {
            ResizeMode::Mirror => self.handle.mirrored_horizontally(),
            ResizeMode::Opposite => self.handle.opposite(),
            _ => self.handle,
        }
    }
    fn anchor(&self, i: usize) -> [f64; 2] {
        let g = self.before.layers[i];
        if self.use_pivot {
            let mut relative = self.pivot.unwrap_or(self.before.info[i].pivot);
            if self.pivot.is_some() && i != self.index {
                if matches!(self.mode, ResizeMode::Mirror | ResizeMode::Opposite) {
                    relative[0] = 1.0 - relative[0];
                }
                if self.mode == ResizeMode::Opposite {
                    relative[1] = 1.0 - relative[1];
                }
            }
            [g.x + relative[0] * g.width, g.y + relative[1] * g.height]
        } else {
            ResizeHandle::positions(g)
                .into_iter()
                .find(|(h, _)| *h == self.handle_for(i).opposite())
                .unwrap()
                .1
        }
    }
}
#[derive(Clone, PartialEq)]
struct Snapshot {
    folders: Vec<String>,
    info: Vec<LayerInfo>,
    selected: Vec<bool>,
    locked: Vec<bool>,
    visible: Vec<bool>,
    layers: Vec<Geometry>,
    formats: Formats,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            folders: Vec::new(),
            info: vec![
                LayerInfo {
                    pivot: [0.5; 2],
                    load_mapping: None,
                    send_mapping: Default::default(),
                    id: 2,
                    name: "Slides".into(),
                    folder: String::new(),
                },
                LayerInfo {
                    pivot: [0.5; 2],
                    load_mapping: None,
                    send_mapping: Default::default(),
                    id: 1,
                    name: "Presenter".into(),
                    folder: String::new(),
                },
            ],
            next_id: 3,
            layers: vec![
                Geometry {
                    x: 120.0,
                    y: 180.0,
                    width: 640.0,
                    height: 360.0,
                },
                Geometry {
                    x: 1160.0,
                    y: 180.0,
                    width: 640.0,
                    height: 360.0,
                },
            ],
            selected: vec![false, true],
            locked: vec![false; 2],
            visible: vec![true; 2],
            formats: Formats::default(),
            content_drag: None,
            content_resize: None,
            mask_drag: None,
            gesture_error: None,
            undo: Vec::new(),
            redo: Vec::new(),
            drag: None,
            resize: None,
        }
    }
}
impl Layout {
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            folders: self.folders.clone(),
            info: self.info.clone(),
            selected: self.selected.clone(),
            locked: self.locked.clone(),
            visible: self.visible.clone(),
            layers: self.layers.clone(),
            formats: self.formats.clone(),
        }
    }
    fn restore(&mut self, s: Snapshot) {
        self.folders = s.folders.clone();
        self.gesture_error = None;
        // Selection remains independent of history for surviving IDs.
        let mut selected = s.selected;
        let locked = s.locked;
        let visible = s.visible;
        for (i, info) in s.info.iter().enumerate() {
            if let Some(j) = self.info.iter().position(|old| old.id == info.id) {
                selected[i] = self.selected[j];
                selected[i] &= visible[i] && !locked[i];
            }
        }
        self.selected = selected;
        self.locked = locked;
        self.visible = visible;
        self.info = s.info;
        self.layers = s.layers;
        self.formats = s.formats;
    }
    /// Current UI contract: change both directions together, with one send target.
    pub fn set_device_link(&mut self, i: usize, link: Option<DeviceLink>) -> Result<(), String> {
        let send = SendMapping::linked(link.clone());
        self.set_device_mappings(i, link, send)
    }

    /// Independent routing model; the current UI only uses `set_device_link`.
    pub fn set_device_mappings(
        &mut self,
        i: usize,
        load: Option<DeviceLink>,
        send: SendMapping,
    ) -> Result<(), String> {
        if i >= self.info.len() {
            return Err("Layer no longer exists.".into());
        }
        if let Some(link) = &load {
            link.validate()?;
        }
        send.validate()?;
        if self.info[i].load_mapping == load && self.info[i].send_mapping == send {
            return Ok(());
        }
        self.finish_drag();
        let before = self.snapshot();
        self.info[i].load_mapping = load;
        self.info[i].send_mapping = send;
        self.remember(before);
        Ok(())
    }
    /// Commit per-layer relative pivots as one undoable edit, without changing geometry.
    pub fn set_layer_pivots(&mut self, pivots: &[(usize, [f64; 2])]) -> Result<(), String> {
        for (i, pivot) in pivots {
            if !self.selectable(*i) {
                return Err("Unlock and show this layer first.".into());
            }
            validate_pivot(*pivot)?;
        }
        self.finish_drag();
        let before = self.snapshot();
        for (i, pivot) in pivots {
            self.info[*i].pivot = *pivot;
        }
        self.remember(before);
        Ok(())
    }
    pub fn layer_info(&self, i: usize) -> &LayerInfo {
        &self.info[i]
    }
    pub fn layer_label(&self, i: usize) -> String {
        format!("{:02} / {}", self.info[i].id, self.info[i].name)
    }
    pub fn rename_layer(&mut self, i: usize, name: &str, folder: &str) -> Result<(), String> {
        if i >= self.layers.len()
            || name.trim().is_empty()
            || name.chars().count() > 128
            || folder.chars().count() > 128
        {
            return Err(
                "Use a layer name of 1–128 characters and a folder of at most 128 characters."
                    .into(),
            );
        }
        self.finish_drag();
        let before = self.snapshot();
        self.info[i].name = name.trim().into();
        self.info[i].folder = folder.trim().into();
        self.remember(before);
        Ok(())
    }
    pub fn rename_layers(&mut self, edits: &[(usize, String, String)]) -> Result<(), String> {
        if edits.len() != self.layers.len()
            || edits.iter().enumerate().any(|(i, (id, name, folder))| {
                *id != self.info[i].id
                    || name.trim().is_empty()
                    || name.chars().count() > 128
                    || folder.chars().count() > 128
            })
        {
            return Err("Layer list changed or names are invalid; use 1–128 characters.".into());
        }
        self.finish_drag();
        let before = self.snapshot();
        for (info, (_, name, folder)) in self.info.iter_mut().zip(edits) {
            info.name = name.trim().into();
            info.folder = folder.trim().into();
        }
        self.remember(before);
        Ok(())
    }
    pub fn add_layer(&mut self) -> Result<usize, String> {
        if self.layers.len() >= 256
            || self.formats.sources.len() >= 1024
            || self.next_id == usize::MAX
        {
            return Err("Layer/source capacity reached.".into());
        }
        self.finish_drag();
        let before = self.snapshot();
        let i = self.layers.len();
        let g = Geometry {
            x: 0.0,
            y: 0.0,
            width: 640.0,
            height: 360.0,
        };
        self.layers.push(g);
        self.info.push(LayerInfo {
            pivot: [0.5; 2],
            load_mapping: None,
            send_mapping: Default::default(),
            id: self.next_id,
            name: format!("Layer {}", self.next_id),
            folder: String::new(),
        });
        self.next_id += 1;
        self.selected.push(false);
        self.locked.push(false);
        self.visible.push(true);
        self.formats.source_for.push(self.formats.sources.len());
        self.formats.sources.push(Source::default());
        self.formats.advanced.push(formats::Crop::default());
        self.formats.layer_fit.push(None);
        self.formats.masks.push(Default::default());
        self.select(Some(i));
        self.remember(before);
        Ok(i)
    }
    /// Move a stable layer ID to a zero-based list position, as one undoable edit.
    /// Geometry, input assignments and device links travel with the layer.
    pub fn move_layer(&mut self, id: usize, to: usize) -> Result<(), String> {
        let from = self
            .info
            .iter()
            .position(|info| info.id == id)
            .ok_or("Layer no longer exists.")?;
        if to >= self.layers.len() {
            return Err("Layer position is outside the list.".into());
        }
        if from == to {
            return Ok(());
        }
        self.finish_drag();
        let before = self.snapshot();
        fn move_item<T>(items: &mut [T], from: usize, to: usize) {
            if from < to {
                items[from..=to].rotate_left(1);
            } else {
                items[to..=from].rotate_right(1);
            }
        }
        move_item(&mut self.info, from, to);
        move_item(&mut self.layers, from, to);
        move_item(&mut self.selected, from, to);
        move_item(&mut self.locked, from, to);
        move_item(&mut self.visible, from, to);
        move_item(&mut self.formats.source_for, from, to);
        move_item(&mut self.formats.advanced, from, to);
        move_item(&mut self.formats.layer_fit, from, to);
        move_item(&mut self.formats.masks, from, to);
        self.remember(before);
        Ok(())
    }
    pub fn remove_selected(&mut self) -> Result<(), String> {
        let indices = self.selected_indices();
        if indices.is_empty() || indices.len() == self.layers.len() {
            return Err("Select layers to remove, leaving at least one layer.".into());
        }
        self.finish_drag();
        let before = self.snapshot();
        for i in indices.into_iter().rev() {
            self.layers.remove(i);
            self.info.remove(i);
            self.selected.remove(i);
            self.locked.remove(i);
            self.visible.remove(i);
            self.formats.source_for.remove(i);
            self.formats.advanced.remove(i);
            self.formats.layer_fit.remove(i);
            self.formats.masks.remove(i);
        }
        self.remember(before);
        Ok(())
    }
    pub fn formats(&self) -> &Formats {
        &self.formats
    }
    pub fn set_output(&mut self, output: [u32; 2]) -> Result<(), String> {
        formats::validate_resolution(output)?;
        self.finish_drag();
        let before = self.snapshot();
        self.formats.output = output;
        self.remember(before);
        Ok(())
    }
    /// Change explicit container fitting and crop together as one undoable edit.
    /// Leaving advanced fitting returns to the normal minimum cover crop.
    pub fn set_layer_fit(
        &mut self,
        i: usize,
        mode: Option<formats::LayerFit>,
        crop: formats::Crop,
    ) -> Result<(), String> {
        if !self.selectable(i) {
            return Err("Unlock and show this layer first.".into());
        }
        let g = self.layers[i];
        let input = self.formats.input(i)?;
        let crop = if mode.is_none() {
            formats::crop_from_framing(input, [g.width, g.height], Framing::default())?
        } else {
            crop.remaining(input)?;
            crop
        };
        self.finish_drag();
        let before = self.snapshot();
        self.formats.layer_fit[i] = mode;
        self.formats.advanced[i] = crop;
        self.remember(before);
        Ok(())
    }
    pub fn set_mask(&mut self, i: usize, mask: formats::Mask) -> Result<(), String> {
        if !self.selectable(i) {
            return Err("Unlock and show this layer first.".into());
        }
        mask.validate()?;
        self.finish_drag();
        let before = self.snapshot();
        self.formats.masks[i] = mask;
        self.remember(before);
        Ok(())
    }
    /// Align the reveal window within its container, preserving image and window size.
    pub fn align_mask(&mut self, i: usize, axes: &[Alignment]) -> Result<(), String> {
        if !self.selectable(i) {
            return Err("Unlock and show this layer first.".into());
        }
        let mut mask = self.formats.masks[i];
        for axis in axes {
            let horizontal = mask.left + mask.right;
            let vertical = mask.top + mask.bottom;
            match axis {
                Alignment::Left => {
                    mask.left = 0.0;
                    mask.right = horizontal;
                }
                Alignment::Right => {
                    mask.left = horizontal;
                    mask.right = 0.0;
                }
                Alignment::Top => {
                    mask.top = 0.0;
                    mask.bottom = vertical;
                }
                Alignment::Bottom => {
                    mask.top = vertical;
                    mask.bottom = 0.0;
                }
                Alignment::HorizontalCenter => {
                    mask.left = horizontal / 2.0;
                    mask.right = horizontal / 2.0;
                }
                Alignment::VerticalCenter => {
                    mask.top = vertical / 2.0;
                    mask.bottom = vertical / 2.0;
                }
            }
        }
        self.set_mask(i, mask)
    }
    pub fn start_mask_drag(&mut self, i: usize, point: [f64; 2], handle: Option<ResizeHandle>) {
        self.finish_drag();
        if self.selectable(i) && self.selected() == Some(i) && point.iter().all(|v| v.is_finite()) {
            self.mask_drag = Some((i, self.snapshot(), point, handle));
        }
    }
    pub fn set_framing(&mut self, i: usize, framing: Framing) -> Result<(), String> {
        if !self.selectable(i) {
            return Err("Unlock and show this layer first.".into());
        }
        if self.formats.layer_fit[i].is_some() {
            return Err("Use the crop edges in Layer properties for explicit layer fit.".into());
        }
        let crop = formats::crop_from_framing(
            self.formats.input(i)?,
            [self.layers[i].width, self.layers[i].height],
            framing,
        )?;
        self.finish_drag();
        let before = self.snapshot();
        self.formats.advanced[i] = crop;
        self.remember(before);
        Ok(())
    }
    /// Align the zoomed source content to its visible box without moving/resizing the box.
    pub fn align_content(&mut self, i: usize, axes: &[Alignment]) -> Result<(), String> {
        if !self.selectable(i) {
            return Err("Unlock and show this layer first.".into());
        }
        let mapping = self.formats.mapping(i, self.layers[i])?;
        let mut framing = mapping.framing;
        for axis in axes {
            match axis {
                Alignment::Left => framing.offset[0] = mapping.offset_limits[0],
                Alignment::Right => framing.offset[0] = -mapping.offset_limits[0],
                Alignment::Top => framing.offset[1] = mapping.offset_limits[1],
                Alignment::Bottom => framing.offset[1] = -mapping.offset_limits[1],
                Alignment::HorizontalCenter => framing.offset[0] = 0.0,
                Alignment::VerticalCenter => framing.offset[1] = 0.0,
            }
        }
        self.set_framing(i, framing)
    }
    pub fn start_content_drag(&mut self, i: usize, point: [f64; 2]) {
        if self
            .formats
            .layer_fit
            .get(i)
            .is_some_and(|fit| fit.is_some_and(|fit| fit != formats::LayerFit::OneToOne))
        {
            return;
        }
        self.finish_drag();
        if self.selectable(i) && self.selected() == Some(i) && point.iter().all(|v| v.is_finite()) {
            self.content_drag = Some((i, self.snapshot(), point));
        }
    }
    /// Resize content in normal crop mode, or trim source pixels in centered 1:1 mode.
    /// The layer container stays fixed; 1:1 ignores proportional scaling and pivots.
    pub fn start_content_resize(&mut self, index: usize, handle: ResizeHandle, point: [f64; 2]) {
        if self
            .formats
            .layer_fit
            .get(index)
            .is_some_and(|mode| mode.is_some_and(|fit| fit != formats::LayerFit::OneToOne))
        {
            return;
        }
        self.finish_drag();
        if !self.selectable(index)
            || self.selected() != Some(index)
            || point.iter().any(|v| !v.is_finite())
        {
            return;
        }
        self.content_resize = Some(Resize {
            pivot: None,
            use_pivot: false,
            mode: ResizeMode::Individual,
            targets: self.selected.clone(),
            index,
            handle,
            before: self.snapshot(),
            start: point,
            keep_ratio: true,
        });
    }

    /// Atomic shared-source edit plus visible-box resizing. Explicit acceptance is required for any center adjustment.
    pub fn configure(
        &mut self,
        i: usize,
        source_id: usize,
        source: Source,
        geometry: Geometry,
        allow_adjustment: bool,
    ) -> Result<String, String> {
        if !self.selectable(i) || source_id >= 1024 {
            return Err("Select an editable layer and an input between 1 and 1024.".into());
        }
        source.input()?;
        geometry.validate()?;
        self.finish_drag();
        let before = self.snapshot();
        let mut next = before.clone();
        // Input identities are independent of layers and created atomically on Apply.
        next.formats.sources.resize(
            next.formats.sources.len().max(source_id + 1),
            Source::default(),
        );
        next.formats.source_for[i] = source_id;
        next.formats.sources[source_id] = source;
        next.layers[i] = geometry;
        let mut adjustments = Vec::new();
        for j in 0..self.layers.len() {
            let affected = j == i
                || (next.formats.source_for[j] == source_id
                    && before.formats.sources.get(source_id) != Some(&source));
            if !affected {
                continue;
            }
            if self.locked(j) {
                return Err(format!(
                    "Source edit also affects locked layer {}. Unlock it first.",
                    j + 1
                ));
            }
            if next.formats.layer_fit[j].is_some() {
                next.formats.advanced[j].remaining(next.formats.input(j)?)?;
                continue;
            }
            let (crop, adjusted) = formats::reframed(
                before.formats.input(j)?,
                [before.layers[j].width, before.layers[j].height],
                before.formats.advanced[j],
                next.formats.input(j)?,
                [next.layers[j].width, next.layers[j].height],
            )?;
            next.formats.advanced[j] = crop;
            if adjusted {
                let m = next.formats.mapping(j, next.layers[j])?;
                adjustments.push(format!(
                    "Layer {}: content offsets adjusted to {:.1}, {:.1} px",
                    j + 1,
                    m.framing.offset[0],
                    m.framing.offset[1]
                ));
            }
        }
        let message = adjustments.join("; ");
        if !allow_adjustment && !adjustments.is_empty() {
            return Err(format!("Review required: {message}"));
        }
        self.restore(next);
        self.remember(before);
        Ok(message)
    }
    /// Provisional geometry report; deliberately not a verified WebRCS command recipe.
    pub fn calculation_report(&self, i: usize) -> Result<String, String> {
        self.formatted_report(i, false)
    }
    pub fn calculation_export(&self, i: usize) -> Result<String, String> {
        self.formatted_report(i, true)
    }
    fn formatted_report(&self, i: usize, export: bool) -> Result<String, String> {
        let p = if export { export_pixels } else { view_pixels };
        let f = &self.formats;
        let g = self.layers[i];
        let source = f.sources[f.source_for[i]];
        if let Some(mode) = f.layer_fit[i] {
            let image = mode.image(f.advanced[i].remaining(source.input()?)?, g)?;
            return Ok(format!(
                "Explicit layer fit: {}\nContainer: {} × {}\nImage before clipping: {} × {} at {}, {}\nCrop is applied before fit; image is clipped to the container. Device subpixel rasterization is unverified.",
                mode.label(),
                p(g.width),
                p(g.height),
                p(image.width),
                p(image.height),
                p(image.x),
                p(image.y)
            ));
        }
        let input = source.input()?;
        let m = f.mapping(i, g)?;
        let a = f.advanced[i];
        let c = source.input_crop;
        let projected = Geometry {
            x: g.x - m.scale * a.left,
            y: g.y - m.scale * a.top,
            width: m.scale * input[0],
            height: m.scale * input[1],
        };
        Ok(format!(
            "PROVISIONAL GEOMETRY — not a verified device recipe\nSource {} / {} / {} x {}\nInput crop L/R/T/B (native px): {} / {} / {} / {}\nInput resolution: {} x {}\nLayer resolution: {} x {}; visible center: {}, {}\nAdvanced crop L/R/T/B (usable-input px): {} / {} / {} / {}\nCombined native retained region: x {}, y {}, w {}, h {}\nContent zoom: {}%; offsets (output px): {}, {}\nProjected full usable input: {} x {}; center {}, {}\nIntended aspect mode: Centered. Device container, crop units and quantization remain unverified.",
            f.source_for[i] + 1,
            source.aspect.label(),
            source.native[0],
            source.native[1],
            p(c.left),
            p(c.right),
            p(c.top),
            p(c.bottom),
            p(input[0]),
            p(input[1]),
            p(g.width),
            p(g.height),
            p(g.center()[0]),
            p(g.center()[1]),
            p(a.left),
            p(a.right),
            p(a.top),
            p(a.bottom),
            p(c.left + a.left),
            p(c.top + a.top),
            p(m.retained[0]),
            p(m.retained[1]),
            view_pixels(m.framing.zoom * 100.0),
            p(m.framing.offset[0]),
            p(m.framing.offset[1]),
            p(projected.width),
            p(projected.height),
            p(projected.center()[0]),
            p(projected.center()[1])
        ))
    }
    pub fn layers(&self) -> &Vec<Geometry> {
        &self.layers
    }
    pub fn locked(&self, index: usize) -> bool {
        self.locked.get(index).copied().unwrap_or(false)
    }
    pub fn visible(&self, index: usize) -> bool {
        self.visible.get(index).copied().unwrap_or(false)
    }
    fn selectable(&self, index: usize) -> bool {
        self.visible(index) && !self.locked(index)
    }
    /// Protection flags are local, undoable edits; cancel any active gesture first.
    pub fn set_locked(&mut self, index: usize, locked: bool) {
        self.cancel_drag();
        let before = self.snapshot();
        if let Some(value) = self.locked.get_mut(index) {
            *value = locked;
            if locked {
                self.selected[index] = false;
            }
        }
        self.remember(before);
    }
    pub fn set_visible(&mut self, index: usize, visible: bool) {
        self.cancel_drag();
        let before = self.snapshot();
        if let Some(value) = self.visible.get_mut(index) {
            *value = visible;
            if !visible {
                self.selected[index] = false;
            }
        }
        self.remember(before);
    }
    /// A single selected layer, or None for zero/multiple selections.
    pub fn selected(&self) -> Option<usize> {
        let indices = self.selected_indices();
        if indices.len() == 1 {
            Some(indices[0])
        } else {
            None
        }
    }
    pub fn selected_indices(&self) -> Vec<usize> {
        (0..self.layers.len())
            .filter(|i| self.selected[*i])
            .collect()
    }
    pub fn selection_mask(&self) -> Vec<bool> {
        self.selected.clone()
    }
    pub fn set_selection(&mut self, selected: impl AsRef<[bool]>) {
        self.finish_drag();
        self.selected = (0..self.layers.len())
            .map(|i| selected.as_ref().get(i).copied().unwrap_or(false) && self.selectable(i))
            .collect();
    }
    pub fn select(&mut self, index: Option<usize>) {
        self.set_selection(
            (0..self.layers.len())
                .map(|i| index == Some(i))
                .collect::<Vec<_>>(),
        );
    }
    pub fn toggle(&mut self, index: usize) {
        self.finish_drag();
        if !self.selectable(index) {
            return;
        }
        if let Some(selected) = self.selected.get_mut(index) {
            *selected = !*selected;
        }
    }
    /// Select rectangles intersecting the marquee; base supplies Shift-additive selection.
    pub fn select_region(&mut self, start: [f64; 2], end: [f64; 2], base: impl AsRef<[bool]>) {
        let left = start[0].min(end[0]);
        let right = start[0].max(end[0]);
        let top = start[1].min(end[1]);
        let bottom = start[1].max(end[1]);
        self.set_selection(
            (0..self.layers.len())
                .map(|i| {
                    let g = self.layers[i];
                    base.as_ref().get(i).copied().unwrap_or(false)
                        || (g.x <= right
                            && g.x + g.width >= left
                            && g.y <= bottom
                            && g.y + g.height >= top)
                })
                .collect::<Vec<_>>(),
        );
    }
    pub fn selection_bounds(&self) -> Option<Geometry> {
        let mut bounds = None::<Geometry>;
        for i in self.selected_indices() {
            let g = self.layers[i];
            bounds = Some(match bounds {
                None => g,
                Some(b) => {
                    let x = b.x.min(g.x);
                    let y = b.y.min(g.y);
                    Geometry {
                        x,
                        y,
                        width: (b.x + b.width).max(g.x + g.width) - x,
                        height: (b.y + b.height).max(g.y + g.height) - y,
                    }
                }
            });
        }
        bounds
    }
    /// Align only selected rectangles; preserve size and the unrelated axis.
    pub fn align(&mut self, alignment: Alignment, target: AlignTarget) -> Result<(), String> {
        self.align_axes(&[alignment], target)
    }
    /// Align one or both axes against one reference, with one undo record.
    pub fn align_axes(
        &mut self,
        alignments: &[Alignment],
        target: AlignTarget,
    ) -> Result<(), String> {
        self.finish_drag();
        let bounds = self.selection_bounds().ok_or("Select layers to align.")?;
        let reference = match target {
            AlignTarget::Selection => bounds,
            AlignTarget::Screen => Geometry {
                x: 0.0,
                y: 0.0,
                width: self.formats.output[0] as f64,
                height: self.formats.output[1] as f64,
            },
            AlignTarget::Layer(i) => *self.layers.get(i).ok_or("Unknown reference layer.")?,
        };
        let mut next = self.layers.clone();
        for i in self.selected_indices() {
            if target == AlignTarget::Layer(i) {
                continue;
            }
            let g = &mut next[i];
            for alignment in alignments {
                match alignment {
                    Alignment::Left => g.x = reference.x,
                    Alignment::Right => g.x = reference.x + reference.width - g.width,
                    Alignment::Top => g.y = reference.y,
                    Alignment::Bottom => g.y = reference.y + reference.height - g.height,
                    Alignment::HorizontalCenter => {
                        g.x = reference.x + (reference.width - g.width) / 2.0
                    }
                    Alignment::VerticalCenter => {
                        g.y = reference.y + (reference.height - g.height) / 2.0
                    }
                }
            }
        }
        self.commit(next)
    }
    fn commit(&mut self, next: Vec<Geometry>) -> Result<(), String> {
        for g in &next {
            g.validate()?;
        }
        let before = self.snapshot();
        let mut formats = self.formats.clone();
        for (i, geometry) in next.iter().enumerate() {
            formats.resize(i, self.layers[i], *geometry)?;
        }
        self.layers = next;
        self.formats = formats;
        self.remember(before);
        Ok(())
    }
    /// Document back-to-front order; selection never changes stacking.
    pub fn viewport_order(&self) -> Vec<usize> {
        (0..self.layers.len()).collect()
    }
    pub fn hit(&self, point: [f64; 2]) -> Option<usize> {
        self.viewport_order()
            .into_iter()
            .rev()
            .find(|i| self.selectable(*i) && self.layers[*i].contains(point))
    }
    fn remember(&mut self, before: Snapshot) {
        if before != self.snapshot() {
            if self.undo.len() == 100 {
                self.undo.remove(0);
            }
            self.undo.push(before);
            self.redo.clear();
        }
    }
    pub fn apply(&mut self, index: usize, geometry: Geometry) -> Result<(), String> {
        let geometry = geometry.validate()?;
        if index >= self.layers.len() {
            return Err("Select an existing layer.".into());
        }
        if !self.selectable(index) {
            return Err("Unlock and show the layer before editing it.".into());
        }
        self.finish_drag();
        let mut next = self.layers.clone();
        next[index] = geometry;
        self.commit(next)
    }
    /// Apply a one-to-one import to a subset of the selection, with one undo record.
    pub fn apply_import(&mut self, values: &[(usize, Geometry)]) -> Result<(), String> {
        let targets = self.selected_indices();
        if values.is_empty() || values.len() > targets.len() {
            return Err(format!(
                "Layer count mismatch: {} selected, {} imported. Map each imported rectangle to a different selected destination.",
                targets.len(),
                values.len()
            ));
        }
        let mut seen = Vec::new();
        let mut next = self.layers.clone();
        for (i, g) in values {
            if !targets.contains(i) || seen.contains(i) || !self.selectable(*i) {
                return Err("Map each imported rectangle to a different selected layer; duplicate or unselected targets are not allowed.".into());
            }
            g.validate()?;
            seen.push(*i);
            next[*i] = *g;
        }
        self.finish_drag();
        self.commit(next)
    }
    /// Import optional per-layer source/crop data without changing shared sources.
    pub fn import_device_layers(&mut self, values: &[DeviceLayerImport]) -> Result<(), String> {
        self.import_device_layers_with_fit(values, &vec![None; values.len()])
    }
    pub fn import_device_layers_with_fit(
        &mut self,
        values: &[DeviceLayerImport],
        fits: &[Option<formats::LayerFit>],
    ) -> Result<(), String> {
        self.import_device_layers_with_appearance(
            values,
            &fits
                .iter()
                .map(|&fit| formats::LayerAppearance {
                    fit,
                    mask: Default::default(),
                })
                .collect::<Vec<_>>(),
        )
    }
    pub fn import_device_layers_with_appearance(
        &mut self,
        values: &[DeviceLayerImport],
        appearances: &[formats::LayerAppearance],
    ) -> Result<(), String> {
        if appearances.len() != values.len() {
            return Err("Device appearance count mismatch.".into());
        }
        let mut draft = self.clone();
        draft.import_mapped_layers(
            &values
                .iter()
                .map(|(id, g, _)| (*id, *g))
                .collect::<Vec<_>>(),
        )?;
        let mut next_new = self.layers.len();
        for ((id, geometry, framing), appearance) in values.iter().zip(appearances) {
            let fit = &appearance.fit;
            let i = if let Some(id) = id {
                draft.info.iter().position(|info| info.id == *id).unwrap()
            } else {
                let i = next_new;
                next_new += 1;
                i
            };
            draft.formats.layer_fit[i] = *fit;
            draft.formats.masks[i] = appearance.mask.validate()?;
            if framing.is_none() {
                if fit.is_some() {
                    return Err("Explicit device fit requires source and crop data.".into());
                }
                draft.formats.advanced[i] = formats::crop_from_framing(
                    draft.formats.input(i)?,
                    [geometry.width, geometry.height],
                    Framing::default(),
                )?;
            }
            if let Some((source, crop)) = framing {
                if let Some(mode) = fit {
                    mode.image(crop.remaining(source.input()?)?, *geometry)?;
                } else {
                    formats::mapping(source.input()?, [geometry.width, geometry.height], *crop)?;
                }
                let current_source = draft.formats.source_for[i];
                let source_id = if draft
                    .formats
                    .source_for
                    .iter()
                    .filter(|&&source| source == current_source)
                    .count()
                    == 1
                {
                    current_source
                } else {
                    if draft.formats.sources.len() >= 1024 {
                        return Err("Source capacity reached.".into());
                    }
                    draft.formats.sources.push(*source);
                    draft.formats.sources.len() - 1
                };
                draft.formats.sources[source_id] = *source;
                draft.formats.source_for[i] = source_id;
                draft.formats.advanced[i] = *crop;
            }
        }
        self.finish_drag();
        let before = self.snapshot();
        self.restore(draft.snapshot());
        self.next_id = draft.next_id;
        self.remember(before);
        Ok(())
    }
    /// Import geometry/crops and its reviewed device links as one undoable edit.
    /// New layers inherit the chosen local folder; existing membership is preserved.
    pub fn import_linked_device_layers(
        &mut self,
        values: &[DeviceLayerImport],
        links: &[DeviceLink],
        new_folder: &str,
    ) -> Result<(), String> {
        self.import_linked_device_layers_with_fit(
            values,
            links,
            new_folder,
            &vec![None; values.len()],
        )
    }
    pub fn import_linked_device_layers_with_fit(
        &mut self,
        values: &[DeviceLayerImport],
        links: &[DeviceLink],
        new_folder: &str,
        fits: &[Option<formats::LayerFit>],
    ) -> Result<(), String> {
        self.import_linked_device_layers_with_appearance(
            values,
            links,
            new_folder,
            &fits
                .iter()
                .map(|&fit| formats::LayerAppearance {
                    fit,
                    mask: Default::default(),
                })
                .collect::<Vec<_>>(),
        )
    }
    pub fn import_linked_device_layers_with_appearance(
        &mut self,
        values: &[DeviceLayerImport],
        links: &[DeviceLink],
        new_folder: &str,
        appearances: &[formats::LayerAppearance],
    ) -> Result<(), String> {
        if values.len() != links.len() || new_folder.chars().count() > 128 {
            return Err("Every imported layer needs a valid device link and folder.".into());
        }
        for link in links {
            link.validate()?;
        }
        let mut draft = self.clone();
        draft.import_device_layers_with_appearance(values, appearances)?;
        let mut next_new = self.layers.len();
        for ((id, _, _), link) in values.iter().zip(links) {
            let i = if let Some(id) = id {
                draft.info.iter().position(|info| info.id == *id).unwrap()
            } else {
                let i = next_new;
                next_new += 1;
                draft.info[i].folder = new_folder.trim().into();
                i
            };
            draft.info[i].load_mapping = Some(link.clone());
            draft.info[i].send_mapping = SendMapping::linked(Some(link.clone()));
        }
        self.finish_drag();
        let before = self.snapshot();
        self.restore(draft.snapshot());
        self.next_id = draft.next_id;
        self.remember(before);
        Ok(())
    }
    /// Import device containers into stable layer IDs, or new layers (`None`).
    /// Independent of selection; the complete mapping is one atomic undo step.
    pub fn import_mapped_layers(
        &mut self,
        values: &[(Option<usize>, Geometry)],
    ) -> Result<(), String> {
        if values.is_empty() {
            return Err("Choose at least one destination layer.".into());
        }
        let mut draft = self.clone();
        let mut seen = Vec::new();
        for &(id, geometry) in values {
            geometry.validate()?;
            let index = if let Some(id) = id {
                if seen.contains(&id) {
                    return Err("Choose a different destination for each device layer.".into());
                }
                seen.push(id);
                let index = self
                    .info
                    .iter()
                    .position(|info| info.id == id)
                    .ok_or("A destination layer was removed. Choose its replacement.")?;
                if !draft.selectable(index) {
                    return Err("Destination layers must be visible and unlocked.".into());
                }
                index
            } else {
                draft.add_layer()?
            };
            draft.apply(index, geometry)?;
        }
        self.finish_drag();
        let before = self.snapshot();
        self.restore(draft.snapshot());
        self.next_id = draft.next_id;
        self.remember(before);
        Ok(())
    }
    /// Apply explicit destination indices without changing the viewport selection.
    /// Mappings must be unique, visible and unlocked; failures are atomic.
    pub fn apply_mapped_import_with_zoom(
        &mut self,
        values: &[(usize, Geometry)],
        zooms: &[(usize, f64)],
    ) -> Result<(), String> {
        if values.iter().any(|(i, _)| !self.selectable(*i)) {
            return Err("Choose visible, unlocked destination layers.".into());
        }
        let mut draft = self.clone();
        let selection: Vec<_> = (0..self.layers.len())
            .map(|i| values.iter().any(|(target, _)| *target == i))
            .collect();
        draft.set_selection(selection);
        draft.apply_import_with_zoom(values, zooms)?;
        self.finish_drag();
        let before = self.snapshot();
        self.restore(draft.snapshot());
        self.remember(before);
        Ok(())
    }
    /// Import geometry and optional crop zoom together as one atomic edit.
    pub fn apply_import_with_zoom(
        &mut self,
        values: &[(usize, Geometry)],
        zooms: &[(usize, f64)],
    ) -> Result<(), String> {
        let mut draft = self.clone();
        draft.apply_import(values)?;
        let mut seen = Vec::new();
        for &(i, zoom) in zooms {
            if !values.iter().any(|(target, _)| *target == i) || seen.contains(&i) {
                return Err("Each zoom must map to one imported layer, without duplicates.".into());
            }
            seen.push(i);
            let g = draft.layers[i];
            let mut framing = draft.formats.mapping(i, g)?.framing;
            let (_, limits) = formats::limits(draft.formats.input(i)?, [g.width, g.height], zoom)?;
            framing.zoom = zoom;
            for (offset, limit) in framing.offset.iter_mut().zip(limits) {
                *offset = offset.clamp(-limit, limit);
            }
            draft.set_framing(i, framing)?;
        }
        self.finish_drag();
        let before = self.snapshot();
        self.restore(draft.snapshot());
        self.remember(before);
        Ok(())
    }
    pub fn nudge(&mut self, dx: f64, dy: f64) -> Result<(), String> {
        self.finish_drag();
        let mut next = self.layers.clone();
        for i in self.selected_indices() {
            next[i].x += dx;
            next[i].y += dy;
        }
        self.commit(next)
    }
    pub fn start_drag(&mut self, point: [f64; 2]) {
        self.finish_drag();
        if let Some(i) = self.hit(point) {
            if !self.selected[i] {
                self.select(Some(i));
            }
            self.drag = Some((self.selected.clone(), self.snapshot(), point));
        } else {
            self.select(None);
        }
    }
    /// Move the existing selection from any pointer position without changing selection.
    pub fn start_selection_drag(&mut self, point: [f64; 2]) {
        self.finish_drag();
        if point.iter().all(|v| v.is_finite()) && !self.selected_indices().is_empty() {
            self.drag = Some((self.selected.clone(), self.snapshot(), point));
        }
    }
    /// Resize one selected layer, keeping its opposite corner fixed.
    pub fn start_resize(
        &mut self,
        index: usize,
        corner: Corner,
        point: [f64; 2],
        keep_ratio: bool,
    ) {
        self.start_handle_resize(index, ResizeHandle::Corner(corner), point, keep_ratio);
    }
    pub fn start_handle_resize(
        &mut self,
        index: usize,
        handle: ResizeHandle,
        point: [f64; 2],
        keep_ratio: bool,
    ) {
        self.finish_drag();
        if !self.selectable(index) || !self.selected[index] {
            return;
        }
        self.select(Some(index));
        self.resize = Some(Resize {
            pivot: None,
            use_pivot: false,
            mode: ResizeMode::Individual,
            targets: self.selected.clone(),
            index,
            handle,
            before: self.snapshot(),
            start: point,
            keep_ratio,
        });
    }
    /// Start proportional resizing from matching or opposite handles, without changing selection.
    pub fn start_selected_resize(
        &mut self,
        index: usize,
        handle: ResizeHandle,
        point: [f64; 2],
        keep_ratio: bool,
        mode: ResizeMode,
    ) -> Result<(), String> {
        if mode == ResizeMode::Individual {
            self.start_handle_resize(index, handle, point, keep_ratio);
            return Ok(());
        }
        let count = self.selected_indices().len();
        if count < 2 || (matches!(mode, ResizeMode::Mirror | ResizeMode::Opposite) && count != 2) {
            return Err(
                "Select at least two layers for Same handles, or exactly two for Mirror horizontally / Opposite handles."
                    .into(),
            );
        }
        if !self.selectable(index) || !self.selected.get(index).copied().unwrap_or(false) {
            return Err("Resize from a selected, visible, unlocked layer.".into());
        }
        self.finish_drag();
        self.resize = Some(Resize {
            pivot: None,
            use_pivot: false,
            mode,
            targets: self.selected.clone(),
            index,
            handle,
            before: self.snapshot(),
            start: point,
            keep_ratio,
        });
        Ok(())
    }
    /// Set a gesture's pivot relative to the initiating layer; partners mirror it by resize mode.
    pub fn set_resize_pivot(&mut self, relative: [f64; 2]) {
        if relative.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
            && let Some(resize) = self.resize.as_mut().or(self.content_resize.as_mut())
        {
            resize.pivot = Some(relative);
        }
    }
    /// Toggle pivot anchoring during a gesture, always recomputing from its initial geometry.
    pub fn use_resize_pivot(&mut self, active: bool) {
        if let Some(resize) = self.resize.as_mut().or(self.content_resize.as_mut()) {
            resize.use_pivot = active;
        }
    }
    pub fn drag_to(&mut self, point: [f64; 2]) {
        self.drag_to_with_precision(point, false);
    }
    fn drag_to_with_precision(&mut self, point: [f64; 2], exact: bool) {
        if point.iter().any(|v| !v.is_finite()) {
            return;
        }
        if let Some((i, before, start, handle)) = &self.mask_drag {
            let Ok(g) = before.formats.mask_reference(*i, before.layers[*i]) else {
                return;
            };
            let mut mask = before.formats.masks[*i];
            let delta = [point[0] - start[0], point[1] - start[1]];
            if let Some(handle) = handle {
                let dx = delta[0] / g.width;
                let dy = delta[1] / g.height;
                if matches!(
                    handle,
                    ResizeHandle::Left | ResizeHandle::Corner(Corner::TopLeft | Corner::BottomLeft)
                ) {
                    mask.left = (mask.left + dx).clamp(0.0, 1.0 - mask.right);
                }
                if matches!(
                    handle,
                    ResizeHandle::Right
                        | ResizeHandle::Corner(Corner::TopRight | Corner::BottomRight)
                ) {
                    mask.right = (mask.right - dx).clamp(0.0, 1.0 - mask.left);
                }
                if matches!(
                    handle,
                    ResizeHandle::Top | ResizeHandle::Corner(Corner::TopLeft | Corner::TopRight)
                ) {
                    mask.top = (mask.top + dy).clamp(0.0, 1.0 - mask.bottom);
                }
                if matches!(
                    handle,
                    ResizeHandle::Bottom
                        | ResizeHandle::Corner(Corner::BottomLeft | Corner::BottomRight)
                ) {
                    mask.bottom = (mask.bottom - dy).clamp(0.0, 1.0 - mask.top);
                }
            } else if let Ok(moved) = mask.translated(delta, g) {
                mask = moved;
            }
            self.formats.masks[*i] = mask;
            return;
        }
        if let Some(r) = &self.content_resize {
            let result = (|| {
                let i = r.index;
                let g = r.before.layers[i];
                let input = r.before.formats.input(i)?;
                if r.before.formats.layer_fit[i] == Some(formats::LayerFit::OneToOne) {
                    let mut crop = r.before.formats.advanced[i];
                    let remaining = crop.remaining(input)?;
                    // The retained image stays centered: moving a displayed edge
                    // by d changes retained size by 2d. Only the chosen source
                    // crop edge changes; pixel scale and container never change.
                    let delta = [2.0 * (point[0] - r.start[0]), 2.0 * (point[1] - r.start[1])];
                    let left = matches!(
                        r.handle,
                        ResizeHandle::Left
                            | ResizeHandle::Corner(Corner::TopLeft | Corner::BottomLeft)
                    );
                    let right = matches!(
                        r.handle,
                        ResizeHandle::Right
                            | ResizeHandle::Corner(Corner::TopRight | Corner::BottomRight)
                    );
                    let top = matches!(
                        r.handle,
                        ResizeHandle::Top
                            | ResizeHandle::Corner(Corner::TopLeft | Corner::TopRight)
                    );
                    let bottom = matches!(
                        r.handle,
                        ResizeHandle::Bottom
                            | ResizeHandle::Corner(Corner::BottomLeft | Corner::BottomRight)
                    );
                    let minimum = remaining.map(|v| v.min(0.001));
                    if left {
                        crop.left =
                            (crop.left + delta[0]).clamp(0.0, input[0] - crop.right - minimum[0]);
                    }
                    if right {
                        crop.right =
                            (crop.right - delta[0]).clamp(0.0, input[0] - crop.left - minimum[0]);
                    }
                    if top {
                        crop.top =
                            (crop.top + delta[1]).clamp(0.0, input[1] - crop.bottom - minimum[1]);
                    }
                    if bottom {
                        crop.bottom =
                            (crop.bottom - delta[1]).clamp(0.0, input[1] - crop.top - minimum[1]);
                    }
                    crop.remaining(input)?;
                    return Ok(crop);
                }
                let m = r.before.formats.mapping(i, g)?;
                let crop = r.before.formats.advanced[i];
                let full = Geometry {
                    x: g.x - crop.left * m.scale,
                    y: g.y - crop.top * m.scale,
                    width: input[0] * m.scale,
                    height: input[1] * m.scale,
                };
                let (zoom, resized) = if r.use_pivot {
                    let pivot = r.anchor(i);
                    let handle = ResizeHandle::positions(full)
                        .into_iter()
                        .find(|(h, _)| *h == r.handle)
                        .unwrap()
                        .1;
                    let axes = [
                        !matches!(r.handle, ResizeHandle::Top | ResizeHandle::Bottom),
                        !matches!(r.handle, ResizeHandle::Left | ResizeHandle::Right),
                    ];
                    let vector = [handle[0] - pivot[0], handle[1] - pivot[1]];
                    let denominator: f64 =
                        (0..2).filter(|&a| axes[a]).map(|a| vector[a].powi(2)).sum();
                    let desired = if denominator > 1e-18 {
                        1.0 + (0..2)
                            .filter(|&a| axes[a])
                            .map(|a| (point[a] - r.start[a]) * vector[a])
                            .sum::<f64>()
                            / denominator
                    } else {
                        1.0
                    };
                    let mut lower = 1.0 / m.framing.zoom;
                    let mut upper = 100.0 / m.framing.zoom;
                    // Bound scaling about the pivot so content still covers the box.
                    // Clamping offsets instead would move the source pixel under the pivot.
                    for (a, b) in [
                        (full.x - pivot[0], g.x - pivot[0]),
                        (pivot[0] - full.x - full.width, pivot[0] - g.x - g.width),
                        (full.y - pivot[1], g.y - pivot[1]),
                        (pivot[1] - full.y - full.height, pivot[1] - g.y - g.height),
                    ] {
                        if a > 1e-9 {
                            upper = upper.min(b / a);
                        } else if a < -1e-9 {
                            lower = lower.max(b / a);
                        }
                    }
                    // The starting crop is valid; include it despite boundary roundoff.
                    let factor = desired.clamp(lower.min(1.0), upper.max(1.0));
                    (
                        (m.framing.zoom * factor).clamp(1.0, 100.0),
                        Geometry {
                            x: pivot[0] + (full.x - pivot[0]) * factor,
                            y: pivot[1] + (full.y - pivot[1]) * factor,
                            width: full.width * factor,
                            height: full.height * factor,
                        },
                    )
                } else {
                    let desired = resized_geometry(
                        full,
                        r.handle,
                        [point[0] - r.start[0], point[1] - r.start[1]],
                        true,
                    );
                    let zoom = (m.framing.zoom * desired.width / full.width).clamp(1.0, 100.0);
                    let (scale, _) = formats::limits(input, [g.width, g.height], zoom)?;
                    (
                        zoom,
                        anchored_resize(full, r.handle, input[0] * scale, input[1] * scale),
                    )
                };
                let (_, limits) = formats::limits(input, [g.width, g.height], zoom)?;
                let center = resized.center();
                let layer_center = g.center();
                let offset = std::array::from_fn(|axis| {
                    (center[axis] - layer_center[axis]).clamp(-limits[axis], limits[axis])
                });
                formats::crop_from_framing(input, [g.width, g.height], Framing { zoom, offset })
            })();
            match result {
                Ok(crop) => {
                    self.formats.advanced[r.index] = crop;
                    self.gesture_error = None;
                }
                Err(e) => self.gesture_error = Some(e),
            }
            return;
        }
        if let Some((i, before, start)) = self.content_drag.clone() {
            if before.formats.layer_fit[i] == Some(formats::LayerFit::OneToOne) {
                match before.formats.advanced[i]
                    .pan_image([point[0] - start[0], point[1] - start[1]])
                {
                    Ok(crop) => {
                        self.formats.advanced[i] = crop;
                        self.gesture_error = None;
                    }
                    Err(error) => self.gesture_error = Some(error),
                }
                return;
            }
            if let Ok(m) = before.formats.mapping(i, before.layers[i]) {
                let offset = std::array::from_fn(|axis| {
                    (m.framing.offset[axis] + point[axis] - start[axis])
                        .clamp(-m.offset_limits[axis], m.offset_limits[axis])
                });
                if let Ok(crop) = formats::crop_from_framing(
                    before.formats.input(i).unwrap(),
                    [before.layers[i].width, before.layers[i].height],
                    Framing {
                        zoom: m.framing.zoom,
                        offset,
                    },
                ) {
                    self.formats.advanced[i] = crop;
                }
            }
            return;
        }
        if let Some(r) = &self.resize {
            let original = r.before.layers[r.index];
            let resize = if exact {
                resized_geometry_exact
            } else {
                resized_geometry
            };
            let next = if r.use_pivot {
                let pivot = r.anchor(r.index);
                let handle = ResizeHandle::positions(original)
                    .into_iter()
                    .find(|(h, _)| *h == r.handle)
                    .unwrap()
                    .1;
                let delta = std::array::from_fn::<_, 2, _>(|axis| {
                    let d = point[axis] - r.start[axis];
                    if exact { d } else { d.round() }
                });
                let vector = [handle[0] - pivot[0], handle[1] - pivot[1]];
                let axes = [
                    !matches!(r.handle, ResizeHandle::Top | ResizeHandle::Bottom),
                    !matches!(r.handle, ResizeHandle::Left | ResizeHandle::Right),
                ];
                let mut factors = [1.0; 2];
                for axis in 0..2 {
                    if axes[axis] && vector[axis].abs() > 1e-9 {
                        factors[axis] += delta[axis] / vector[axis];
                    }
                }
                if r.keep_ratio {
                    let denominator: f64 = (0..2)
                        .filter(|&a| axes[a])
                        .map(|a| vector[a] * vector[a])
                        .sum();
                    let factor = if denominator > 1e-18 {
                        1.0 + (0..2)
                            .filter(|&a| axes[a])
                            .map(|a| delta[a] * vector[a])
                            .sum::<f64>()
                            / denominator
                    } else {
                        1.0
                    };
                    factors = [factor.max((1.0 / original.width).max(1.0 / original.height)); 2];
                }
                Geometry {
                    x: original.x,
                    y: original.y,
                    width: (original.width * factors[0]).max(1.0),
                    height: (original.height * factors[1]).max(1.0),
                }
            } else {
                resize(
                    original,
                    r.handle,
                    [point[0] - r.start[0], point[1] - r.start[1]],
                    r.keep_ratio,
                )
            };
            let result = (|| {
                let mut layers = r.before.layers.clone();
                let mut formats = r.before.formats.clone();
                let mut sx = next.width / original.width;
                let mut sy = next.height / original.height;
                if r.mode != ResizeMode::Individual {
                    for (i, g) in layers.iter().enumerate() {
                        if r.targets[i] {
                            sx = sx.max(1.0 / g.width);
                            sy = sy.max(1.0 / g.height);
                        }
                    }
                    if r.keep_ratio {
                        sx = sx.max(sy);
                        sy = sx;
                    }
                }
                for (i, geometry) in layers.iter_mut().enumerate() {
                    if !r.targets[i] {
                        continue;
                    }
                    let before = *geometry;
                    let handle = if i != r.index {
                        match r.mode {
                            ResizeMode::Mirror => r.handle.mirrored_horizontally(),
                            ResizeMode::Opposite => r.handle.opposite(),
                            ResizeMode::Individual | ResizeMode::Same => r.handle,
                        }
                    } else {
                        r.handle
                    };
                    *geometry = if r.use_pivot {
                        let pivot = r.anchor(i);
                        Geometry {
                            x: pivot[0] + (before.x - pivot[0]) * sx,
                            y: pivot[1] + (before.y - pivot[1]) * sy,
                            width: before.width * sx,
                            height: before.height * sy,
                        }
                    } else if r.mode == ResizeMode::Individual {
                        next
                    } else {
                        anchored_resize(before, handle, before.width * sx, before.height * sy)
                    };
                    geometry.validate()?;
                    formats.resize(i, before, *geometry)?;
                }
                Ok::<_, String>((layers, formats))
            })();
            match result {
                Ok((layers, formats)) => {
                    self.layers = layers;
                    self.formats = formats;
                    self.gesture_error = None;
                }
                Err(e) => self.gesture_error = Some(e),
            }
            return;
        }
        if let Some((selected, before, start)) = self.drag.clone() {
            let mut next = before.layers;
            for i in 0..self.layers.len() {
                if selected[i] {
                    next[i].x += (point[0] - start[0]).round();
                    next[i].y += (point[1] - start[1]).round();
                }
            }
            if next.iter().all(|g| g.validate().is_ok()) {
                self.layers = next;
            }
        }
    }
    pub fn resizing(&self) -> bool {
        self.resize.is_some()
    }
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
            || self.resize.is_some()
            || self.content_drag.is_some()
            || self.content_resize.is_some()
            || self.mask_drag.is_some()
    }
    pub fn finish_drag(&mut self) {
        if let Some((_, before, _, _)) = self.mask_drag.take() {
            self.remember(before);
        }
        if let Some(r) = self.content_resize.take() {
            self.remember(r.before);
        }
        if let Some((_, before, _)) = self.content_drag.take() {
            self.remember(before);
        }
        if let Some(r) = self.resize.take() {
            self.remember(r.before);
        }
        if let Some((_, before, _)) = self.drag.take() {
            self.remember(before);
        }
    }
    pub fn cancel_drag(&mut self) {
        if let Some((_, before, _, _)) = self.mask_drag.take() {
            self.restore(before);
        }
        if let Some(r) = self.content_resize.take() {
            self.restore(r.before);
        }
        self.gesture_error = None;
        if let Some((_, before, _)) = self.content_drag.take() {
            self.restore(before);
        }
        if let Some(r) = self.resize.take() {
            self.restore(r.before);
        }
        if let Some((_, before, _)) = self.drag.take() {
            self.restore(before);
        }
    }
    pub fn undo(&mut self) {
        self.finish_drag();
        if let Some(before) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(before);
        }
    }
    pub fn redo(&mut self) {
        self.finish_drag();
        if let Some(after) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(after);
        }
    }
}

#[derive(Clone, Debug)]
pub struct DrawioShape {
    pub id: String,
    pub name: Option<String>,
    pub geometry: Geometry,
    /// Effective visible outline thickness, in diagram pixels (default 1).
    pub stroke_width: f64,
    pub background: bool,
}
impl DrawioShape {
    /// Outline expansion applies only when importing size; position-only keeps
    /// the original draw.io top-left and the destination layer dimensions.
    pub fn paste_geometry(
        &self,
        before: Geometry,
        include_size: bool,
        include_outline: bool,
    ) -> Result<Geometry, String> {
        let mut g = self.geometry;
        if !include_size {
            g.width = before.width;
            g.height = before.height;
        } else if include_outline {
            g.x -= self.stroke_width / 2.0;
            g.y -= self.stroke_width / 2.0;
            g.width += self.stroke_width;
            g.height += self.stroke_width;
        }
        g.validate()
    }
}

/// Read raw/percent-encoded mxGraphModel or a single-page mxfile (compressed or plain).
/// Values map 1:1 to pixels.
/// Nested, relative, rotated and nonrectangular geometry is intentionally refused.
pub fn parse_drawio(input: &str) -> Result<Vec<DrawioShape>, String> {
    if input.len() > exchange::MAX_EXCHANGE_BYTES {
        return Err("Diagram exceeds 16 MiB.".into());
    }
    let input = input.trim();
    let decoded;
    let xml = if input.starts_with('<') {
        input
    } else {
        decoded = decode_percent(input)?;
        decoded.trim()
    };
    let xml = drawio_model(xml)?;
    let doc = roxmltree::Document::parse(&xml).map_err(|e| format!("Invalid draw.io XML: {e}"))?;
    let model = doc.root_element();
    if let Some(version) = model.attribute("data-midra-layout-geometry")
        && version != "1"
    {
        return Err("Unsupported Canvas Pilot draw.io exchange version.".into());
    }
    let root = model
        .children()
        .find(|n| n.has_tag_name("root"))
        .ok_or("draw.io root is missing.")?;
    let mut cells = HashMap::new();
    for n in root.children().filter(|n| n.is_element()) {
        // draw.io stores global custom properties on a wrapped root cell.
        if matches!(n.tag_name().name(), "object" | "UserObject") && n.attribute("id") == Some("0")
        {
            let children: Vec<_> = n.children().filter(|c| c.is_element()).collect();
            if children.len() == 1
                && children[0].has_tag_name("mxCell")
                && ["vertex", "edge", "parent"]
                    .iter()
                    .all(|a| children[0].attribute(*a).is_none())
            {
                if cells.insert("0", children[0]).is_some() {
                    return Err("Duplicate draw.io cell ID.".into());
                }
                continue;
            }
        }
        if !n.has_tag_name("mxCell") {
            return Err("Wrapped objects are not supported yet. Copy plain rectangles.".into());
        }
        let id = n.attribute("id").ok_or("A cell has no ID.")?;
        if cells.insert(id, n).is_some() {
            return Err("Duplicate draw.io cell ID.".into());
        }
    }
    let mut result = Vec::new();
    for cell in root.children().filter(|n| n.has_tag_name("mxCell")) {
        if cell.attribute("edge") == Some("1") {
            return Err("Connections are not supported. Copy rectangles only.".into());
        }
        if cell.attribute("vertex") != Some("1") {
            continue;
        }
        let id = cell.attribute("id").unwrap();
        let parent = cell
            .attribute("parent")
            .and_then(|p| cells.get(p))
            .ok_or("Missing parent; grouped/partial geometry is unsupported.")?;
        let is_layer = parent.attribute("vertex").is_none()
            && parent
                .attribute("parent")
                .and_then(|p| cells.get(p))
                .is_some_and(|p| {
                    p.attribute("parent").is_none() && p.attribute("vertex").is_none()
                });
        if !is_layer {
            return Err(
                "Grouped or nested rectangles are not supported yet. Ungroup before copying."
                    .into(),
            );
        }
        let mut styles = HashMap::new();
        for entry in cell
            .attribute("style")
            .unwrap_or("")
            .split(';')
            .filter(|s| !s.is_empty())
        {
            let (key, value) = entry
                .split_once('=')
                .ok_or("Named/inherited draw.io styles are unsupported.")?;
            styles.insert(key, value);
            if (key == "shape" && !matches!(value, "rectangle" | "image"))
                || (matches!(key, "rotation" | "flipH" | "flipV") && value != "0")
            {
                return Err("Only unrotated rectangles are supported; remove shape transforms before copying.".into());
            }
        }
        let style_number = |key, default: f64, max: f64| -> Result<f64, String> {
            let value = styles.get(key).map_or(Ok(default), |s| {
                s.parse::<f64>().map_err(|_| format!("Invalid {key}."))
            })?;
            if !value.is_finite() || !(0.0..=max).contains(&value) {
                return Err(format!("Invalid {key}: expected 0–{max}."));
            }
            Ok(value)
        };
        let width = style_number("strokeWidth", 1.0, 1_000_000.0)?;
        let stroke_opacity = style_number("strokeOpacity", 100.0, 100.0)?;
        let opacity = style_number("opacity", 100.0, 100.0)?;
        let hidden = styles
            .get("strokeColor")
            .is_some_and(|c| matches!(*c, "none" | "transparent"))
            || stroke_opacity == 0.0
            || opacity == 0.0;
        let stroke_width = if hidden { 0.0 } else { width };
        let geometries: Vec<_> = cell.children().filter(|n| n.is_element()).collect();
        if geometries.len() != 1 || !geometries[0].has_tag_name("mxGeometry") {
            return Err("Expected one plain geometry per rectangle.".into());
        }
        let g = geometries[0];
        if g.attribute("relative").is_some_and(|v| v != "0") || g.children().any(|n| n.is_element())
        {
            return Err("Relative, offset or alternate bounds are not supported yet.".into());
        }
        let number = |name, default| -> Result<f64, String> {
            g.attribute(name)
                .or(default)
                .ok_or_else(|| format!("Missing {name}."))?
                .parse()
                .map_err(|_| format!("Invalid {name}."))
        };
        let geometry = Geometry {
            x: number("x", Some("0"))?,
            y: number("y", Some("0"))?,
            width: number("width", None)?,
            height: number("height", None)?,
        }
        .validate()?;
        result.push(DrawioShape {
            name: import::drawio_name(cell.attribute("value"), styles.get("html") == Some(&"1")),
            id: id.to_owned(),
            geometry,
            stroke_width,
            background: cell.attribute("data-midra-role") == Some("background")
                || id == "midra-background",
        });
        if result.iter().filter(|s| !s.background).count() > 256
            || result.iter().filter(|s| s.background).count() > 1
        {
            return Err("Import is limited to 256 layers plus one background.".into());
        }
    }
    if result.is_empty() {
        return Err("No rectangles found in the draw.io clipboard.".into());
    }
    Ok(result)
}
pub(crate) fn drawio_model(xml: &str) -> Result<String, String> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::io::Read;
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("Invalid draw.io XML: {e}"))?;
    let root = doc.root_element();
    if root.has_tag_name("mxGraphModel") {
        return Ok(xml.into());
    }
    if !root.has_tag_name("mxfile") {
        return Err("Choose draw.io XML or a .drawio file.".into());
    }
    let pages: Vec<_> = root
        .children()
        .filter(|n| n.has_tag_name("diagram"))
        .collect();
    if pages.len() != 1 {
        return Err("Import one draw.io page at a time; export the required page as XML.".into());
    }
    if let Some(model) = pages[0].children().find(|n| n.has_tag_name("mxGraphModel")) {
        return Ok(xml[model.range()].into());
    }
    let data = STANDARD
        .decode(pages[0].text().unwrap_or("").trim())
        .map_err(|e| format!("Invalid compressed diagram: {e}"))?;
    let mut encoded = String::new();
    flate2::read::DeflateDecoder::new(data.as_slice())
        .take(exchange::MAX_EXCHANGE_BYTES as u64 * 3 + 1)
        .read_to_string(&mut encoded)
        .map_err(|e| format!("Cannot decompress diagram: {e}"))?;
    if encoded.len() > exchange::MAX_EXCHANGE_BYTES * 3 {
        return Err("Decompressed diagram exceeds limit.".into());
    }
    let model = decode_percent(&encoded)?;
    if model.len() > exchange::MAX_EXCHANGE_BYTES {
        return Err("Decompressed diagram exceeds 16 MiB.".into());
    }
    if !roxmltree::Document::parse(&model)
        .is_ok_and(|d| d.root_element().has_tag_name("mxGraphModel"))
    {
        return Err("Compressed diagram must contain mxGraphModel XML.".into());
    }
    Ok(model)
}
fn decode_percent(input: &str) -> Result<String, String> {
    let mut out = Vec::with_capacity(input.len());
    let mut bytes = input.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hex = |c: u8| (c as char).to_digit(16).map(|v| v as u8);
            let a = bytes
                .next()
                .and_then(hex)
                .ok_or("Invalid percent escape in clipboard.")?;
            let b = bytes
                .next()
                .and_then(hex)
                .ok_or("Invalid percent escape in clipboard.")?;
            out.push(a * 16 + b);
        } else {
            out.push(b);
        } // '+' is literal in encodeURIComponent, not a space.
    }
    String::from_utf8(out).map_err(|_| "Clipboard is not valid UTF-8.".into())
}

/// A device import destination, visible geometry and optional source/crop pair.
pub type DeviceLayerImport = (
    Option<usize>,
    Geometry,
    Option<(formats::Source, formats::Crop)>,
);
