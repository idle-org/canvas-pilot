use super::*;
use canvas_pilot::formats::{Aspect, Crop, Framing, Source};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum PropertyTab {
    Source,
    Mapping,
    Preview,
    #[default]
    Layer,
    Crop,
    Mask,
}

pub(super) struct FormatDraft {
    layer: usize,
    baseline: (usize, Source, [f64; 2], [f64; 2], Framing),
    crop_before: Option<Layout>,
    mask_before: Option<Layout>,
    layer_before: Option<Layout>,
    source_id: usize,
    source: Source,
    input_size: [f64; 2],
    size: [f64; 2],
    layer_aspect: Aspect,
    ratio_edit: ratio_ui::RatioEdit,
    framing: Framing,
    error: String,
    closing: bool,
}
impl FormatDraft {
    pub(super) fn pending(&self, _layout: &Layout) -> bool {
        (
            self.source_id,
            self.source,
            self.input_size,
            self.size,
            self.framing,
        ) != self.baseline
            || self.ratio_edit.dirty
    }
    fn refresh_fit(&mut self, layout: &Layout) {
        self.framing = layout
            .formats()
            .mapping(self.layer, layout.layers()[self.layer])
            .unwrap()
            .framing;
        self.baseline.4 = self.framing;
        self.crop_before = None;
        self.layer_before = None;
        self.error.clear();
    }
    fn clamp_offsets(&mut self, layout: &Layout) -> Result<[f64; 2], String> {
        let geometry = layout.layers()[self.layer];
        let (_, bounds) = canvas_pilot::formats::limits(
            layout.formats().input(self.layer)?,
            [geometry.width, geometry.height],
            self.framing.zoom,
        )?;
        for (offset, bound) in self.framing.offset.iter_mut().zip(bounds) {
            *offset = offset.clamp(-bound, bound);
        }
        Ok(bounds)
    }
    fn apply_framing(&mut self, layout: &mut Layout, dragging: bool) -> Result<(), String> {
        self.clamp_offsets(layout)?;
        if dragging && self.crop_before.is_none() {
            self.crop_before = Some(layout.clone());
        }
        let mut next = self.crop_before.as_ref().unwrap_or(layout).clone();
        next.set_framing(self.layer, self.framing)?;
        *layout = next;
        self.framing = layout
            .formats()
            .mapping(self.layer, layout.layers()[self.layer])?
            .framing;
        self.baseline.4 = self.framing;
        Ok(())
    }
    fn apply_layer_size(
        &mut self,
        layout: &mut Layout,
        reference: PositionReference,
        dragging: bool,
    ) -> Result<(), String> {
        if dragging && self.layer_before.is_none() {
            self.layer_before = Some(layout.clone());
        }
        let mut next = self.layer_before.as_ref().unwrap_or(layout).clone();
        let g = next.layers()[self.layer];
        let output = next.formats().output;
        let [x, y] = reference.offsets_on(g, output);
        let geometry = reference.geometry_on(x, y, self.size[0], self.size[1], output)?;
        next.apply(self.layer, geometry)?;
        self.framing = next.formats().mapping(self.layer, geometry)?.framing;
        self.baseline.3 = self.size;
        self.baseline.4 = self.framing;
        *layout = next;
        Ok(())
    }
    fn commit_ratio(&mut self, locked: bool) -> Result<(), String> {
        self.ratio_edit.commit()?;
        if locked && let Some(ratio) = self.ratio_edit.target {
            let size = [self.size[0], self.size[0] / ratio];
            if let Err(error) = canvas_pilot::formats::validate_size(size) {
                self.ratio_edit.dirty = true;
                return Err(error);
            }
            self.size = size;
            self.ratio_edit = Default::default();
        }
        Ok(())
    }
    fn apply(
        &mut self,
        layout: &mut Layout,
        corner: PositionReference,
        locked: bool,
    ) -> Result<(), String> {
        self.commit_ratio(locked)?;
        let input = self.source.input()?;
        if input
            .iter()
            .zip(self.input_size)
            .any(|(a, b)| !b.is_finite() || (a - b).abs() > 1e-7)
        {
            return Err("Input resolution is invalid or does not match the crop edges.".into());
        }
        let g = layout.layers()[self.layer];
        let [x, y] = corner.offsets_on(g, layout.formats().output);
        let geometry =
            corner.geometry_on(x, y, self.size[0], self.size[1], layout.formats().output)?;
        let framing_changed = self.framing != layout.formats().mapping(self.layer, g)?.framing;
        if framing_changed {
            canvas_pilot::formats::crop_from_framing(input, self.size, self.framing)?;
        }
        layout.configure(self.layer, self.source_id, self.source, geometry, true)?;
        if framing_changed {
            layout.set_framing(self.layer, self.framing)?;
        }
        Ok(())
    }
    pub(super) fn is_for(&self, layer: usize) -> bool {
        self.layer == layer
    }
    pub(super) fn new(layout: &Layout, i: usize) -> Self {
        let source_id = layout.formats().source_for[i];
        let source = layout.formats().sources[source_id];
        let g = layout.layers()[i];
        Self {
            layer: i,
            crop_before: None,
            mask_before: None,
            layer_before: None,
            baseline: (
                source_id,
                source,
                source.input().unwrap(),
                [g.width, g.height],
                layout.formats().mapping(i, g).unwrap().framing,
            ),
            source_id,
            source,
            input_size: source.input().unwrap(),
            size: [g.width, g.height],
            layer_aspect: Aspect::Custom,
            ratio_edit: Default::default(),
            framing: layout.formats().mapping(i, g).unwrap().framing,
            error: String::new(),
            closing: false,
        }
    }
}
pub(super) struct OutputDraft {
    size: [f64; 2],
    aspect: Aspect,
    locked: bool,
    error: String,
    observed_size: [f64; 2],
}
impl OutputDraft {
    pub(super) fn pending(&self, layout: &Layout) -> bool {
        self.size != layout.formats().output.map(f64::from)
    }
    pub(super) fn new(output: [u32; 2]) -> Self {
        Self {
            size: output.map(f64::from),
            aspect: Aspect::ALL
                .into_iter()
                .find(|aspect| {
                    aspect.ratio().is_some_and(|ratio| {
                        (output[0] as f64 / output[1] as f64 - ratio).abs() < 1e-9
                    })
                })
                .unwrap_or(Aspect::Custom),
            locked: false,
            error: String::new(),
            observed_size: output.map(f64::from),
        }
    }
}
fn aspect_combo(ui: &mut egui::Ui, id: &str, aspect: &mut Aspect) -> bool {
    let old = *aspect;
    egui::ComboBox::from_id_salt(id)
        .selected_text(aspect.label())
        .show_ui(ui, |ui| {
            for a in Aspect::ALL {
                ui.stable_selectable_value(aspect, a, a.label());
            }
        });
    old != *aspect
}
fn nonnegative_number(text: &str) -> Option<f64> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
}
fn dimensions(ui: &mut egui::Ui, size: &mut [f64; 2], prefix: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(prefix);
        ui.add(
            egui::DragValue::new(&mut size[0])
                .speed(1.0)
                .custom_formatter(|v, _| number(v))
                .prefix(tr(ui.ctx(), "ui.w")),
        );
        ui.add(
            egui::DragValue::new(&mut size[1])
                .speed(1.0)
                .custom_formatter(|v, _| number(v))
                .prefix(tr(ui.ctx(), "ui.h")),
        );
        if size[0] > 0.0 && size[1] > 0.0 {
            ui.label(format!("Ratio {}", ratio_ui::label(size[0] / size[1])));
        }
    });
}
fn resolution(ui: &mut egui::Ui, size: &mut [u32; 2], prefix: &str, ratio: Option<f64>) {
    ui.horizontal_wrapped(|ui| {
        ui.label(prefix);
        if ui
            .add(
                egui::DragValue::new(&mut size[0])
                    .speed(1)
                    .prefix(tr(ui.ctx(), "ui.w")),
            )
            .changed()
            && let Some(r) = ratio
        {
            size[1] = (size[0] as f64 / r).round() as u32;
        }
        if ui
            .add(
                egui::DragValue::new(&mut size[1])
                    .speed(1)
                    .prefix(tr(ui.ctx(), "ui.h")),
            )
            .changed()
            && let Some(r) = ratio
        {
            size[0] = (size[1] as f64 * r).round() as u32;
        }
        if size[0] > 0 && size[1] > 0 {
            ui.label(format!(
                "Ratio {}",
                ratio_ui::label(size[0] as f64 / size[1] as f64)
            ));
        }
    });
}
fn presets(ui: &mut egui::Ui, size: &mut [u32; 2], aspect: &mut Aspect) {
    ui.horizontal_wrapped(|ui| {
        for (name, value) in [
            ("HD", [1280, 720]),
            ("Full HD", [1920, 1080]),
            ("UHD 4K", [3840, 2160]),
        ] {
            if ui.button(name).clicked() {
                *size = value;
                *aspect = Aspect::SixteenNine;
            }
        }
    });
}
impl Editor {
    pub(super) fn output_window(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.output_draft.take() else {
            return;
        };
        if d.error.is_empty() && d.size == d.observed_size && d.pending(&self.layout) {
            let locked = d.locked;
            d = OutputDraft::new(self.layout.formats().output);
            d.locked = locked;
        }
        let mut open = true;
        egui::Window::new(tr(ctx, "ui.output-format-viewport-resolution")).id(egui::Id::new("Output format / viewport resolution")).open(&mut open).default_width(470.0).show(ctx,|ui| {
            ui.horizontal_wrapped(|ui| {for (name,value) in [("HD",[1280.0,720.0]),("Full HD",[1920.0,1080.0]),("UHD 4K",[3840.0,2160.0])] {
                if ui.button(name).clicked() {d.size=value;d.aspect=Aspect::SixteenNine;}
            }});
            ui.horizontal(|ui| {
                ui.label(tr(ui.ctx(), "ui.output-format"));
                if aspect_combo(ui,"output-aspect",&mut d.aspect) && let Some(r)=d.aspect.ratio() { d.size[1]=(d.size[0]/r).round(); d.locked=true; }
                ui.checkbox(&mut d.locked,tr(ui.ctx(), "ui.link-dimensions"));
            });
            let before=d.size;
            dimensions(ui,&mut d.size,"Custom resolution");
            if d.locked && before.iter().all(|v|v.is_finite() && *v>0.0) {
                let ratio=d.aspect.ratio().unwrap_or(before[0]/before[1]);
                if before[0]!=d.size[0] {d.size[1]=(d.size[0]/ratio).round();}
                else if before[1]!=d.size[1] {d.size[0]=(d.size[1]*ratio).round();}
            }
            ui.label(format!("Proposed output: {:.1} × {:.1} px. Linked dimensions round the other side to whole pixels.",d.size[0],d.size[1]));
            ui.label(tr(ui.ctx(), "ui.changing-output-preserves-layer-positions-and-sizes-custom-dimensions-are-planning-si"));
            if d.size != d.observed_size {
                d.observed_size = d.size;
                match canvas_pilot::formats::resolution_from_values(d.size).and_then(|size|self.layout.set_output(size)) { Ok(())=>{ self.zoom=1.0;self.pan=Vec2::ZERO;self.sync(); d.error.clear(); },Err(e)=>d.error=e }
            }
            if !d.error.is_empty() { ui.colored_label(theme::palette(ui.ctx()).amber,&d.error); }
            ui.separator();
            ui.label(tr(ui.ctx(), "ui.viewport-camera-does-not-change-layer-or-crop-geometry"));
            ui.horizontal(|ui| {
                let mut zoom=self.zoom*100.0;
                if ui.add(egui::DragValue::new(&mut zoom).range(0.01..=25600.0).suffix(tr(ui.ctx(), "ui.of-fit-to-screen"))).changed() { self.zoom=zoom/100.0; }
                if ui.button(tr(ui.ctx(), "ui.fit-screen")).clicked() {self.zoom=1.0;self.pan=Vec2::ZERO;}
                if ui.button(tr(ui.ctx(), "ui.1-1-display-pixels")).clicked() {self.camera_one_to_one=true;}
            });
        });
        if open {
            self.output_draft = Some(d);
        }
    }
    pub(super) fn finish_crop_edit(&mut self) {
        if let Some(draft) = self
            .format_draft
            .as_mut()
            .filter(|d| d.pending(&self.layout))
        {
            draft.closing = true;
        } else {
            self.format_draft = None;
        }
        self.edit_content = false;
        self.edit_mask = false;
        self.alt_mask = false;
    }

    pub(super) fn sync_crop_tab_mode(&mut self) {
        let mask = self.property_tab == PropertyTab::Mask;
        let active = (self.property_tab == PropertyTab::Crop || mask)
            && self.format_draft.as_ref().is_some_and(|d| {
                !d.closing
                    && self.layout.selected() == Some(d.layer)
                    && (mask
                        || self.layout.formats().layer_fit[d.layer]
                            .is_none_or(|fit| fit == canvas_pilot::formats::LayerFit::OneToOne))
            });
        if active || self.crop_tab_editing {
            self.edit_content = active;
            self.edit_mask = active && mask;
        }
        self.crop_tab_editing = active;
    }
    pub(super) fn formats_window(&mut self, ctx: &egui::Context) {
        self.formats_window_contents(ctx);
        self.sync_crop_tab_mode();
    }
    fn formats_window_contents(&mut self, ctx: &egui::Context) {
        let position_reference = self.position_reference();
        let Some(mut d) = self.format_draft.take() else {
            return;
        };
        if self.layout.selected() != Some(d.layer) {
            return;
        }
        let i = d.layer;
        let mut open = true;
        egui::Window::new(rust_i18n::t!("layer.properties-title", locale = &localization::locale(ctx), name = self.layout.layer_label(i)).into_owned()).id(egui::Id::new(("layer-properties", self.layout.layer_info(i).id))).open(&mut open).default_width(540.0).default_height(720.0).min_height(380.0).vscroll(true).show(ctx,|ui| {
            ui.horizontal(|ui| {
                ui.stable_selectable_value(&mut self.property_tab,PropertyTab::Source,tr(ui.ctx(), "ui.source"));
                ui.stable_selectable_value(&mut self.property_tab,PropertyTab::Layer,tr(ui.ctx(), "ui.layer-ac9ffc"));
                ui.stable_selectable_value(&mut self.property_tab,PropertyTab::Crop,tr(ui.ctx(), "ui.crop"));
                ui.stable_selectable_value(&mut self.property_tab,PropertyTab::Mask,tr(ui.ctx(), "ui.mask"));
                ui.stable_selectable_value(&mut self.property_tab,PropertyTab::Mapping,tr(ui.ctx(), "ui.mapping"));
                ui.stable_selectable_value(&mut self.property_tab,PropertyTab::Preview,tr(ui.ctx(), "ui.preview"));
            });
            ui.separator();
            if self.property_tab == PropertyTab::Mapping {
                ui.heading(tr(ui.ctx(), "ui.layer-mapping"));
                self.layer_mapping_controls(ui, i);
                ui.separator();
            }
            if self.property_tab == PropertyTab::Source {
            ui.heading(tr(ui.ctx(), "ui.advanced-source-input-settings"));
            self.advanced_input_controls(ui, i);
            ui.separator();
            ui.label(tr(ui.ctx(), "ui.editor-source-model-local-geometry"));
            ui.small(tr(ui.ctx(), "ui.these-dimensions-describe-the-editor-source-not-a-device-timing-command-local-input-c"));
            ui.add_enabled_ui(self.device.input_edits.enabled, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr(ui.ctx(), "ui.editor-input"));
                egui::ComboBox::from_id_salt("source-id").selected_text(format!("Input {}",d.source_id+1)).show_ui(ui,|ui| {
                    let count = self.layout.formats().sources.len().max(8).max(d.source_id + 1);
                    for id in 0..count { if ui.stable_selectable_value(&mut d.source_id,id,format!("Input {}",id+1)).changed() { d.source=self.layout.formats().sources.get(id).copied().unwrap_or_default();d.input_size=d.source.input().unwrap(); } }
                    ui.separator();
                    if ui.add_enabled(count < 1024, egui::Button::new(tr(ui.ctx(), "ui.new-editor-input"))).clicked() {
                        d.source_id = count;
                        d.source = Default::default();
                        d.input_size = d.source.input().unwrap();
                        ui.close();
                    }
                });
            });
                ui.small(tr(ui.ctx(), "ui.editor-inputs-share-source-format-and-input-crop-between-editor-layers-inputs-1-8-are"));

            let affected:Vec<_>=(0..self.layout.layers().len()).filter(|j| *j==i || self.layout.formats().source_for[*j]==d.source_id).map(|j|self.layout.layer_label(j)).collect();
            ui.label(format!("Input edits affect: {}",affected.join(", ")));
            let old_native=d.source.native;
            ui.horizontal(|ui| {
                ui.label(tr(ui.ctx(), "ui.source-format"));
                if aspect_combo(ui,"source-aspect",&mut d.source.aspect) && let Some(r)=d.source.aspect.ratio() { d.source.native[1]=(d.source.native[0] as f64/r).round() as u32; }
            });
            presets(ui,&mut d.source.native,&mut d.source.aspect);
            resolution(ui,&mut d.source.native,"Native pixels",d.source.aspect.ratio());
            if old_native!=d.source.native { d.source.input_crop=Crop::default();d.input_size=d.source.native.map(f64::from); }
            ui.label(tr(ui.ctx(), "ui.custom-input-resolution-retains-the-source-format-above"));
            let old_input_size=d.input_size;
            dimensions(ui,&mut d.input_size,"Input resolution");
            if old_input_size!=d.input_size {
                match Crop::centered(d.source.native.map(f64::from),d.input_size) {
                    Ok(c)=>{d.source.input_crop=c;d.error.clear();},Err(e)=>d.error=e
                }
            }
            ui.horizontal(|ui| {
                if ui.button(tr(ui.ctx(), "ui.center-input-crop-to-this-size")).clicked() {
                    match Crop::centered(d.source.native.map(f64::from),d.input_size) { Ok(c)=>{d.source.input_crop=c;d.error.clear();},Err(e)=>d.error=e }
                }
                if ui.button(tr(ui.ctx(), "ui.use-full-input")).clicked() { d.source.input_crop=Crop::default();d.input_size=d.source.native.map(f64::from); }
            });
            let old_crop=d.source.input_crop;
            ui.horizontal_wrapped(|ui| {
                for (title,value) in [("L ",&mut d.source.input_crop.left),("R ",&mut d.source.input_crop.right),("T ",&mut d.source.input_crop.top),("B ",&mut d.source.input_crop.bottom)] { ui.add(egui::DragValue::new(value).range(0.0..=f64::MAX).custom_parser(nonnegative_number).speed(1.0).custom_formatter(|v,_|number(v)).prefix(title)); }
            });
            if old_crop!=d.source.input_crop && let Ok(size)=d.source.input() { d.input_size=size; }
            ui.label(tr(ui.ctx(), "ui.input-crop-edges-native-source-pixels-native-size-changes-reset-the-draft-input-crop"));
            });
            }
            if self.property_tab == PropertyTab::Layer {
            let before_size = d.size;
            ui.heading(tr(ui.ctx(), "ui.layer-resolution-final-visible-box"));
            ui.label(tr(ui.ctx(), "ui.resizing-automatically-adjusts-the-layer-crop-to-fit-the-new-dimensions"));
            ui.horizontal(|ui| {
                ui.label(tr(ui.ctx(), "ui.format"));
                if aspect_combo(ui,"layer-aspect",&mut d.layer_aspect) && let Some(r)=d.layer_aspect.ratio() {
                    d.ratio_edit.text=ratio_ui::label(r);d.ratio_edit.target=Some(r);d.ratio_edit.dirty=false;
                    if let Err(error)=d.commit_ratio(self.keep_aspect_ratio) {d.error=error;}
                }
            });
            let lock_changed=ui.checkbox(&mut self.keep_aspect_ratio,tr(ui.ctx(), "ui.keep-aspect-ratio-when-resizing")).changed();
            let ratio_changed=d.ratio_edit.show(ui,d.size[0]/d.size[1]);
            if ratio_changed {d.layer_aspect=Aspect::Custom;}
            if ratio_changed || (lock_changed && self.keep_aspect_ratio) {
                match d.commit_ratio(self.keep_aspect_ratio) {Ok(())=>d.error.clear(),Err(error)=>d.error=error}
            }
            if !self.keep_aspect_ratio && d.ratio_edit.target.is_some() {ui.small(tr(ui.ctx(), "ui.ratio-ready-enable-the-lock-to-resize"));}
            if lock_changed {self.settings.save(&self.application_settings(ui.ctx()));}
            let old=d.size;
            dimensions(ui,&mut d.size,"Visible pixels");
            if self.keep_aspect_ratio {
                let ratio=old[0]/old[1];
                if d.size[0]!=old[0] { d.size[1]=d.size[0]/ratio; } else if d.size[1]!=old[1] { d.size[0]=d.size[1]*ratio; }
            }
            if d.size != before_size {
                match d.apply_layer_size(&mut self.layout, position_reference, ui.input(|input| input.pointer.primary_down())) {
                    Ok(()) => { self.sync(); d.error.clear(); },
                    Err(error) => d.error = error,
                }
            }
            ui.small(tr(ui.ctx(), "ui.layer-size-changes-apply-as-you-edit-undo-restores-earlier-values"));
            }
            if !ui.input(|input| input.pointer.primary_down()) { d.layer_before = None; }
            if self.property_tab == PropertyTab::Source {
            let g=self.layout.layers()[i];
            let [x,y]=self.position_reference().offsets_on(g,self.layout.formats().output);
            let geometry=self.position_reference().geometry_on(x,y,d.size[0],d.size[1],self.layout.formats().output);
            let mut preview=self.layout.clone();
            let input_matches=d.source.input().is_ok_and(|size| (size[0]-d.input_size[0]).abs()<1e-7 && (size[1]-d.input_size[1]).abs()<1e-7);
            let reviewed=if input_matches {geometry.and_then(|geometry|preview.configure(i,d.source_id,d.source,geometry,true))} else {Err("Input resolution is invalid or does not match the crop edges.".into())};
            match &reviewed {
                Ok(message) if !message.is_empty()=>{ ui.label(message); },
                Err(e)=>{ ui.colored_label(theme::palette(ui.ctx()).amber,e); },
                _=>{}
            }
            let pending = d.pending(&self.layout);
            if apply_button(ui,&tr(ui.ctx(), "action.apply-source"), pending && self.device.input_edits.enabled).clicked() {
                match d.apply(&mut self.layout, position_reference, self.keep_aspect_ratio) { Ok(())=>{self.sync();d=FormatDraft::new(&self.layout,i);},Err(e)=>d.error=e }
            }
            ui.small(tr(ui.ctx(), "ui.source-changes-require-apply-switching-tabs-keeps-pending-source-edits"));
            }
            if self.property_tab == PropertyTab::Mask {
                ui.heading(tr(ui.ctx(), "ui.layer-mask-blanking"));
                ui.label(tr(ui.ctx(), "ui.the-magenta-box-reveals-pixels-everything-outside-it-is-blanked-image-scale-crop-and"));
                let old=self.layout.formats().masks[i];
                let mut mask=old;
                let g=self.layout.layers()[i];
                ui.horizontal(|ui| {
                    ui.label(tr(ui.ctx(), "mask.origin"));
                    egui::ComboBox::from_id_salt("mask-origin")
                        .selected_text(tr(ui.ctx(), if mask.origin == canvas_pilot::formats::MaskOrigin::Layer { "mask.layer" } else { "mask.source" }))
                        .show_ui(ui, |ui| {
                            ui.stable_selectable_value(&mut mask.origin, canvas_pilot::formats::MaskOrigin::Layer, tr(ui.ctx(), "mask.layer"));
                            ui.stable_selectable_value(&mut mask.origin, canvas_pilot::formats::MaskOrigin::Source, tr(ui.ctx(), "mask.source"));
                        });
                });
                ui.small(tr(ui.ctx(), "mask.origin-help"));
                let source_relative = mask.origin == canvas_pilot::formats::MaskOrigin::Source;
                let size = if source_relative { self.layout.formats().input(i).unwrap_or([g.width,g.height]) } else { [g.width,g.height] };
                ui.horizontal_wrapped(|ui| {
                    for (label,edge,opposite,size) in [("Left ",&mut mask.left,old.right,size[0]),("Right ",&mut mask.right,old.left,size[0]),("Top ",&mut mask.top,old.bottom,size[1]),("Bottom ",&mut mask.bottom,old.top,size[1])] {
                        let mut pixels=*edge*size;
                        if ui.add(egui::DragValue::new(&mut pixels).range(0.0..=(1.0-opposite)*size).speed(1.0).prefix(label).suffix(tr(ui.ctx(), if source_relative { "mask.source-px" } else { "ui.output-px" }))).changed() { *edge=(pixels/size).clamp(0.0,1.0-opposite); }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button(tr(ui.ctx(), "ui.reset-mask")).clicked() {mask=canvas_pilot::formats::Mask { origin: mask.origin, ..Default::default() };}
                    if ui.button(tr(ui.ctx(), "ui.center-mask")).clicked() {let x=(mask.left+mask.right)/2.0;let y=(mask.top+mask.bottom)/2.0;mask.left=x;mask.right=x;mask.top=y;mask.bottom=y;}
                });
                if mask != old {
                    let dragging=ui.input(|input|input.pointer.primary_down());
                    if dragging && d.mask_before.is_none() { d.mask_before=Some(self.layout.clone()); }
                    let mut next=d.mask_before.as_ref().unwrap_or(&self.layout).clone();
                    if let Err(e)=next.set_mask(i,mask) { d.error=e; } else {self.layout=next;self.sync();d.error.clear();}
                }
                if !ui.input(|input|input.pointer.primary_down()) { d.mask_before=None; }
                ui.small(tr(ui.ctx(), "ui.drag-any-edge-or-corner-independently-drag-inside-the-layer-to-move-the-window-arrows"));
                ui.small(tr(ui.ctx(), "mask.proportions-help"));
                let (_,area)=ui.allocate_space(Vec2::new(ui.available_width(),170.0));
                paint_layer_source(ui.painter(),fit_rect(area,[g.width,g.height]),&self.layout,i,self.layer_preview(i));
            }
            if self.property_tab == PropertyTab::Crop {
            ui.heading(tr(ui.ctx(), "ui.layer-crop-zoom-position"));
            let mut advanced_fit = self.layout.formats().layer_fit[i].is_some();
            if ui.checkbox(&mut advanced_fit, tr(ui.ctx(), "ui.advanced-layer-fit")).changed() {
                let mode = advanced_fit.then_some(canvas_pilot::formats::LayerFit::Centered);
                let crop = self.layout.formats().advanced[i];
                match self.layout.set_layer_fit(i, mode, crop) {
                    Ok(()) => { self.edit_content = false; self.sync(); d.refresh_fit(&self.layout); }
                    Err(e) => d.error = e,
                }
            }
            if let Some(current_fit) = self.layout.formats().layer_fit[i] {
                let mut fit = current_fit;
                egui::ComboBox::from_id_salt("explicit-layer-fit").selected_text(fit.label()).show_ui(ui, |ui| {
                    for option in canvas_pilot::formats::LayerFit::ALL { ui.stable_selectable_value(&mut fit, option, option.label()); }
                });
                ui.label(tr(ui.ctx(), "ui.crop-first-then-fit-into-the-layer-rectangle-resize-changes-the-container-1-1-keeps-t"));
                let old_crop = self.layout.formats().advanced[i];
                let mut crop = old_crop;
                let input = self.layout.formats().input(i).unwrap();
                ui.horizontal_wrapped(|ui| {
                    for (label, edge, bound) in [("Left ",&mut crop.left,input[0]-old_crop.right), ("Right ",&mut crop.right,input[0]-old_crop.left), ("Top ",&mut crop.top,input[1]-old_crop.bottom), ("Bottom ",&mut crop.bottom,input[1]-old_crop.top)] {
                        ui.add(egui::DragValue::new(edge).range(0.0..=(bound-0.001).max(0.0)).speed(1.0).prefix(label).suffix(tr(ui.ctx(), "ui.source-px")));
                    }
                });
                if ui.button(tr(ui.ctx(), "ui.reset-layer-crop")).clicked() { crop = Crop::default(); }
                if fit != current_fit || crop != old_crop {
                    match self.layout.set_layer_fit(i, Some(fit), crop) {
                        Ok(()) => { self.sync(); d.refresh_fit(&self.layout); }
                        Err(e) => d.error = e,
                    }
                }
                let current = self.layout.layers()[i];
                let (_, area) = ui.allocate_space(Vec2::new(ui.available_width(),170.0));
                paint_layer_source(ui.painter(),fit_rect(area,[current.width,current.height]), &self.layout, i, self.layer_preview(i));
                if current_fit == canvas_pilot::formats::LayerFit::OneToOne {
                    ui.small(tr(ui.ctx(), "ui.drag-helper-edges-to-trim-without-zooming-drag-anywhere-inside-the-layer-including-em"));
                } else if matches!(current_fit, canvas_pilot::formats::LayerFit::Centered | canvas_pilot::formats::LayerFit::Cropped) {
                    ui.small(tr(ui.ctx(), "ui.this-mode-fits-automatically-the-canvas-crop-helper-is-disabled-numeric-crop-edges-re"));
                } else {
                    ui.small(tr(ui.ctx(), "ui.edit-crop-edges-here-visual-zoom-drag-controls-are-for-normal-crop-mode"));
                }
                ui.small(tr(ui.ctx(), "ui.disabling-advanced-fit-restores-normal-cover-framing-device-subpixel-rendering-is-app"));
            } else {

            ui.label(tr(ui.ctx(), "ui.zoom-and-position-change-the-crop-values-inside-this-layer-keeping-its-visible-box-fi"));
            let before_framing=d.framing;
            let mut percent=d.framing.zoom*100.0;
            ui.horizontal(|ui| {
                if ui.add(egui::DragValue::new(&mut percent).range(100.0..=10000.0).custom_parser(nonnegative_number).speed(1.0).custom_formatter(|v,_|number(v)).suffix(tr(ui.ctx(), "ui.content-zoom"))).changed() {d.framing.zoom=percent/100.0;}
                match d.clamp_offsets(&self.layout) {
                    Ok(bounds) => {
                        for (axis, bound) in bounds.into_iter().enumerate() {
                            ui.add(egui::DragValue::new(&mut d.framing.offset[axis]).range(-bound..=bound).speed(1.0).custom_formatter(|v,_|number(v)).prefix(if axis == 0 { "X " } else { "Y " }));
                        }
                    }
                    Err(error) => d.error = error,
                }
            });
            let current=self.layout.layers()[i];
            if let Ok((_,limit))=canvas_pilot::formats::limits(self.layout.formats().input(i).unwrap(),[current.width,current.height],d.framing.zoom) {
                ui.label(format!("Offsets in output px: X ±{:.1}, Y ±{:.1}. Positive moves content right/down.",limit[0],limit[1]));
            }
            ui.horizontal(|ui| {
                let apply=d.framing != before_framing;
                let recenter=ui.button(tr(ui.ctx(), "ui.recenter")).clicked();
                let reset=ui.button(tr(ui.ctx(), "ui.reset-crop")).clicked();
                if recenter { d.framing=self.layout.formats().mapping(i,current).unwrap().framing; d.framing.offset=[0.0;2]; }
                if reset {d.framing=Framing::default();}
                if apply||recenter||reset {
                    match d.apply_framing(&mut self.layout, ui.input(|input| input.pointer.primary_down())) {Ok(())=>{self.sync();d.error.clear();},Err(e)=>d.error=e}
                }
            });
            if !ui.input(|input| input.pointer.primary_down()) { d.crop_before = None; }
            ui.small(tr(ui.ctx(), "ui.crop-zoom-and-offsets-apply-as-you-edit-offsets-stop-at-the-content-edges-and-adjust"));
            }
            }
            if self.property_tab == PropertyTab::Preview {
            ui.heading(tr(ui.ctx(), "ui.input-preview-crop-guides"));
            self.crop_preview_controls(ui, i);
            let current = self.layout.layers()[i];
            let (_,rect)=ui.allocate_space(Vec2::new(ui.available_width(),170.0));
            let preview_rect=fit_rect(rect,[current.width,current.height]);
            paint_layer_source(ui.painter(),preview_rect,&self.layout,i,self.layer_preview(i));
            ui.label(tr(ui.ctx(), "ui.captured-stills-follow-this-layer-s-input-and-dynamic-crops-guides-are-drawn-above-th"));
            }
            if !d.error.is_empty() {ui.colored_label(theme::palette(ui.ctx()).amber,&d.error);}
            ui.collapsing("Calculated crops / provisional WebRCS mapping",|ui| {
                if let Ok(report)=self.layout.calculation_report(i) {
                    ui.label(&report);
                    if ui.button(tr(ui.ctx(), "ui.copy-provisional-calculation")).clicked() && let Ok(export)=self.layout.calculation_export(i) {ctx.copy_text(export);}
                }
            });
        });
        if !open && d.pending(&self.layout) {
            d.closing = true;
        }
        if d.closing {
            match pending_close(ctx, "formats-close", &d.error) {
                Some(true) => {
                    match d.apply(&mut self.layout, position_reference, self.keep_aspect_ratio) {
                        Ok(()) => {
                            self.sync();
                            return;
                        }
                        Err(e) => {
                            d.error = e;
                            d.closing = false;
                            open = true;
                        }
                    }
                }
                Some(false) => return,
                None => {}
            }
        }
        if open || d.closing {
            self.format_draft = Some(d);
        }
    }
}
fn fit_rect(rect: Rect, size: [f64; 2]) -> Rect {
    let k = (rect.width() / size[0] as f32).min(rect.height() / size[1] as f32);
    Rect::from_center_size(rect.center(), Vec2::new(size[0] as f32, size[1] as f32) * k)
}
/// Full usable input bounds in the same coordinate space as the visible box.
pub(super) fn content_bounds(rect: Rect, source: Source, crop: Crop) -> Option<Rect> {
    let input = source.input().ok()?;
    let retained = crop.remaining(input).ok()?;
    let k = rect.width() as f64 / retained[0];
    Some(Rect::from_min_size(
        rect.min - Vec2::new((crop.left * k) as f32, (crop.top * k) as f32),
        Vec2::new((input[0] * k) as f32, (input[1] * k) as f32),
    ))
}

/// Reveal only the usable input discarded by the layer crop, behind other layers.
pub(super) fn paint_content_overflow(
    painter: &egui::Painter,
    rect: Rect,
    source: Source,
    crop: Crop,
    preview: Option<(egui::TextureId, f32)>,
) {
    let Some(full) = content_bounds(rect, source, crop) else {
        return;
    };
    // Disjoint strips avoid painting the visible box twice or doubling opacity.
    for clip in outside_strips(full, rect) {
        let clip = clip.intersect(painter.clip_rect());
        if clip.is_positive() {
            let mut ghost = painter.with_clip_rect(clip);
            ghost.multiply_opacity(0.5);
            paint_source_clipped(&ghost, rect, source, crop, clip, preview);
        }
    }
}

/// Show the usable source as a faded guide behind the actual 1:1 image.
pub(super) fn paint_one_to_one_helper(
    painter: &egui::Painter,
    image: Rect,
    source: Source,
    crop: Crop,
    preview: Option<(egui::TextureId, f32)>,
) {
    if let Some(full) = content_bounds(image, source, crop) {
        let mut ghost = painter.clone();
        ghost.multiply_opacity(0.3);
        paint_source_clipped(&ghost, image, source, crop, full, preview);
    }
}

pub(super) fn layer_mask_rect(rect: Rect, layout: &Layout, i: usize) -> Option<Rect> {
    let formats = layout.formats();
    let mask = formats.masks[i];
    let reference = formats.mask_reference(i, layout.layers()[i]).ok()?;
    let layer = layout.layers()[i];
    let k = rect.width() as f64 / layer.width;
    let mask_rect = Rect::from_min_size(
        rect.min
            + Vec2::new(
                ((reference.x - layer.x) * k) as f32,
                ((reference.y - layer.y) * k) as f32,
            ),
        Vec2::new((reference.width * k) as f32, (reference.height * k) as f32),
    );
    let clip = Rect::from_min_max(
        mask_rect.min
            + Vec2::new(
                mask_rect.width() * mask.left as f32,
                mask_rect.height() * mask.top as f32,
            ),
        mask_rect.max
            - Vec2::new(
                mask_rect.width() * mask.right as f32,
                mask_rect.height() * mask.bottom as f32,
            ),
    );
    Some(clip)
}

/// Four disjoint strips; a collapsed or disjoint reveal leaves the whole area hidden.
pub(super) fn outside_strips(outer: Rect, reveal: Rect) -> [Rect; 4] {
    let reveal = outer.intersect(reveal);
    if !reveal.is_positive() {
        return [outer, Rect::NOTHING, Rect::NOTHING, Rect::NOTHING];
    }
    [
        Rect::from_min_max(outer.min, Pos2::new(outer.right(), reveal.top())),
        Rect::from_min_max(Pos2::new(outer.left(), reveal.bottom()), outer.max),
        Rect::from_min_max(Pos2::new(outer.left(), reveal.top()), reveal.left_bottom()),
        Rect::from_min_max(
            reveal.right_top(),
            Pos2::new(outer.right(), reveal.bottom()),
        ),
    ]
}

/// Editor-only context: hidden source remains readable, with hatching marking blanking.
pub(super) fn paint_mask_helper(
    painter: &egui::Painter,
    rect: Rect,
    layout: &Layout,
    i: usize,
    preview: Option<(egui::TextureId, f32)>,
) {
    let Some(reveal) = layer_mask_rect(rect, layout, i) else {
        return;
    };
    for strip in outside_strips(rect, reveal) {
        let clip = strip.intersect(painter.clip_rect());
        if !clip.is_positive() {
            continue;
        }
        let clipped = painter.with_clip_rect(clip);
        let mut ghost = clipped.clone();
        ghost.multiply_opacity(0.25);
        paint_layer_content(&ghost, rect, layout, i, preview);
        // Spacing is in screen points, so zoom cannot generate an unbounded grid.
        let spacing = 12.0;
        let mut sum = ((clip.left() + clip.top()) / spacing).floor() * spacing;
        let end = clip.right() + clip.bottom();
        let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(222, 108, 220, 85));
        while sum <= end {
            clipped.line_segment(
                [
                    Pos2::new(sum - clip.top(), clip.top()),
                    Pos2::new(sum - clip.bottom(), clip.bottom()),
                ],
                stroke,
            );
            sum += spacing;
        }
    }
}

pub(super) fn paint_layer_source(
    painter: &egui::Painter,
    rect: Rect,
    layout: &Layout,
    i: usize,
    preview: Option<(egui::TextureId, f32)>,
) {
    let Some(clip) = layer_mask_rect(rect, layout, i) else {
        return;
    };
    if !clip.is_positive() {
        return;
    }
    let clipped = painter.with_clip_rect(clip.intersect(rect));
    paint_layer_content(&clipped, rect, layout, i, preview);
}

fn paint_layer_content(
    painter: &egui::Painter,
    rect: Rect,
    layout: &Layout,
    i: usize,
    preview: Option<(egui::TextureId, f32)>,
) {
    let formats = layout.formats();
    let source = formats.sources[formats.source_for[i]];
    let crop = formats.advanced[i];
    let Some(fit) = formats.layer_fit[i] else {
        paint_source(painter, rect, source, crop, preview);
        return;
    };
    let g = layout.layers()[i];
    let Ok(retained) = source.input().and_then(|input| crop.remaining(input)) else {
        return;
    };
    let Ok(image) = fit.image(retained, g) else {
        return;
    };
    let scale = rect.width() as f64 / g.width;
    let image_rect = Rect::from_min_size(
        rect.min
            + Vec2::new(
                ((image.x - g.x) * scale) as f32,
                ((image.y - g.y) * scale) as f32,
            ),
        Vec2::new((image.width * scale) as f32, (image.height * scale) as f32),
    );
    let clip = image_rect.intersect(rect);
    if clip.is_positive() {
        paint_source_clipped(painter, image_rect, source, crop, clip, preview);
    }
}

pub(super) fn paint_source(
    painter: &egui::Painter,
    rect: Rect,
    source: Source,
    crop: Crop,
    preview: Option<(egui::TextureId, f32)>,
) {
    paint_source_clipped(painter, rect, source, crop, rect, preview);
}

fn paint_source_clipped(
    painter: &egui::Painter,
    rect: Rect,
    source: Source,
    crop: Crop,
    clip: Rect,
    preview: Option<(egui::TextureId, f32)>,
) {
    // The caller owns clipping: crop helpers deliberately extend beyond rect.
    // paint_layer_source already applies the actual layer and mask boundaries.
    let mut guides = painter.clone();
    if let Some((texture, opacity)) = preview {
        let Ok(input) = source.input() else {
            return;
        };
        let Ok(retained) = crop.remaining(input) else {
            return;
        };
        let k = rect.width() as f64 / retained[0];
        let ky = rect.height() as f64 / retained[1];
        let native = Rect::from_min_size(
            rect.min
                - Vec2::new(
                    ((source.input_crop.left + crop.left) * k) as f32,
                    ((source.input_crop.top + crop.top) * ky) as f32,
                ),
            Vec2::new(
                (source.native[0] as f64 * k) as f32,
                (source.native[1] as f64 * ky) as f32,
            ),
        );
        painter
            .with_clip_rect(painter.clip_rect().intersect(clip))
            .image(
                texture,
                native,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        if opacity <= 0.0 {
            return;
        }
        guides.multiply_opacity(opacity);
    }
    paint_pattern_clipped(&guides, rect, source, crop, clip);
}

fn paint_pattern_clipped(
    painter: &egui::Painter,
    rect: Rect,
    source: Source,
    crop: Crop,
    clip: Rect,
) {
    let palette = theme::palette(painter.ctx());
    let Ok(input) = source.input() else {
        return;
    };
    let Ok(retained) = crop.remaining(input) else {
        return;
    };
    let k = rect.width() as f64 / retained[0];
    let ky = rect.height() as f64 / retained[1];
    let p = painter.with_clip_rect(painter.clip_rect().intersect(clip));
    let point = |x: f64, y: f64| {
        rect.min
            + Vec2::new(
                ((x - source.input_crop.left - crop.left) * k) as f32,
                ((y - source.input_crop.top - crop.top) * ky) as f32,
            )
    };
    for row in 0..6 {
        for col in 0..10 {
            let x = col as f64 * source.native[0] as f64 / 10.0;
            let y = row as f64 * source.native[1] as f64 / 6.0;
            let cell = Rect::from_min_max(
                point(x, y),
                point(
                    x + source.native[0] as f64 / 10.0,
                    y + source.native[1] as f64 / 6.0,
                ),
            );
            if !cell.intersects(clip) {
                continue;
            }
            let tile = (row + col) % 2;
            let label_color = if theme::Theme::current(painter.ctx()) == theme::Theme::HighContrast
            {
                palette.pattern[1 - tile]
            } else {
                palette.text
            };
            p.rect_filled(cell, 0.0, palette.pattern[tile]);
            p.rect_stroke(
                cell,
                0.0,
                Stroke::new(1.0_f32, palette.muted),
                StrokeKind::Inside,
            );
            if cell.width() > 35.0 && cell.height() > 25.0 {
                p.text(
                    cell.center(),
                    Align2::CENTER_CENTER,
                    format!("{},{}", col, row),
                    FontId::proportional(11.0),
                    label_color,
                );
            }
        }
    }
    let center = point(source.native[0] as f64 / 2.0, source.native[1] as f64 / 2.0);
    let radius = source.native[0].min(source.native[1]) as f64 * 0.2;
    p.add(egui::epaint::EllipseShape::stroke(
        center,
        Vec2::new((radius * k) as f32, (radius * ky) as f32),
        Stroke::new(2.0_f32, palette.amber),
    ));
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click_text, frame, mouse};
    use super::*;
    #[test]
    fn enter_from_crop_or_mask_reviews_pending_properties_without_discarding() {
        for tab in [PropertyTab::Crop, PropertyTab::Mask] {
            let ctx = egui::Context::default();
            let mut app = Editor::default();
            app.layout.select(Some(1));
            app.property_tab = tab;
            let mut draft = FormatDraft::new(&app.layout, 1);
            draft.size = [800.0, 450.0];
            app.format_draft = Some(draft);
            frame(&mut app, &ctx, vec![]);
            let applied = app.project();
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert!(app.format_draft.as_ref().unwrap().closing);
            assert_eq!(app.project(), applied);
            click_text(&mut app, &ctx, "Discard");
            assert!(app.format_draft.is_none());
            assert_eq!(app.project(), applied);
        }
    }

    fn enter_value(app: &mut Editor, ctx: &egui::Context, label: &str, text: &str) {
        // DragValue renders prefix, number and suffix as separate atoms in egui 0.35.
        frame(app, ctx, vec![]);
        let output = frame(app, ctx, vec![]);
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        let pos = texts
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, t)| {
                let mut joined = String::new();
                for end in i..texts.len().min(i + 3) {
                    joined.push_str(texts[end].galley.text());
                    if joined == label {
                        let number = texts[i..=end]
                            .iter()
                            .find(|t| t.galley.text().trim().parse::<f64>().is_ok())
                            .copied()
                            .unwrap_or(t);
                        return Some(number.pos + number.galley.size() / 2.0);
                    }
                }
                None
            })
            .unwrap_or_else(|| panic!("Numeric control not rendered: {label}"));
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(app, ctx, vec![mouse(pos, false)]);
        frame(app, ctx, vec![egui::Event::Paste(text.into())]);
        frame(
            app,
            ctx,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        frame(app, ctx, vec![]);
    }

    #[test]
    fn advanced_fit_checkbox_modes_and_crop_controls_preserve_normal_default() {
        let ctx = egui::Context::default();
        let mut app = Editor {
            property_tab: PropertyTab::Crop,
            ..Default::default()
        };
        let i = app.layout.selected().unwrap();
        app.format_draft = Some(FormatDraft::new(&app.layout, i));
        app.format_draft.as_mut().unwrap().source_id = 1 - app.layout.formats().source_for[i];
        let pending_source_id = app.format_draft.as_ref().unwrap().source_id;
        frame(&mut app, &ctx, vec![]);
        assert!(app.layout.formats().layer_fit[i].is_none());
        click_text(&mut app, &ctx, "Advanced layer fit");
        assert_eq!(
            app.layout.formats().layer_fit[i],
            Some(canvas_pilot::formats::LayerFit::Centered)
        );
        assert!(!app.edit_content);
        click_text(&mut app, &ctx, "Centered (fit)");
        click_text(&mut app, &ctx, "1:1 pixels");
        assert!(app.edit_content);
        click_text(&mut app, &ctx, "1:1 pixels");
        click_text(&mut app, &ctx, "Fullscreen (stretch)");
        assert!(!app.edit_content);
        assert_eq!(
            app.layout.formats().layer_fit[i],
            Some(canvas_pilot::formats::LayerFit::Fullscreen)
        );
        click_text(&mut app, &ctx, "Reset layer crop");
        assert_eq!(app.layout.formats().advanced[i], Crop::default());
        click_text(&mut app, &ctx, "Advanced layer fit");
        assert!(app.layout.formats().layer_fit[i].is_none());
        assert_eq!(
            app.format_draft.as_ref().unwrap().source_id,
            pending_source_id
        );
        assert!(app.format_draft.as_ref().unwrap().pending(&app.layout));
        assert!(
            app.layout
                .formats()
                .mapping(i, app.layout.layers()[i])
                .is_ok()
        );
    }
    #[test]
    fn layer_size_autoapplies_without_committing_source_drafts() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let before = app.layout.layers().clone();
        let sources = app.layout.formats().sources.clone();
        let mut d = FormatDraft::new(&app.layout, 1);
        d.source.native = [1280, 720];
        d.input_size = [1280.0, 720.0];
        app.format_draft = Some(d);
        enter_value(&mut app, &ctx, "W 640.0", "800");
        assert_eq!(app.layout.layers()[1].width, 800.0);
        assert_eq!(app.layout.layers()[1].height, 450.0);
        assert_eq!(app.layout.formats().sources, sources);
        assert_eq!(
            app.format_draft.as_ref().unwrap().source.native,
            [1280, 720]
        );
        let output = frame(&mut app, &ctx, vec![]);
        assert!(!output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Apply source changes")));
        app.layout.undo();
        assert_eq!(app.layout.layers(), &before);
    }

    #[test]
    fn layer_size_drag_is_atomic_and_invalid_edits_keep_geometry() {
        let mut layout = Layout::default();
        let before = layout.layers().clone();
        let formats = layout.formats().clone();
        let mut d = FormatDraft::new(&layout, 1);
        for width in [700.0, 750.0, 800.0] {
            d.size = [width, 450.0];
            d.apply_layer_size(&mut layout, PositionPoint::Center.into(), true)
                .unwrap();
            assert_eq!(layout.layers()[1].center(), before[1].center());
        }
        let resized = layout.layers().clone();
        d.size = [-1.0, 450.0];
        assert!(
            d.apply_layer_size(&mut layout, PositionPoint::Center.into(), true)
                .is_err()
        );
        assert_eq!(layout.layers(), &resized);
        layout.undo();
        assert_eq!(layout.layers(), &before);
        assert_eq!(layout.formats(), &formats);
        layout.redo();
        assert_eq!(layout.layers(), &resized);
    }

    #[test]
    fn draft_ratio_changes_only_apply_to_document_on_apply() {
        let mut layout = Layout::default();
        let before = layout.layers().clone();
        let mut draft = FormatDraft::new(&layout, 1);
        draft.ratio_edit.text = "4:3".into();
        draft.ratio_edit.dirty = true;
        draft.commit_ratio(false).unwrap();
        assert_eq!(draft.size, [before[1].width, before[1].height]);
        draft.commit_ratio(true).unwrap();
        assert_eq!(draft.size, [before[1].width, before[1].width * 0.75]);
        assert_eq!(layout.layers(), &before);
        assert!(draft.pending(&layout));
        draft
            .apply(&mut layout, PositionPoint::Center.into(), true)
            .unwrap();
        assert_eq!(layout.layers()[1].center(), before[1].center());
        assert_eq!(layout.layers()[1].height, before[1].width * 0.75);
        layout.undo();
        assert_eq!(layout.layers(), &before);
        draft.ratio_edit.text = "0:9".into();
        draft.ratio_edit.dirty = true;
        assert!(
            draft
                .apply(&mut layout, PositionPoint::Center.into(), true)
                .is_err()
        );
        assert_eq!(layout.layers(), &before);
    }

    #[test]
    fn input_crop_controls_reject_negative_paste_and_keep_fractional_values() {
        let ctx = egui::Context::default();
        let mut app = Editor {
            property_tab: PropertyTab::Source,
            ..Default::default()
        };
        app.device.input_edits.enabled = true;
        let crop = Crop {
            left: 20.0,
            right: 20.0,
            top: 20.0,
            bottom: 20.0,
        };
        let source = Source {
            input_crop: crop,
            ..Source::default()
        };
        app.layout
            .configure(1, 1, source, app.layout.layers()[1], true)
            .unwrap();
        app.sync();
        app.format_draft = Some(FormatDraft::new(&app.layout, 1));
        for label in ["L 20.0", "R 20.0", "T 20.0", "B 20.0"] {
            enter_value(&mut app, &ctx, label, "-12.5");
            assert_eq!(app.format_draft.as_ref().unwrap().source.input_crop, crop);
        }
        enter_value(&mut app, &ctx, "L 20.0", "NaN");
        assert_eq!(app.format_draft.as_ref().unwrap().source.input_crop, crop);
        enter_value(&mut app, &ctx, "L 20.0", "12.5");
        assert_eq!(
            app.format_draft.as_ref().unwrap().source.input_crop.left,
            12.5
        );
        enter_value(&mut app, &ctx, "L 12.5", "0");
        assert_eq!(
            app.format_draft.as_ref().unwrap().source.input_crop.left,
            0.0
        );
        let output = frame(&mut app, &ctx, vec![]);
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        let start = texts
            .windows(2)
            .find_map(|pair| {
                (pair[0].galley.text() == "R " && pair[1].galley.text() == "20.0")
                    .then(|| pair[1].pos + pair[1].galley.size() / 2.0)
            })
            .unwrap();
        let end = start - Vec2::new(100.0, 0.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        frame(&mut app, &ctx, vec![mouse(end, false)]);
        assert_eq!(
            app.format_draft.as_ref().unwrap().source.input_crop.right,
            0.0
        );
    }

    #[test]
    fn crop_zoom_rejects_negative_entry_but_content_offsets_allow_negative_values() {
        let ctx = egui::Context::default();
        let mut app = Editor {
            property_tab: PropertyTab::Crop,
            ..Default::default()
        };
        app.format_draft = Some(FormatDraft::new(&app.layout, 1));
        enter_value(&mut app, &ctx, "100.0% content zoom", "-50");
        assert_eq!(app.format_draft.as_ref().unwrap().framing.zoom, 1.0);
        enter_value(&mut app, &ctx, "100.0% content zoom", "50");
        assert_eq!(app.format_draft.as_ref().unwrap().framing.zoom, 1.0);
        enter_value(&mut app, &ctx, "100.0% content zoom", "20000");
        assert!((app.format_draft.as_ref().unwrap().framing.zoom - 100.0).abs() < 1e-9);
        enter_value(&mut app, &ctx, "10000.0% content zoom", "200");
        enter_value(&mut app, &ctx, "X 0.0", "-10");
        assert!(
            app.layout
                .formats()
                .mapping(1, app.layout.layers()[1])
                .unwrap()
                .framing
                .offset[0]
                < 0.0
        );
        let crop = app.layout.formats().advanced[1];
        assert!(
            [crop.left, crop.right, crop.top, crop.bottom]
                .iter()
                .all(|v| *v >= 0.0)
        );
    }

    #[test]
    fn crop_offsets_clamp_to_bounds_and_reducing_zoom_keeps_content_inside() {
        let mut app = Editor::default();
        let geometry = app.layout.layers()[1];
        let before = app.layout.formats().clone();
        let mut draft = FormatDraft::new(&app.layout, 1);
        draft.framing = Framing {
            zoom: 2.0,
            offset: [1e6, -1e6],
        };
        draft.apply_framing(&mut app.layout, false).unwrap();
        let mapping = app.layout.formats().mapping(1, geometry).unwrap();
        assert!((mapping.framing.offset[0] - mapping.offset_limits[0]).abs() < 1e-7);
        assert!((mapping.framing.offset[1] + mapping.offset_limits[1]).abs() < 1e-7);
        let zoomed = app.layout.formats().clone();
        draft.framing.zoom = 1.0;
        draft.apply_framing(&mut app.layout, false).unwrap();
        assert_eq!(draft.framing.offset, [0.0, 0.0]);
        assert_eq!(app.layout.layers()[1], geometry);
        app.layout.undo();
        assert_eq!(*app.layout.formats(), zoomed);
        app.layout.undo();
        assert_eq!(*app.layout.formats(), before);
    }

    #[test]
    fn crop_tab_clamps_both_axes_without_an_offset_error() {
        let ctx = egui::Context::default();
        let mut app = Editor {
            property_tab: PropertyTab::Crop,
            ..Default::default()
        };
        app.layout
            .apply(
                1,
                canvas_pilot::Geometry {
                    width: 300.0,
                    height: 250.0,
                    ..app.layout.layers()[1]
                },
            )
            .unwrap();
        let mut draft = FormatDraft::new(&app.layout, 1);
        // A non-matching aspect still permits horizontal movement at 100% zoom.
        draft.framing.offset = [-1e6, 1e6];
        app.format_draft = Some(draft);
        frame(&mut app, &ctx, vec![]);
        let draft = app.format_draft.as_ref().unwrap();
        assert!(draft.error.is_empty());
        let mapping = app
            .layout
            .formats()
            .mapping(1, app.layout.layers()[1])
            .unwrap();
        assert!(mapping.offset_limits[0] > 0.0);
        assert!((mapping.framing.offset[0] + mapping.offset_limits[0]).abs() < 1e-7);
        assert!(mapping.framing.offset[1].abs() < 1e-7);
        assert!(!draft.pending(&app.layout));
    }

    #[test]
    fn crop_control_drag_applies_without_a_button_and_keeps_pending_source_edits() {
        let ctx = egui::Context::default();
        let mut app = Editor {
            property_tab: PropertyTab::Crop,
            ..Default::default()
        };
        let before = app.layout.formats().clone();
        let mut draft = FormatDraft::new(&app.layout, 1);
        draft.size = [800.0, 450.0];
        app.format_draft = Some(draft);
        frame(&mut app, &ctx, vec![]);
        let output = frame(&mut app, &ctx, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text().ends_with("% content zoom") => {
                    Some(t.pos + Vec2::new(12.0, 5.0))
                }
                _ => None,
            })
            .expect("content zoom control");
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        for distance in [15.0, 30.0, 50.0] {
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos + Vec2::new(distance, 0.0))],
            );
        }
        frame(
            &mut app,
            &ctx,
            vec![mouse(pos + Vec2::new(50.0, 0.0), false)],
        );
        assert!(
            app.layout
                .formats()
                .mapping(1, app.layout.layers()[1])
                .unwrap()
                .framing
                .zoom
                > 1.0
        );
        assert_eq!(app.format_draft.as_ref().unwrap().size, [800.0, 450.0]);
        assert_eq!(app.layout.layers()[1].width, 640.0);
        app.layout.undo();
        assert_eq!(*app.layout.formats(), before);
    }

    #[test]
    fn output_preset_applies_without_an_apply_click() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let geometry = app.layout.layers().clone();
        app.output_draft = Some(OutputDraft::new(app.layout.formats().output));
        click_text(&mut app, &ctx, "HD");
        assert_eq!(app.layout.formats().output, [1280, 720]);
        assert_eq!(*app.layout.layers(), geometry);
        app.layout.undo();
        assert_eq!(app.layout.formats().output, [1920, 1080]);
    }

    #[test]
    fn crop_drag_updates_immediately_and_undo_restores_the_whole_gesture() {
        let mut app = Editor::default();
        let before = app.layout.formats().clone();
        let mut draft = FormatDraft::new(&app.layout, 1);
        for zoom in [1.2, 1.5, 2.0] {
            draft.framing.zoom = zoom;
            draft.apply_framing(&mut app.layout, true).unwrap();
            assert!(
                (app.layout
                    .formats()
                    .mapping(1, app.layout.layers()[1])
                    .unwrap()
                    .framing
                    .zoom
                    - zoom)
                    .abs()
                    < 1e-9
            );
            assert!(!draft.pending(&app.layout));
        }
        app.layout.undo();
        assert_eq!(*app.layout.formats(), before);
    }

    #[test]
    fn closing_formats_applies_or_discards_and_invalid_apply_keeps_the_draft() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let before = app.layout.layers().clone();
        let mut draft = FormatDraft::new(&app.layout, 1);
        draft.size = [800.0, 450.0];
        draft.closing = true;
        app.format_draft = Some(draft);
        click_text(&mut app, &ctx, "Discard");
        assert!(app.format_draft.is_none());
        assert_eq!(*app.layout.layers(), before);
        let mut draft = FormatDraft::new(&app.layout, 1);
        draft.input_size = [99999.0, 1080.0];
        draft.closing = true;
        app.format_draft = Some(draft);
        click_text(&mut app, &ctx, "Apply");
        let draft = app.format_draft.as_mut().unwrap();
        assert!(!draft.closing);
        assert!(!draft.error.is_empty());
        assert_eq!(*app.layout.layers(), before);
        draft.input_size = [1920.0, 1080.0];
        draft.size = [800.0, 450.0];
        draft.closing = true;
        click_text(&mut app, &ctx, "Apply");
        assert!(app.format_draft.is_none());
        assert_eq!(app.layout.layers()[1].width, 800.0);
        app.layout.undo();
        assert_eq!(*app.layout.layers(), before);
    }

    #[test]
    fn crop_tab_owns_crop_mode_until_closed_including_pending_close() {
        let ctx = egui::Context::default();
        let mut app = Editor {
            property_tab: PropertyTab::Crop,
            ..Default::default()
        };
        app.format_draft = Some(FormatDraft::new(&app.layout, 1));
        frame(&mut app, &ctx, vec![]);
        assert!(app.edit_content);
        // Closing with pending edits stops crop mode while awaiting a decision.
        let d = app.format_draft.as_mut().unwrap();
        d.size = [800.0, 450.0];
        d.closing = true;
        frame(&mut app, &ctx, vec![]);
        assert!(!app.edit_content);
        click_text(&mut app, &ctx, "Discard");
        assert!(app.format_draft.is_none());
        assert!(!app.edit_content);
        // Reopening the remembered Crop tab reactivates the tool.
        app.format_draft = Some(FormatDraft::new(&app.layout, 1));
        frame(&mut app, &ctx, vec![]);
        assert!(app.edit_content);
        app.format_draft = None;
        frame(&mut app, &ctx, vec![]);
        assert!(!app.edit_content);
        // Standalone crop editing still works without a properties window.
        app.edit_content = true;
        frame(&mut app, &ctx, vec![]);
        assert!(app.edit_content);
    }

    #[test]
    fn unrelated_sync_preserves_source_draft_but_refreshes_clean_properties() {
        let mut app = Editor::default();
        let mut draft = FormatDraft::new(&app.layout, 1);
        draft.size = [800.0, 450.0];
        app.format_draft = Some(draft);
        app.sync();
        assert_eq!(app.format_draft.as_ref().unwrap().size, [800.0, 450.0]);
        app.format_draft = Some(FormatDraft::new(&app.layout, 1));
        app.layout.select(Some(0));
        app.sync();
        assert!(app.format_draft.as_ref().unwrap().is_for(0));
    }

    #[test]
    fn property_tabs_keep_pending_edits_and_show_only_their_controls() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let before = app.layout.layers().clone();
        let mut d = FormatDraft::new(&app.layout, 1);
        d.size = [800.0, 450.0];
        d.framing.zoom = 2.0;
        app.format_draft = Some(d);
        for (title, tab, heading) in [
            (
                "Source",
                PropertyTab::Source,
                "Advanced source / input settings",
            ),
            ("Mapping", PropertyTab::Mapping, "Layer mapping"),
            (
                "Preview",
                PropertyTab::Preview,
                "Input preview & crop guides",
            ),
            ("Crop", PropertyTab::Crop, "Layer crop / zoom & position"),
            (
                "Layer",
                PropertyTab::Layer,
                "Layer resolution / final visible box",
            ),
        ] {
            click_text(&mut app, &ctx, title);
            assert_eq!(app.property_tab, tab);
            assert_eq!(app.edit_content, tab == PropertyTab::Crop);
            let output = frame(&mut app, &ctx, vec![]);
            let headings: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t)
                        if [
                            "Advanced source / input settings",
                            "Layer mapping",
                            "Input preview & crop guides",
                            "Layer crop / zoom & position",
                            "Layer resolution / final visible box",
                        ]
                        .contains(&t.galley.text()) =>
                    {
                        Some(t.galley.text())
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(headings, vec![heading]);
            let d = app.format_draft.as_ref().unwrap();
            assert_eq!(d.size, [800.0, 450.0]);
            assert_eq!(d.framing.zoom, 2.0);
            assert_eq!(*app.layout.layers(), before);
        }
    }

    #[test]
    fn output_auto_applies_edits_without_resetting_camera_on_idle_or_undo() {
        let mut app = Editor {
            zoom: 2.0,
            pan: Vec2::new(10.0, 20.0),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.output_draft = Some(OutputDraft::new(app.layout.formats().output));
        super::super::tests::frame(&mut app, &ctx, vec![]);
        assert_eq!(app.zoom, 2.0);
        assert_eq!(app.pan, Vec2::new(10.0, 20.0));
        app.output_draft.as_mut().unwrap().size = [1280.0, 720.0];
        super::super::tests::frame(&mut app, &ctx, vec![]);
        assert_eq!(app.layout.formats().output, [1280, 720]);
        app.zoom = 3.0;
        super::super::tests::frame(&mut app, &ctx, vec![]);
        assert_eq!(app.zoom, 3.0);
        app.layout.undo();
        super::super::tests::frame(&mut app, &ctx, vec![]);
        assert_eq!(app.layout.formats().output, [1920, 1080]);
        assert_eq!(app.zoom, 3.0);
        assert_eq!(app.output_draft.as_ref().unwrap().size, [1920.0, 1080.0]);
    }

    #[test]
    fn output_dialog_rejects_fractional_pixels_and_preserves_layer_geometry() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let before = app.layout.layers().clone();
        app.output_draft = Some(OutputDraft::new([1920, 1080]));
        app.output_draft.as_mut().unwrap().size = [1920.5, 1080.0];
        super::super::tests::frame(&mut app, &ctx, vec![]);
        assert!(!app.output_draft.as_ref().unwrap().error.is_empty());
        assert_eq!(app.layout.formats().output, [1920, 1080]);
        app.output_draft.as_mut().unwrap().size = [2560.0, 1600.0];
        super::super::tests::frame(&mut app, &ctx, vec![]);
        assert_eq!(app.layout.formats().output, [2560, 1600]);
        assert_eq!(*app.layout.layers(), before);
        assert_eq!(
            app.fields[0],
            number(2560.0 - before[1].x - before[1].width)
        );
        app.layout.undo();
        assert_eq!(app.layout.formats().output, [1920, 1080]);
    }
    #[test]
    fn advanced_source_edits_require_explicit_checkbox() {
        let mut app = Editor {
            property_tab: PropertyTab::Source,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let before = app.layout.formats().clone();
        let mut draft = FormatDraft::new(&app.layout, 1);
        draft.input_size = [1920.0, 1000.0];
        draft.source.input_crop = Crop::centered([1920.0, 1080.0], draft.input_size).unwrap();
        app.format_draft = Some(draft);
        click_text(&mut app, &ctx, "Apply source changes");
        assert_eq!(app.layout.formats(), &before);
        assert!(!app.device.input_edits.enabled);
        click_text(
            &mut app,
            &ctx,
            "Edit advanced input settings and include my edits when sending",
        );
        assert!(app.device.input_edits.enabled);
        click_text(&mut app, &ctx, "Apply source changes");
        assert_eq!(app.layout.formats().input(1).unwrap(), [1920.0, 1000.0]);
        assert!(!app.device.sending);
    }

    #[test]
    fn source_and_layer_dialog_applies_worked_example_with_one_undo() {
        let mut app = Editor {
            property_tab: PropertyTab::Source,
            keep_aspect_ratio: false,
            ..Default::default()
        };
        app.device.input_edits.enabled = true;
        let ctx = egui::Context::default();
        let before = app.layout.layers().clone();
        let formats = app.layout.formats().clone();
        let mut d = FormatDraft::new(&app.layout, 1);
        d.input_size = [1920.0, 1000.0];
        d.source.input_crop = Crop::centered([1920.0, 1080.0], d.input_size).unwrap();
        d.size = [300.0, 250.0];
        app.format_draft = Some(d);
        click_text(&mut app, &ctx, "Apply source changes");
        assert_eq!(app.layout.formats().input(1).unwrap(), [1920.0, 1000.0]);
        let g = app.layout.layers()[1];
        assert_eq!([g.width, g.height], [300.0, 250.0]);
        assert_eq!(app.layout.formats().advanced[1].left, 360.0);
        assert_eq!(app.layout.layers()[0], before[0]);
        app.layout.undo();
        assert_eq!(*app.layout.layers(), before);
        assert_eq!(*app.layout.formats(), formats);
    }
}

#[cfg(test)]
mod preview_paint_tests {
    use super::*;
    #[test]
    fn masked_helper_is_translucent_hatched_and_confined_to_hidden_regions() {
        use canvas_pilot::formats::{LayerFit, Mask, MaskOrigin};
        let rect = Rect::from_min_size(Pos2::new(200.0, 200.0), Vec2::new(320.0, 180.0));
        for fit in std::iter::once(None).chain(LayerFit::ALL.into_iter().map(Some)) {
            for origin in [MaskOrigin::Layer, MaskOrigin::Source] {
                let mut app = Editor::default();
                app.layout.set_layer_fit(1, fit, Crop::default()).unwrap();
                for edge in [0.45, 0.5] {
                    app.layout
                        .set_mask(
                            1,
                            Mask {
                                origin,
                                left: edge,
                                right: edge,
                                top: edge,
                                bottom: edge,
                            },
                        )
                        .unwrap();
                    let before = app.project();
                    let reveal = layer_mask_rect(rect, &app.layout, 1).unwrap();
                    for preview in [None, Some((egui::TextureId::User(91), 0.0))] {
                        let ctx = egui::Context::default();
                        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                            paint_mask_helper(ui.painter(), rect, &app.layout, 1, preview)
                        });
                        assert!(!output.shapes.is_empty(), "{fit:?} {origin:?}");
                        assert!(output.shapes.iter().all(|s| rect.contains_rect(s.clip_rect)
                            && !s.clip_rect.intersect(reveal).is_positive()));
                        assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::LineSegment { stroke, .. } if stroke.color == Color32::from_rgba_unmultiplied(222,108,220,85))));
                        if preview.is_some() {
                            let meshes: Vec<_> = output
                                .shapes
                                .iter()
                                .filter_map(|s| {
                                    if let egui::Shape::Mesh(m) = &s.shape {
                                        Some(m)
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            assert!(!meshes.is_empty());
                            assert!(meshes.iter().all(|m| {
                                m.vertices
                                    .iter()
                                    .all(|v| v.color.a() == Color32::WHITE.gamma_multiply(0.25).a())
                            }));
                        }
                    }
                    assert_eq!(app.project(), before);
                }
            }
        }
        let app = Editor::default();
        let ctx = egui::Context::default();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            paint_mask_helper(ui.painter(), rect, &app.layout, 1, None)
        });
        assert!(
            output.shapes.is_empty(),
            "an unmasked layer needs no overlay"
        );
    }

    #[test]
    fn masked_overlay_is_visible_only_while_editing_the_selected_layer() {
        use crate::gui::tests::frame;
        use canvas_pilot::formats::Mask;
        let mut app = Editor::default();
        app.layout.select(Some(1));
        app.layout
            .set_mask(
                1,
                Mask {
                    left: 0.25,
                    ..Mask::default()
                },
            )
            .unwrap();
        let before = app.project();
        let ctx = egui::Context::default();
        let hatched = |out: &egui::FullOutput| {
            out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::LineSegment { stroke, .. } if stroke.color == Color32::from_rgba_unmultiplied(222,108,220,85)))
        };
        assert!(!hatched(&frame(&mut app, &ctx, vec![])));
        app.edit_content = true;
        assert!(hatched(&frame(&mut app, &ctx, vec![])));
        app.edit_mask = true;
        assert!(hatched(&frame(&mut app, &ctx, vec![])));
        app.edit_content = false;
        app.edit_mask = false;
        assert!(!hatched(&frame(&mut app, &ctx, vec![])));
        app.layout.select(Some(0));
        app.edit_content = true;
        assert!(!hatched(&frame(&mut app, &ctx, vec![])));
        app.layout.select(Some(1));
        assert_eq!(app.project(), before);
    }

    #[test]
    fn crop_helpers_render_translucent_content_outside_the_retained_rectangle() {
        let rect = Rect::from_min_size(Pos2::new(200.0, 200.0), Vec2::new(300.0, 300.0));
        let source = Source {
            aspect: canvas_pilot::formats::Aspect::Square,
            native: [1000, 1000],
            ..Source::default()
        };
        let crop = Crop {
            left: 100.0,
            right: 100.0,
            top: 100.0,
            bottom: 100.0,
        };
        let full = content_bounds(rect, source, crop).unwrap();
        let probes = [
            Pos2::new((full.left() + rect.left()) / 2.0, rect.center().y),
            Pos2::new((full.right() + rect.right()) / 2.0, rect.center().y),
            Pos2::new(rect.center().x, (full.top() + rect.top()) / 2.0),
            Pos2::new(rect.center().x, (full.bottom() + rect.bottom()) / 2.0),
        ];
        for one_to_one in [false, true] {
            for preview in [None, Some((egui::TextureId::User(91), 0.0))] {
                let ctx = egui::Context::default();
                let output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 700.0))),
                        ..Default::default()
                    },
                    |ui| {
                        if one_to_one {
                            paint_one_to_one_helper(ui.painter(), rect, source, crop, preview);
                        } else {
                            paint_content_overflow(ui.painter(), rect, source, crop, preview);
                        }
                    },
                );
                for point in probes {
                    assert!(
                        output.shapes.iter().any(|shape| {
                            let alpha = match &shape.shape {
                                egui::Shape::Rect(rect) => rect.fill.a(),
                                egui::Shape::Mesh(mesh) => {
                                    mesh.vertices.first().map_or(0, |v| v.color.a())
                                }
                                _ => 0,
                            };
                            alpha > 0
                                && alpha < 255
                                && shape.clip_rect.contains(point)
                                && shape.shape.visual_bounding_rect().contains(point)
                        }),
                        "missing faded guide at {point:?}, 1:1={one_to_one}, preview={preview:?}"
                    );
                }
                if !one_to_one {
                    assert!(
                        output
                            .shapes
                            .iter()
                            .all(|shape| !shape.clip_rect.intersect(rect).is_positive()),
                        "overflow must not paint the visible layer twice"
                    );
                }
            }
        }
    }

    #[test]
    fn high_contrast_coordinate_labels_contrast_with_both_tile_colors() {
        let ctx = egui::Context::default();
        theme::Theme::HighContrast.apply(&ctx);
        let rect = Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(1000.0, 600.0));
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            paint_pattern_clipped(ui.painter(), rect, Source::default(), Crop::default(), rect);
        });
        let mut labels = 0;
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape {
                let (col, row) = text.galley.text().split_once(',').unwrap();
                let tile = (col.parse::<usize>().unwrap() + row.parse::<usize>().unwrap()) % 2;
                let expected = if tile == 0 {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::BLACK
                };
                assert!(
                    text.galley
                        .job
                        .sections
                        .iter()
                        .all(|section| section.format.color == expected)
                );
                labels += 1;
            }
        }
        assert_eq!(labels, 60);
    }

    #[test]
    fn actual_layer_stays_clipped_with_layer_and_source_masks() {
        use canvas_pilot::formats::{Mask, MaskOrigin};
        let mut app = Editor::default();
        app.layout
            .set_framing(
                1,
                canvas_pilot::formats::Framing {
                    zoom: 2.0,
                    offset: [0.0; 2],
                },
            )
            .unwrap();
        let rect = Rect::from_min_size(Pos2::new(200.0, 200.0), Vec2::new(320.0, 180.0));
        for origin in [MaskOrigin::Layer, MaskOrigin::Source] {
            app.layout
                .set_mask(
                    1,
                    Mask {
                        origin,
                        left: 0.1,
                        right: 0.1,
                        top: 0.1,
                        bottom: 0.1,
                    },
                )
                .unwrap();
            for preview in [None, Some((egui::TextureId::User(91), 0.0))] {
                let ctx = egui::Context::default();
                let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    paint_layer_source(ui.painter(), rect, &app.layout, 1, preview)
                });
                assert!(!output.shapes.is_empty());
                assert!(
                    output
                        .shapes
                        .iter()
                        .all(|shape| rect.contains_rect(shape.clip_rect)),
                    "mask {origin:?} leaked beyond layer"
                );
                if origin == MaskOrigin::Layer {
                    let masked = rect.shrink2(rect.size() * 0.1);
                    assert!(
                        output
                            .shapes
                            .iter()
                            .all(|shape| masked.contains_rect(shape.clip_rect))
                    );
                }
            }
        }
    }

    #[test]
    fn captured_texture_follows_both_crops_with_guides_above_and_opaque_fallback() {
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(Pos2::new(100., 100.), Vec2::new(132., 88.));
        let source = Source {
            input_crop: Crop {
                left: 100.,
                right: 100.,
                top: 0.,
                bottom: 0.,
            },
            ..Source::default()
        };
        let crop = Crop {
            left: 200.,
            right: 200.,
            top: 100.,
            bottom: 100.,
        };
        for preview in [Some((egui::TextureId::User(91), 0.35)), None] {
            let output = ctx.run_ui(egui::RawInput::default(), |root_ui| {
                let ctx = &root_ui.ctx().clone();
                let p = ctx.layer_painter(egui::LayerId::background());
                paint_source(&p, rect, source, crop, preview);
            });
            let shapes = &output.shapes;
            let image = shapes.iter().position(|s| matches!(&s.shape, egui::Shape::Mesh(m) if m.texture_id == egui::TextureId::User(91)));
            let guides: Vec<_> = shapes
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    if let egui::Shape::Rect(r) = &s.shape
                        && r.fill != Color32::TRANSPARENT
                    {
                        Some((i, r.fill))
                    } else {
                        None
                    }
                })
                .collect();
            assert!(!guides.is_empty());
            if preview.is_some() {
                let index = image.unwrap();
                let egui::Shape::Mesh(mesh) = &shapes[index].shape else {
                    unreachable!()
                };
                let min = mesh
                    .vertices
                    .iter()
                    .map(|v| v.pos)
                    .fold(Pos2::new(f32::INFINITY, f32::INFINITY), |a, b| a.min(b));
                let max = mesh
                    .vertices
                    .iter()
                    .map(|v| v.pos)
                    .fold(Pos2::new(f32::NEG_INFINITY, f32::NEG_INFINITY), |a, b| {
                        a.max(b)
                    });
                assert_eq!(min, Pos2::new(70., 90.));
                assert_eq!(max, Pos2::new(262., 198.));
                assert!(
                    guides
                        .iter()
                        .all(|(i, c)| *i > index && c.a() < 255 && c.a() > 0)
                );
                assert_eq!(shapes[index].clip_rect, rect);
            } else {
                assert!(image.is_none());
                assert!(guides.iter().all(|(_, c)| c.a() == 255));
            }
        }
    }
}
