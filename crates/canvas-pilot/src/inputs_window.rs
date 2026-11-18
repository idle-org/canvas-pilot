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
#[derive(Clone, Copy, Debug)]
pub(super) struct CaptureGeometry {
    pub native: [f64; 2],
    pub crop: Crop,
}
impl CaptureGeometry {
    pub fn rectangles(self, full: Rect, native: [f64; 2]) -> Option<(Rect, Rect)> {
        if self.native != native || self.crop.remaining(native).is_err() {
            return None;
        }
        let uv = Rect::from_min_max(
            Pos2::new(
                (self.crop.left / native[0]) as f32,
                (self.crop.top / native[1]) as f32,
            ),
            Pos2::new(
                (1. - self.crop.right / native[0]) as f32,
                (1. - self.crop.bottom / native[1]) as f32,
            ),
        );
        let rect = Rect::from_min_max(
            full.min + uv.min.to_vec2() * full.size(),
            full.min + uv.max.to_vec2() * full.size(),
        );
        Some((rect, uv))
    }
}
#[derive(Default)]
pub(super) struct InputsWindow {
    pub open: bool,
    pub selected: u32,
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
    review: bool,
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
            self.inputs.review = false;
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
            .open(&mut open).default_size([1040.0,700.0]).vscroll(true).show(ctx,|ui|{
            ui.label("Shared preprocessing · affects every layer using this device input, including Program.");
            ui.small("Drafts stay in this session. Nothing is sent until Review and Send input settings.");
            let Some(snapshot)=&self.device.snapshot else {
                ui.label("Connect to a supported device to inspect its inputs.");
                if ui.button("Device connection…").clicked(){self.device.connection_open=true;}
                return;
            };

            let inputs=canvas_pilot::device_capture::inputs(snapshot);
            if self.inputs.pending.is_some() {ui.spinner();ui.label("Applying / verifying input settings… Capture will refresh after verification.");return;}

            if self.inputs.selected==0 {self.inputs.selected=inputs.first().map(|i|i.id).unwrap_or(0);}
            ui.horizontal(|ui|{
                ui.add_enabled_ui(self.inputs.pending.is_none(),|ui|{
                    egui::ComboBox::from_id_salt("device-input-picker").selected_text(format!("Input {}",self.inputs.selected)).show_ui(ui,|ui|{
                        for input in &inputs {
                            if ui.selectable_value(&mut self.inputs.selected,input.id,format!("Input {}{}{}",input.id,if input.signal{""}else{" · no signal"},if self.inputs.drafts.get(&input.id).is_some_and(InputDraft::dirty){" · edited"}else{""})).changed(){
                                self.inputs.capture=None;self.inputs.capture_key=None;self.inputs.review=false;self.inputs.drag=None;self.inputs.edit_start=None;
                            }
                        }
                    });
                });
                if ui.button("Refresh device…").clicked(){self.device.connection_open=true;}
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
                match InputDraft::read(snapshot,id){Ok(draft)=>{entry.insert(draft);},Err(e)=>{ui.label(e);return;}}
            }
            let state=&mut self.inputs;
            let draft=state.drafts.get_mut(&id).unwrap();
            ui.label(format!("Input {} · plug {} · {}",id,draft.plug,draft.native.map(|n|format!("{} × {} source pixels",n[0],n[1])).unwrap_or("No valid signal dimensions".into())));
            let consumers: Vec<_> = snapshot.screens.iter().flat_map(|screen| screen.layers.iter().filter(move |layer|snapshot.profile.input_from_source(&layer.source)==Some(id)).map(move|layer|format!("Screen {} / {} · Layer {}",screen.id,screen.preview_bank,layer.id))).collect();
            ui.collapsing(format!("Known Preview consumers ({})",consumers.len()),|ui|{for consumer in consumers {ui.label(consumer);}ui.small("Program layers, other consumers, backgrounds and multiviewer may also use this input.");});
            let before=draft.values.clone();
            let mut undoing=false;
            ui.add_enabled_ui(state.pending.is_none() && !self.device.busy(),|ui|{
                ui.horizontal(|ui|{
                    if ui.button("Undo input edit").clicked() && let Some(previous)=state.undo.entry(id).or_default().pop(){draft.values=previous;draft.restore_kept_groups();undoing=true;state.edit_start=None;}
                    if ui.button("Discard draft").clicked(){draft.reset();}
                    if ui.button("Reload selected input").clicked(){refresh=true;}
                    ui.add_enabled(inputs.iter().any(|i|i.id==id && i.enabled) || state.live_capture,egui::Checkbox::new(&mut state.live_capture,"Capture input (live)"));
                });
                ui.columns(2,|columns|{
                    let ui=&mut columns[0];
                    ui.heading("Input crop");
                    if state.live_capture {
                        if capture_pending || state.capture.is_none() {ui.label("Refreshing capture…");}
                        else if state.capture_geometry.is_none() {ui.label("Capture geometry unavailable; showing the sizing grid.");}
                        else {ui.small("Live capture · device crop, independent of draft edits");}
                    }
                    ui.add_enabled_ui(!draft.keeps_group("Crop"), |ui| { crop_editor(ui,draft,&mut state.pivot,(&mut state.view_zoom,&mut state.view_pan),&mut state.drag,&mut state.message,state.capture.as_ref().filter(|_|state.live_capture).zip(state.capture_geometry).map(|(r,g)|(r.texture.id(),g))); });
                    ui.small("Unknown pixels show the sizing grid. Capture uses the device crop, never the draft crop.");
                    let ui=&mut columns[1];
                    for group in ["Aspect","Crop","Color","Processing","Keying","LUT"] {
                        if !draft.fields.iter().any(|f|f.group==group){continue;}
                        egui::CollapsingHeader::new(group).default_open(matches!(group,"Aspect"|"Crop")).show(ui,|ui|{
                            let mut keep = draft.keeps_group(group);
                            if ui.checkbox(&mut keep,"Keep device values").on_hover_text("Leave this category untouched when sending. Checking discards its draft edits.").changed() {
                                draft.keep_group(group,keep);
                                state.review=false;
                                if group=="Crop" {state.drag=None;}
                            }
                            ui.add_enabled_ui(!keep, |ui| {
                            for field in &draft.fields {
                                if field.group!=group{continue;}
                                let value=draft.values.get_mut(&field.path).unwrap();
                                ui.horizontal(|ui|{
                                    ui.label(&field.label);
                                    match &field.kind {
                                        Kind::Number(min,max)=>{
                                            if field.path.ends_with("/customRatio") {
                                                let mut ratio=value.as_i64().unwrap() as f64/1000.;
                                                if ui.add(egui::DragValue::new(&mut ratio).range(0.25..=10.).speed(0.001).suffix(":1")).changed(){*value=json!((ratio*1000.).round() as i64);}
                                            } else {
                                                let mut v=value.as_i64().unwrap();if ui.add(egui::DragValue::new(&mut v).range(*min..=*max)).changed(){*value=json!(v);}
                                            }
                                        }
                                        Kind::Toggle=>{let mut v=value.as_bool().unwrap();if ui.checkbox(&mut v,"").changed(){*value=json!(v);}}
                                        Kind::Choice(options)=>{let mut v=value.as_str().unwrap().to_owned();egui::ComboBox::from_id_salt(&field.path).selected_text(choice_label(&v)).show_ui(ui,|ui|{for option in options {ui.selectable_value(&mut v,option.clone(),choice_label(option));}});*value=json!(v);}
                                    }
                                });
                            }
                            });
                        });
                    }
                    ui.small("LUT controls require an allocated resource. Signal/HDCP and resource allocation remain in WebRCS.");
                });
            });
            if before!=draft.values {state.review=false;if !undoing && state.edit_start.is_none(){state.edit_start=Some(before);}}
            if !ui.input(|i|i.pointer.any_down()) && let Some(before)=state.edit_start.take(){let history=state.undo.entry(id).or_default();history.push(before);if history.len()>100{history.remove(0);}}
            ui.separator();
            match draft.changes(snapshot) {
                Ok(changes)=>{
                    if ui.add_enabled(!changes.is_empty() && state.pending.is_none() && !self.device.busy(),egui::Button::new("Review input changes")).clicked(){state.review=true;}
                    if state.review {
                        ui.label("These are shared input changes, not a Preview-only layer edit.");
                        for change in &changes {ui.small(format!("{}: {} → {}",change.path.iter().skip(8).cloned().collect::<Vec<_>>().join("/"),change.before,change.after));}
                        if ui.button("Send input settings").clicked(){send=Some(draft.clone());state.review=false;}
                    }
                }
                Err(error)=>{ui.colored_label(theme::palette(ui.ctx()).amber,error);}
            }
            if state.pending.is_some(){ui.spinner();ui.label("Sending and verifying input settings…");}
            if !state.message.is_empty(){ui.label(&state.message);}
        });
        self.inputs.open = open;
        if refresh && !self.device.busy() {
            self.inputs.review = false;
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

fn crop_editor(
    ui: &mut egui::Ui,
    draft: &mut InputDraft,
    pivot: &mut [usize; 2],
    view: (&mut f32, &mut Vec2),
    drag: &mut Option<CropDrag>,
    message: &mut String,
    preview: Option<(egui::TextureId, CaptureGeometry)>,
) {
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
    ui.horizontal(|ui| {
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
                egui::Slider::new(&mut zoom, 100.0..=maximum)
                    .logarithmic(true)
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
            && let Err(error) = draft.zoom_crop(
                zoom / current,
                [pivot[0] as f64 / 2.0, pivot[1] as f64 / 2.0],
            )
        {
            *message = error;
        }
        if ui.button("Reset crop").clicked() {
            let _ = draft.set_crop(Crop::default());
        }
    });
    ui.horizontal(|ui| {
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
        Vec2::new(ui.available_width().max(100.), 260.),
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
    super::format_ui::paint_pattern_clipped(&painter, full, source, Crop::default(), full);
    if let Some((texture, geometry)) = preview
        && let Some((known, uv)) = geometry.rectangles(full, native)
    {
        painter.image(texture, known, uv, Color32::WHITE);
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
    fn captured_pixels_stay_in_original_source_region_when_crop_expands() {
        let geometry = CaptureGeometry {
            native: [1920., 1080.],
            crop: Crop {
                left: 480.,
                right: 480.,
                top: 270.,
                bottom: 270.,
            },
        };
        let full = Rect::from_min_size(Pos2::new(10., 20.), Vec2::new(192., 108.));
        let (known, uv) = geometry.rectangles(full, [1920., 1080.]).unwrap();
        assert_eq!(
            known,
            Rect::from_min_size(Pos2::new(58., 47.), Vec2::new(96., 54.))
        );
        assert_eq!(
            uv,
            Rect::from_min_max(Pos2::new(0.25, 0.25), Pos2::new(0.75, 0.75))
        );
        assert!(geometry.rectangles(full, [1280., 720.]).is_none());
        // Mapping is fixed at capture time, independent of the editable crop box.
        assert_eq!(geometry.rectangles(full, [1920., 1080.]).unwrap().0, known);
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
