use super::*;

#[derive(Clone)]
struct ScaleDraft(f32, f32);

pub(super) fn apply_scaling(ctx: &egui::Context, ui_scale: f32, text_scale: Option<f32>) {
    ctx.data_mut(|data| data.remove_by_type::<ScaleDraft>());
    ctx.set_zoom_factor(ui_scale);
    // Derive from defaults each time: toggles and theme changes must not compound sizes.
    let mut fonts = egui::Style::default().text_styles;
    for font in fonts.values_mut() {
        font.size *= text_scale.unwrap_or(1.0);
    }
    ctx.global_style_mut(|style| style.text_styles = fonts);
}

// Keep the displayed draft separate from the saved size until interaction ends.
fn scale_slider(ui: &mut egui::Ui, scale: &mut f32, maximum: f32) -> egui::Response {
    let id = ui.next_auto_id().with("scale-draft");
    let ScaleDraft(original, draft) = ui
        .data(|data| data.get_temp::<ScaleDraft>(id))
        .unwrap_or(ScaleDraft(*scale, *scale));
    let mut percent = if original == *scale { draft } else { *scale } * 100.0;
    let response = ui.add(
        egui::Slider::new(&mut percent, 50.0..=maximum)
            .suffix(" %")
            .step_by(5.0),
    );
    let editing =
        ui.input(|input| input.pointer.any_down()) || ui.ctx().egui_wants_keyboard_input();
    if editing {
        ui.data_mut(|data| data.insert_temp(id, ScaleDraft(*scale, percent / 100.0)));
    } else {
        *scale = percent / 100.0;
        ui.data_mut(|data| data.remove::<ScaleDraft>(id));
    }
    response
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum Section {
    #[default]
    Appearance,
    Languages,
    Editing,
    Reference,
    Storage,
    Device,
}
impl Section {
    const ALL: [(Self, &'static str); 6] = [
        (Self::Appearance, "ui.appearance"),
        (Self::Languages, "language.packs"),
        (Self::Editing, "ui.editing"),
        (Self::Reference, "ui.reference-images"),
        (Self::Storage, "ui.files-settings"),
        (Self::Device, "ui.device-integration"),
    ];
}
pub(super) struct LanguagePreview {
    pub(super) previous: localization::Language,
    previous_locale: String,
    deadline: std::time::Instant,
}

pub(super) enum PackAction {
    Install(crate::language_packs::Prepared),
    Remove(crate::language_packs::Installed),
}

impl Editor {
    fn preview_language(&mut self, ctx: &egui::Context, language: localization::Language) {
        if language == self.language {
            return;
        }
        let previous = self
            .language_preview
            .as_ref()
            .map_or_else(|| self.language.clone(), |p| p.previous.clone());
        let previous_locale = self
            .language_preview
            .as_ref()
            .map_or_else(|| localization::locale(ctx), |p| p.previous_locale.clone());
        self.language_preview = Some(LanguagePreview {
            previous,
            previous_locale,
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(20),
        });
        self.language = language.clone();
        language.apply(ctx);
    }

    fn finish_language_preview(&mut self, ctx: &egui::Context, keep: bool) {
        let Some(preview) = self.language_preview.take() else {
            return;
        };
        if keep {
            self.settings.save(&self.application_settings(ctx));
        } else {
            self.language = preview.previous;
            self.language.apply(ctx);
        }
    }

    pub(super) fn language_confirmation(&mut self, ctx: &egui::Context) {
        let Some(preview) = &self.language_preview else {
            return;
        };
        let remaining = preview
            .deadline
            .saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            self.finish_language_preview(ctx, false);
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        let response = egui::Modal::new(egui::Id::new("confirm-language")).show(ctx, |ui| {
            ui.set_max_width(480.0);
            ui.heading(tr(ctx, "language.confirm"));
            ui.label(
                rust_i18n::t!(
                    "language.rollback",
                    locale = &localization::locale(ctx),
                    seconds = remaining.as_secs() + 1
                )
                .into_owned(),
            );
            // Use the previously displayed language, including the resolved System locale.
            if preview.previous_locale != localization::locale(ctx) {
                ui.label(
                    rust_i18n::t!(
                        "language.rollback",
                        locale = &preview.previous_locale,
                        seconds = remaining.as_secs() + 1
                    )
                    .into_owned(),
                );
            }
            let keep = ui.button(tr(ctx, "language.keep")).clicked();
            let cancel = ui
                .button(
                    rust_i18n::t!("language.cancel", locale = &preview.previous_locale)
                        .into_owned(),
                )
                .clicked();
            (keep, cancel)
        });
        if response.inner.1 || response.should_close() {
            self.finish_language_preview(ctx, false);
        } else if response.inner.0 {
            self.finish_language_preview(ctx, true);
        }
    }

    fn refresh_pack_language(
        &mut self,
        ctx: &egui::Context,
        previous: &[crate::language_packs::Description],
    ) {
        if let localization::Language::Pack(locale) = &self.language
            && previous.iter().any(|p| &p.locale == locale)
            && !crate::language_packs::available(ctx)
                .iter()
                .any(|p| &p.locale == locale)
        {
            self.language = localization::Language::En;
        }
        self.language.apply(ctx);
        self.settings.save(&self.application_settings(ctx));
    }

    fn prepare_language_pack(&mut self, ctx: &egui::Context, path: &std::path::Path) {
        self.finish_language_preview(ctx, false);
        let result = self
            .settings
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .ok_or_else(|| "Settings directory unavailable".to_owned())
            .and_then(|directory| {
                crate::language_packs::prepare(ctx, &directory.join("language-packs"), path)
            });
        let previous = crate::language_packs::available(ctx);
        match result {
            Ok(pending) if pending.needs_confirmation() => {
                self.language_pack_action = Some(PackAction::Install(pending))
            }
            Ok(pending) => match crate::language_packs::commit(ctx, pending, false) {
                Ok(()) => self.refresh_pack_language(ctx, &previous),
                Err(error) => self.error = error,
            },
            Err(error) => self.error = error,
        }
    }

    fn language_pack_settings(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        ui.heading(tr(&ctx, "language.packs"));
        if ui.button(tr(&ctx, "language.install-pack")).clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Language pack", &["zip"])
                .pick_file()
        {
            self.prepare_language_pack(&ctx, &path);
        }
        ui.weak("Choose a ZIP to install or update. Newer revisions update immediately; equal or older revisions ask before replacing.");
        let packs = crate::language_packs::installed(&ctx);
        if packs.is_empty() {
            ui.label("No language packs installed.");
        }
        for pack in packs {
            ui.separator();
            egui::CollapsingHeader::new(
                egui::RichText::new(format!("{} · revision {}", pack.id, pack.revision)).strong(),
            )
            .id_salt(("language-pack", &pack.id))
            .default_open(true)
            .show(ui, |ui| {
                for language in &pack.languages {
                    ui.label(format!("{} — {}", language.native_name, language.name));
                }
                ui.add_space(4.0);
                if ui.button(tr(&ctx, "language.remove-pack")).clicked() {
                    self.finish_language_preview(&ctx, false);
                    self.language_pack_action = Some(PackAction::Remove(pack.clone()));
                }
            });
        }
    }

    pub(super) fn language_pack_confirmation(&mut self, ctx: &egui::Context) {
        let Some(action) = &self.language_pack_action else {
            return;
        };
        let response = egui::Modal::new(egui::Id::new("confirm-language-pack")).show(ctx, |ui| {
            ui.set_max_width(460.0);
            let label = match action {
                PackAction::Install(pack) => {
                    ui.heading(tr(ctx, "language.overwrite-pack"));
                    ui.label(format!("{}: installed revision {} → revision {}", pack.id(), pack.previous_revision.map_or_else(|| "unknown".into(), |v| v.to_string()), pack.revision()));
                    ui.label("This is not a newer revision. Replacing it overwrites the installed translations.");
                    "language.overwrite-pack"
                },
                PackAction::Remove(pack) => {
                    ui.heading(tr(ctx, "language.remove-pack"));
                    ui.label(&pack.id);
                    ui.label(pack.languages.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "));
                    ui.label("All languages in this pack will be removed. If the current language is removed, the interface switches to English. You can reinstall the ZIP later.");
                    "language.remove-pack"
                }
            };
            (ui.button(tr(ctx, label)).clicked(), ui.button(tr(ctx, "ui.cancel")).clicked())
        });
        if response.inner.1 || response.should_close() {
            self.language_pack_action = None;
        } else if response.inner.0 {
            let previous = crate::language_packs::available(ctx);
            let result = match self.language_pack_action.take().unwrap() {
                PackAction::Install(pack) => crate::language_packs::commit(ctx, pack, true),
                PackAction::Remove(pack) => crate::language_packs::remove(ctx, &pack),
            };
            match result {
                Ok(()) => self.refresh_pack_language(ctx, &previous),
                Err(error) => self.error = error,
            }
        }
    }

    pub(super) fn preferences_window(&mut self, ctx: &egui::Context) {
        if !self.preferences_open {
            return;
        }
        let before = self.application_settings(ctx);
        let mut open = self.preferences_open;
        egui::Window::new(tr(ctx, "ui.settings")).id(egui::Id::new("Settings"))
            .open(&mut open)
            .default_width(660.0_f32.min(ctx.content_rect().width() - 32.0))
            .min_width(320.0_f32.min(ctx.content_rect().width() - 32.0))
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr(ctx, "language.label"));
                    egui::ComboBox::from_id_salt("settings-language")
                        .height(440.0)
                        .selected_text(self.language.name(ctx))
                        .show_ui(ui, |ui| {
                            for language in localization::Language::ALL.into_iter().chain(crate::language_packs::available(ctx).into_iter().map(|p| localization::Language::Pack(p.locale))) {
                                let name = language.name(ctx);
                                if ui.stable_selectable_label(self.language == language, name).clicked() {
                                    self.preview_language(ctx, language);
                                }
                            }
                        });
                });
                ui.horizontal_wrapped(|ui| {
                if ui.button(tr(ui.ctx(), "ui.close")).clicked() { self.preferences_open = false; }
                if ui.small_button(tr(ctx, "access.reset")).clicked() {
                    self.ui_scale = 1.0;
                    self.text_scale = None;
                    apply_scaling(ctx, self.ui_scale, self.text_scale);
                }
                });
                ui.weak(tr(ctx, "language.preview"));
                ui.separator();
                egui::ScrollArea::vertical().id_salt("settings-body")
                    .max_height((ctx.content_rect().height() - ui.cursor().min.y - 120.0).max(40.0))
                    .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(145.0);
                        for (section, name) in Section::ALL {
                            ui.stable_selectable_value(&mut self.preferences_section, section, tr(ctx, name));
                        }
                    });
                    ui.separator();
                        ui.vertical(|ui| {
                        ui.set_min_height(60.0);
                        match self.preferences_section {
                            Section::Appearance => {
                                ui.heading(tr(ui.ctx(), "ui.appearance"));
                                let scaling_before = (self.ui_scale, self.text_scale);
                                ui.label(tr(ctx, "access.interface-scale"));
                                scale_slider(ui, &mut self.ui_scale, 400.0);
                                let mut separate = self.text_scale.is_some();
                                if ui.checkbox(&mut separate, tr(ctx, "access.text-scale")).changed() {
                                    self.text_scale = separate.then_some(1.0);
                                }
                                if let Some(scale) = &mut self.text_scale {
                                    scale_slider(ui, scale, 200.0);
                                    ui.weak(tr(ctx, "access.text-help"));
                                }
                                if scaling_before != (self.ui_scale, self.text_scale) { apply_scaling(ctx, self.ui_scale, self.text_scale); }
                                if ui.button(tr(ctx, "access.contrast")).clicked() { theme::Theme::HighContrast.select(ctx); }
                                ui.separator();
                                ui.label(rust_i18n::t!("theme.current", locale = &localization::locale(ctx), name = theme::Theme::current(ctx).name()).into_owned());
                                if ui.button(tr(ui.ctx(), "ui.browse-all-themes")).clicked() { self.themes_open = true; }
                                ui.label(tr(ui.ctx(), "ui.recently-used"));
                                for choice in theme::Theme::recent(ctx) {
                                    if ui.stable_selectable_label(choice == theme::Theme::current(ctx), choice.name()).clicked() {
                                        choice.select(ctx);
                                    }
                                }
                                ui.separator();
                                ui.checkbox(&mut self.show_toolbar_icon_names, tr(ui.ctx(), "ui.show-toolbar-icon-names"));
                                ui.checkbox(&mut self.show_pattern, tr(ui.ctx(), "ui.show-source-patterns-on-other-layers-while-cropping"));
                                ui.weak(tr(ui.ctx(), "ui.themes-also-apply-to-the-viewport-imported-pictures-retain-their-colors"));
                            }
                            Section::Languages => self.language_pack_settings(ui),
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
                                    self.ui_scale = defaults.ui_scale;
                                    self.text_scale = defaults.text_scale;
                                    apply_scaling(ctx, self.ui_scale, self.text_scale);
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
    fn manager_pack(revision: u64, locale: &str) -> Vec<u8> {
        use std::io::Write;
        let manifest = serde_json::json!({"version":2,"id":"manager-test","revision":revision,"languages":[{"locale":locale,"name":"Manager test","native_name":"Manager test"}]});
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, value) in [
            ("manifest.json", manifest),
            (
                &format!("{locale}.json"),
                serde_json::json!({"ui.close":format!("Close revision {revision}")}),
            ),
        ] {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer
                .write_all(&serde_json::to_vec(&value).unwrap())
                .unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn pack_manager_updates_confirms_overwrites_and_removes_active_language_live() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let source = dir.path().join("pack.zip");
        let (store, mut saved) = settings::Store::at(path.clone());
        saved.language = localization::Language::En;
        let ctx = egui::Context::default();
        let mut app = Editor::with_settings(&ctx, store, saved);
        app.preferences_open = true;
        app.fields[0] = "-".into();
        let project = app.project().to_json().unwrap();
        let draft = app.fields.clone();
        click_label(&mut app, &ctx, "Language packs");
        assert_eq!(app.preferences_section, Section::Languages);
        std::fs::write(&source, manager_pack(1, "test-manager")).unwrap();
        app.prepare_language_pack(&ctx, &source);
        assert!(app.language_pack_action.is_none());
        app.preview_language(&ctx, localization::Language::Pack("test-manager".into()));
        app.finish_language_preview(&ctx, true);
        std::fs::write(&source, manager_pack(2, "test-manager")).unwrap();
        app.prepare_language_pack(&ctx, &source);
        assert!(app.language_pack_action.is_none());
        assert_eq!(tr(&ctx, "ui.close"), "Close revision 2");
        app.prepare_language_pack(&ctx, &source);
        assert!(matches!(
            app.language_pack_action,
            Some(PackAction::Install(_))
        ));
        click_label(&mut app, &ctx, "Cancel");
        assert!(app.language_pack_action.is_none());
        std::fs::write(&source, manager_pack(1, "test-manager")).unwrap();
        app.prepare_language_pack(&ctx, &source);
        assert!(matches!(
            app.language_pack_action,
            Some(PackAction::Install(_))
        ));
        click_label(&mut app, &ctx, "Overwrite pack");
        assert_eq!(tr(&ctx, "ui.close"), "Close revision 1");
        click_label(&mut app, &ctx, "Remove pack…");
        click_label(&mut app, &ctx, "Cancel");
        assert_eq!(crate::language_packs::available(&ctx).len(), 1);
        click_label(&mut app, &ctx, "Remove pack…");
        click_label(&mut app, &ctx, "Remove pack…");
        assert!(crate::language_packs::available(&ctx).is_empty());
        assert_eq!(app.language, localization::Language::En);
        assert_eq!(localization::locale(&ctx), "en");
        assert_eq!(
            settings::Store::at(path).1.language,
            localization::Language::En
        );
        assert!(!dir.path().join("language-packs/manager-test.zip").exists());
        assert_eq!(app.project().to_json().unwrap(), project);
        assert_eq!(app.fields, draft);
        // An unrelated installation preserves an already-missing preference.
        app.language = localization::Language::Pack("missing-manager".into());
        app.language.apply(&ctx);
        std::fs::write(&source, manager_pack(2, "test-other-manager")).unwrap();
        app.prepare_language_pack(&ctx, &source);
        assert_eq!(
            app.language,
            localization::Language::Pack("missing-manager".into())
        );
        app.preview_language(
            &ctx,
            localization::Language::Pack("test-other-manager".into()),
        );
        app.finish_language_preview(&ctx, true);
        // Updating away the active locale switches to English without confirmation.
        std::fs::write(&source, manager_pack(3, "test-manager")).unwrap();
        app.prepare_language_pack(&ctx, &source);
        assert!(app.language_pack_action.is_none());
        assert_eq!(app.language, localization::Language::En);
        assert_eq!(app.project().to_json().unwrap(), project);
        assert_eq!(app.fields, draft);
    }

    #[test]
    fn scale_slider_commits_only_after_pointer_release() {
        for maximum in [200.0, 400.0] {
            let ctx = egui::Context::default();
            let mut scale = 1.0;
            let mut rect = egui::Rect::NOTHING;
            let mut run = |events| {
                let _ = ctx.run_ui(
                    egui::RawInput {
                        events,
                        ..Default::default()
                    },
                    |root_ui| {
                        egui::CentralPanel::default().show(root_ui, |ui| {
                            rect = scale_slider(ui, &mut scale, maximum).rect;
                        });
                    },
                );
                (scale, rect)
            };
            let (_, rect) = run(vec![]);
            let start = rect.left_center() + egui::vec2(30.0, 0.0);
            assert_eq!(
                run(vec![egui::Event::PointerMoved(start), mouse(start, true)]).0,
                1.0
            );
            let end = rect.left_center() + egui::vec2(85.0, 0.0);
            assert_eq!(run(vec![egui::Event::PointerMoved(end)]).0, 1.0);
            let (committed, _) = run(vec![mouse(end, false)]);
            assert!(committed > 1.0, "released value: {committed}");
            assert_eq!(run(vec![]).0, committed);
        }
    }

    #[test]
    fn scaling_is_idempotent_and_preserves_project_and_theme() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let project = app.project().to_json().unwrap();
        let base = ctx.global_style().text_styles[&egui::TextStyle::Body].size;
        apply_scaling(&ctx, 1.5, Some(1.25));
        apply_scaling(&ctx, 1.5, Some(1.25));
        let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
        assert_eq!(ctx.zoom_factor(), 1.5);
        assert_eq!(
            ctx.global_style().text_styles[&egui::TextStyle::Body].size,
            base * 1.25
        );
        theme::Theme::HighContrast.apply(&ctx);
        assert_eq!(
            ctx.global_style().text_styles[&egui::TextStyle::Body].size,
            base * 1.25
        );
        app.ui_scale = 1.5;
        app.text_scale = Some(1.25);
        assert_eq!(app.application_settings(&ctx).ui_scale, 1.5);
        assert_eq!(app.application_settings(&ctx).text_scale, Some(1.25));
        assert_eq!(app.project().to_json().unwrap(), project);
        let draft_id = egui::Id::new("pending-scale");
        ctx.data_mut(|data| data.insert_temp(draft_id, ScaleDraft(1.0, 3.0)));
        apply_scaling(&ctx, 1.0, None);
        assert!(
            ctx.data(|data| data.get_temp::<ScaleDraft>(draft_id))
                .is_none()
        );
        assert_eq!(
            ctx.global_style().text_styles[&egui::TextStyle::Body].size,
            base
        );
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
            let previous = app.language.clone();
            let current_name = app.language.name(&ctx);
            click_label(&mut app, &ctx, &current_name);
            click_label(&mut app, &ctx, &language.name(&ctx));
            assert_eq!(app.language, language);
            assert_eq!(app.application_settings(&ctx).language, previous);
            let yes = tr(&ctx, "language.keep");
            click_label(&mut app, &ctx, &yes);
            assert_eq!(settings::Store::at(path.clone()).1.language, language);
            assert_eq!(app.project().to_json().unwrap(), project);
            assert_eq!(
                canvas_pilot::clipboard::encode(&app.layout, &[0], "Summary").unwrap(),
                clipboard
            );
            assert_eq!(app.fields, draft);
            assert!(!app.device.sending);
        }
        click_label(&mut app, &ctx, "English — English");
        click_label(&mut app, &ctx, "Français — French");
        click_label(&mut app, &ctx, "Oui");
        click_label(&mut app, &ctx, "Fermer");
        assert!(!app.preferences_open);
        let (store, saved) = settings::Store::at(path);
        let restored_ctx = egui::Context::default();
        let mut restored = Editor::with_settings(&restored_ctx, store, saved);
        assert_eq!(localization::locale(&restored_ctx), "fr");
        restored.preferences_open = true;
        click_label(&mut restored, &restored_ctx, "Français — Français");
        click_label(&mut restored, &restored_ctx, "English — Anglais");
        assert_eq!(localization::locale(&restored_ctx), "en");
    }

    #[test]
    fn language_preview_cancel_escape_timeout_and_restart_keep_previous_choice() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let (mut store, mut saved) = settings::Store::at(path.clone());
        saved.language = localization::Language::Fr;
        store.save(&saved);
        let ctx = egui::Context::default();
        let mut app = Editor::with_settings(&ctx, store, saved);
        let project = app.project().to_json().unwrap();
        app.preview_language(&ctx, localization::Language::Ja);
        // Even an unrelated preferences save or restart must not retain the preview.
        app.settings.save(&app.application_settings(&ctx));
        assert_eq!(
            settings::Store::at(path.clone()).1.language,
            localization::Language::Fr
        );
        click_label(&mut app, &ctx, "Non, j’ai choisi cette langue par erreur");
        assert_eq!(localization::locale(&ctx), "fr");
        assert!(app.language_preview.is_none());

        app.preview_language(&ctx, localization::Language::ZhCn);
        frame(&mut app, &ctx, vec![]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(app.language, localization::Language::Fr);
        app.preview_language(&ctx, localization::Language::Es);
        app.language_preview.as_mut().unwrap().deadline = std::time::Instant::now();
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.language, localization::Language::Fr);
        assert_eq!(localization::locale(&ctx), "fr");
        assert!(app.language_preview.is_none());
        app.preview_language(&ctx, localization::Language::Fr);
        assert!(app.language_preview.is_none());
        assert_eq!(app.project().to_json().unwrap(), project);
    }

    #[test]
    fn installed_languages_use_confirmation_preserve_project_and_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let (store, saved) = settings::Store::at(path.clone());
        let ctx = egui::Context::default();
        let mut app = Editor::with_settings(&ctx, store, saved);
        let source = dir.path().join("source.zip");
        std::fs::write(
            &source,
            include_bytes!("../../../docs/language-packs/language-pack.zip"),
        )
        .unwrap();
        crate::language_packs::install(&ctx, &dir.path().join("language-packs"), &source).unwrap();
        let original = app.project();
        app.preferences_open = true;
        // The combo scrolls once installed languages extend the built-in list.
        app.preview_language(&ctx, localization::Language::Pack("uk".into()));
        let yes = tr(&ctx, "language.keep");
        click_label(&mut app, &ctx, &yes);
        assert_eq!(
            settings::Store::at(path.clone()).1.language,
            localization::Language::Pack("uk".into())
        );
        app.preview_language(&ctx, localization::Language::Pack("bho".into()));
        assert_eq!(
            app.application_settings(&ctx).language,
            localization::Language::Pack("uk".into())
        );
        app.language_preview.as_mut().unwrap().deadline = std::time::Instant::now();
        frame(&mut app, &ctx, vec![]);
        assert_eq!(localization::locale(&ctx), "pack:uk");
        assert_eq!(app.project(), original);
        let (store, saved) = settings::Store::at(path.clone());
        let restarted_ctx = egui::Context::default();
        let restored = Editor::with_settings(&restarted_ctx, store, saved);
        assert_eq!(tr(&restarted_ctx, "ui.settings"), "Налаштування");
        assert_eq!(restored.project(), original);
        std::fs::remove_file(dir.path().join("language-packs/example-uk-bho.zip")).unwrap();
        let (store, saved) = settings::Store::at(path);
        let missing_ctx = egui::Context::default();
        let missing = Editor::with_settings(&missing_ctx, store, saved);
        assert_eq!(localization::locale(&missing_ctx), "en");
        assert_eq!(missing.language, localization::Language::Pack("uk".into()));
        assert!(missing.language.name(&missing_ctx).contains("unavailable"));
    }

    #[test]
    fn opening_project_preserves_language_scaling_and_pending_preview() {
        let dir = tempfile::tempdir().unwrap();
        let (store, mut saved) = settings::Store::at(dir.path().join("settings.json"));
        saved.language = localization::Language::Ne;
        saved.ui_scale = 1.25;
        saved.text_scale = Some(1.1);
        let ctx = egui::Context::default();
        let mut app = Editor::with_settings(&ctx, store, saved);
        app.install_project(&ctx, app.project()).unwrap();
        assert_eq!(app.language, localization::Language::Ne);
        assert_eq!(app.ui_scale, 1.25);
        assert_eq!(app.text_scale, Some(1.1));
        assert_eq!(localization::locale(&ctx), "ne");

        app.preview_language(&ctx, localization::Language::Hi);
        app.install_project(&ctx, app.project()).unwrap();
        assert_eq!(app.language, localization::Language::Hi);
        assert_eq!(
            app.application_settings(&ctx).language,
            localization::Language::Ne
        );
        app.finish_language_preview(&ctx, false);
        assert_eq!(app.language, localization::Language::Ne);
        assert_eq!(localization::locale(&ctx), "ne");
    }

    #[test]
    fn enlarged_settings_keep_recovery_controls_visible() {
        for language in localization::Language::ALL {
            let ctx = egui::Context::default();
            language.apply(&ctx);
            apply_scaling(&ctx, 1.0, Some(1.5));
            let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(500.0, 360.0));
            let mut app = Editor {
                language: language.clone(),
                ui_scale: 2.0,
                text_scale: Some(1.5),
                preferences_open: true,
                ..Default::default()
            };
            let mut output = egui::FullOutput::default();
            for _ in 0..5 {
                output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ctx| app.preferences_window(ctx),
                );
            }
            for key in ["ui.close", "access.reset"] {
                let label = tr(&ctx, key);
                let rect = output
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t) if t.galley.text() == label => {
                            Some(Rect::from_min_size(t.pos, t.galley.size()))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("Missing {key} in {language:?}"));
                assert!(screen.contains_rect(rect), "{language:?}: {key}: {rect:?}");
            }
        }
    }

    #[test]
    fn settings_language_and_close_stay_visible_at_minimum_window_size() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 720.0));
        for language in localization::Language::ALL {
            for (section, _) in Section::ALL {
                let ctx = egui::Context::default();
                language.apply(&ctx);
                let mut app = Editor {
                    language: language.clone(),
                    preferences_open: true,
                    preferences_section: section,
                    ..Editor::default()
                };
                let mut output = egui::FullOutput::default();
                for _ in 0..3 {
                    output = ctx.run_ui(
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
                        .unwrap_or_else(|| panic!("Missing {label}: {language:?} / {section:?}"));
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
