use super::*;

#[derive(Clone, Copy, Default, PartialEq)]
pub(super) enum Section {
    #[default]
    Appearance,
    Editing,
    Reference,
    Storage,
    Device,
}
impl Section {
    const ALL: [(Self, &'static str); 5] = [
        (Self::Appearance, "Appearance"),
        (Self::Editing, "Editing"),
        (Self::Reference, "Reference images"),
        (Self::Storage, "Files & settings"),
        (Self::Device, "Device integration"),
    ];
}
impl Editor {
    pub(super) fn preferences_window(&mut self, ctx: &egui::Context) {
        if !self.preferences_open {
            return;
        }
        let before = self.application_settings(ctx);
        let mut open = self.preferences_open;
        egui::Window::new("Settings")
            .open(&mut open)
            .default_width(660.0)
            .min_width(540.0)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(145.0);
                        for (section, name) in Section::ALL {
                            ui.selectable_value(&mut self.preferences_section, section, name);
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_min_height(265.0);
                        match self.preferences_section {
                            Section::Appearance => {
                                ui.heading("Appearance");
                                ui.label(format!("Current theme: {}", theme::Theme::current(ctx).name()));
                                if ui.button("Browse all themes…").clicked() { self.themes_open = true; }
                                ui.label("Recently used");
                                for choice in theme::Theme::recent(ctx) {
                                    if ui.selectable_label(choice == theme::Theme::current(ctx), choice.name()).clicked() {
                                        choice.apply(ctx);
                                    }
                                }
                                ui.separator();
                                ui.checkbox(&mut self.show_toolbar_icon_names, "Show toolbar icon names");
                                ui.checkbox(&mut self.show_pattern, "Show source patterns on other layers while cropping");
                                ui.weak("Themes also apply to the viewport. Imported pictures retain their colors.");
                            }
                            Section::Editing => {
                                ui.heading("Editing");
                                self.resize_ratio_option(ui);
                                ui.label("Applies to layer handles and numeric width/height edits. Crop zoom always keeps the source proportions.");
                                ui.separator();
                                self.snapping_options(ui);
                                ui.weak("Applies to layer movement/resize, not numeric entry or crop-content handles.");
                                ui.separator();
                                ui.label("Overlay grid");
                                self.overlay_grid_options(ui);
                                ui.separator();
                                ui.label("Ctrl: temporarily invert Pivot tool · Drag pivot to position");
                                ui.label("Double-click pivot: reset to center · Shift/Ctrl+Shift: invert matching grid tool");
                                ui.label("Arrow keys: move 1 pixel · Shift-arrow: move 10 pixels");
                                ui.label("Shift-click: add/remove a layer from selection");
                                ui.label("Tools: V Move · H Pan · Z Zoom (outside text fields)");
                                ui.label("Space-drag or middle button: pan · Scroll: zoom");
                                ui.label("Double-click a layer: adjust crop · Escape: cancel gesture");
                            }
                            Section::Reference => {
                                ui.heading("Reference images");
                                self.backdrop_options(ui);
                                ui.separator();
                                ui.label("Image placement, visibility and opacity belong to the current project.");
                                if ui.button("Reference image settings…").clicked() { self.reference_open = true; }
                                if ui.button("Import reference picture…").clicked() { self.import_reference_file(ctx); }
                                ui.weak("PNG, JPEG and BMP. Drop a picture into the viewport to import it.");
                            }
                            Section::Storage => {
                                ui.heading("Files & settings");
                                ui.label("Application settings save automatically when changed.");
                                if let Some(path) = &self.settings.path {
                                    ui.label(path.display().to_string());
                                    if ui.button("Copy settings path").clicked() { ctx.copy_text(path.display().to_string()); }
                                } else { ui.label("No configuration location is available."); }
                                ui.separator();
                                ui.label("Project JSON stores layout, sources, crops and the reference picture. Exchange exports store layer geometry.");
                                ui.weak("Opening a project may restore its viewport options without rewriting your startup defaults.");
                                if ui.button("Restore default settings").clicked() {
                                    let defaults = settings::Settings::default();
                                    theme::Theme::restore_recent(ctx, defaults.recent_themes);
                                    defaults.theme.apply(ctx);
                                    self.device.allow_program = false;
                                    self.device.autoconnect_at_startup = false;
                                    self.snapping = defaults.snapping;
                                    self.keep_aspect_ratio = defaults.keep_aspect_ratio;
                                    self.show_pattern = defaults.show_source_pattern;
                                    self.show_toolbar_icon_names = defaults.show_toolbar_icon_names;
                                }
                            }
                            Section::Device => {
                                ui.heading("Device integration");
                                ui.checkbox(&mut self.device.autoconnect_at_startup, "Try autoconnecting at startup");
                                ui.checkbox(&mut self.device.allow_program, "Allow sending to Program");
                                ui.label("Read Preview layer containers from a supported Midra 4K or Aquilon device and review their mapping before import.");
                                if ui.button("Open device connection…").clicked() { self.device.connection_open = true; }
                                ui.weak("Read snapshots and explicitly send reviewed size, position and crop changes.");
                            }
                        }
                    });
                });
                ui.separator();
                ui.label("Changes take effect immediately and save automatically.");
                if let Some(error) = &self.settings.error { ui.colored_label(ui.visuals().error_fg_color, error); }
                if self.settings.can_retry() && ui.button("Retry saving settings").clicked() {
                    self.settings.save(&self.application_settings(ctx));
                }
                if ui.button("Close").clicked() { self.preferences_open = false; }
            });
        self.preferences_open &= open;
        let after = self.application_settings(ctx);
        if before != after {
            self.settings.save(&after);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::{frame, mouse};
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
            .unwrap_or_else(|| panic!("missing preference: {label}"));
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(app, ctx, vec![mouse(pos, false)]);
    }
    #[test]
    fn preferences_edit_and_persist_without_touching_document_or_drafts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let (store, saved) = settings::Store::at(path.clone());
        let ctx = egui::Context::default();
        let mut app = Editor::with_settings(&ctx, store, saved);
        let before = app.layout.layers().clone();
        app.fields[0] = "123.5".into();
        let draft = app.fields.clone();
        app.preferences_open = true;
        click_label(&mut app, &ctx, "Show toolbar icon names");
        click_label(&mut app, &ctx, "Editing");
        click_label(&mut app, &ctx, "Keep aspect ratio when resizing");
        click_label(&mut app, &ctx, "Snap movement & resizing");
        let (_, saved) = settings::Store::at(path);
        assert!(saved.snapping.enabled);
        assert!(saved.show_toolbar_icon_names);
        assert!(!saved.keep_aspect_ratio);
        assert_eq!(*app.layout.layers(), before);
        assert_eq!(app.fields, draft);
        click_label(&mut app, &ctx, "Close");
        assert!(!app.preferences_open);
    }
    #[test]
    fn command_comma_opens_preferences() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Comma,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            }],
        );
        assert!(app.preferences_open);
    }
}
