use super::*;

impl Editor {
    /// Layer selection is available beside import/export actions as well as on canvas.
    pub(super) fn transfer_layer_picker(&mut self, ui: &mut egui::Ui, label: &str) -> bool {
        let mut changed = false;
        ui.menu_button(label, |ui| {
            egui::ScrollArea::vertical()
                .max_height(240.0)
                .show(ui, |ui| {
                    for i in 0..self.layout.layers().len() {
                        let mut selected = self.layout.selected_indices().contains(&i);
                        let available = self.layout.visible(i) && !self.layout.locked(i);
                        let suffix = if self.layout.locked(i) {
                            " (locked)"
                        } else if !self.layout.visible(i) {
                            " (hidden)"
                        } else {
                            ""
                        };
                        if ui
                            .add_enabled(
                                available,
                                egui::Checkbox::new(
                                    &mut selected,
                                    format!("{}{suffix}", self.layout.layer_label(i)),
                                ),
                            )
                            .changed()
                        {
                            self.layout.toggle(i);
                            changed = true;
                        }
                    }
                });
        });
        if changed {
            self.sync();
        }
        changed
    }

    pub(super) fn import_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(225.0);
        if ui.button(tr(ui.ctx(), "ui.device-layers")).clicked() {
            self.device.prepare_send = false;
            self.device.open = true;
            ui.close();
        }
        ui.separator();
        ui.weak(tr(ui.ctx(), "ui.layer-values"));
        self.transfer_layer_picker(ui, "Choose destination layers…");
        if ui.button(tr(ui.ctx(), "ui.geometry-file")).clicked() {
            ui.close();
            self.import_geometry_file();
        }
        if ui
            .button(tr(ui.ctx(), "ui.clipboard-values"))
            .on_hover_text(tr(
                ui.ctx(),
                "ui.read-the-clipboard-and-review-layer-mapping-ctrl-cmd-v-also-works",
            ))
            .clicked()
        {
            ui.close();
            Self::request_layer_paste(ui.ctx());
        }
        ui.separator();
        ui.weak(tr(ui.ctx(), "ui.images-input-snapshots"));
        self.reference_actions_menu(ui);
    }
    pub(super) fn overlay_grid_options(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(
            &mut self.snapping.show_grid,
            tr(ui.ctx(), "ui.show-overlay-grid"),
        );
        ui.checkbox(
            &mut self.snapping.grid,
            tr(ui.ctx(), "ui.snap-to-overlay-grid"),
        );
        if self.snapping.grid && !self.snapping.enabled {
            ui.weak(tr(
                ui.ctx(),
                "ui.enable-snapping-with-the-magnet-options-to-snap-to-the-grid",
            ));
        }
        ui.horizontal(|ui| {
            ui.label(tr(ui.ctx(), "ui.columns"));
            ui.add(egui::DragValue::new(&mut self.snapping.grid_divisions[0]).range(1..=64));
        });
        ui.horizontal(|ui| {
            ui.label(tr(ui.ctx(), "ui.rows"));
            ui.add(egui::DragValue::new(&mut self.snapping.grid_divisions[1]).range(1..=64));
        });
        ui.weak(tr(
            ui.ctx(),
            "ui.equal-divisions-of-the-output-visibility-and-snapping-are-independent",
        ));
    }
    pub(super) fn snapping_options(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(
            &mut self.snapping.enabled,
            tr(ui.ctx(), "ui.snap-movement-resizing"),
        );
        ui.add_enabled_ui(self.snapping.enabled, |ui| {
            ui.checkbox(&mut self.snapping.pixels, tr(ui.ctx(), "ui.pixel-edges"));
            ui.checkbox(
                &mut self.snapping.screen_edges,
                tr(ui.ctx(), "ui.screen-edges-center-quarters"),
            );
            ui.checkbox(
                &mut self.snapping.layer_edges,
                tr(ui.ctx(), "ui.visible-layer-edges"),
            )
            .on_hover_text(tr(
                ui.ctx(),
                "ui.includes-contact-between-mirrored-opposite-resizing-layers",
            ));
        });
        ui.weak(tr(
            ui.ctx(),
            "ui.alt-invert-snapping-shift-10-px-ctrl-shift-100-px",
        ));
    }
    pub(super) fn snapping_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button(tr(ui.ctx(), "ui.snapping"), |ui| {
            let before = self.snapping;
            self.snapping_options(ui);
            if before != self.snapping {
                self.settings.save(&self.application_settings(ui.ctx()));
            }
            if let Some(error) = &self.settings.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        });
    }
    pub(super) fn backdrop_options(&mut self, ui: &mut egui::Ui) {
        ui.label(tr(ui.ctx(), "ui.example-backdrop-title-saved-in-project"));
        if ui
            .add(
                egui::TextEdit::singleline(&mut self.backdrop_title)
                    .char_limit(128)
                    .desired_width(260.0),
            )
            .changed()
        {
            self.backdrop_title.retain(|c| !c.is_control());
        }
        ui.small(tr(
            ui.ctx(),
            "ui.updates-immediately-leave-blank-to-hide-the-title",
        ));
        if ui.button(tr(ui.ctx(), "ui.reset-example-title")).clicked() {
            self.backdrop_title = canvas_pilot::project::DEFAULT_BACKDROP_TITLE.into();
        }
        if !self.references.is_empty() {
            ui.weak(tr(
                ui.ctx(),
                "ui.the-reference-image-replaces-this-example-backdrop",
            ));
        }
    }
    pub(super) fn viewport_options_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button(tr(ui.ctx(), "ui.open-all-settings")).clicked() {
            self.preferences_open = true;
            ui.close();
        }
        ui.separator();
        self.snapping_menu(ui);
        ui.menu_button(tr(ui.ctx(), "ui.example-backdrop"), |ui| {
            self.backdrop_options(ui)
        });
        let before = self.application_settings(ui.ctx());
        let current = theme::Theme::current(ui.ctx());
        ui.menu_button(format!("Theme: {}", current.name()), |ui| {
            for choice in theme::Theme::recent(ui.ctx()) {
                if ui
                    .stable_selectable_label(current == choice, choice.name())
                    .clicked()
                {
                    choice.apply(ui.ctx());
                    ui.close();
                }
            }
            ui.separator();
            if ui.button(tr(ui.ctx(), "ui.more-themes")).clicked() {
                self.themes_open = true;
                ui.close();
            }
        });
        ui.separator();
        ui.checkbox(
            &mut self.outline_layers,
            tr(ui.ctx(), "ui.outline-only-layers-this-project"),
        );
        ui.checkbox(
            &mut self.show_toolbar_icon_names,
            tr(ui.ctx(), "ui.show-toolbar-icon-names"),
        );
        self.resize_ratio_option(ui);
        ui.checkbox(
            &mut self.show_pattern,
            tr(
                ui.ctx(),
                "ui.show-source-patterns-on-other-layers-while-cropping",
            ),
        );
        let after = self.application_settings(ui.ctx());
        if before != after {
            self.settings.save(&after);
        }
        if let Some(path) = &self.settings.path {
            ui.separator();
            ui.add(egui::Label::new("Options save automatically").selectable(false))
                .on_hover_text(path.display().to_string());
        }
        if let Some(error) = &self.settings.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
            if self.settings.can_retry()
                && ui.button(tr(ui.ctx(), "ui.retry-saving-options")).clicked()
            {
                self.settings.save(&after);
            }
        }
    }
    pub(super) fn theme_window(&mut self, ctx: &egui::Context) {
        if !self.themes_open {
            return;
        }
        let current = theme::Theme::current(ctx);
        let mut selected = None;
        egui::Window::new(tr(ctx, "ui.all-themes"))
            .id(egui::Id::new("All themes"))
            .open(&mut self.themes_open)
            .default_width(540.0)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(tr(
                    ui.ctx(),
                    "ui.choose-a-theme-your-three-most-recent-choices-stay-in-the-menu",
                ));
                ui.separator();
                ui.columns(2, |columns| {
                    for (index, choice) in theme::Theme::ALL.into_iter().enumerate() {
                        columns[index % 2].horizontal(|ui| {
                            let palette = choice.palette();
                            for color in [palette.bg, palette.layer, palette.cyan] {
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 2.0, color);
                            }
                            if ui
                                .stable_selectable_label(current == choice, choice.name())
                                .on_hover_text(if choice.is_light() {
                                    "Light theme"
                                } else {
                                    "Dark theme"
                                })
                                .clicked()
                            {
                                selected = Some(choice);
                            }
                        });
                    }
                });
            });
        if let Some(choice) = selected {
            choice.apply(ctx);
            self.themes_open = false;
            self.settings.save(&self.application_settings(ctx));
        }
    }
    pub(super) fn fit_screen(&mut self) {
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
    }
    pub(super) fn fit_selection(&mut self) {
        if let Some(g) = self.layout.selection_bounds() {
            let [sw, sh] = self.layout.formats().output.map(|v| v as f32);
            self.zoom =
                ((sw as f64 / g.width).min(sh as f64 / g.height) as f32 * 0.8).clamp(0.0001, 256.0);
            let c = g.center();
            self.pan = Vec2::new(sw / 2.0 - c[0] as f32, sh / 2.0 - c[1] as f32);
        }
    }
    pub(super) fn application_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(185.0);
        if ui.button(tr(ui.ctx(), "ui.open-all-settings")).clicked() {
            self.preferences_open = true;
            ui.close();
        }
        ui.separator();
        if ui.button(tr(ui.ctx(), "ui.device-connection")).clicked() {
            self.device.connection_open = true;
            ui.close();
        }
        ui.menu_button(tr(ui.ctx(), "ui.project"), |ui| self.project_menu(ui));
        ui.menu_button(tr(ui.ctx(), "ui.import"), |ui| self.import_menu(ui));
        ui.menu_button(tr(ui.ctx(), "ui.export"), |ui| self.export_menu(ui));
        ui.menu_button(tr(ui.ctx(), "ui.edit"), |ui| {
            if ui.button(tr(ui.ctx(), "ui.undo")).clicked() {
                self.layout.undo();
                self.sync();
                ui.close();
            }
            if ui.button(tr(ui.ctx(), "ui.redo")).clicked() {
                self.layout.redo();
                self.sync();
                ui.close();
            }
            ui.separator();
            if ui
                .add_enabled(
                    !self.layout.selected_indices().is_empty(),
                    egui::Button::new(tr(ui.ctx(), "ui.copy-layer-values")),
                )
                .clicked()
            {
                self.copy(ui.ctx());
                ui.close();
            }
            if ui
                .button(tr(ui.ctx(), "ui.select-all-editable-layers"))
                .clicked()
            {
                self.layout.set_selection(
                    (0..self.layout.layers().len())
                        .map(|i| self.layout.visible(i) && !self.layout.locked(i))
                        .collect::<Vec<_>>(),
                );
                self.sync();
                ui.close();
            }
            if ui.button(tr(ui.ctx(), "ui.deselect-all")).clicked() {
                self.layout.select(None);
                self.edit_content = false;
                self.sync();
                ui.close();
            }
        });
        ui.menu_button(tr(ui.ctx(), "ui.view"), |ui| {
            if ui.button(tr(ui.ctx(), "ui.fit-screen")).clicked() {
                self.fit_screen();
                ui.close();
            }
            if ui
                .add_enabled(
                    self.layout.selection_bounds().is_some(),
                    egui::Button::new(tr(ui.ctx(), "ui.zoom-to-selection")),
                )
                .clicked()
            {
                self.fit_selection();
                ui.close();
            }
            if ui.button(tr(ui.ctx(), "ui.1-1-display-pixels")).clicked() {
                self.camera_one_to_one = true;
                ui.close();
            }
            ui.separator();
            if ui.button(tr(ui.ctx(), "ui.output-format")).clicked() {
                self.output_draft = Some(OutputDraft::new(self.layout.formats().output));
                ui.close();
            }
            if ui
                .button(tr(ui.ctx(), "ui.reference-image-settings"))
                .clicked()
            {
                self.reference_open = true;
                ui.close();
            }
        });
        ui.menu_button(tr(ui.ctx(), "ui.layers"), |ui| {
            if ui
                .button(tr(ui.ctx(), "ui.manage-layers-folders"))
                .clicked()
            {
                self.manage_layers();
                ui.close();
            }
            if ui.button(tr(ui.ctx(), "ui.add-layer")).clicked() {
                self.add_layer();
                ui.close();
            }
            if ui
                .add_enabled(
                    !self.layout.selected_indices().is_empty()
                        && self.layout.selected_indices().len() < self.layout.layers().len(),
                    egui::Button::new(tr(ui.ctx(), "ui.remove-selected-layers")),
                )
                .clicked()
            {
                self.remove_layers();
                ui.close();
            }
            ui.separator();
            ui.menu_button(tr(ui.ctx(), "ui.selected-layer-actions"), |ui| {
                if self.layout.selected_indices().is_empty() {
                    ui.weak(tr(ui.ctx(), "ui.select-a-layer-to-edit-its-properties"));
                }
                self.layer_context_menu(ui);
            });
            ui.separator();
            for i in 0..self.layout.layers().len() {
                ui.menu_button(self.layout.layer_label(i), |ui| {
                    if ui
                        .add_enabled(
                            self.layout.visible(i) && !self.layout.locked(i),
                            egui::Button::new(tr(ui.ctx(), "ui.select-layer")),
                        )
                        .clicked()
                    {
                        self.layout.select(Some(i));
                        self.edit_content = false;
                        self.sync();
                        ui.close();
                    }
                    let mut visible = self.layout.visible(i);
                    if ui
                        .checkbox(&mut visible, tr(ui.ctx(), "ui.visible"))
                        .changed()
                    {
                        self.layout.set_visible(i, visible);
                        self.sync();
                    }
                    let mut locked = self.layout.locked(i);
                    if ui
                        .checkbox(&mut locked, tr(ui.ctx(), "ui.locked"))
                        .changed()
                    {
                        self.layout.set_locked(i, locked);
                        self.sync();
                    }
                });
            }
        });
        ui.menu_button(tr(ui.ctx(), "ui.options"), |ui| {
            self.viewport_options_menu(ui)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{frame, mouse};
    use super::*;
    fn click_label(app: &mut Editor, ctx: &egui::Context, label: &str) {
        frame(app, ctx, vec![]);
        let output = frame(app, ctx, vec![]);
        let pos = output
            .shapes
            .iter()
            .rev()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("menu item not found: {label}"));
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(app, ctx, vec![mouse(pos, false)]);
    }
    #[test]
    fn options_are_saved_and_restored_in_a_fresh_editor() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let (store, saved) = settings::Store::at(path.clone());
        let ctx = egui::Context::default();
        let mut app = Editor::with_settings(&ctx, store, saved);
        click_label(&mut app, &ctx, "Menu");
        click_label(&mut app, &ctx, "Options");
        for label in [
            "Show toolbar icon names",
            "Keep aspect ratio when resizing",
            "Show source patterns on other layers while cropping",
        ] {
            click_label(&mut app, &ctx, label);
        }
        click_label(&mut app, &ctx, "Theme: Midra Dark");
        click_label(&mut app, &ctx, "More themes…");
        click_label(&mut app, &ctx, "Corporate Light");
        let (store, saved) = settings::Store::at(path);
        assert!(store.error.is_none());
        let fresh_ctx = egui::Context::default();
        let restored = Editor::with_settings(&fresh_ctx, store, saved);
        assert_eq!(
            theme::Theme::recent(&fresh_ctx),
            vec![theme::Theme::CorporateLight, theme::Theme::Midra]
        );
        assert!(restored.show_toolbar_icon_names);
        assert!(!restored.keep_aspect_ratio);
        assert!(restored.show_pattern);
        assert_eq!(
            theme::Theme::current(&fresh_ctx),
            theme::Theme::CorporateLight
        );
    }
    #[test]
    fn theme_menu_switches_chrome_without_changing_document() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let boxes = app.layout.layers().clone();
        let formats = app.layout.formats().clone();
        for choice in theme::Theme::ALL {
            click_label(&mut app, &ctx, "Menu");
            click_label(&mut app, &ctx, "Options");
            let label = format!("Theme: {}", theme::Theme::current(&ctx).name());
            click_label(&mut app, &ctx, &label);
            click_label(&mut app, &ctx, "More themes…");
            click_label(&mut app, &ctx, choice.name());
            assert!(theme::Theme::current(&ctx) == choice);
            assert_eq!(ctx.global_style().visuals.dark_mode, !choice.is_light());
            let output = frame(&mut app, &ctx, vec![]);
            for fill in [
                choice.palette().bg,
                choice.palette().canvas,
                choice.palette().layer,
            ] {
                assert!(output.shapes.iter().any(|s| matches!(&s.shape,
                    egui::Shape::Rect(r) if r.fill == fill)));
            }
            assert_eq!(*app.layout.layers(), boxes);
            assert_eq!(*app.layout.formats(), formats);
        }
    }
    #[test]
    fn main_menu_routes_view_options_and_selection_commands() {
        for (section, action) in [
            ("View", "Fit screen"),
            (
                "Options",
                "Show source patterns on other layers while cropping",
            ),
            ("Edit", "Select all editable layers"),
        ] {
            let mut app = Editor {
                zoom: 2.0,
                pan: Vec2::new(20.0, 30.0),
                ..Default::default()
            };
            app.layout.set_locked(1, true);
            let boxes = app.layout.layers().clone();
            let ctx = egui::Context::default();
            click_label(&mut app, &ctx, "Menu");
            click_label(&mut app, &ctx, section);
            click_label(&mut app, &ctx, action);
            match section {
                "View" => {
                    assert_eq!(app.zoom, 1.0);
                    assert_eq!(app.pan, Vec2::ZERO);
                }
                "Options" => assert!(app.show_pattern),
                _ => assert_eq!(app.layout.selection_mask(), [true, false]),
            }
            assert_eq!(*app.layout.layers(), boxes);
        }
    }
}
