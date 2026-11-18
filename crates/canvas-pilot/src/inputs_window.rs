//! Device inputs are edited independently of editor layer selection.
use super::*;
use canvas_pilot::{
    device_inputs::{InputDraft, Kind},
    formats::Crop,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

type Values = BTreeMap<String, Value>;
type CropDrag = (Crop, Pos2, [i8; 2], Values);
type SendJob = std::sync::mpsc::Receiver<Result<canvas_pilot::device::Snapshot, String>>;
pub(super) use canvas_pilot::device_capture::CaptureGeometry;
fn capture_rectangles(
    geometry: CaptureGeometry,
    full: Rect,
    native: [f64; 2],
) -> Option<(Rect, Rect)> {
    if geometry.native != native || geometry.crop.remaining(native).is_err() {
        return None;
    }
    let region = Rect::from_min_max(
        Pos2::new(
            (geometry.crop.left / native[0]) as f32,
            (geometry.crop.top / native[1]) as f32,
        ),
        Pos2::new(
            (1. - geometry.crop.right / native[0]) as f32,
            (1. - geometry.crop.bottom / native[1]) as f32,
        ),
    );
    let rect = Rect::from_min_max(
        full.min + region.min.to_vec2() * full.size(),
        full.min + region.max.to_vec2() * full.size(),
    );
    let uv = match geometry.framing {
        canvas_pilot::device_capture::CaptureFraming::FullFrame => region,
        canvas_pilot::device_capture::CaptureFraming::Cropped => {
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.))
        }
    };
    Some((rect, uv))
}

#[derive(Default)]
pub(super) struct InputsWindow {
    pub open: bool,
    pub selected: u32,
    category: String,
    host: String,
    drafts: BTreeMap<u32, InputDraft>,
    undo: BTreeMap<u32, Vec<Values>>,
    edit_start: Option<Values>,
    drag: Option<CropDrag>,
    pivot: [usize; 2],
    view_zoom: f32,
    view_pan: Vec2,
    pub live_capture: bool,
    pub capture: Option<super::super::reference::Reference>,
    pub capture_geometry: Option<CaptureGeometry>,
    pub capture_key: Option<(String, u32, String)>,
    pending: Option<SendJob>,
    sending_input: u32,
    reading: bool,
    sending_host: String,
    pub message: String,
}
impl Editor {
    pub(super) fn inputs_window(&mut self, ctx: &egui::Context) {
        if self.inputs.host != self.device.host {
            // A running write is never redirected to a new host.
            self.inputs.drafts.clear();
            self.inputs.undo.clear();
            self.inputs.capture = None;
            self.inputs.capture_key = None;
            self.inputs.host = self.device.host.clone();
        }
        if let Some(rx) = &self.inputs.pending {
            match rx.try_recv() {
                Ok(result) => {
                    self.inputs.pending = None;
                    self.device.input_sending = false;
                    if self.inputs.sending_host != self.device.host {
                        self.inputs.message="Input send finished for the previous connection. Refresh that device to inspect the outcome.".into();
                        return;
                    }
                    match result {
                        Ok(snapshot) => {
                            self.inputs.drafts.remove(&self.inputs.sending_input);
                            self.inputs.undo.remove(&self.inputs.sending_input);
                            self.inputs.capture = None;
                            self.inputs.capture_key = None;
                            self.device.snapshot = Some(snapshot);
                            self.inputs.message = if self.inputs.reading {
                                "Input controls loaded. Edits remain local until sent.".into()
                            } else {
                                "Input settings verified. Reload affected layers before sending their layout if the content size changed.".into()
                            };
                        }
                        Err(error) => self.inputs.message = error,
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.inputs.pending = None;
                    self.device.input_sending = false;
                    self.inputs.message =
                        "Input send worker stopped; refresh before retrying.".into();
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        if !self.inputs.open {
            return;
        }
        let capture_pending = self.input_capture_pending();
        let mut open = true;
        let mut send = None;
        let mut refresh = false;
        egui::Window::new("Input settings").id(egui::Id::new("device-input-settings"))
            .open(&mut open).default_size([1100.0,760.0]).min_width(360.0).show(ctx,|ui|{
            ui.small("Shared input settings · changes affect every layer using this input, including Program.");
            let Some(snapshot)=&self.device.snapshot else {
                ui.label("Connect to a supported device to inspect its inputs.");
                if ui.button("Device connection…").clicked(){self.device.connection_open=true;}
                return;
            };

            let inputs=canvas_pilot::device_capture::inputs(snapshot);
            if self.inputs.pending.is_some() {ui.spinner();ui.label("Applying / verifying input settings… Capture will refresh after verification.");return;}

            if self.inputs.selected==0 {self.inputs.selected=inputs.first().map(|i|i.id).unwrap_or(0);}
            ui.horizontal_wrapped(|ui|{
                ui.label("Device input");
                ui.add_enabled_ui(self.inputs.pending.is_none(),|ui|{
                    egui::ComboBox::from_id_salt("device-input-picker").selected_text(format!("Input {}",self.inputs.selected)).show_ui(ui,|ui|{
                        for input in &inputs {
                            ui.visuals_mut().widgets.hovered.bg_stroke.color = egui::Color32::TRANSPARENT;
                            if ui.stable_selectable_value(&mut self.inputs.selected,input.id,format!("Input {}{}{}",input.id,if input.signal{""}else{" · no signal"},if self.inputs.drafts.get(&input.id).is_some_and(InputDraft::dirty){" · edited"}else{""})).changed(){
                                self.inputs.capture=None;self.inputs.capture_key=None;self.inputs.drag=None;self.inputs.edit_start=None;
                            }
                        }
                    });
                });
                if ui.button("Connection…").clicked(){self.device.connection_open=true;}
            });
            let id=self.inputs.selected;
            if snapshot.profile==canvas_pilot::device::Profile::Aquilon6 {
                let root=format!("/device/inputList/items/IN_{id}");
                let plug=snapshot.raw.pointer(&format!("{root}/status/pp/plug")).and_then(Value::as_str).unwrap_or("");
                if snapshot.raw.pointer(&format!("{root}/plugList/items/{plug}/settings/cropping/control/pp/left")).is_none() {
                    ui.label("Load the selected input’s preprocessing controls.");
                    if ui.button("Read input settings").clicked(){refresh=true;}
                    return;
                }
            }

            if let std::collections::btree_map::Entry::Vacant(entry)=self.inputs.drafts.entry(id) {
                match InputDraft::read(snapshot,id){Ok(mut draft)=>{let groups: std::collections::BTreeSet<_> = draft.fields.iter().map(|f|f.group).collect(); for group in groups {draft.keep_group(group,true);} entry.insert(draft);},Err(e)=>{ui.label(e);return;}}
            }
            let state=&mut self.inputs;
            let draft=state.drafts.get_mut(&id).unwrap();
            ui.label(format!("Input {} · plug {} · {}",id,draft.plug,draft.native.map(|n|format!("{} × {} source pixels",n[0],n[1])).unwrap_or("No valid signal dimensions".into())));
            let consumers: Vec<_> = snapshot.screens.iter().flat_map(|screen| screen.layers.iter().filter(move |layer|snapshot.profile.input_from_source(&layer.source)==Some(id)).map(move|layer|format!("Screen {} / {} · Layer {}",screen.id,screen.preview_bank,layer.id))).collect();
            ui.collapsing(format!("Known Preview consumers ({})",consumers.len()),|ui|{for consumer in consumers {ui.label(consumer);}ui.small("Program layers, other consumers, backgrounds and multiviewer may also use this input.");});
            let before=draft.values.clone();
            let mut undoing=false;
            let changes=draft.changes(snapshot);
            ui.horizontal_wrapped(|ui| {
                let count=changes.as_ref().map_or(0,Vec::len);
                if ui.add_enabled_ui(!self.device.busy(), |ui| super::apply_button(ui,"Send changes",count>0)).inner.clicked(){send=Some(draft.clone());}
                ui.label(if count==0 {"No pending changes".into()} else {format!("{count} pending changes")});
                ui.separator();
                if ui.add_enabled(state.undo.get(&id).is_some_and(|h|!h.is_empty()) && !self.device.busy(),egui::Button::new("Undo input edit")).clicked() && let Some(previous)=state.undo.entry(id).or_default().pop(){draft.values=previous;draft.restore_kept_groups();undoing=true;state.edit_start=None;}
                if ui.add_enabled(draft.dirty() && !self.device.busy(),egui::Button::new("Discard draft")).clicked(){draft.reset();}
                if ui.add_enabled(!self.device.busy(),egui::Button::new("Reload selected input")).on_hover_text("Replace this input’s draft with fresh device values.").clicked(){refresh=true;}
            });
            if let Err(error)=changes {ui.colored_label(theme::palette(ui.ctx()).amber,error);}
            if !state.message.is_empty(){ui.label(&state.message);}
            ui.separator();
            // Only the editor body scrolls; the input selector and Send stay reachable.
            egui::ScrollArea::vertical().id_salt("input-settings-body")
                .max_height(ui.available_height().max(180.0)).show(ui,|ui| {
            ui.add_enabled_ui(!self.device.busy(),|ui|{
                let width=ui.available_width();
                let wide=width>=760.0;
                let gap=ui.spacing().item_spacing.x;
                let crop_width=if wide {(width-gap)*0.62} else {width};
                let settings_width=if wide {width-gap-crop_width} else {width};
                let layout=if wide {egui::Layout::left_to_right(egui::Align::Min)} else {egui::Layout::top_down(egui::Align::Min)};
                ui.with_layout(layout,|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(crop_width);
                        ui.heading("Input crop");
                        if keep_category(ui,draft,"Crop") {state.drag=None;}
                        ui.horizontal_wrapped(|ui| {
                            ui.add_enabled(inputs.iter().any(|i|i.id==id && i.enabled) || state.live_capture,egui::Checkbox::new(&mut state.live_capture,"Capture input (live)"));
                            if state.live_capture {
                                if capture_pending || state.capture.is_none() {ui.spinner();ui.label("Refreshing capture…");}
                                else if state.capture_geometry.is_none() {ui.label("Capture geometry unavailable; showing the sizing grid.");}
                                else {ui.small("Live · applied device crop");}
                            } else {ui.small("Sizing grid");}
                        });
                        crop_editor(ui,draft,&mut state.pivot,(&mut state.view_zoom,&mut state.view_pan),&mut state.drag,&mut state.message,state.capture.as_ref().filter(|_|state.live_capture).zip(state.capture_geometry).map(|(r,g)|(r.texture.id(),g)));
                        ui.small("The picture follows the applied device crop. Unknown pixels show the sizing grid.");
                        egui::CollapsingHeader::new("Crop values & presets").show(ui,|ui|{
                            ui.add_enabled_ui(!draft.keeps_group("Crop"),|ui|{category_fields(ui,draft,"Crop");});
                        });
                    });
                    ui.vertical(|ui| {
                        ui.set_width(settings_width);
                        ui.heading("Input processing");
                        let groups:Vec<_>=["Aspect","Color","Processing","Keying","LUT"].into_iter().filter(|group|draft.fields.iter().any(|f|f.group==*group)).collect();
                        if !groups.contains(&state.category.as_str()) {state.category=groups.first().copied().unwrap_or("").into();}
                        if groups.is_empty(){ui.small("No additional settings available for this input.");return;}
                        egui::ComboBox::from_id_salt("input-settings-category").selected_text(&state.category).show_ui(ui,|ui|{
                            for group in groups {ui.selectable_value(&mut state.category,group.to_owned(),group);}
                        });
                        let group=state.category.as_str();
                        keep_category(ui,draft,group);
                        ui.add_space(6.0);
                        ui.add_enabled_ui(!draft.keeps_group(group),|ui|{category_fields(ui,draft,group);});
                        ui.add_space(8.0);
                        ui.small("Only categories you unlock can be changed. Draft edits stay local until Send changes.");
                        ui.collapsing("Device-only settings",|ui|{ui.small("Signal/HDCP and resource allocation remain in WebRCS. LUT controls require an allocated resource.");});
                    });
                });
            });
            });
            if before!=draft.values && !undoing && state.edit_start.is_none(){state.edit_start=Some(before);}
            if !ui.input(|i|i.pointer.any_down()) && let Some(before)=state.edit_start.take(){let history=state.undo.entry(id).or_default();history.push(before);if history.len()>100{history.remove(0);}}
            ui.separator();

        });
        self.inputs.open = open;
        if refresh && !self.device.busy() {
            self.inputs.reading = true;
            let (tx, rx) = std::sync::mpsc::channel();
            self.inputs.pending = Some(rx);
            self.inputs.sending_input = self.inputs.selected;
            self.inputs.sending_host = self.device.host.clone();
            self.device.input_sending = true;
            let host = self.device.host.clone();
            let input = self.inputs.selected;
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let _ = tx.send(canvas_pilot::device::fetch_input(&host, input));
                ctx.request_repaint();
            });
        }
        if let Some(draft) = send {
            self.inputs.capture = None;
            self.inputs.capture_geometry = None;
            self.inputs.capture_key = None;
            self.inputs.reading = false;
            let (tx, rx) = std::sync::mpsc::channel();
            self.inputs.pending = Some(rx);
            self.inputs.sending_input = draft.input;
            self.inputs.sending_host = self.device.host.clone();
            self.device.input_sending = true;
            let host = self.device.host.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let _ = tx.send(canvas_pilot::device_send::send_inputs(&host, &draft));
                ctx.request_repaint();
            });
        }
    }
}
fn keep_category(ui: &mut egui::Ui, draft: &mut InputDraft, group: &str) -> bool {
    let mut keep = draft.keeps_group(group);
    let changed = ui
        .checkbox(&mut keep, "Keep device values")
        .on_hover_text(
            "Leave this category untouched when sending. Checking discards its draft edits.",
        )
        .changed();
    if changed {
        draft.keep_group(group, keep);
    }
    changed
}
fn category_fields(ui: &mut egui::Ui, draft: &mut InputDraft, group: &str) {
    egui::Grid::new(("input-category-fields", group))
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            for field in &draft.fields {
                if field.group != group {
                    continue;
                }
                let value = draft.values.get_mut(&field.path).unwrap();
                ui.label(&field.label);
                match &field.kind {
                    Kind::Number(min, max) => {
                        if field.path.ends_with("/customRatio") {
                            let mut ratio = value.as_i64().unwrap() as f64 / 1000.;
                            if ui
                                .add(
                                    egui::DragValue::new(&mut ratio)
                                        .range(0.25..=10.)
                                        .speed(0.001)
                                        .suffix(":1"),
                                )
                                .changed()
                            {
                                *value = json!((ratio * 1000.).round() as i64);
                            }
                        } else {
                            let mut v = value.as_i64().unwrap();
                            if ui
                                .add(egui::DragValue::new(&mut v).range(*min..=*max))
                                .changed()
                            {
                                *value = json!(v);
                            }
                        }
                    }
                    Kind::Toggle => {
                        let mut v = value.as_bool().unwrap();
                        if ui.checkbox(&mut v, "").changed() {
                            *value = json!(v);
                        }
                    }
                    Kind::Choice(options) => {
                        let mut v = value.as_str().unwrap().to_owned();
                        egui::ComboBox::from_id_salt(&field.path)
                            .width(145.0)
                            .selected_text(choice_label(&v))
                            .show_ui(ui, |ui| {
                                for option in options {
                                    ui.selectable_value(
                                        &mut v,
                                        option.clone(),
                                        choice_label(option),
                                    );
                                }
                            });
                        *value = json!(v);
                    }
                }
                ui.end_row();
            }
        });
}

// Display labels only: the original enum strings remain in draft/device values.
fn choice_label(value: &str) -> String {
    match value {
        "LUT" | "RGB" | "YUV" | "HDR" | "SDR" => return value.into(),
        "KEEP_INPUT_RATIO" => return "Keep input ratio".into(),
        "SIGNAL" => return "Signal aspect ratio".into(),
        _ => {}
    }
    for (prefix, label) in [("LETTERBOX_", "Letterbox"), ("PILLARBOX_", "Pillarbox")] {
        if let Some(ratio) = value.strip_prefix(prefix) {
            return format!("{label} {}:1", ratio.replace('_', "."));
        }
    }
    if let Some((a, b)) = value.split_once('_')
        && !a.is_empty()
        && !b.is_empty()
        && a.bytes().chain(b.bytes()).all(|c| c.is_ascii_digit())
    {
        return format!("{a}:{b}");
    }
    let text = value.replace('_', " ").to_lowercase();
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => text,
    }
}

fn crop_hit(rect: Rect, pos: Pos2) -> Option<[i8; 2]> {
    const GRAB: f32 = 14.0;
    if !rect.expand(GRAB).contains(pos) {
        return None;
    }
    let edge = |v: f32, low: f32, high: f32| {
        let a = (v - low).abs();
        let b = (v - high).abs();
        if a.min(b) > GRAB {
            0
        } else if a <= b {
            -1
        } else {
            1
        }
    };
    Some([
        edge(pos.x, rect.left(), rect.right()),
        edge(pos.y, rect.top(), rect.bottom()),
    ])
}

fn set_crop_zoom(
    draft: &mut InputDraft,
    percent: f64,
    current: f64,
    pivot: [f64; 2],
) -> Result<(), String> {
    if percent == 100.0 {
        draft.set_crop(Crop::default())
    } else {
        draft.zoom_crop(percent / current, pivot)
    }
}

fn crop_editor(
    ui: &mut egui::Ui,
    draft: &mut InputDraft,
    pivot: &mut [usize; 2],
    view: (&mut f32, &mut Vec2),
    drag: &mut Option<CropDrag>,
    message: &mut String,
    preview: Option<(egui::TextureId, CaptureGeometry)>,
) {
    let editable = !draft.keeps_group("Crop") && ui.is_enabled();
    if !editable {
        *drag = None;
    }
    let (view_zoom, pan) = view;
    let Some(native) = draft.native else {
        ui.label("A valid signal is required for visual cropping.");
        return;
    };
    if let Err(error) = draft.crop().remaining(native) {
        ui.label(error);
        return;
    }
    if *view_zoom == 0.0 {
        *view_zoom = 1.0;
        *pivot = [1, 1];
    }
    ui.add_enabled_ui(editable, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("Crop zoom");
            let retained = draft.crop().remaining(native).unwrap();
            let current = 100.0 * (native[0] * native[1] / (retained[0] * retained[1])).sqrt();
            let maximum = 100.0 * (native[0] * native[1]).sqrt();
            let mut zoom = current;
            let mut changed = ui.button("−").clicked();
            if changed {
                zoom /= 1.1;
            }
            changed |= ui
                .add(
                    egui::Slider::new(&mut zoom, 100.0..=500.0)
                        .clamping(egui::SliderClamping::Never)
                        .suffix("%")
                        .max_decimals(1),
                )
                .changed();
            if ui.button("+").clicked() {
                changed = true;
                zoom *= 1.1;
            }
            zoom = zoom.clamp(100.0, maximum);
            if changed
                && (zoom - current).abs() > 0.0001
                && let Err(error) = set_crop_zoom(
                    draft,
                    zoom,
                    current,
                    [pivot[0] as f64 / 2.0, pivot[1] as f64 / 2.0],
                )
            {
                *message = error;
            }
            if ui.button("Reset crop").clicked() {
                let _ = draft.set_crop(Crop::default());
            }
        });
    });
    ui.horizontal_wrapped(|ui| {
        ui.add_enabled_ui(editable, |ui| {
            ui.label("Pivot");
            egui::Grid::new("input-crop-pivot")
                .spacing([2., 2.])
                .show(ui, |ui| {
                    for y in 0..3 {
                        for x in 0..3 {
                            ui.selectable_value(
                                pivot,
                                [x, y],
                                if *pivot == [x, y] { "●" } else { "○" },
                            );
                        }
                        ui.end_row();
                    }
                });
        });
        ui.label("View zoom");
        ui.add(
            egui::DragValue::new(view_zoom)
                .range(0.25..=8.0)
                .speed(0.05),
        );
        if ui.button("Fit view").clicked() {
            *view_zoom = 1.;
            *pan = Vec2::ZERO;
        }
    });
    ui.small("Drag edges/corners or the crop box. Wheel: view zoom; middle drag: pan. Escape cancels a crop drag.");
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(
            ui.available_width().max(100.),
            (ui.available_width() * 0.55).clamp(230.0, 400.0),
        ),
        egui::Sense::click_and_drag(),
    );
    let painter = ui.painter_at(rect);
    let palette = theme::palette(ui.ctx());
    painter.rect_filled(rect, 3., egui::Color32::from_gray(18));
    let scale =
        (rect.width() / native[0] as f32).min(rect.height() / native[1] as f32) * 0.9 * *view_zoom;
    if response.dragged_by(egui::PointerButton::Middle) {
        *pan += ui.input(|i| i.pointer.delta());
    }
    if response.hovered() && drag.is_none() && !ui.input(|i| i.pointer.primary_down()) {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            *view_zoom = (*view_zoom * (scroll * 0.002).exp()).clamp(0.25, 8.0);
        }
    }
    let full = Rect::from_center_size(
        rect.center() + *pan,
        Vec2::new(native[0] as f32, native[1] as f32) * scale,
    );
    let source = canvas_pilot::formats::Source {
        native: native.map(|n| n as u32),
        aspect: canvas_pilot::formats::Aspect::Custom,
    };
    // Protection disables editing controls, not the visibility of the source picture.
    let mut content_painter = painter.clone();
    content_painter.set_opacity(1.0);
    super::format_ui::paint_pattern_clipped(&content_painter, full, source, Crop::default(), full);
    if let Some((texture, geometry)) = preview
        && let Some((known, uv)) = capture_rectangles(geometry, full, native)
    {
        content_painter.image(texture, known, uv, Color32::WHITE);
    }
    painter.rect_stroke(full, 0., Stroke::new(1., palette.muted), StrokeKind::Inside);
    let crop = draft.crop();
    let retained = Rect::from_min_max(
        full.min + Vec2::new(crop.left as f32, crop.top as f32) * scale,
        full.max - Vec2::new(crop.right as f32, crop.bottom as f32) * scale,
    );
    painter.rect_stroke(
        retained,
        0.,
        Stroke::new(2., palette.cyan),
        StrokeKind::Inside,
    );
    for point in [
        retained.left_top(),
        retained.center_top(),
        retained.right_top(),
        retained.left_center(),
        retained.right_center(),
        retained.left_bottom(),
        retained.center_bottom(),
        retained.right_bottom(),
    ] {
        painter.rect_filled(
            Rect::from_center_size(point, Vec2::splat(8.)),
            0.,
            palette.cyan,
        );
    }
    painter.circle_filled(
        retained.min + retained.size() * Vec2::new(pivot[0] as f32 / 2., pivot[1] as f32 / 2.),
        4.,
        palette.amber,
    );
    if !editable {
        return;
    }
    let hover_edges = response.hover_pos().and_then(|pos| crop_hit(retained, pos));
    if let Some(edges) = drag.as_ref().map(|d| d.2).or(hover_edges) {
        let cursor = match edges {
            [0, 0] => {
                if drag.is_some() {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::Grab
                }
            }
            [0, _] => egui::CursorIcon::ResizeVertical,
            [_, 0] => egui::CursorIcon::ResizeHorizontal,
            [x, y] if x == y => egui::CursorIcon::ResizeNwSe,
            _ => egui::CursorIcon::ResizeNeSw,
        };
        ui.ctx().set_cursor_icon(cursor);
    }
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
        && let Some(edges) = crop_hit(retained, pos)
    {
        *drag = Some((crop, pos, edges, draft.values.clone()));
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        if let Some((_, _, _, before)) = drag.take() {
            draft.values = before;
        }
    } else if response.dragged_by(egui::PointerButton::Primary)
        && let (Some((before, start, edges)), Some(pos)) = (
            drag.as_ref().map(|(c, p, e, _)| (*c, *p, *e)),
            response.interact_pointer_pos(),
        )
    {
        let delta = (pos - start) / scale;
        let mut next = before;
        if edges == [0, 0] {
            let dx = (delta.x as f64).clamp(-before.left, before.right);
            let dy = (delta.y as f64).clamp(-before.top, before.bottom);
            next.left += dx;
            next.right -= dx;
            next.top += dy;
            next.bottom -= dy;
        } else {
            if edges[0] < 0 {
                next.left = (before.left + delta.x as f64).clamp(0., native[0] - before.right - 1.);
            }
            if edges[0] > 0 {
                next.right =
                    (before.right - delta.x as f64).clamp(0., native[0] - before.left - 1.);
            }
            if edges[1] < 0 {
                next.top = (before.top + delta.y as f64).clamp(0., native[1] - before.bottom - 1.);
            }
            if edges[1] > 0 {
                next.bottom =
                    (before.bottom - delta.y as f64).clamp(0., native[1] - before.top - 1.);
            }
        }
        if let Err(e) = draft.set_crop(next) {
            *message = e;
        }
    }
    if response.drag_stopped() {
        *drag = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protected_crop_still_composites_capture_at_full_opacity() {
        let app = input_app();
        let mut draft = InputDraft::read(app.device.snapshot.as_ref().unwrap(), 1).unwrap();
        let ctx = egui::Context::default();
        let texture = ctx.load_texture(
            "capture-test",
            egui::ColorImage::filled([4, 4], Color32::RED),
            Default::default(),
        );
        let geometry = CaptureGeometry {
            framing: canvas_pilot::device_capture::CaptureFraming::FullFrame,
            native: [1920., 1080.],
            crop: Crop {
                left: 480.,
                right: 480.,
                top: 270.,
                bottom: 270.,
            },
        };
        let mut pivot = [1, 1];
        let mut zoom = 1.;
        let mut pan = Vec2::ZERO;
        let mut drag = None;
        let mut message = String::new();
        let output = ctx.run_ui(Default::default(), |ui| {
            ui.add_enabled_ui(false, |ui| {
                crop_editor(
                    ui,
                    &mut draft,
                    &mut pivot,
                    (&mut zoom, &mut pan),
                    &mut drag,
                    &mut message,
                    Some((texture.id(), geometry)),
                )
            });
        });
        let mesh = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id == texture.id() => Some(mesh),
                _ => None,
            })
            .expect("capture must be drawn over the pattern even when crop editing is protected");
        assert!(mesh.vertices.iter().all(|v| v.color == Color32::WHITE));
        assert!(
            mesh.vertices
                .iter()
                .all(|v| (0.25..=0.75).contains(&v.uv.x) && (0.25..=0.75).contains(&v.uv.y))
        );
        assert_eq!(draft.crop().left, 24.);
    }

    fn processing_app() -> Editor {
        let mut app = input_app();
        let settings = &mut app.device.snapshot.as_mut().unwrap().raw["device"]["inputList"]["items"]
            ["IN_1"]["plugList"]["items"]["1"]["settings"];
        settings["aspect"] = json!({"pp":{"signal":"NATIVE","transformTo":"SIGNAL","customRatio":1778,"layerFill":"LAYER_SETTING"}});
        settings["color"] = json!({"pp":{"brightness":128,"contrast":128}});
        app
    }
    #[test]
    fn input_categories_switch_without_editing_or_sending() {
        let mut app = processing_app();
        let ctx = egui::Context::default();
        super::super::tests::frame(&mut app, &ctx, vec![]);
        let before = app.inputs.drafts[&1].values.clone();
        super::super::tests::click_text(&mut app, &ctx, "Aspect");
        super::super::tests::click_text(&mut app, &ctx, "Color");
        assert_eq!(app.inputs.category, "Color");
        let output = super::super::tests::frame(&mut app, &ctx, vec![]);
        assert!(
            output
                .shapes
                .iter()
                .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="Brightness"))
        );
        assert_eq!(app.inputs.drafts[&1].values, before);
        assert!(app.inputs.drafts[&1].keeps_group("Color"));
        assert!(app.inputs.pending.is_none());
        assert!(!app.device.input_sending);
    }
    #[test]
    fn input_workspace_stacks_on_small_screens_and_keeps_actions_visible() {
        for width in [640., 1440.] {
            let mut app = processing_app();
            let ctx = egui::Context::default();
            let mut draw = || {
                ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 900.))),
                        ..Default::default()
                    },
                    |_| app.inputs_window(&ctx),
                )
            };
            draw();
            draw();
            let output = draw();
            let position = |label: &str| {
                output
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos),
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("{label} missing at width {width}"))
            };
            let crop = position("Input crop");
            let processing = position("Input processing");
            let send = position("Send changes");
            assert!(send.y < crop.y && send.x >= 0. && send.x < width);
            if width < 760. {
                assert!(processing.y > crop.y + 200.);
            } else {
                assert!(processing.x > crop.x + 300.);
                assert!((processing.y - crop.y).abs() < 10.);
            }
        }
    }

    #[test]
    fn send_changes_is_above_the_crop_editor() {
        let mut app = input_app();
        let ctx = egui::Context::default();
        super::super::tests::frame(&mut app, &ctx, vec![]);
        let output = super::super::tests::frame(&mut app, &ctx, vec![]);
        let y = |label: &str| {
            output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos.y),
                    _ => None,
                })
                .unwrap()
        };
        assert!(y("Send changes") < y("Input crop"));
    }
    #[test]
    fn input_categories_start_protected_and_full_zoom_clears_asymmetric_crop() {
        let mut app = input_app();
        let ctx = egui::Context::default();
        super::super::tests::frame(&mut app, &ctx, vec![]);
        let draft = app.inputs.drafts.get_mut(&1).unwrap();
        assert!(
            draft
                .fields
                .iter()
                .all(|field| draft.keeps_group(field.group))
        );
        assert!(!draft.dirty());
        draft.keep_group("Crop", false);
        draft
            .set_crop(Crop {
                left: 1.,
                right: 0.,
                top: 0.,
                bottom: 0.,
            })
            .unwrap();
        let native = draft.native.unwrap();
        let retained = draft.crop().remaining(native).unwrap();
        let current = 100. * (native[0] * native[1] / (retained[0] * retained[1])).sqrt();
        set_crop_zoom(draft, 100., current, [0.5, 0.5]).unwrap();
        assert_eq!(draft.crop(), Crop::default());
        set_crop_zoom(draft, 600., 100., [0.5, 0.5]).unwrap();
        assert_eq!(draft.crop().remaining(native).unwrap(), [320., 180.]);
    }
    #[test]
    fn enum_labels_are_readable_without_changing_wire_values() {
        for (wire, display) in [
            ("16_9", "16:9"),
            ("1_1", "1:1"),
            ("NATIVE", "Native"),
            ("LUT", "LUT"),
            ("KEEP_INPUT_RATIO", "Keep input ratio"),
            ("LETTERBOX_1_85", "Letterbox 1.85:1"),
            ("LAYER_SETTING", "Layer setting"),
        ] {
            assert_eq!(choice_label(wire), display);
        }
        let rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(20., 100.));
        assert_eq!(crop_hit(rect, Pos2::new(19., 50.)), Some([1, 0]));
        assert_eq!(crop_hit(rect, Pos2::new(-12., 50.)), Some([-1, 0]));
        assert_eq!(crop_hit(rect, Pos2::new(-30., 50.)), None);
    }

    #[test]
    fn edge_drag_uses_press_position_even_after_pointer_leaves_grab_area() {
        let app = input_app();
        let mut draft = InputDraft::read(app.device.snapshot.as_ref().unwrap(), 1).unwrap();
        draft.set_crop(Crop::default()).unwrap();
        let ctx = egui::Context::default();
        let mut pivot = [1, 1];
        let mut zoom = 1.;
        let mut pan = Vec2::ZERO;
        let mut drag = None;
        let mut message = String::new();
        let mut frame = |events| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(700., 600.))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    crop_editor(
                        ui,
                        &mut draft,
                        &mut pivot,
                        (&mut zoom, &mut pan),
                        &mut drag,
                        &mut message,
                        None,
                    );
                },
            )
        };
        let output = frame(vec![]);
        let rect = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r) if r.stroke.width == 2.0 => Some(r.rect),
                _ => None,
            })
            .unwrap();
        let start = rect.right_center();
        frame(vec![
            egui::Event::PointerMoved(start),
            super::super::tests::mouse(start, true),
        ]);
        let end = start - Vec2::new(40., 0.);
        frame(vec![egui::Event::PointerMoved(end)]);
        frame(vec![super::super::tests::mouse(end, false)]);
        assert_eq!(draft.crop().left, 0.);
        assert!(draft.crop().right > 100.);
        assert_eq!(draft.crop().top, 0.);
        assert!(message.is_empty());
    }

    #[test]
    fn protected_crop_allows_view_navigation_but_not_crop_drag() {
        let app = input_app();
        let mut draft = InputDraft::read(app.device.snapshot.as_ref().unwrap(), 1).unwrap();
        draft.keep_group("Crop", true);
        let original = draft.values.clone();
        let ctx = egui::Context::default();
        let mut pivot = [1, 1];
        let mut zoom = 1.;
        let mut pan = Vec2::ZERO;
        let mut drag = None;
        let mut message = String::new();
        let mut frame = |events| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(700., 600.))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    crop_editor(
                        ui,
                        &mut draft,
                        &mut pivot,
                        (&mut zoom, &mut pan),
                        &mut drag,
                        &mut message,
                        None,
                    );
                },
            )
        };
        let output = frame(vec![]);
        let rect = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r) if r.stroke.width == 2.0 => Some(r.rect),
                _ => None,
            })
            .unwrap();
        let start = rect.right_center();
        frame(vec![
            egui::Event::PointerMoved(start),
            super::super::tests::mouse(start, true),
        ]);
        let end = start - Vec2::new(40., 0.);
        frame(vec![egui::Event::PointerMoved(end)]);
        frame(vec![super::super::tests::mouse(end, false)]);
        let center = rect.center();
        frame(vec![
            egui::Event::PointerMoved(center),
            egui::Event::MouseWheel {
                phase: egui::TouchPhase::Move,
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0., 120.),
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let middle = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Middle,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(vec![middle(center, true)]);
        let moved = center + Vec2::new(30., 20.);
        frame(vec![egui::Event::PointerMoved(moved)]);
        frame(vec![middle(moved, false)]);
        assert!(zoom > 1.);
        assert!(pan.length() > 0.);
        assert!(drag.is_none());
        assert_eq!(draft.values, original);
        assert!(message.is_empty());
    }

    #[test]
    fn captured_pixels_stay_in_original_source_region_when_crop_expands() {
        let geometry = CaptureGeometry {
            framing: canvas_pilot::device_capture::CaptureFraming::FullFrame,
            native: [1920., 1080.],
            crop: Crop {
                left: 480.,
                right: 480.,
                top: 270.,
                bottom: 270.,
            },
        };
        let full = Rect::from_min_size(Pos2::new(10., 20.), Vec2::new(192., 108.));
        let (known, uv) = capture_rectangles(geometry, full, [1920., 1080.]).unwrap();
        assert_eq!(
            known,
            Rect::from_min_size(Pos2::new(58., 47.), Vec2::new(96., 54.))
        );
        assert_eq!(
            uv,
            Rect::from_min_max(Pos2::new(0.25, 0.25), Pos2::new(0.75, 0.75))
        );
        let cropped = CaptureGeometry {
            framing: canvas_pilot::device_capture::CaptureFraming::Cropped,
            ..geometry
        };
        let (cropped_rect, cropped_uv) = capture_rectangles(cropped, full, [1920., 1080.]).unwrap();
        assert_eq!(cropped_rect, known);
        assert_eq!(
            cropped_uv,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.))
        );
        assert!(capture_rectangles(geometry, full, [1280., 720.]).is_none());
        // Mapping is fixed at capture time, independent of the editable crop box.
        assert_eq!(
            capture_rectangles(geometry, full, [1920., 1080.])
                .unwrap()
                .0,
            known
        );
    }
    fn input_app() -> Editor {
        let mut app = Editor::default();
        let raw = json!({"device":{"inputList":{"items":{"IN_1":{
            "status":{"pp":{"isAvailable":true,"plug":"1"}},
            "snapshot":{"pp":{"enable":true}},
            "plugList":{"items":{"1":{
                "status":{"signal":{"pp":{"isValid":true,"imageWidth":1920,"imageHeight":1080,"aspectWidth":1920,"aspectHeight":1080}}},
                "settings":{"cropping":{"control":{"pp":{"left":24,"right":0,"top":12,"bottom":0}}}}
            }}}
        }}}}});
        app.device.snapshot = Some(canvas_pilot::device::Snapshot {
            identity: "Aquilon fixture".into(),
            profile: canvas_pilot::device::Profile::Aquilon6,
            screens: vec![],
            raw,
        });
        app.inputs.open = true;
        app
    }
    #[test]
    fn keep_crop_disables_editor_and_survives_undo() {
        let mut app = input_app();
        let ctx = egui::Context::default();
        super::super::tests::click_text(&mut app, &ctx, "Keep device values");
        super::super::tests::click_text(&mut app, &ctx, "Reset crop");
        super::super::tests::click_text(&mut app, &ctx, "Keep device values");
        assert!(app.inputs.drafts[&1].keeps_group("Crop"));
        assert_eq!(app.inputs.drafts[&1].crop().left, 24.);
        super::super::tests::click_text(&mut app, &ctx, "Reset crop");
        assert_eq!(app.inputs.drafts[&1].crop().left, 24.);
        super::super::tests::click_text(&mut app, &ctx, "Undo input edit");
        assert_eq!(app.inputs.drafts[&1].crop().left, 24.);
        assert!(!app.inputs.drafts[&1].dirty());
        super::super::tests::click_text(&mut app, &ctx, "Keep device values");
        super::super::tests::click_text(&mut app, &ctx, "Reset crop");
        assert_eq!(app.inputs.drafts[&1].crop(), Crop::default());
        assert!(!app.device.input_sending);
        assert!(app.inputs.pending.is_none());
    }
    #[test]
    fn shared_input_reset_and_undo_do_not_modify_layers_or_send() {
        let mut app = input_app();
        let layers = app.layout.layers().clone();
        let ctx = egui::Context::default();
        super::super::tests::click_text(&mut app, &ctx, "Keep device values");
        super::super::tests::click_text(&mut app, &ctx, "Reset crop");
        assert_eq!(app.inputs.drafts[&1].crop(), Crop::default());
        super::super::tests::click_text(&mut app, &ctx, "Undo input edit");
        assert_eq!(app.inputs.drafts[&1].crop().left, 24.);
        assert!(app.inputs.undo[&1].is_empty());
        assert_eq!(app.layout.layers(), &layers);
        assert!(!app.device.input_sending);
        assert!(app.inputs.pending.is_none());
    }
}
