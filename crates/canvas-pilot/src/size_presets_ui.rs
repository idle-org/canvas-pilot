use super::*;
use canvas_pilot::size_presets::SizePreset;

#[derive(Clone, Copy)]
enum Icon {
    Fullscreen,
    Quarter,
    Tiles,
    Portrait,
    Landscape,
}
impl Icon {
    fn key(self) -> &'static str {
        match self {
            Self::Fullscreen => "preset.fullscreen",
            Self::Quarter => "preset.quarter",
            Self::Tiles => "preset.tiling",
            Self::Portrait => "preset.portrait",
            Self::Landscape => "preset.landscape",
        }
    }
}

fn paint_icon(ui: &egui::Ui, rect: Rect, icon: Icon, enabled: bool) {
    let palette = theme::palette(ui.ctx());
    let color = if enabled {
        palette.text
    } else {
        palette.muted.gamma_multiply(0.5)
    };
    let stroke = Stroke::new(1.5, color);
    let center = rect.center();
    let size = match icon {
        Icon::Portrait => Vec2::new(13.0, 22.0),
        Icon::Landscape => Vec2::new(24.0, 13.0),
        _ => Vec2::new(24.0, 18.0),
    };
    let screen = Rect::from_center_size(center, size);
    ui.painter()
        .rect_stroke(screen, 1.0, stroke, StrokeKind::Inside);
    match icon {
        Icon::Fullscreen => {
            ui.painter().rect_filled(
                screen.shrink(4.0),
                0.0,
                palette.cyan.gamma_multiply(if enabled { 0.9 } else { 0.3 }),
            );
        }
        Icon::Quarter | Icon::Tiles => {
            let columns = if matches!(icon, Icon::Tiles) { 3 } else { 2 };
            for column in 1..columns {
                let x = screen.left() + screen.width() * column as f32 / columns as f32;
                ui.painter().line_segment(
                    [Pos2::new(x, screen.top()), Pos2::new(x, screen.bottom())],
                    stroke,
                );
            }
            ui.painter()
                .line_segment([screen.left_center(), screen.right_center()], stroke);
            ui.painter().rect_filled(
                Rect::from_min_size(
                    screen.min + Vec2::splat(2.0),
                    Vec2::new(
                        screen.width() / columns as f32 - 3.0,
                        screen.height() / 2.0 - 3.0,
                    ),
                ),
                0.0,
                palette.cyan.gamma_multiply(if enabled { 0.9 } else { 0.3 }),
            );
        }
        Icon::Portrait | Icon::Landscape => {
            ui.painter().line_segment(
                [screen.min + Vec2::splat(4.0), screen.max - Vec2::splat(4.0)],
                stroke,
            );
        }
    }
}

impl Editor {
    pub(super) fn size_presets_enabled(&self) -> bool {
        !self.layout.selected_indices().is_empty()
            && !self.layout.dragging()
            && !self
                .format_draft
                .as_ref()
                .is_some_and(|draft| draft.pending(&self.layout))
            && !(self.fields_edited && self.fields_pending())
            && !self.ratio_edit.dirty
    }
    pub(super) fn crop_preset_buttons(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.label("Crop borders to ratio");
        ui.horizontal(|ui| {
            for aspect in [canvas_pilot::formats::Aspect::FourThree, canvas_pilot::formats::Aspect::SixteenNine, canvas_pilot::formats::Aspect::SixteenTen] {
                if ui.add_enabled(self.size_presets_enabled(), egui::Button::new(aspect.label()))
                    .on_hover_text("Trim centered borders and shrink the layer; keep its center. Matching ratios do nothing.").clicked() {
                    match self.layout.apply_crop_preset(aspect) {
                        Ok(()) => {
                            if self.format_draft.is_some() {
                                self.format_draft = self.layout.selected().map(|i| FormatDraft::new(&self.layout, i));
                            }
                            self.sync(); changed = true;
                        }
                        Err(error) => self.error = error,
                    }
                }
            }
        });
        changed
    }
    fn apply_size_preset(&mut self, preset: SizePreset) {
        if !self.size_presets_enabled() {
            return;
        }
        match self
            .layout
            .apply_size_preset_with_proportions(preset, self.size_keep_proportions)
        {
            Ok(()) => {
                if self.format_draft.is_some() {
                    self.format_draft = self
                        .layout
                        .selected()
                        .map(|i| FormatDraft::new(&self.layout, i));
                }
                self.sync();
            }
            Err(error) => self.error = error,
        }
    }
    pub(super) fn size_preset_toolbar(&mut self, ui: &mut egui::Ui, rect: Rect, captions: bool) {
        let enabled = self.size_presets_enabled();
        let palette = theme::palette(ui.ctx());
        let label = "Size presets";
        let response = ui
            .add_enabled_ui(enabled, |ui| {
                let response =
                    ui.interact(rect, egui::Id::new("size-presets"), egui::Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label)
                });
                if response.hovered() || response.has_focus() {
                    ui.painter()
                        .rect_filled(rect.shrink(1.0), 3.0, palette.purple);
                }
                paint_icon(ui, rect, Icon::Tiles, enabled);
                response
            })
            .inner;
        egui::Popup::menu(&response)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.label(label);
                ui.checkbox(&mut self.size_keep_proportions, "Keep proportions")
                    .on_hover_text("Fit each layer inside its destination, centered, without changing its aspect ratio. Applies to fullscreen, quarters, halves and tiling.");
                ui.separator();
                for icon in [Icon::Fullscreen, Icon::Quarter, Icon::Tiles, Icon::Portrait, Icon::Landscape] {
                    ui.horizontal(|ui| {
                        let (_, rect) = ui.allocate_space(Vec2::new(30.0, 26.0));
                        paint_icon(ui, rect, icon, true);
                        let label = tr(ui.ctx(), icon.key());
                        match icon {
                            Icon::Quarter => { ui.collapsing(label, |ui| self.quarter_presets_menu(ui)); }
                            Icon::Tiles => { ui.collapsing(label, |ui| self.tiling_presets_menu(ui)); }
                            _ => {
                                if ui.button(label).clicked() {
                                    self.apply_size_preset(match icon {
                                        Icon::Fullscreen => SizePreset::Fullscreen,
                                        Icon::Portrait => SizePreset::Portrait,
                                        _ => SizePreset::Landscape,
                                    });
                                    egui::Popup::close_all(ui.ctx());
                                }
                            }
                        }
                    });
                }
            });
        response.on_hover_text(if enabled {
            label.to_owned()
        } else {
            format!("{label} — select layers and finish pending numeric edits first")
        });
        if captions {
            ui.painter().text(
                rect.center_bottom() + Vec2::new(0.0, 7.0),
                Align2::CENTER_TOP,
                label,
                FontId::proportional(11.0),
                palette.muted,
            );
        }
    }
    fn quarter_presets_menu(&mut self, ui: &mut egui::Ui) {
        let count = self.layout.selected_indices().len();
        ui.label(tr(ui.ctx(), "preset.quarter"));
        if count == 1 {
            let labels = [
                ["Top left", "Top middle", "Top right"],
                ["Middle left", "Center", "Middle right"],
                ["Bottom left", "Bottom middle", "Bottom right"],
            ];
            for (row, labels) in labels.into_iter().enumerate() {
                ui.horizontal(|ui| {
                    for (column, label) in labels.into_iter().enumerate() {
                        let (_, rect) = ui.allocate_space(Vec2::new(48.0, 38.0));
                        let response = ui.interact(
                            rect,
                            egui::Id::new(("quarter-position", column, row)),
                            egui::Sense::click(),
                        );
                        let p = theme::palette(ui.ctx());
                        let screen = rect.shrink(5.0);
                        ui.painter().rect_stroke(
                            screen,
                            1.0,
                            Stroke::new(1.0, p.text),
                            StrokeKind::Inside,
                        );
                        let quarter = Rect::from_min_size(
                            screen.min
                                + Vec2::new(
                                    column as f32 * screen.width() / 4.0,
                                    row as f32 * screen.height() / 4.0,
                                ),
                            screen.size() / 2.0,
                        );
                        ui.painter().rect_filled(
                            quarter,
                            0.0,
                            if response.hovered() { p.amber } else { p.cyan },
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)
                        });
                        if response.on_hover_text(label).clicked() {
                            self.apply_size_preset(SizePreset::Quarter {
                                column: column as u8,
                                row: row as u8,
                            });
                            egui::Popup::close_all(ui.ctx());
                        }
                    }
                });
            }
        } else {
            if count == 2 {
                for (label, columns, rows) in [("Up / Down", 1, 2), ("Left / Right", 2, 1)] {
                    if ui.button(label).clicked() {
                        self.apply_size_preset(SizePreset::Tiles { columns, rows });
                        egui::Popup::close_all(ui.ctx());
                    }
                }
            }
            if count >= 3 && ui.button("Four quarters").clicked() {
                self.apply_size_preset(SizePreset::Tiles {
                    columns: 2,
                    rows: 2,
                });
                egui::Popup::close_all(ui.ctx());
            }
            if count > 4 {
                ui.weak("After the fourth layer, quarter positions repeat.");
            }
            ui.weak("Layer-list order: left to right, then top to bottom.");
        }
    }
    fn tiling_presets_menu(&mut self, ui: &mut egui::Ui) {
        ui.label(tr(ui.ctx(), "preset.tiling"));
        egui::Grid::new("tiling-dimensions")
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                for (axis, label) in ["X (columns)", "Y (rows)"].into_iter().enumerate() {
                    ui.label(label);
                    ui.scope(|ui| {
                        ui.visuals_mut().widgets.inactive.bg_stroke =
                            Stroke::new(1.0, theme::palette(ui.ctx()).line);
                        ui.add_sized(
                            [72.0, 24.0],
                            egui::DragValue::new(&mut self.size_tiles[axis]).range(1..=256),
                        );
                    });
                    ui.end_row();
                }
            });
        let count = self.layout.selected_indices().len();
        let [columns, rows] = self.size_tiles;
        ui.label(format!(
            "{count} selected layers · {} cells",
            u32::from(columns) * u32::from(rows)
        ));
        ui.weak("Layer-list order: left to right, then top to bottom.");
        if count > usize::from(columns) * usize::from(rows) {
            ui.weak("More layers than cells: positions repeat and layers overlap.");
        }
        if ui
            .add_enabled(
                self.size_presets_enabled(),
                egui::Button::new("Apply tiling"),
            )
            .clicked()
        {
            self.apply_size_preset(SizePreset::Tiles { columns, rows });
            egui::Popup::close_all(ui.ctx());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click_text, frame, mouse};
    use super::*;
    fn click_id(app: &mut Editor, ctx: &egui::Context, id: egui::Id) {
        frame(app, ctx, vec![]);
        frame(app, ctx, vec![]);
        let point = ctx.read_response(id).unwrap().rect.center();
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(point), mouse(point, true)],
        );
        frame(app, ctx, vec![mouse(point, false)]);
    }
    fn icon(app: &mut Editor, ctx: &egui::Context, key: &str) {
        click_id(app, ctx, egui::Id::new("size-presets"));
        if app.size_presets_enabled() {
            let label = tr(ctx, key);
            let output = frame(app, ctx, vec![]);
            let expanded = output
                .shapes
                .iter()
                .filter(|shape| {
                    matches!(&shape.shape,
                egui::Shape::Text(text) if text.galley.text() == label)
                })
                .count()
                > 1;
            if !expanded {
                click_text(app, ctx, &label);
            }
        }
    }
    #[test]
    fn icon_box_applies_single_quarter_fullscreen_and_orientation() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout.select(Some(1));
        app.sync();
        icon(&mut app, &ctx, "preset.quarter");
        click_id(
            &mut app,
            &ctx,
            egui::Id::new(("quarter-position", 2_usize, 2_usize)),
        );
        assert_eq!(
            app.layout.layers()[1],
            Geometry {
                x: 960.0,
                y: 540.0,
                width: 960.0,
                height: 540.0
            }
        );
        icon(&mut app, &ctx, "preset.portrait");
        assert_eq!(
            app.layout.layers()[1],
            Geometry {
                x: 960.0,
                y: 540.0,
                width: 540.0,
                height: 960.0
            }
        );
        icon(&mut app, &ctx, "preset.landscape");
        assert_eq!(app.layout.layers()[1].width, 960.0);
        icon(&mut app, &ctx, "preset.fullscreen");
        assert_eq!(
            app.layout.layers()[1],
            Geometry {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0
            }
        );
        app.layout.undo();
        assert_eq!(app.layout.layers()[1].x, 960.0);
    }
    #[test]
    fn multi_selection_submenu_changes_with_count_and_tiling_uses_requested_grid() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout.set_selection([true, true]);
        app.sync();
        icon(&mut app, &ctx, "preset.quarter");
        let output = frame(&mut app, &ctx, vec![]);
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "Four quarters")));
        click_text(&mut app, &ctx, "Left / Right");
        assert_eq!(app.layout.layers()[1].width, 960.0);
        assert_eq!(app.layout.layers()[0].x, 960.0);
        for _ in 0..3 {
            app.layout.add_layer().unwrap();
        }
        app.layout.set_selection([true; 5]);
        app.sync();
        icon(&mut app, &ctx, "preset.quarter");
        let output = frame(&mut app, &ctx, vec![]);
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "Left / Right")));
        click_text(&mut app, &ctx, "Four quarters");
        assert_eq!(app.layout.layers()[0], app.layout.layers()[4]);
        app.size_tiles = [3, 2];
        icon(&mut app, &ctx, "preset.tiling");
        click_text(&mut app, &ctx, "Apply tiling");
        assert_eq!(app.layout.layers()[4].width, 640.0);
        assert_eq!(app.layout.layers()[4].height, 540.0);
        assert_eq!(app.layout.layers()[0].x, 640.0);
        assert_eq!(app.layout.layers()[0].y, 540.0);
    }
    #[test]
    fn proportions_toggle_fits_halves_and_is_shared_with_fullscreen() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout.set_selection([true, true]);
        app.sync();
        let original = app.layout.layers().clone();
        click_id(&mut app, &ctx, egui::Id::new("size-presets"));
        click_text(&mut app, &ctx, "Keep proportions");
        assert!(app.size_keep_proportions);
        click_text(&mut app, &ctx, "Quarter");
        click_text(&mut app, &ctx, "Left / Right");
        for (i, g) in app.layout.layers().iter().enumerate() {
            assert!((g.width / g.height - original[i].width / original[i].height).abs() < 1e-8);
            assert!(g.width <= 960.0 && g.height <= 1080.0);
            assert!((g.y + g.height / 2.0 - 540.0).abs() < 1e-8);
        }
        icon(&mut app, &ctx, "preset.fullscreen");
        assert!(app.size_keep_proportions);
        for (i, g) in app.layout.layers().iter().enumerate() {
            assert!((g.width / g.height - original[i].width / original[i].height).abs() < 1e-8);
        }
    }
    #[test]
    fn empty_selection_and_pending_numeric_values_disable_presets() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout.select(None);
        app.sync();
        let before = app.project();
        icon(&mut app, &ctx, "preset.fullscreen");
        assert_eq!(app.project(), before);
        app.layout.select(Some(1));
        app.sync();
        app.fields[0] = "-".into();
        app.fields_edited = true;
        let before = app.project();
        app.apply_size_preset(SizePreset::Fullscreen);
        assert_eq!(app.project(), before);
        assert_eq!(app.fields[0], "-");
    }
}
