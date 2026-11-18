use super::*;
use canvas_pilot::device_capture;
pub(super) const INPUT_REFERENCE_ROW_HEIGHT: f32 = 32.0;
type CaptureTarget = (usize, usize, u32, String);
type CaptureJob = std::sync::mpsc::Receiver<Result<super::super::reference::Reference, String>>;

fn input_label(inputs: &[device_capture::Input], selected: u32) -> String {
    if selected == 0 {
        return "Choose input…".into();
    }
    let suffix = match inputs.iter().find(|input| input.id == selected) {
        Some(input) if !input.signal => " · No signal",
        Some(_) => "",
        None => " · Unavailable",
    };
    format!("Input {selected}{suffix}")
}
fn input_picker(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    selected: &mut u32,
    inputs: &[device_capture::Input],
    show_empty: &mut bool,
) {
    egui::ComboBox::from_id_salt(salt)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .selected_text(input_label(inputs, *selected))
        .show_ui(ui, |ui| {
            ui.visuals_mut().widgets.hovered.bg_stroke.color = egui::Color32::TRANSPARENT;
            ui.checkbox(show_empty, "Show inputs without signal")
                .on_hover_text(tr(
                    ui.ctx(),
                    "ui.signal-status-comes-from-the-last-connection-refresh",
                ));
            ui.separator();
            let mut any = false;
            for input in inputs.iter().filter(|input| *show_empty || input.signal) {
                any = true;
                if ui
                    .stable_selectable_value(selected, input.id, input_label(inputs, input.id))
                    .clicked()
                {
                    ui.close();
                }
            }
            if !any {
                ui.small(if *show_empty {
                    "No available inputs."
                } else {
                    "No inputs with signal. Enable the toggle above to see empty inputs."
                });
            }
        });
}

struct LayerPreview {
    input: u32,
    host: String,
    source: usize,
    texture: Option<egui::TextureHandle>,
    bytes: Vec<u8>,
    paused: bool,
    automatic: bool,
    capture_on_selection: bool,
    name: String,
    enabled: bool,
    guides: f32,
}
struct Refresh {
    seconds: u64,
    last_input: u32,
    pending_input: Option<u32>,
}
impl Default for Refresh {
    fn default() -> Self {
        Self {
            seconds: 10,
            last_input: 0,
            pending_input: None,
        }
    }
}
#[derive(Default)]
pub(super) struct CaptureUi {
    pub open: bool,
    refresh: Refresh,
    input: u32,
    target_layer: Option<usize>,
    keep_refreshing: bool,
    show_empty_inputs: bool,
    capture_on_selection: bool,
    layers: std::collections::BTreeMap<usize, LayerPreview>,
    host: String,
    pending: Option<CaptureJob>,
    pending_layer: Option<CaptureTarget>,
    external_input: Option<u32>,
    external_geometry:
        Option<std::sync::mpsc::Receiver<Option<super::inputs_window::CaptureGeometry>>>,
    preview: Option<super::super::reference::Reference>,
    error: String,
    last_request: Option<std::time::Instant>,
    #[cfg(test)]
    test_now: Option<std::time::Instant>,
    discard_result: bool,
}
impl CaptureUi {
    pub(super) fn save_workspace(&self, layout: &Layout, w: &mut canvas_pilot::project::Workspace) {
        use base64::Engine;
        w.capture_interval_seconds = self.refresh.seconds;
        w.show_empty_inputs = self.show_empty_inputs;
        w.previews = self
            .layers
            .iter()
            .filter_map(|(&id, p)| {
                let i = (0..layout.layers().len()).find(|&i| layout.layer_info(i).id == id)?;
                if layout.formats().source_for[i] != p.source {
                    return None;
                }
                Some(canvas_pilot::project::Preview {
                    layer_id: id,
                    source_id: p.source + 1,
                    input: p.input,
                    // The connection text box may contain an unfinished address.
                    // It is not a capture binding until it is a valid endpoint.
                    host: if canvas_pilot::device::address(&p.host).is_ok() {
                        p.host.clone()
                    } else {
                        String::new()
                    },
                    enabled: p.enabled,
                    automatic: p.automatic,
                    guide_opacity: p.guides,
                    name: p.name.clone(),
                    image_base64: base64::engine::general_purpose::STANDARD.encode(&p.bytes),
                })
            })
            .collect();
    }
    pub(super) fn from_workspace(
        ctx: &egui::Context,
        w: &canvas_pilot::project::Workspace,
    ) -> Result<Self, String> {
        let mut capture = Self::default();
        capture.refresh.seconds = w.capture_interval_seconds;
        capture.show_empty_inputs = w.show_empty_inputs;
        for p in &w.previews {
            let bytes = p.bytes()?;
            let texture = if bytes.is_empty() {
                None
            } else {
                Some(
                    super::super::reference::Reference::load(
                        ctx,
                        &egui::DroppedFile {
                            name: p.name.clone(),
                            bytes: Some(bytes.clone().into()),
                            ..Default::default()
                        },
                    )?
                    .texture,
                )
            };
            capture.layers.insert(
                p.layer_id,
                LayerPreview {
                    input: p.input,
                    host: p.host.clone(),
                    source: p.source_id - 1,
                    texture,
                    bytes,
                    paused: false,
                    automatic: p.automatic,
                    capture_on_selection: false,
                    name: p.name.clone(),
                    enabled: p.enabled,
                    guides: p.guide_opacity,
                },
            );
        }
        Ok(capture)
    }
    pub(super) fn clear_preview(&mut self) {
        self.preview = None;
        self.capture_on_selection = false;
        self.layers.clear();
        self.open = false;
        self.discard_result = true;
    }
    fn now(&self) -> std::time::Instant {
        #[cfg(test)]
        if let Some(now) = self.test_now {
            return now;
        }
        std::time::Instant::now()
    }

    fn cooling_down(&self) -> bool {
        self.last_request.is_some_and(|t| {
            self.now().saturating_duration_since(t) < std::time::Duration::from_secs(5)
        })
    }
}
impl Editor {
    pub(super) fn input_capture_pending(&self) -> bool {
        self.capture.pending.is_some() && self.capture.external_input == Some(self.inputs.selected)
    }
    fn live_input_capture(&self) -> Option<u32> {
        (self.inputs.open && self.inputs.live_capture && !self.device.busy())
            .then_some(self.inputs.selected)
            .filter(|input| {
                self.device.snapshot.as_ref().is_some_and(|s| {
                    device_capture::inputs(s)
                        .iter()
                        .any(|i| i.id == *input && i.enabled)
                })
            })
    }
    pub(super) fn open_snapshot_import(&mut self) {
        self.capture.target_layer = self.layout.selected().map(|i| self.layout.layer_info(i).id);
        self.capture.keep_refreshing = false;
        self.capture.open = true;
    }

    // Selection captures share the manual five-second throttle, not the refresh interval.
    // Keep only the latest input per target while another request is in flight.
    fn next_selection_capture(&mut self) -> Option<(u32, Option<CaptureTarget>)> {
        let available = self
            .device
            .snapshot
            .as_ref()
            .map(device_capture::inputs)
            .unwrap_or_default();
        let valid_input =
            |input| self.device.online() && available.iter().any(|a| a.id == input && a.enabled);
        self.capture.capture_on_selection &= self.capture.open && valid_input(self.capture.input);
        for (id, preview) in &mut self.capture.layers {
            preview.capture_on_selection &= preview.enabled
                && preview.host == self.device.host
                && valid_input(preview.input)
                && (0..self.layout.layers().len()).any(|i| {
                    self.layout.layer_info(i).id == *id
                        && self.layout.formats().source_for[i] == preview.source
                });
        }
        if self.device.busy() || self.capture.pending.is_some() || self.capture.cooling_down() {
            return None;
        }
        if self.capture.capture_on_selection {
            self.capture.capture_on_selection = false;
            return Some((self.capture.input, None));
        }
        for (id, preview) in &mut self.capture.layers {
            if preview.capture_on_selection {
                preview.capture_on_selection = false;
                return Some((
                    preview.input,
                    Some((*id, preview.source, preview.input, preview.host.clone())),
                ));
            }
        }
        None
    }

    fn next_preview_refresh(&self) -> Option<u32> {
        if (self.capture.open && self.live_input_capture().is_none())
            || self.device.busy()
            || !self.device.online()
            || self.capture.pending.is_some()
            || self.capture.last_request.is_some_and(|t| {
                self.capture.now().saturating_duration_since(t).as_secs()
                    < self.capture.refresh.seconds.max(5)
            })
        {
            return None;
        }
        let available = self
            .device
            .snapshot
            .as_ref()
            .map(device_capture::inputs)
            .unwrap_or_default();
        let mut inputs: std::collections::BTreeSet<_> = (0..self.layout.layers().len())
            .filter(|&i| self.layout.visible(i))
            .filter_map(|i| {
                self.capture
                    .layers
                    .get(&self.layout.layer_info(i).id)
                    .filter(|p| {
                        p.enabled
                            && p.automatic
                            && !p.paused
                            && p.source == self.layout.formats().source_for[i]
                            && p.host == self.device.host
                            && available.iter().any(|a| a.id == p.input && a.enabled)
                    })
                    .map(|p| p.input)
            })
            .collect();
        if let Some(input) = self.live_input_capture() {
            inputs.insert(input);
        }
        inputs
            .iter()
            .copied()
            .find(|&i| i > self.capture.refresh.last_input)
            .or_else(|| inputs.first().copied())
    }
    fn start_input_capture(&mut self, ctx: &egui::Context, input: u32, automatic: bool) {
        self.capture.host = self.device.host.clone();
        self.capture.pending_layer = None;
        self.capture.last_request = Some(self.capture.now());
        self.capture.discard_result = false;
        self.capture.error.clear();
        self.capture.refresh.pending_input = automatic.then_some(input);
        if automatic {
            self.capture.refresh.last_input = input;
        } else if self.capture.external_input.is_none() {
            self.capture.preview = None;
        }
        self.capture.external_input = self.live_input_capture().filter(|id| *id == input);
        let expected_plug = self.capture.external_input.and_then(|_| {
            let snapshot = self.device.snapshot.as_ref()?;
            snapshot
                .raw
                .pointer(&format!(
                    "/device/inputList/items/{}_{input}/status/pp/plug",
                    if snapshot.profile == canvas_pilot::device::Profile::Aquilon6 {
                        "IN"
                    } else {
                        "INPUT"
                    }
                ))?
                .as_str()
                .map(str::to_owned)
        });
        if let Some(plug) = &expected_plug {
            self.inputs.capture_key = Some((self.device.host.clone(), input, plug.clone()));
        }
        let external = self.capture.external_input.is_some();
        let (geometry_tx, geometry_rx) = std::sync::mpsc::channel();
        self.capture.external_geometry = external.then_some(geometry_rx);
        let (tx, rx) = std::sync::mpsc::channel();
        self.capture.pending = Some(rx);
        let host = self.device.host.clone();
        let ctx = ctx.clone();
        let profile = self
            .device
            .snapshot
            .as_ref()
            .map(|s| s.profile)
            .unwrap_or_default();
        std::thread::spawn(move || {
            let result = (|| {
                let before = if external {
                    Some(canvas_pilot::device::fetch_input(&host, input)?)
                } else {
                    None
                };
                let bytes = device_capture::capture_for(&host, profile, input)?;
                let reference = super::super::reference::Reference::load(
                    &ctx,
                    &egui::DroppedFile {
                        name: format!("Device input {input} · {host}"),
                        bytes: Some(bytes.into()),
                        ..Default::default()
                    },
                )?;
                if let Some(before) = before {
                    let draft = canvas_pilot::device_inputs::InputDraft::read(&before, input)?;
                    if expected_plug.as_deref() != Some(draft.plug.as_str()) {
                        return Err("Input plug changed; refresh input settings.".into());
                    }
                    let after = canvas_pilot::device::fetch_input(&host, input)?;
                    draft.changes(&after)?;
                    let geometry = device_capture::capture_geometry(&before, input, reference.size)
                        .ok()
                        .filter(|geometry| {
                            device_capture::capture_geometry(&after, input, reference.size)
                                .ok()
                                .as_ref()
                                == Some(geometry)
                        });
                    let _ = geometry_tx.send(geometry);
                }
                Ok(reference)
            })();
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }

    pub(super) fn layer_preview(&self, i: usize) -> Option<(egui::TextureId, f32)> {
        self.capture
            .layers
            .get(&self.layout.layer_info(i).id)
            .filter(|p| p.enabled && p.source == self.layout.formats().source_for[i])
            .and_then(|p| p.texture.as_ref().map(|t| (t.id(), p.guides)))
    }
    pub(super) fn preselect_linked_capture_inputs(&mut self) {
        let Some(snapshot) = self.device.snapshot.as_ref() else {
            return;
        };
        let inputs = device_capture::inputs(snapshot);
        for i in 0..self.layout.layers().len() {
            let info = self.layout.layer_info(i);
            let Some(link) = &info.load_mapping else {
                continue;
            };
            if link.family != snapshot.profile.family() {
                continue;
            }
            let Some(layer) = snapshot
                .screens
                .iter()
                .find(|s| s.id == link.screen)
                .and_then(|s| s.layers.iter().find(|l| l.id == link.layer))
            else {
                continue;
            };
            let Some(input) = snapshot
                .profile
                .input_from_source(&layer.source)
                .filter(|id| inputs.iter().any(|input| input.id == *id))
            else {
                continue;
            };
            let source = self.layout.formats().source_for[i];
            let preview = self
                .capture
                .layers
                .entry(info.id)
                .or_insert_with(|| LayerPreview {
                    input: 0,
                    host: self.device.host.clone(),
                    source,
                    texture: None,
                    bytes: Vec::new(),
                    paused: false,
                    automatic: false,
                    capture_on_selection: false,
                    name: String::new(),
                    enabled: false,
                    guides: 0.35,
                });
            // Seed only untouched controls; never replace an operator's capture choice.
            if preview.input == 0 && !preview.enabled && preview.texture.is_none() {
                preview.input = input;
                preview.host = self.device.host.clone();
                preview.source = source;
                preview.automatic = false;
                preview.capture_on_selection = false;
            }
        }
    }
    pub(super) fn crop_preview_quick_controls(&mut self, ui: &mut egui::Ui, i: usize) {
        let id = self.layout.layer_info(i).id;
        let Some(preview) = self.capture.layers.get_mut(&id) else {
            ui.weak(tr(ui.ctx(), "ui.configure-capture-to-choose-an-input"));
            return;
        };
        ui.separator();
        ui.checkbox(
            &mut preview.enabled,
            tr(ui.ctx(), "ui.enable-input-preview"),
        );
        let has_image = preview.enabled
            && preview.texture.is_some()
            && preview.source == self.layout.formats().source_for[i];
        let mut opacity = if has_image {
            preview.guides * 100.0
        } else {
            100.0
        };
        if ui
            .add_enabled(
                has_image,
                egui::Slider::new(&mut opacity, 0.0..=100.0)
                    .text(tr(ui.ctx(), "ui.crop-guide-opacity"))
                    .suffix("%"),
            )
            .changed()
        {
            preview.guides = opacity / 100.0;
        }
    }
    fn set_reference_picture(&mut self, i: usize, reference: super::super::reference::Reference) {
        let id = self.layout.layer_info(i).id;
        if let Some(choice) = self.device.source_choices.get_mut(&id) {
            choice.follow = false;
        }
        self.capture.layers.insert(
            id,
            LayerPreview {
                input: 0,
                host: String::new(),
                source: self.layout.formats().source_for[i],
                texture: Some(reference.texture),
                bytes: reference.bytes,
                paused: false,
                automatic: false,
                capture_on_selection: false,
                name: reference.name,
                enabled: true,
                guides: 0.35,
            },
        );
    }
    fn import_layer_reference_picture(&mut self, ctx: &egui::Context, i: usize) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(tr(ctx, "ui.reference-picture"))
            .add_filter("Pictures", &["png", "jpg", "jpeg", "bmp"])
            .pick_file()
        {
            match super::super::reference::Reference::load(
                ctx,
                &egui::DroppedFile {
                    path: Some(path),
                    ..Default::default()
                },
            ) {
                Ok(reference) => {
                    self.set_reference_picture(i, reference);
                    self.capture.error.clear();
                }
                Err(error) => self.capture.error = error,
            }
        }
    }
    pub(super) fn input_reference_count(&self) -> usize {
        (0..self.layout.layers().len())
            .filter(|&i| {
                self.capture
                    .layers
                    .get(&self.layout.layer_info(i).id)
                    .is_some_and(|p| {
                        p.texture.is_some() && p.source == self.layout.formats().source_for[i]
                    })
            })
            .count()
    }
    pub(super) fn input_reference_rows(&mut self, ui: &mut egui::Ui, ink: &Ink, y: f32, w: f32) {
        if self.input_reference_count() == 0 {
            return;
        }
        let palette = theme::palette(ui.ctx());
        ink.text(
            0.,
            y + 8.,
            &tr(ui.ctx(), "ui.input-reference-pictures"),
            11.,
            palette.purple,
        );
        let mut row = 0;
        let mut remove = None;
        let mut selected = false;
        for i in 0..self.layout.layers().len() {
            let info = self.layout.layer_info(i);
            let id = info.id;
            let layer_name = info.name.clone();
            let Some(p) =
                self.capture.layers.get_mut(&id).filter(|p| {
                    p.texture.is_some() && p.source == self.layout.formats().source_for[i]
                })
            else {
                continue;
            };
            let row_y = y + 44. + row as f32 * INPUT_REFERENCE_ROW_HEIGHT;
            row += 1;
            let rect = Rect::from_min_size(
                ink.point(0., row_y),
                Vec2::new(w, INPUT_REFERENCE_ROW_HEIGHT - 6.),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut p.enabled, "")
                        .on_hover_text(tr(ui.ctx(), "ui.enable-input-preview"));
                    let label = format!("{}: {}", layer_name, p.name);
                    if ui
                        .add_sized(
                            Vec2::new((ui.available_width() - 32.0).max(20.0), 24.0),
                            egui::Button::new(label).truncate(),
                        )
                        .on_hover_text(tr(ui.ctx(), "ui.reference-picture-help"))
                        .clicked()
                    {
                        self.layout.select(Some(i));
                        self.reference_selected = None;
                        self.edit_content = false;
                        selected = true;
                    }
                    if ui
                        .small_button("×")
                        .on_hover_text(tr(ui.ctx(), "ui.remove-reference-image"))
                        .clicked()
                    {
                        remove = Some(id);
                    }
                });
            });
        }
        if let Some(id) = remove {
            self.capture.layers.remove(&id);
        }
        if selected {
            self.sync();
        }
    }
    pub(super) fn crop_preview_controls(&mut self, ui: &mut egui::Ui, i: usize) {
        if ui
            .button(tr(ui.ctx(), "ui.load-reference-picture"))
            .on_hover_text(tr(ui.ctx(), "ui.reference-picture-help"))
            .clicked()
        {
            self.import_layer_reference_picture(ui.ctx(), i);
        }
        let local_picture = self
            .capture
            .layers
            .get(&self.layout.layer_info(i).id)
            .is_some_and(|p| p.input == 0 && !p.bytes.is_empty());
        if local_picture {
            ui.small(tr(ui.ctx(), "ui.reference-picture-help"));
        }
        let id = self.layout.layer_info(i).id;
        let source = self.layout.formats().source_for[i];
        let inputs = self
            .device
            .snapshot
            .as_ref()
            .map(device_capture::inputs)
            .unwrap_or_default();
        let online = self.device.online();
        let idle = self.capture.pending.is_none();
        let cooling = self.capture.cooling_down();
        let chosen = self.selected_physical_input(i);
        let choice =
            self.device
                .source_choices
                .entry(id)
                .or_insert(super::device_ui::SourceChoice {
                    input: chosen,
                    ..Default::default()
                });
        ui.checkbox(&mut choice.follow,tr(ui.ctx(), "ui.follow-input")).on_hover_text("Use the input selected in Source properties for this preview. This does not route the device layer.");
        let following = choice.follow;
        let preview = self
            .capture
            .layers
            .entry(id)
            .or_insert_with(|| LayerPreview {
                input: 0,
                host: self.device.host.clone(),
                source,
                texture: None,
                bytes: Vec::new(),
                paused: false,
                name: String::new(),
                enabled: false,
                automatic: false,
                capture_on_selection: false,
                guides: 0.35,
            });
        if preview.source != source {
            preview.source = source;
            preview.host = self.device.host.clone();
            preview.texture = None;
            preview.bytes.clear();
            preview.name.clear();
            preview.automatic = false;
            preview.capture_on_selection = false;
        }
        if ui
            .checkbox(
                &mut preview.enabled,
                tr(ui.ctx(), "ui.enable-input-preview"),
            )
            .changed()
            && preview.enabled
            && following
        {
            preview.capture_on_selection = preview.input != 0;
            preview.paused = false;
        }
        let mut capture = false;
        let mut changed_input = None;
        ui.add_enabled_ui(preview.enabled, |ui| {
            let before = preview.input;
            ui.add_enabled_ui(!following, |ui| {input_picker(ui, ("layer-preview-input", id), &mut preview.input, &inputs, &mut self.capture.show_empty_inputs);});
            if before != preview.input {
                changed_input = Some(preview.input);
                preview.host = self.device.host.clone();
                preview.paused = false;
                preview.texture = None;
                preview.bytes.clear();
                preview.name.clear();
                preview.capture_on_selection = true;
                ui.ctx().request_repaint();
            }
            let available = online && inputs.iter().any(|input| input.id == preview.input && input.enabled);
            capture = ui.add_enabled(available && idle && !cooling, egui::Button::new(tr(ui.ctx(), "ui.capture-still")))
                .on_hover_text(tr(ui.ctx(), "ui.fetch-the-selected-input-and-replace-this-layer-s-preview-directly")).clicked();
            if ui.add_enabled(available || preview.automatic, egui::Checkbox::new(&mut preview.automatic, "Auto-refresh this preview")).changed() {
                preview.paused = false;
                preview.host = self.device.host.clone();
            }
            if preview.paused && preview.automatic {
                ui.small(tr(ui.ctx(), "ui.saved-auto-refresh-is-paused"));
                if ui.add_enabled(available, egui::Button::new(tr(ui.ctx(), "ui.resume-auto-refresh"))).clicked() {
                    preview.paused = false;
                    preview.host = self.device.host.clone();
                }
            }
            ui.add_enabled(preview.automatic, egui::Slider::new(&mut self.capture.refresh.seconds, 5..=300).text(tr(ui.ctx(), "ui.request-interval")).suffix(tr(ui.ctx(), "ui.s")));
            ui.small(tr(ui.ctx(), "ui.one-input-per-interval-across-automatic-previews-shared-inputs-use-one-request-leave"));
            let has_image = preview.enabled && preview.texture.is_some();
            let mut opacity = if has_image { preview.guides * 100.0 } else { 100.0 };
            if ui.add_enabled(has_image, egui::Slider::new(&mut opacity, 0.0..=100.0).text(tr(ui.ctx(), "ui.crop-guide-opacity")).suffix("%")).changed() {
                preview.guides = opacity / 100.0;
            }
            if !preview.name.is_empty() { ui.small(&preview.name); }
            if !online && !local_picture { ui.small(tr(ui.ctx(), "ui.connect-to-a-device-to-choose-and-capture-an-input")); }
            else if preview.input == 0 && !local_picture { ui.small(tr(ui.ctx(), "ui.choose-an-input-to-capture-its-first-still-automatically")); }
            else if !available && !local_picture { ui.small(tr(ui.ctx(), "ui.input-snapshots-are-unavailable-enable-snapshots-in-webrcs-then-refresh-the-connectio")); }
            if !idle { ui.spinner(); ui.small(tr(ui.ctx(), "ui.fetching-an-input-snapshot")); }
            else if cooling { ui.small(tr(ui.ctx(), "ui.capture-available-again-shortly")); ui.ctx().request_repaint_after(std::time::Duration::from_secs(1)); }
        });
        if let Some(input) = changed_input {
            self.adapt_layer_to_input(i, input);
        }
        if !self.capture.error.is_empty() {
            ui.colored_label(ui.visuals().error_fg_color, &self.capture.error);
        }
        if capture {
            let preview = self.capture.layers.get_mut(&id).unwrap();
            preview.host = self.device.host.clone();
            preview.paused = false;
            self.capture
                .layers
                .get_mut(&id)
                .unwrap()
                .capture_on_selection = false;
            let input = self.capture.layers[&id].input;
            self.start_input_capture(ui.ctx(), input, false);
            self.capture.pending_layer = Some((
                id,
                self.layout.formats().source_for[i],
                input,
                self.device.host.clone(),
            ));
        }
    }
    pub(super) fn capture_input_error(&mut self, error: String) {
        self.capture.error = error;
    }
    pub(super) fn rebind_input_preview_source(&mut self, i: usize) {
        if let Some(preview) = self.capture.layers.get_mut(&self.layout.layer_info(i).id) {
            preview.source = self.layout.formats().source_for[i];
        }
    }
    fn sync_followed_inputs(&mut self) {
        for i in 0..self.layout.layers().len() {
            let id = self.layout.layer_info(i).id;
            let Some(choice) = self.device.source_choices.get(&id).filter(|c| c.follow) else {
                continue;
            };
            let Some(preview) = self.capture.layers.get_mut(&id) else {
                continue;
            };
            if preview.input != choice.input || preview.host != self.device.host {
                preview.input = choice.input;
                preview.host = self.device.host.clone();
                preview.texture = None;
                preview.bytes.clear();
                preview.name.clear();
                preview.capture_on_selection = preview.enabled && choice.input != 0;
                preview.paused = false;
            }
        }
    }
    pub(super) fn capture_window(&mut self, ctx: &egui::Context) {
        self.sync_followed_inputs();
        if self.capture.host != self.device.host || !self.device.online() {
            self.capture.discard_result = true;
            self.capture.preview = None;
            self.capture.capture_on_selection = false;
            for preview in self.capture.layers.values_mut() {
                preview.capture_on_selection = false;
            }
            self.capture.host = self.device.host.clone();
        }
        if let Some(rx) = &self.capture.pending {
            let result = match rx.try_recv() {
                Ok(result) => Some(result),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(_) => Some(Err("Input capture stopped unexpectedly.".into())),
            };
            if let Some(result) = result {
                self.capture.pending = None;
                let automatic = self.capture.refresh.pending_input.take();
                let direct = self.capture.pending_layer.take();
                let external = self.capture.external_input.take();
                let input = direct
                    .as_ref()
                    .map(|target| target.2)
                    .or(automatic)
                    .or(external)
                    .unwrap_or(self.capture.input);
                // Pool by device/input, including targets selected while this request ran.
                // Consume their one-shot requests on failure too: one failed fetch must
                // not turn into one retry per waiting layer.
                for (id, preview) in &mut self.capture.layers {
                    if preview.enabled
                        && preview.input == input
                        && preview.host == self.device.host
                        && (0..self.layout.layers().len()).any(|i| {
                            self.layout.layer_info(i).id == *id
                                && self.layout.formats().source_for[i] == preview.source
                        })
                    {
                        let requested = preview.capture_on_selection
                            || (preview.automatic && !preview.paused)
                            || direct.as_ref().is_some_and(|(target, source, _, host)| {
                                *target == *id && *source == preview.source && *host == preview.host
                            });
                        preview.capture_on_selection = false;
                        if requested
                            && !self.capture.discard_result
                            && let Ok(reference) = &result
                        {
                            preview.texture = Some(reference.texture.clone());
                            preview.bytes = reference.bytes.clone();
                            preview.name = reference.name.clone();
                        }
                    }
                }
                let dialog_requested = self.capture.open
                    && self.capture.input == input
                    && self.capture.capture_on_selection;
                if dialog_requested {
                    self.capture.capture_on_selection = false;
                }
                if let Some(input) = external
                    && !self.capture.discard_result
                    && self.inputs.open
                    && self.inputs.live_capture
                    && !self.device.busy()
                    && self
                        .inputs
                        .capture_key
                        .as_ref()
                        .is_some_and(|(host, id, plug)| {
                            host == &self.device.host
                                && *id == input
                                && self.inputs.selected == input
                                && self
                                    .device
                                    .snapshot
                                    .as_ref()
                                    .and_then(|s| {
                                        s.raw.pointer(&format!(
                                            "/device/inputList/items/{}_{input}/status/pp/plug",
                                            if s.profile == canvas_pilot::device::Profile::Aquilon6
                                            {
                                                "IN"
                                            } else {
                                                "INPUT"
                                            }
                                        ))
                                    })
                                    .and_then(serde_json::Value::as_str)
                                    == Some(plug.as_str())
                        })
                {
                    match &result {
                        Ok(reference) => {
                            self.inputs.capture_geometry = self
                                .capture
                                .external_geometry
                                .take()
                                .and_then(|rx| rx.try_recv().ok())
                                .flatten();
                            self.inputs.capture = Some(reference.clone());
                        }
                        Err(error) => self.inputs.message = error.clone(),
                    }
                }
                if self.capture.discard_result {
                    self.capture.discard_result = false;
                } else {
                    match result {
                        Ok(reference) => {
                            if dialog_requested
                                || (external.is_none() && direct.is_none() && automatic.is_none())
                            {
                                self.capture.preview = Some(reference);
                            }
                        }
                        Err(error) => self.capture.error = error,
                    }
                }
            }
        }
        if let Some((input, direct)) = self.next_selection_capture() {
            self.start_input_capture(ctx, input, false);
            self.capture.pending_layer = direct;
        }
        if let Some(input) = self.next_preview_refresh() {
            self.start_input_capture(ctx, input, true);
        }
        if self
            .capture
            .layers
            .values()
            .any(|p| p.enabled && ((p.automatic && !p.paused) || p.capture_on_selection))
            || self.capture.capture_on_selection
            || (self.inputs.open && self.inputs.live_capture)
        {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        if !self.capture.open {
            return;
        }
        let cooling_down = self.capture.cooling_down();
        let inputs = self
            .device
            .snapshot
            .as_ref()
            .map(device_capture::inputs)
            .unwrap_or_default();
        if !inputs.iter().any(|i| i.id == self.capture.input) {
            let before = self.capture.input;
            self.capture.input = inputs
                .iter()
                .find(|i| i.signal && i.enabled)
                .or_else(|| {
                    inputs
                        .iter()
                        .find(|i| self.capture.show_empty_inputs || i.signal)
                })
                .map(|i| i.id)
                .unwrap_or(0);
            if before != self.capture.input && self.capture.preview.is_none() {
                self.capture.capture_on_selection = true;
                ctx.request_repaint();
            }
        }
        if self.capture.target_layer.is_some_and(|id| {
            !(0..self.layout.layers().len()).any(|i| self.layout.layer_info(i).id == id)
        }) {
            self.capture.target_layer = None;
        }
        let mut open = true;
        let mut capture = false;
        let mut add = false;
        let mut use_layer = false;
        egui::Window::new(tr(ctx, "ui.import-input-snapshot")).id(egui::Id::new("Import input snapshot")).open(&mut open).default_width(560.0).collapsible(false).show(ctx, |ui| {
            ui.label(tr(ui.ctx(), "ui.capture-an-input-snapshot-then-import-it-into-a-layer-or-add-it-as-a-fixed-reference"));
            ui.small(tr(ui.ctx(), "ui.snapshots-are-reduced-resolution-images-not-a-live-feed-their-size-depends-on-the-dev"));
            let before = self.capture.input;
            ui.add_enabled_ui(self.capture.pending.is_none(), |ui| {
            input_picker(ui, "import-snapshot-input", &mut self.capture.input, &inputs, &mut self.capture.show_empty_inputs);
            });
            if before != self.capture.input { self.capture.preview = None; self.capture.capture_on_selection = true; self.capture.error.clear(); ctx.request_repaint(); }
            let input = inputs.iter().find(|i| i.id == self.capture.input);
            if input.is_some_and(|i| !i.enabled) {
                ui.label(tr(ui.ctx(), "ui.snapshots-are-disabled-for-this-input-enable-input-snapshots-in-webrcs-and-refresh-th"));
            }
            ui.label(tr(ui.ctx(), "ui.destination"));
            egui::ComboBox::from_id_salt("snapshot-destination").selected_text(
                self.capture.target_layer.and_then(|id| (0..self.layout.layers().len()).find(|&i| self.layout.layer_info(i).id == id))
                    .map(|i| format!("Layer: {}", self.layout.layer_label(i))).unwrap_or_else(|| "Reference image".into())
            ).show_ui(ui, |ui| {
                ui.stable_selectable_value(&mut self.capture.target_layer, None, tr(ui.ctx(), "ui.reference-image"));
                for i in 0..self.layout.layers().len() {
                    ui.stable_selectable_value(&mut self.capture.target_layer, Some(self.layout.layer_info(i).id), format!("Layer: {}", self.layout.layer_label(i)));
                }
            });
            if self.capture.target_layer.is_some() {
                ui.checkbox(&mut self.capture.keep_refreshing, tr(ui.ctx(), "ui.keep-refreshing-this-layer"));
                ui.add_enabled(self.capture.keep_refreshing, egui::Slider::new(&mut self.capture.refresh.seconds, 5..=300).text(tr(ui.ctx(), "ui.request-interval")).suffix(tr(ui.ctx(), "ui.s")));
            } else {
                ui.small(tr(ui.ctx(), "ui.reference-images-are-fixed-stills-and-never-auto-refresh"));
            }
            ui.horizontal(|ui| {
                capture = ui.add_enabled(!cooling_down && self.capture.pending.is_none() && input.is_some_and(|i| i.enabled), egui::Button::new(tr(ui.ctx(), "ui.capture-still"))).clicked();
                if self.capture.pending.is_some() {
                    ui.spinner();
                    if ui.button(tr(ui.ctx(), "ui.cancel-capture")).clicked() { self.capture.discard_result = true; }
                }
            });
            ui.small(tr(ui.ctx(), "ui.at-most-one-request-at-a-time-with-at-least-5-seconds-between-requests"));

            if cooling_down { ui.small(tr(ui.ctx(), "ui.capture-available-again-shortly")); ctx.request_repaint_after(std::time::Duration::from_secs(1)); }
            if let Some(reference) = &self.capture.preview {
                ui.label(format!("{} · {} × {} px", reference.name, reference.size[0], reference.size[1]));
                ui.add(egui::Image::new(&reference.texture).max_size(Vec2::new(512.0, 320.0)));
                if self.capture.target_layer.is_none() && let Some((severe, warning)) = reference.warning(self.layout.formats().output) {
                    ui.colored_label(if severe { ui.visuals().error_fg_color } else { theme::palette(ctx).amber }, warning);
                }
                if self.capture.target_layer.is_some() {
                    use_layer = ui.button(tr(ui.ctx(), "ui.import-into-layer")).clicked();
                } else {
                    add = ui.button(tr(ui.ctx(), "ui.add-as-reference")).clicked();
                }
            }
            if !self.capture.error.is_empty() { ui.colored_label(ui.visuals().error_fg_color, &self.capture.error); }
        });
        self.capture.open = open;
        if !open
            && self.capture.pending_layer.is_none()
            && self.capture.refresh.pending_input.is_none()
        {
            self.capture.discard_result = true;
        }
        if capture {
            self.capture.capture_on_selection = false;
            self.start_input_capture(ctx, self.capture.input, false);
        }
        if use_layer
            && let Some(id) = self.capture.target_layer
            && let Some(i) =
                (0..self.layout.layers().len()).find(|&i| self.layout.layer_info(i).id == id)
            && let Some(reference) = &self.capture.preview
        {
            self.capture.layers.insert(
                self.layout.layer_info(i).id,
                LayerPreview {
                    input: self.capture.input,
                    host: self.device.host.clone(),
                    source: self.layout.formats().source_for[i],
                    texture: Some(reference.texture.clone()),
                    bytes: reference.bytes.clone(),
                    paused: false,
                    automatic: self.capture.keep_refreshing,
                    capture_on_selection: false,
                    name: reference.name.clone(),
                    enabled: true,
                    guides: 0.35,
                },
            );
            self.capture.open = false;
        }
        if add && let Some(reference) = self.capture.preview.take() {
            if self.references.len() + self.reference_pending.len() >= 32
                || self.references.iter().map(|r| r.bytes.len()).sum::<usize>()
                    + reference.bytes.len()
                    > 64 * 1024 * 1024
            {
                self.capture.error = "Reference image limit reached; remove an image first.".into();
                self.capture.preview = Some(reference);
            } else {
                self.references.push(reference);
                self.reference_selected = Some(self.references.len() - 1);
                self.capture.open = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click_text, frame};
    use super::*;

    #[test]
    fn local_reference_picture_survives_project_roundtrip_without_capture_or_geometry_changes() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        let before = app.layout.layers().clone();
        let formats = app.layout.formats().clone();
        let reference = tiny_reference(&ctx, "offline.png");
        let bytes = reference.bytes.clone();
        let id = app.layout.layer_info(0).id;
        app.device.source_choices.insert(
            id,
            super::super::device_ui::SourceChoice {
                input: 4,
                change: false,
                follow: true,
            },
        );
        app.set_reference_picture(0, reference);
        app.sync_followed_inputs();
        assert!(!app.device.source_choices[&id].follow);
        assert!(!app.device.source_choices[&id].change);
        assert_eq!(*app.layout.layers(), before);
        assert_eq!(*app.layout.formats(), formats);
        assert_eq!(app.input_reference_count(), 1);
        assert!(app.layer_preview(0).is_some());
        assert!(app.next_selection_capture().is_none());
        assert!(app.next_preview_refresh().is_none());
        let saved = app.project();
        let json = saved.to_json().unwrap();
        let mut restored = Editor::default();
        restored
            .install_project(
                &ctx,
                canvas_pilot::project::Project::from_json(&json).unwrap(),
            )
            .unwrap();
        assert_eq!(restored.project(), saved);
        assert_eq!(restored.capture.layers[&id].bytes, bytes);
        assert_eq!(restored.capture.layers[&id].input, 0);
        assert!(restored.capture.layers[&id].host.is_empty());
        assert!(!restored.capture.layers[&id].automatic);
        assert!(restored.layer_preview(0).is_some());
        restored.capture.layers.get_mut(&id).unwrap().enabled = false;
        assert!(restored.layer_preview(0).is_none());
        assert_eq!(restored.input_reference_count(), 1);
        restored.capture.layers.remove(&id);
        assert_eq!(restored.input_reference_count(), 0);
    }

    #[test]
    fn input_reference_list_selects_its_layer() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        app.set_reference_picture(0, tiny_reference(&ctx, "offline.png"));
        app.layout.select(Some(1));
        let label = format!("{}: offline.png", app.layout.layer_info(0).name);
        click_text(&mut app, &ctx, &label);
        assert_eq!(app.layout.selected(), Some(0));
        assert!(app.reference_selected.is_none());
        assert!(app.layer_preview(0).is_some());
    }

    #[test]
    fn local_reference_rejects_a_capture_already_in_flight() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        let id = app.layout.layer_info(0).id;
        let source = app.layout.formats().source_for[0];
        let (tx, rx) = std::sync::mpsc::channel();
        app.capture.pending = Some(rx);
        app.capture.pending_layer = Some((id, source, 4, app.device.host.clone()));
        app.set_reference_picture(0, tiny_reference(&ctx, "offline.png"));
        tx.send(Ok(tiny_reference(&ctx, "late device image")))
            .unwrap();
        app.capture_window(&ctx);
        assert_eq!(app.capture.layers[&id].name, "offline.png");
        assert!(app.capture.pending.is_none());
    }

    #[test]
    fn following_source_invalidates_old_capture_without_routing() {
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(Default::default(), |ui| app.crop_preview_controls(ui, 0));
        let id = app.layout.layer_info(0).id;
        let preview = app.capture.layers.get_mut(&id).unwrap();
        preview.enabled = true;
        preview.input = 1;
        preview.bytes = vec![1, 2, 3];
        let choice = app.device.source_choices.get_mut(&id).unwrap();
        choice.input = 5;
        choice.follow = true;
        app.sync_followed_inputs();
        let preview = &app.capture.layers[&id];
        assert_eq!(preview.input, 5);
        assert!(preview.bytes.is_empty());
        assert!(preview.capture_on_selection);
        assert!(!app.device.source_choices[&id].change);
        app.device.source_choices.get_mut(&id).unwrap().follow = false;
        app.device.source_choices.get_mut(&id).unwrap().input = 6;
        app.sync_followed_inputs();
        assert_eq!(app.capture.layers[&id].input, 5);
    }
    #[test]
    fn project_restores_stills_and_resumes_auto_refresh_only_on_matching_connection() {
        use canvas_pilot::project::Project;
        let ctx = egui::Context::default();
        let reference = tiny_reference(&ctx, "Input 4 still");
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        app.device.host = "192.168.10.146:3000".into();
        let id = app.layout.layer_info(0).id;
        app.capture.layers.insert(
            id,
            LayerPreview {
                input: 4,
                host: app.device.host.clone(),
                source: app.layout.formats().source_for[0],
                texture: Some(reference.texture),
                bytes: reference.bytes.clone(),
                paused: false,
                automatic: true,
                capture_on_selection: false,
                name: reference.name,
                enabled: true,
                guides: 0.27,
            },
        );
        app.capture.refresh.seconds = 31;
        app.capture.show_empty_inputs = true;
        app.snapping.grid_divisions = [13, 7];
        let saved = app.project();
        let mut restored = Editor::default();
        restored
            .install_project(&ctx, Project::from_json(&saved.to_json().unwrap()).unwrap())
            .unwrap();
        assert_eq!(restored.project(), saved);
        assert_eq!(restored.capture.layers[&id].bytes, reference.bytes);
        assert!(restored.layer_preview(0).is_some());
        assert!(!restored.capture.layers[&id].paused);
        assert!(restored.capture.pending.is_none());
        assert!(restored.capture.last_request.is_none());
        restored.device.host = app.device.host;
        restored.device.snapshot = Some(canvas_pilot::device::Snapshot {
            profile: Default::default(),
            identity: "Eikos4K".into(),
            screens: Vec::new(),
            raw: serde_json::json!({"device":{"inputList":{"items":{
                "INPUT_4":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}}}
            }}}}),
        });
        assert!(restored.next_selection_capture().is_none());
        assert_eq!(restored.next_preview_refresh(), Some(4));
        let host = restored.device.host.clone();
        restored.device.host = "other-device:3000".into();
        assert!(restored.next_preview_refresh().is_none());
        restored.device.host = host;
        restored.capture.layers.get_mut(&id).unwrap().enabled = false;
        assert!(restored.next_preview_refresh().is_none());
        restored.capture.layers.get_mut(&id).unwrap().enabled = true;
        restored.capture.layers.get_mut(&id).unwrap().automatic = false;
        assert!(restored.next_preview_refresh().is_none());
        restored.capture.layers.get_mut(&id).unwrap().automatic = true;
        assert_eq!(restored.next_preview_refresh(), Some(4));
        let mut invalid = saved.clone();
        invalid.workspace.previews[0].image_base64 = "bm90IGFuIGltYWdl".into();
        assert!(restored.install_project(&ctx, invalid).is_err());
        assert_eq!(restored.project(), saved);
        restored
            .install_project(&ctx, Project::from_layout(&Layout::default()))
            .unwrap();
        assert!(restored.capture.layers.is_empty());
        assert!(restored.layer_preview(0).is_none());
    }

    #[test]
    fn saving_ignores_stale_capture_sources_and_unfinished_connection_text() {
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        let id = app.layout.layer_info(0).id;
        app.capture.layers.insert(
            id,
            LayerPreview {
                input: 0,
                host: "192.168.".into(),
                source: app.layout.formats().source_for[0],
                texture: None,
                bytes: Vec::new(),
                paused: false,
                automatic: false,
                capture_on_selection: false,
                name: String::new(),
                enabled: false,
                guides: 0.35,
            },
        );
        let saved = app.project();
        assert!(saved.workspace.previews[0].host.is_empty());
        assert!(saved.to_json().is_ok());
        app.capture.layers.get_mut(&id).unwrap().source = usize::MAX;
        assert!(app.project().workspace.previews.is_empty());
    }

    #[test]
    fn linked_input_preselection_is_inert_and_preserves_operator_choices() {
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        let ctx = egui::Context::default();
        let geometry = app.layout.layers()[0];
        app.device.host = "127.0.0.1:3000".into();
        app.layout
            .set_device_link(
                0,
                Some(canvas_pilot::DeviceLink {
                    family: Default::default(),
                    screen: "2".into(),
                    layer: "1".into(),
                }),
            )
            .unwrap();
        app.device.snapshot = Some(canvas_pilot::device::Snapshot {
            profile: Default::default(),
            identity: "Test".into(),
            screens: vec![canvas_pilot::device::Screen {
                id: "2".into(),
                resolution: [1920, 1080],
                preview_bank: "UP".into(),
                program_bank: "DOWN".into(),
                layers: vec![canvas_pilot::device::Layer {
                    fit: None,
                    mask: Default::default(),
                    id: "1".into(),
                    source: "INPUT_4".into(),
                    geometry,
                    container: geometry,
                    framing: None,
                    unsupported: None,
                    warning: None,
                }],
            }],
            raw: serde_json::json!({"device":{"inputList":{"items":{
                "INPUT_4":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}}}
            }}}}),
        });
        let id = app.layout.layer_info(0).id;
        app.preselect_linked_capture_inputs();
        let preview = &app.capture.layers[&id];
        assert_eq!(preview.input, 4);
        assert!(!preview.enabled && !preview.automatic && !preview.capture_on_selection);
        assert!(preview.texture.is_none());
        app.capture_window(&ctx);
        assert!(app.capture.pending.is_none());
        assert!(app.capture.last_request.is_none());
        app.capture.layers.get_mut(&id).unwrap().enabled = true;
        app.capture_window(&ctx);
        assert!(
            app.capture.pending.is_none(),
            "Enabling a preselected preview does not fetch"
        );
        app.capture.layers.get_mut(&id).unwrap().input = 7;
        app.preselect_linked_capture_inputs();
        assert_eq!(app.capture.layers[&id].input, 7);
        app.capture.layers.clear();
        for source in ["NONE", "COLOR_1", "INPUT_99"] {
            app.device.snapshot.as_mut().unwrap().screens[0].layers[0].source = source.into();
            app.preselect_linked_capture_inputs();
            assert!(!app.capture.layers.contains_key(&id));
        }
    }

    fn tiny_reference(
        ctx: &egui::Context,
        name: &str,
    ) -> super::super::super::reference::Reference {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::new(2, 2)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        super::super::super::reference::Reference::load(
            ctx,
            &egui::DroppedFile {
                name: name.into(),
                bytes: Some(bytes.into_inner().into()),
                ..Default::default()
            },
        )
        .unwrap()
    }
    #[test]
    fn direct_preview_workflow_replaces_only_its_layer_and_auto_can_start_empty() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        app.device.host = "127.0.0.1".into();
        app.capture.host = app.device.host.clone();
        app.device.snapshot = Some(canvas_pilot::device::Snapshot {
            profile: Default::default(),
            identity: "Test".into(),
            screens: vec![],
            raw: serde_json::json!({"device":{"inputList":{"items":{
                "INPUT_3":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}},"plugList":{"items":{"1":{"status":{"signal":{"pp":{"isValid":true}}}}}}},
                "INPUT_7":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}}}
            }}}}),
        });
        let i = app.layout.selected().unwrap();
        let id = app.layout.layer_info(i).id;
        let source = app.layout.formats().source_for[i];
        app.format_draft = Some(FormatDraft::new(&app.layout, i));
        app.property_tab = PropertyTab::Preview;
        // The frozen clock holds cooldown across arbitrarily slow GUI frames.
        app.capture.last_request = Some(app.capture.now());
        frame(&mut app, &ctx, vec![]);
        assert!(!app.capture.layers[&id].enabled);
        click_text(&mut app, &ctx, "Enable input preview");
        click_text(&mut app, &ctx, "Choose input…");
        click_text(&mut app, &ctx, "Input 3");
        assert!(app.capture.layers[&id].enabled);
        assert_eq!(app.capture.layers[&id].input, 3);
        assert!(app.capture.layers[&id].capture_on_selection);
        assert_eq!(app.next_selection_capture(), None); // five-second throttle
        app.capture.refresh.seconds = 300;
        app.capture.last_request = Some(app.capture.now() - std::time::Duration::from_secs(6));
        // An in-flight request still blocks the selection capture, even after cooldown.
        let (_tx, rx) = std::sync::mpsc::channel();
        app.capture.pending = Some(rx);
        assert_eq!(app.next_selection_capture(), None);
        app.capture.pending = None;
        assert_eq!(
            app.next_selection_capture(),
            Some((3, Some((id, source, 3, app.device.host.clone()))))
        );
        assert_eq!(app.next_selection_capture(), None); // one still, not recurring
        app.capture.last_request = Some(app.capture.now());
        assert!(app.layer_preview(i).is_none());
        let output = frame(&mut app, &ctx, vec![]);
        let y = |label| {
            output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos.y),
                    _ => None,
                })
                .unwrap()
        };
        assert!(y("Input 3") < y("Capture still"));
        assert!(y("Capture still") < y("Auto-refresh this preview"));
        click_text(&mut app, &ctx, "Auto-refresh this preview");
        app.capture.last_request = None;
        assert_eq!(app.next_preview_refresh(), Some(3)); // no initial still required
        app.capture.last_request = Some(app.capture.now());
        click_text(&mut app, &ctx, "Auto-refresh this preview");
        assert_eq!(app.next_preview_refresh(), None);
        app.format_draft = None;
        app.layout.select(Some(1 - i)); // completion stays bound to the original layer
        let mut previous = None;
        for name in ["first still", "recaptured still"] {
            let (tx, rx) = std::sync::mpsc::channel();
            app.capture.pending = Some(rx);
            app.capture.pending_layer = Some((id, source, 3, app.device.host.clone()));
            tx.send(Ok(tiny_reference(&ctx, name))).unwrap();
            app.capture_window(&ctx);
            let texture = app.layer_preview(i).unwrap().0;
            assert_ne!(previous, Some(texture));
            previous = Some(texture);
            assert_eq!(app.capture.layers[&id].name, name);
            assert!(app.layer_preview(1 - i).is_none());
            assert!(app.capture.preview.is_none());
            assert!(app.references.is_empty());
            assert!(!app.capture.open);
        }
        // Changing the chosen input while a fetch is pending must reject its image.
        let (tx, rx) = std::sync::mpsc::channel();
        app.capture.pending = Some(rx);
        app.capture.pending_layer = Some((id, source, 3, app.device.host.clone()));
        app.capture.layers.get_mut(&id).unwrap().input = 7;
        tx.send(Ok(tiny_reference(&ctx, "stale input"))).unwrap();
        app.capture_window(&ctx);
        assert_eq!(app.capture.layers[&id].name, "recaptured still");
    }
    #[test]
    fn selection_capture_coalesces_and_cancels_invalid_targets() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        app.device.snapshot = Some(canvas_pilot::device::Snapshot {
            profile: Default::default(),
            identity: "Test".into(),
            screens: vec![],
            raw: serde_json::json!({"device":{"inputList":{"items":{
                "INPUT_3":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}}},
                "INPUT_7":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}}}
            }}}}),
        });
        app.capture.host = app.device.host.clone();
        app.capture.last_request = Some(app.capture.now());
        app.capture.open = true;
        app.capture.input = 3;
        app.capture.capture_on_selection = true;
        assert_eq!(app.next_selection_capture(), None);
        app.capture.input = 7; // only the most recent selection is captured
        app.capture.last_request = None;
        assert_eq!(app.next_selection_capture(), Some((7, None)));
        assert_eq!(app.next_selection_capture(), None);
        app.capture.capture_on_selection = true;
        app.capture.open = false;
        assert_eq!(app.next_selection_capture(), None);
        let i = app.layout.selected().unwrap();
        let id = app.layout.layer_info(i).id;
        app.format_draft = Some(FormatDraft::new(&app.layout, i));
        app.property_tab = PropertyTab::Preview;
        frame(&mut app, &ctx, vec![]);
        let p = app.capture.layers.get_mut(&id).unwrap();
        p.enabled = true;
        p.input = 3;
        p.capture_on_selection = true;
        p.host = "different-host".into();
        assert_eq!(app.next_selection_capture(), None);
        let p = app.capture.layers.get_mut(&id).unwrap();
        p.host = app.device.host.clone();
        p.capture_on_selection = true;
        p.enabled = false;
        assert_eq!(app.next_selection_capture(), None);
        let p = app.capture.layers.get_mut(&id).unwrap();
        p.enabled = true;
        p.capture_on_selection = true;
        p.source += 1;
        assert_eq!(app.next_selection_capture(), None);
        app.capture.capture_on_selection = true;
        app.capture.clear_preview();
        assert!(!app.capture.capture_on_selection);
    }
    #[test]
    fn live_input_refresh_stops_when_closed_disabled_or_sending() {
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        app.device.snapshot = Some(canvas_pilot::device::Snapshot {
            profile: Default::default(),
            identity: "test".into(),
            screens: vec![],
            raw: serde_json::json!({"device":{"inputList":{"items":{
                "INPUT_3":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}}}
            }}}}),
        });
        app.inputs.selected = 3;
        app.inputs.live_capture = true;
        assert_eq!(app.next_preview_refresh(), None);
        app.inputs.open = true;
        assert_eq!(app.next_preview_refresh(), Some(3));
        app.inputs.live_capture = false;
        assert_eq!(app.next_preview_refresh(), None);
        app.inputs.live_capture = true;
        app.device.input_sending = true;
        assert_eq!(app.next_preview_refresh(), None);
        app.device.input_sending = false;
        app.capture.last_request = Some(app.capture.now());
        assert_eq!(app.next_preview_refresh(), None);
    }

    #[test]
    fn invalidated_capture_is_not_reused_after_send() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        app.device.snapshot = Some(canvas_pilot::device::Snapshot {
            profile: Default::default(),
            identity: "test".into(),
            screens: vec![],
            raw: serde_json::json!({}),
        });
        app.inputs.open = true;
        app.inputs.live_capture = true;
        app.inputs.selected = 3;
        // Sending clears the capture key; an already running capture must not reappear.
        app.inputs.capture_key = None;
        app.capture.host = app.device.host.clone();
        app.capture.last_request = Some(app.capture.now());
        app.capture.external_input = Some(3);
        let (tx, rx) = std::sync::mpsc::channel();
        app.capture.pending = Some(rx);
        tx.send(Ok(tiny_reference(&ctx, "old crop"))).unwrap();
        app.capture_window(&ctx);
        assert!(app.inputs.capture.is_none());
    }
    #[test]
    fn captures_pool_waiters_by_input_for_direct_automatic_and_dialog_requests() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        app.device.snapshot = Some(canvas_pilot::device::Snapshot {
            profile: Default::default(),
            identity: "Test".into(),
            screens: vec![],
            raw: serde_json::json!({"device":{"inputList":{"items":{
                "INPUT_3":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}}}
            }}}}),
        });
        app.capture.host = app.device.host.clone();
        let id = app.layout.layer_info(0).id;
        let source = app.layout.formats().source_for[0];
        for i in 0..2 {
            app.capture.layers.insert(
                app.layout.layer_info(i).id,
                LayerPreview {
                    input: 3,
                    host: app.device.host.clone(),
                    source: app.layout.formats().source_for[i],
                    texture: None,
                    bytes: Vec::new(),
                    paused: false,
                    automatic: false,
                    capture_on_selection: true,
                    name: String::new(),
                    enabled: true,
                    guides: 0.35,
                },
            );
        }
        for mode in 0..5 {
            for preview in app.capture.layers.values_mut() {
                preview.capture_on_selection = true;
                preview.texture = None;
                preview.bytes.clear();
            }
            app.capture.last_request = Some(app.capture.now());
            let (tx, rx) = std::sync::mpsc::channel();
            app.capture.pending = Some(rx);
            app.capture.pending_layer =
                (mode == 0 || mode == 3).then(|| (id, source, 3, app.device.host.clone()));
            app.capture.refresh.pending_input = (mode == 1).then_some(3);
            app.capture.input = 3;
            if mode == 4 {
                app.inputs.open = true;
                app.inputs.live_capture = true;
                app.inputs.selected = 3;
                app.inputs.capture_key = Some((app.device.host.clone(), 3, "1".into()));
                app.capture.external_input = Some(3);
                let (geometry_tx, geometry_rx) = std::sync::mpsc::channel();
                geometry_tx
                    .send(Some(super::super::inputs_window::CaptureGeometry {
                        framing: device_capture::CaptureFraming::FullFrame,
                        native: [1920., 1080.],
                        crop: canvas_pilot::formats::Crop::default(),
                    }))
                    .unwrap();
                app.capture.external_geometry = Some(geometry_rx);
            }
            if mode == 3 {
                tx.send(Err("Test failure".into())).unwrap();
            } else {
                tx.send(Ok(tiny_reference(&ctx, "shared capture"))).unwrap();
            }
            app.capture_window(&ctx);
            assert!(app.capture.pending.is_none());
            assert!(app.capture.layers.values().all(|p| !p.capture_on_selection));
            if mode == 3 {
                assert!(app.layer_preview(0).is_none());
                assert!(app.layer_preview(1).is_none());
                assert_eq!(app.capture.error, "Test failure");
            } else {
                assert_eq!(
                    app.layer_preview(0).unwrap().0,
                    app.layer_preview(1).unwrap().0
                );
            }
            if mode == 4 {
                assert_eq!(
                    app.inputs.capture.as_ref().unwrap().texture.id(),
                    app.layer_preview(0).unwrap().0
                );
                assert!(app.inputs.capture_geometry.is_some());
                assert_eq!(app.next_preview_refresh(), None); // shared interval, no duplicate capture
                app.inputs.live_capture = false;
            }
            app.capture.last_request = None;
            assert_eq!(app.next_selection_capture(), None); // no duplicate queued GET
        }
        // A waiter choosing another input during the fetch must keep its own request.
        let second = app.layout.layer_info(1).id;
        let p = app.capture.layers.get_mut(&second).unwrap();
        p.input = 7;
        p.texture = None;
        p.capture_on_selection = true;
        app.device.snapshot.as_mut().unwrap().raw["device"]["inputList"]["items"]["INPUT_7"] = serde_json::json!({"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}}});
        app.capture.last_request = Some(app.capture.now());
        let (tx, rx) = std::sync::mpsc::channel();
        app.capture.pending = Some(rx);
        app.capture.pending_layer = Some((id, source, 3, app.device.host.clone()));
        tx.send(Ok(tiny_reference(&ctx, "input three"))).unwrap();
        app.capture_window(&ctx);
        assert!(app.layer_preview(1).is_none());
        assert!(app.capture.layers[&second].capture_on_selection);
    }
    #[test]
    fn automatic_capture_updates_only_opted_in_layers_even_when_inputs_match() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        app.device.snapshot = Some(canvas_pilot::device::Snapshot {
            profile: Default::default(),
            identity: "Test".into(),
            screens: vec![],
            raw: serde_json::json!({}),
        });
        app.capture.host = app.device.host.clone();
        for i in 0..2 {
            app.capture.layers.insert(
                app.layout.layer_info(i).id,
                LayerPreview {
                    input: 3,
                    host: app.device.host.clone(),
                    source: app.layout.formats().source_for[i],
                    texture: None,
                    bytes: Vec::new(),
                    paused: false,
                    name: String::new(),
                    enabled: true,
                    automatic: i == 0,
                    capture_on_selection: false,
                    guides: 0.35,
                },
            );
        }
        let (tx, rx) = std::sync::mpsc::channel();
        app.capture.pending = Some(rx);
        app.capture.refresh.pending_input = Some(3);
        tx.send(Ok(tiny_reference(&ctx, "automatic first image")))
            .unwrap();
        app.capture_window(&ctx);
        assert!(app.layer_preview(0).is_some());
        assert!(app.layer_preview(1).is_none());
        let id = app.layout.layer_info(0).id;
        app.capture.layers.get_mut(&id).unwrap().automatic = false;
        let before = app.layer_preview(0).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        app.capture.pending = Some(rx);
        app.capture.refresh.pending_input = Some(3);
        tx.send(Ok(tiny_reference(&ctx, "disabled while fetching")))
            .unwrap();
        app.capture_window(&ctx);
        assert_eq!(app.layer_preview(0).unwrap(), before);
    }
    #[test]
    fn snapshot_import_targets_any_layer_or_a_fixed_reference_without_selection() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        app.device.host = "127.0.0.1".into();
        app.capture.host = app.device.host.clone();
        app.device.snapshot = Some(canvas_pilot::device::Snapshot {
            profile: Default::default(),
            identity: "Test".into(),
            screens: vec![],
            raw: serde_json::json!({"device":{"inputList":{"items":{
                "INPUT_3":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}},"plugList":{"items":{"1":{"status":{"signal":{"pp":{"isValid":true}}}}}}}
            }}}}),
        });
        app.layout.select(None);
        app.capture.last_request = Some(app.capture.now());
        for (i, automatic) in [(0, true), (1, false)] {
            app.open_snapshot_import();
            app.capture.preview = Some(tiny_reference(&ctx, "imported snapshot"));
            click_text(&mut app, &ctx, "Reference image");
            let label = format!("Layer: {}", app.layout.layer_label(i));
            click_text(&mut app, &ctx, &label);
            if automatic {
                click_text(&mut app, &ctx, "Keep refreshing this layer");
            }
            click_text(&mut app, &ctx, "Import into layer");
            let id = app.layout.layer_info(i).id;
            assert_eq!(app.capture.layers[&id].automatic, automatic);
            assert!(app.layer_preview(i).is_some());
            assert!(app.layout.selected_indices().is_empty());
            assert!(app.references.is_empty());
        }
        app.open_snapshot_import();
        app.capture.keep_refreshing = true; // ignored for a reference destination
        app.capture.preview = Some(tiny_reference(&ctx, "fixed reference"));
        let output = frame(&mut app, &ctx, vec![]);
        assert!(!output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Keep refreshing this layer")));
        let textures = [app.layer_preview(0), app.layer_preview(1)];
        click_text(&mut app, &ctx, "Add as reference");
        assert_eq!(app.references.len(), 1);
        assert_eq!(app.project().references.len(), 1);
        assert_eq!([app.layer_preview(0), app.layer_preview(1)], textures);
        assert_eq!(app.capture.layers.len(), 2);
    }
    #[test]
    fn both_input_pickers_hide_empty_inputs_and_preserve_explicit_selection() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        app.device.snapshot = Some(canvas_pilot::device::Snapshot {
            profile: Default::default(),
            identity: "Test".into(),
            screens: vec![],
            raw: serde_json::json!({"device":{"inputList":{"items":{
                "INPUT_3":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}},"plugList":{"items":{"1":{"status":{"signal":{"pp":{"isValid":true}}}}}}},
                "INPUT_7":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}}}
            }}}}),
        });
        app.capture.host = app.device.host.clone();
        app.capture.last_request = Some(app.capture.now());
        let i = app.layout.selected().unwrap();
        let id = app.layout.layer_info(i).id;
        app.format_draft = Some(FormatDraft::new(&app.layout, i));
        app.property_tab = PropertyTab::Preview;
        let contains = |output: &egui::FullOutput, label: &str| {
            output
                .shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == label))
        };
        click_text(&mut app, &ctx, "Enable input preview");
        click_text(&mut app, &ctx, "Choose input…");
        let output = frame(&mut app, &ctx, vec![]);
        assert!(contains(&output, "Input 3"));
        assert!(!contains(&output, "Input 7 · No signal"));
        click_text(&mut app, &ctx, "Show inputs without signal");
        assert!(app.capture.show_empty_inputs);
        click_text(&mut app, &ctx, "Input 7 · No signal");
        assert_eq!(app.capture.layers[&id].input, 7);
        assert!(contains(
            &frame(&mut app, &ctx, vec![]),
            "Input 7 · No signal"
        ));
        app.capture.show_empty_inputs = false;
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.capture.layers[&id].input, 7); // filtering never changes a target
        app.format_draft = None;
        app.open_snapshot_import();
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.capture.input, 3);
        click_text(&mut app, &ctx, "Input 3");
        assert!(!contains(
            &frame(&mut app, &ctx, vec![]),
            "Input 7 · No signal"
        ));
        click_text(&mut app, &ctx, "Show inputs without signal");
        click_text(&mut app, &ctx, "Input 7 · No signal");
        assert_eq!(app.capture.input, 7);
        app.capture.show_empty_inputs = false;
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.capture.input, 7);
        app.capture.input = 0;
        app.device.snapshot.as_mut().unwrap().raw["device"]["inputList"]["items"]["INPUT_3"]["plugList"]
            ["items"]["1"]["status"]["signal"]["pp"]["isValid"] = serde_json::json!(false);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.capture.input, 0); // never silently select an empty input
        click_text(&mut app, &ctx, "Choose input…");
        assert!(contains(
            &frame(&mut app, &ctx, vec![]),
            "No inputs with signal. Enable the toggle above to see empty inputs."
        ));
    }
    #[test]
    fn capture_is_manual_throttled_and_only_adds_on_request() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        app.device.host = "127.0.0.1".into();
        app.device.snapshot = Some(canvas_pilot::device::Snapshot {
            profile: Default::default(),
            identity: "Test".into(),
            screens: vec![],
            raw: serde_json::json!({"device":{"inputList":{"items":{"INPUT_3":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}},"plugList":{"items":{"1":{"status":{"signal":{"pp":{"isValid":true}}}}}}}}}}}),
        });
        app.capture.host = app.device.host.clone();
        app.open_snapshot_import();
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let reference = super::super::super::reference::Reference::load(
            &ctx,
            &egui::DroppedFile {
                name: "Captured input".into(),
                bytes: Some(bytes.into_inner().into()),
                ..Default::default()
            },
        )
        .unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        app.capture.pending = Some(rx);
        app.capture.last_request = Some(app.capture.now());
        frame(&mut app, &ctx, vec![]);
        assert!(app.capture.cooling_down());
        assert!(app.references.is_empty());
        tx.send(Ok(reference)).unwrap();
        frame(&mut app, &ctx, vec![]);
        assert!(app.capture.preview.is_some());
        assert!(app.references.is_empty());
        let requested = app.capture.last_request;
        click_text(&mut app, &ctx, "Capture still");
        assert!(app.capture.pending.is_none());
        assert_eq!(app.capture.last_request, requested);
        click_text(&mut app, &ctx, "Import into layer");
        let selected = app.layout.selected().unwrap();
        assert_eq!(app.layer_preview(selected).unwrap().1, 0.35);
        assert!(!app.edit_content);
        assert!(app.references.is_empty());
        // Automatic refresh is opt-in, throttled, and pauses for manual capture.
        assert_eq!(app.next_preview_refresh(), None);
        for preview in app.capture.layers.values_mut() {
            preview.automatic = true;
        }
        assert_eq!(app.next_preview_refresh(), None);
        app.capture.last_request = Some(app.capture.now() - std::time::Duration::from_secs(11));
        assert_eq!(app.next_preview_refresh(), Some(3));
        app.capture.refresh.last_input = 3;
        assert_eq!(app.next_preview_refresh(), Some(3)); // round-robin wraps
        let id = app.layout.layer_info(selected).id;
        app.capture.layers.get_mut(&id).unwrap().enabled = false;
        assert_eq!(app.next_preview_refresh(), None);
        app.capture.layers.get_mut(&id).unwrap().enabled = true;
        app.capture.layers.get_mut(&id).unwrap().host = "other device".into();
        assert_eq!(app.next_preview_refresh(), None);
        app.capture.layers.get_mut(&id).unwrap().host = app.device.host.clone();
        // Failed refresh retains the last good texture and obeys the timer.
        let texture = app.layer_preview(selected).unwrap().0;
        let (tx, rx) = std::sync::mpsc::channel();
        app.capture.pending = Some(rx);
        app.capture.refresh.pending_input = Some(3);
        app.capture.last_request = Some(app.capture.now());
        assert_eq!(app.next_preview_refresh(), None);
        tx.send(Err("temporary failure".into())).unwrap();
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.layer_preview(selected).unwrap().0, texture);
        assert_eq!(app.capture.error, "temporary failure");
        assert!(app.references.is_empty());
        // A successful automatic result updates the texture, never the references.
        let (tx, rx) = std::sync::mpsc::channel();
        app.capture.pending = Some(rx);
        app.capture.refresh.pending_input = Some(3);
        tx.send(Ok(app.capture.preview.take().unwrap())).unwrap();
        frame(&mut app, &ctx, vec![]);
        assert!(app.capture.preview.is_none());
        assert!(app.references.is_empty());
        assert_eq!(app.layer_preview(selected).unwrap().1, 0.35);
        for preview in app.capture.layers.values_mut() {
            preview.automatic = false;
        }
        // Restore a dialog still for the separate Add as reference action.
        let reference = super::super::super::reference::Reference::load(
            &ctx,
            &egui::DroppedFile {
                name: "Captured input".into(),
                bytes: Some({
                    let mut bytes = std::io::Cursor::new(Vec::new());
                    image::RgbaImage::new(2, 2)
                        .write_to(&mut bytes, image::ImageFormat::Png)
                        .unwrap();
                    bytes.into_inner().into()
                }),
                ..Default::default()
            },
        )
        .unwrap();
        app.capture.preview = Some(reference);
        app.capture.target_layer = None;
        app.capture.open = true;
        assert_eq!(app.next_preview_refresh(), None);
        click_text(&mut app, &ctx, "Add as reference");
        assert_eq!(app.references.len(), 1);
        assert_eq!(app.project().references.len(), 1);
        assert!(!app.capture.open);
        app.capture.last_request = Some(app.capture.now() - std::time::Duration::from_secs(6));
        assert!(!app.capture.cooling_down());
    }
    #[test]
    fn refresh_rotates_inputs_and_skips_changed_sources() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        app.device.snapshot = Some(canvas_pilot::device::Snapshot {
            profile: Default::default(),
            identity: "Test".into(),
            screens: vec![],
            raw: serde_json::json!({"device":{"inputList":{"items":{
                "INPUT_3":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}},"plugList":{"items":{"1":{"status":{"signal":{"pp":{"isValid":true}}}}}}},
                "INPUT_7":{"status":{"pp":{"isAvailable":true,"plug":"1"}},"snapshot":{"pp":{"enable":true}}}
            }}}}),
        });
        for preview in app.capture.layers.values_mut() {
            preview.automatic = true;
        }
        for (i, input) in [3, 7].into_iter().enumerate() {
            app.capture.layers.insert(
                app.layout.layer_info(i).id,
                LayerPreview {
                    input,
                    host: app.device.host.clone(),
                    source: app.layout.formats().source_for[i],
                    bytes: Vec::new(),
                    paused: false,
                    texture: Some(ctx.load_texture(
                        format!("Input {input}"),
                        egui::ColorImage::filled([2, 2], Color32::RED),
                        Default::default(),
                    )),
                    automatic: true,
                    capture_on_selection: false,
                    name: String::new(),
                    enabled: true,
                    guides: 0.35,
                },
            );
        }
        let started = app.capture.now();
        app.capture.last_request = Some(started);
        for seconds in [5, 10, 300] {
            app.capture.refresh.seconds = seconds;
            let boundary = started + std::time::Duration::from_secs(seconds);
            app.capture.test_now = Some(boundary - std::time::Duration::from_nanos(1));
            assert_eq!(app.capture.cooling_down(), seconds == 5);
            assert_eq!(app.next_preview_refresh(), None);
            app.capture.test_now = Some(boundary);
            assert!(!app.capture.cooling_down());
            assert_eq!(app.next_preview_refresh(), Some(3));
        }
        app.capture.last_request = None;
        assert_eq!(app.next_preview_refresh(), Some(3));
        app.capture.refresh.last_input = 3;
        assert_eq!(app.next_preview_refresh(), Some(7));
        app.capture.refresh.last_input = 7;
        assert_eq!(app.next_preview_refresh(), Some(3));
        let id = app.layout.layer_info(0).id;
        app.capture.layers.get_mut(&id).unwrap().source = usize::MAX;
        assert_eq!(app.next_preview_refresh(), Some(7));
        app.capture.open = true;
        assert_eq!(app.next_preview_refresh(), None);
        app.capture.open = false;
        app.device.snapshot = None;
        assert_eq!(app.next_preview_refresh(), None);
    }
    #[test]
    fn cancelling_keeps_request_in_flight_and_discards_its_result() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        let (tx, rx) = std::sync::mpsc::channel();
        app.capture.pending = Some(rx);
        app.capture.clear_preview();
        assert!(app.capture.pending.is_some());
        tx.send(Err("cancelled response".into())).unwrap();
        app.capture_window(&ctx);
        assert!(app.capture.pending.is_none());
        assert!(app.capture.error.is_empty());
        assert!(app.references.is_empty());
    }
    #[test]
    fn layer_preview_toggle_keeps_opacity_and_follows_stable_layer_id() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.capture.test_now = Some(std::time::Instant::now());
        let i = app.layout.selected().unwrap();
        let id = app.layout.layer_info(i).id;
        let texture = ctx.load_texture(
            "test preview",
            egui::ColorImage::filled([2, 2], Color32::RED),
            egui::TextureOptions::default(),
        );
        app.capture.layers.insert(
            id,
            LayerPreview {
                input: 3,
                host: app.device.host.clone(),
                source: app.layout.formats().source_for[i],
                texture: Some(texture),
                bytes: Vec::new(),
                paused: false,
                automatic: false,
                capture_on_selection: false,
                name: "Input still".into(),
                enabled: true,
                guides: 0.25,
            },
        );
        assert_eq!(app.layer_preview(i).unwrap().1, 0.25);
        app.capture.layers.get_mut(&id).unwrap().enabled = false;
        assert!(app.layer_preview(i).is_none());
        app.capture.layers.get_mut(&id).unwrap().enabled = true;
        assert_eq!(app.layer_preview(i).unwrap().1, 0.25);
        assert!(app.layer_preview(1 - i).is_none());
        app.capture.layers.get_mut(&id).unwrap().source = usize::MAX;
        assert!(app.layer_preview(i).is_none());
        app.capture.clear_preview();
        assert!(app.capture.layers.is_empty());
    }
}
