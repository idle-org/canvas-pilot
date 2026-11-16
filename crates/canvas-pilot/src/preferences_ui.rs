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
        (Self::Appearance, "ui.appearance"),
        (Self::Editing, "ui.editing"),
        (Self::Reference, "ui.reference-images"),
        (Self::Storage, "ui.files-settings"),
        (Self::Device, "ui.device-integration"),
    ];
}
impl Editor {
    pub(super) fn preferences_window(&mut self, ctx: &egui::Context) {
        if !self.preferences_open {
            return;
        }
        let before = self.application_settings(ctx);
        let mut open = self.preferences_open;
        egui::Window::new(tr(ctx, "ui.settings")).id(egui::Id::new("Settings"))
            .open(&mut open)
            .default_width(660.0)
            .min_width(540.0)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr(ctx, "language.label"));
                    egui::ComboBox::from_id_salt("settings-language")
                        .selected_text(self.language.name(ctx))
                        .show_ui(ui, |ui| {
                            for language in localization::Language::ALL {
                                let name = language.name(ctx);
                                if ui.selectable_value(&mut self.language, language, name).changed() {
                                    self.language.apply(ctx);
                                }
                            }
                        });
                });
                ui.weak(tr(ctx, "language.preview"));
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(145.0);
                        for (section, name) in Section::ALL {
                            ui.selectable_value(&mut self.preferences_section, section, tr(ctx, name));
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().id_salt("settings-content")
                        .max_height((ctx.screen_rect().height() - 260.0).max(180.0))
                        .min_scrolled_height((ctx.screen_rect().height() - 260.0).clamp(265.0, 540.0))
                        .show(ui, |ui| {
                        ui.vertical(|ui| {
                        ui.set_min_height(265.0);
                        match self.preferences_section {
                            Section::Appearance => {
                                ui.heading(tr(ui.ctx(), "ui.appearance"));
                                ui.label(rust_i18n::t!("theme.current", locale = localization::locale(ctx), name = theme::Theme::current(ctx).name()).into_owned());
                                if ui.button(tr(ui.ctx(), "ui.browse-all-themes")).clicked() { self.themes_open = true; }
                                ui.label(tr(ui.ctx(), "ui.recently-used"));
                                for choice in theme::Theme::recent(ctx) {
                                    if ui.selectable_label(choice == theme::Theme::current(ctx), choice.name()).clicked() {
                                        choice.apply(ctx);
                                    }
                                }
                                ui.separator();
                                ui.checkbox(&mut self.show_toolbar_icon_names, tr(ui.ctx(), "ui.show-toolbar-icon-names"));
                                ui.checkbox(&mut self.show_pattern, tr(ui.ctx(), "ui.show-source-patterns-on-other-layers-while-cropping"));
                                ui.weak(tr(ui.ctx(), "ui.themes-also-apply-to-the-viewport-imported-pictures-retain-their-colors"));
                            }
                            Section::Editing => {
                                ui.heading(tr(ui.ctx(), "ui.editing"));
                                self.resize_ratio_option(ui);
                                ui.label(tr(ui.ctx(), "ui.applies-to-layer-handles-and-numeric-width-height-edits-crop-zoom-always-keeps-the-so"));
                                ui.separator();
                                self.snapping_options(ui);
                                ui.weak(tr(ui.ctx(), "ui.applies-to-layer-movement-resize-not-numeric-entry-or-crop-content-handles"));
                                ui.separator();
                                ui.label(tr(ui.ctx(), "ui.overlay-grid"));
                                self.overlay_grid_options(ui);
                                ui.separator();
                                ui.label(tr(ui.ctx(), "ui.ctrl-temporarily-invert-pivot-tool-drag-pivot-to-position"));
                                ui.label(tr(ui.ctx(), "ui.double-click-pivot-reset-to-center-shift-ctrl-shift-invert-matching-grid-tool"));
                                ui.label(tr(ui.ctx(), "ui.arrow-keys-move-1-pixel-shift-arrow-move-10-pixels"));
                                ui.label(tr(ui.ctx(), "ui.shift-click-add-remove-a-layer-from-selection"));
                                ui.label(tr(ui.ctx(), "ui.tools-v-move-h-pan-z-zoom-outside-text-fields"));
                                ui.label(tr(ui.ctx(), "ui.space-drag-or-middle-button-pan-scroll-zoom"));
                                ui.label(tr(ui.ctx(), "ui.double-click-a-layer-adjust-crop-escape-cancel-gesture"));
                            }
                            Section::Reference => {
                                ui.heading(tr(ui.ctx(), "ui.reference-images"));
                                self.backdrop_options(ui);
                                ui.separator();
                                ui.label(tr(ui.ctx(), "ui.image-placement-visibility-and-opacity-belong-to-the-current-project"));
                                if ui.button(tr(ui.ctx(), "ui.reference-image-settings")).clicked() { self.reference_open = true; }
                                if ui.button(tr(ui.ctx(), "ui.import-reference-picture")).clicked() { self.import_reference_file(ctx); }
                                ui.weak(tr(ui.ctx(), "ui.png-jpeg-and-bmp-drop-a-picture-into-the-viewport-to-import-it"));
                            }
                            Section::Storage => {
                                ui.heading(tr(ui.ctx(), "ui.files-settings"));
                                ui.label(tr(ui.ctx(), "ui.application-settings-save-automatically-when-changed"));
                                egui::CollapsingHeader::new(tr(ctx, "language.font-license")).id_salt("font-licenses").show(ui, |ui| { ui.label(include_str!("../assets/fonts/OFL.txt")); });
                                if let Some(path) = &self.settings.path {
                                    ui.label(path.display().to_string());
                                    if ui.button(tr(ui.ctx(), "ui.copy-settings-path")).clicked() { ctx.copy_text(path.display().to_string()); }
                                } else { ui.label(tr(ui.ctx(), "ui.no-configuration-location-is-available")); }
                                ui.separator();
                                ui.label(tr(ui.ctx(), "ui.project-json-stores-layout-sources-crops-and-the-reference-picture-exchange-exports-s"));
                                ui.weak(tr(ui.ctx(), "ui.opening-a-project-may-restore-its-viewport-options-without-rewriting-your-startup-def"));
                                if ui.button(tr(ui.ctx(), "ui.restore-default-settings")).clicked() {
                                    let defaults = settings::Settings::default();
                                    self.language = defaults.language;
                                    self.language.apply(ctx);
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
                                ui.heading(tr(ui.ctx(), "ui.device-integration"));
                                ui.checkbox(&mut self.device.autoconnect_at_startup, tr(ui.ctx(), "ui.try-autoconnecting-at-startup"));
                                ui.checkbox(&mut self.device.allow_program, tr(ui.ctx(), "ui.allow-sending-to-program"));
                                ui.label(tr(ui.ctx(), "ui.read-preview-layer-containers-from-a-supported-midra-4k-or-aquilon-device-and-review"));
                                if ui.button(tr(ui.ctx(), "ui.open-device-connection")).clicked() { self.device.connection_open = true; }
                                ui.weak(tr(ui.ctx(), "ui.read-snapshots-and-explicitly-send-reviewed-size-position-and-crop-changes"));
                            }
                        }
                        });
                    });
                });
                ui.separator();
                ui.label(tr(ui.ctx(), "ui.changes-take-effect-immediately-and-save-automatically"));
                if let Some(error) = &self.settings.error { ui.colored_label(ui.visuals().error_fg_color, error); }
                if self.settings.can_retry() && ui.button(tr(ui.ctx(), "ui.retry-saving-settings")).clicked() {
                    self.settings.save(&self.application_settings(ctx));
                }
                if ui.button(tr(ui.ctx(), "ui.close")).clicked() { self.preferences_open = false; }
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
    fn language_switch_is_immediate_persistent_and_keeps_project_and_drafts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let (store, mut saved) = settings::Store::at(path.clone());
        saved.language = localization::Language::En;
        let ctx = egui::Context::default();
        let mut app = Editor::with_settings(&ctx, store, saved);
        app.fields[0] = "-".into();
        app.fields_edited = true;
        let project = app.project().to_json().unwrap();
        let draft = app.fields.clone();
        app.preferences_open = true;
        let clipboard = canvas_pilot::clipboard::encode(&app.layout, &[0], "Summary").unwrap();
        for language in localization::Language::ALL
            .into_iter()
            .skip(2)
            .chain(std::iter::once(localization::Language::En))
        {
            let current_name = app.language.name(&ctx);
            click_label(&mut app, &ctx, &current_name);
            click_label(&mut app, &ctx, &language.name(&ctx));
            assert_eq!(settings::Store::at(path.clone()).1.language, language);
            assert_eq!(app.project().to_json().unwrap(), project);
            assert_eq!(
                canvas_pilot::clipboard::encode(&app.layout, &[0], "Summary").unwrap(),
                clipboard
            );
            assert_eq!(app.fields, draft);
            assert!(!app.device.sending);
        }
        click_label(&mut app, &ctx, "English");
        click_label(&mut app, &ctx, "Français");
        click_label(&mut app, &ctx, "Fermer");
        assert!(!app.preferences_open);
        let (store, saved) = settings::Store::at(path);
        let restored_ctx = egui::Context::default();
        let mut restored = Editor::with_settings(&restored_ctx, store, saved);
        assert_eq!(localization::locale(&restored_ctx), "fr");
        restored.preferences_open = true;
        click_label(&mut restored, &restored_ctx, "Français");
        click_label(&mut restored, &restored_ctx, "English");
        assert_eq!(localization::locale(&restored_ctx), "en");
    }

    #[test]
    fn settings_language_and_close_stay_visible_at_minimum_window_size() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 720.0));
        for language in localization::Language::ALL {
            for (section, _) in Section::ALL {
                let ctx = egui::Context::default();
                language.apply(&ctx);
                let mut app = Editor {
                    language,
                    preferences_open: true,
                    preferences_section: section,
                    ..Editor::default()
                };
                let mut output = egui::FullOutput::default();
                for _ in 0..3 {
                    output = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(screen),
                            ..Default::default()
                        },
                        |ctx| app.preferences_window(ctx),
                    );
                }
                for label in [tr(&ctx, "language.label"), tr(&ctx, "ui.close")] {
                    let rect = output
                        .shapes
                        .iter()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.text() == label => {
                                Some(Rect::from_min_size(text.pos, text.galley.size()))
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| panic!("Missing {label}"));
                    assert!(
                        screen.contains_rect(rect),
                        "Control outside window: {label}: {rect:?}"
                    );
                }
            }
        }
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
