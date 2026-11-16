use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use canvas_pilot::project::{Project, Reference as SavedReference, write_file};
use std::path::PathBuf;
pub(super) struct PreparedProject {
    project: Project,
    capture: capture_ui::CaptureUi,
    layout: Layout,
    references: Vec<super::super::reference::Reference>,
}
type LoadedProject = (PreparedProject, Option<PathBuf>, Option<u64>);
type ProjectJob = std::sync::mpsc::Receiver<Result<Option<LoadedProject>, String>>;
type ExportJob = std::sync::mpsc::Receiver<Result<(PathBuf, Vec<u8>), String>>;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ExportScope {
    #[default]
    Selected,
    All,
    Visible,
    Specific,
}
impl ExportScope {
    fn label(self) -> &'static str {
        match self {
            Self::Selected => "Selected layers",
            Self::All => "All layers",
            Self::Visible => "Visible layers only",
            Self::Specific => "Specific layer",
        }
    }
}

#[derive(Default)]
pub(super) struct FileUi {
    pub(super) project_path: Option<PathBuf>,
    pub(super) manual_saved_at: Option<u64>,
    pub(super) new_requested: bool,
    pub(super) new_after_save: bool,
    pub(super) new_ready: bool,
    opening: Option<PathBuf>,
    pub(super) loading: Option<ProjectJob>,
    loading_recovery: bool,
    writing: Option<(PathBuf, Vec<u8>)>,
    pub(super) message: Option<String>,
    exchange_options: canvas_pilot::exchange::Options,
    export_open: bool,
    export_scope: ExportScope,
    export_layer: Option<usize>,
    export_codec: canvas_pilot::interchange::Codec,
    export_job: Option<ExportJob>,
}
impl PreparedProject {
    fn prepare(ctx: &egui::Context, project: Project) -> Result<Self, String> {
        let layout = project.layout()?;
        let references = project
            .references
            .iter()
            .map(|r| {
                let file = egui::DroppedFile {
                    name: r.name.clone(),
                    bytes: Some(r.bytes()?.into()),
                    ..Default::default()
                };
                let mut reference = super::super::reference::Reference::load(ctx, &file)?;
                reference.visible = r.visible;
                reference.opacity = r.opacity;
                reference.preserve_ratio = r.preserve_ratio;
                reference.native_size = r.native_size;
                reference.offset = r.offset;
                Ok::<_, String>(reference)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let capture = capture_ui::CaptureUi::from_workspace(ctx, &project.workspace)?;
        Ok(Self {
            capture,
            project,
            layout,
            references,
        })
    }
}
impl Editor {
    pub(super) fn project(&self) -> Project {
        let mut p = Project::from_layout(&self.layout);
        p.input_proposals = self
            .device
            .input_edits
            .saved
            .iter()
            .filter(|proposal| {
                p.layers.iter().any(|l| {
                    l.id == proposal.layer_id
                        && l.device_link.as_ref() == Some(&proposal.device_link)
                })
            })
            .cloned()
            .collect();
        p.editor.position_point = self.corner;
        p.editor.position_layer_point = self.position_layer_point;
        p.editor.backdrop_title = self.backdrop_title.clone();
        p.editor.keep_aspect_ratio = self.keep_aspect_ratio;
        p.editor.viewport_zoom = self.zoom;
        p.editor.viewport_pan = [self.pan.x, self.pan.y];
        p.editor.show_source_pattern = self.show_pattern;
        p.editor.outline_layers = self.outline_layers;
        p.editor.show_toolbar_icon_names = self.show_toolbar_icon_names;
        p.references = self
            .references
            .iter()
            .map(|r| SavedReference {
                name: r.name.clone(),
                image_base64: STANDARD.encode(&r.bytes),
                visible: r.visible,
                opacity: r.opacity,
                preserve_ratio: r.preserve_ratio,
                native_size: r.native_size,
                offset: r.offset,
            })
            .collect();
        p.workspace.snapping = self.snapping;
        p.workspace.resize_mode = self.resize_mode;
        p.workspace.grid_spacing = self.grid_tool;
        p.workspace.collapsed_folders = self
            .layer_ui
            .collapsed
            .iter()
            .filter(|folder| {
                p.folders.contains(folder) || p.layers.iter().any(|l| &l.folder == *folder)
            })
            .cloned()
            .collect();
        self.capture.save_workspace(&self.layout, &mut p.workspace);
        p
    }
    pub(super) fn install_project(
        &mut self,
        ctx: &egui::Context,
        project: Project,
    ) -> Result<(), String> {
        if self.device.sending {
            return Err(
                "Wait for the device send and readback before opening another project.".into(),
            );
        }
        let prepared = PreparedProject::prepare(ctx, project)?;
        self.install_prepared(prepared)
    }
    fn install_prepared(&mut self, prepared: PreparedProject) -> Result<(), String> {
        if self.device.sending {
            return Err(
                "Wait for the device send and readback before opening another project.".into(),
            );
        }
        let PreparedProject {
            capture,
            project,
            layout,
            references,
        } = prepared;
        let e = project.editor;
        self.device.reset_mappings();
        self.device.input_edits.clear();
        self.device.input_edits.saved = project.input_proposals;
        self.capture.clear_preview();
        let mut layer_ui = layers_ui::LayerUi::default();
        layer_ui.collapsed = project.workspace.collapsed_folders.into_iter().collect();
        // Replace only after both the document and embedded images are ready.
        *self = Self {
            device: std::mem::take(&mut self.device),
            capture,
            settings: std::mem::take(&mut self.settings),
            language: self.language.clone(),
            language_preview: self.language_preview.take(),
            ui_scale: self.ui_scale,
            text_scale: self.text_scale,
            recovery: std::mem::take(&mut self.recovery),
            snapping: project.workspace.snapping,
            resize_mode: project.workspace.resize_mode,
            grid_tool: project.workspace.grid_spacing,
            layer_ui,
            pivot_tool: self.pivot_tool,
            navigation_tool: self.navigation_tool,
            layout,
            reference_selected: references.len().checked_sub(1),
            references,
            corner: e.position_point,
            position_layer_point: e.position_layer_point,
            backdrop_title: e.backdrop_title,
            keep_aspect_ratio: e.keep_aspect_ratio,
            zoom: e.viewport_zoom,
            pan: Vec2::from(e.viewport_pan),
            show_pattern: e.show_source_pattern,
            outline_layers: e.outline_layers,
            show_toolbar_icon_names: e.show_toolbar_icon_names,
            ..Default::default()
        };
        self.sync();
        Ok(())
    }
    pub(super) fn start_project_job(
        &mut self,
        ctx: &egui::Context,
        recovery: bool,
        read: impl FnOnce() -> Result<Option<(Project, Option<PathBuf>, Option<u64>)>, String>
        + Send
        + 'static,
    ) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.files.loading = Some(rx);
        self.files.loading_recovery = recovery;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = read().and_then(|loaded| {
                loaded
                    .map(|(project, path, time)| {
                        PreparedProject::prepare(&ctx, project).map(|p| (p, path, time))
                    })
                    .transpose()
            });
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
    pub(super) fn poll_project_job(&mut self) {
        let Some(rx) = &self.files.loading else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(_) => Err("Project reader stopped unexpectedly.".into()),
        };
        let recovery = self.files.loading_recovery;
        self.files.loading = None;
        let result = result.and_then(|loaded| {
            if let Some((prepared, path, time)) = loaded {
                self.install_prepared(prepared)?;
                self.files.message = if recovery {
                    None
                } else {
                    Some(
                        path.as_ref()
                            .map(|p| format!("Opened {}", p.display()))
                            .unwrap_or_else(|| {
                                "Restored embedded parameters into an unsaved project.".into()
                            }),
                    )
                };
                self.files.project_path = path;
                self.files.manual_saved_at = time;
            }
            Ok(())
        });
        if let Err(error) = result {
            let message = format!("Cannot open project; current work kept: {error}");
            if recovery {
                self.recovery.restore_failed = true;
                self.recovery.error = Some(format!(
                    "Recovery could not be restored: {error}. The file is preserved."
                ));
            }
            self.files.message = Some(message);
        }
    }
    #[cfg(test)]
    pub(super) fn wait_project_job(&mut self) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while self.files.loading.is_some() && std::time::Instant::now() < until {
            self.poll_project_job();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(self.files.loading.is_none());
    }
    pub(super) fn save_project_as(&mut self) {
        self.layout.finish_drag();
        match self.project().to_json() {
            Ok(contents) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Canvas Pilot project", &["json"])
                    .set_file_name("layout.pilot.json")
                    .save_file()
                {
                    self.queue_write(path, contents);
                } else {
                    self.files.new_after_save = false;
                }
            }
            Err(e) => {
                self.files.new_after_save = false;
                self.files.message = Some(e);
            }
        }
    }
    fn queue_write(&mut self, path: PathBuf, contents: impl Into<Vec<u8>>) {
        let contents = contents.into();
        if path.try_exists().unwrap_or(true) {
            self.files.writing = Some((path, contents));
        } else {
            self.write_project_file(path, contents, false);
        }
    }
    fn write_project_file(&mut self, path: PathBuf, contents: impl Into<Vec<u8>>, overwrite: bool) {
        let contents = contents.into();
        self.files.message = Some(match write_file(&path, &contents, overwrite) {
            Ok(()) => {
                if std::str::from_utf8(&contents).is_ok_and(|text| {
                    text.trim_start().starts_with('{') && Project::from_json(text).is_ok()
                }) {
                    self.files.project_path = Some(path.clone());
                    self.files.manual_saved_at = Some(recovery_ui::now_seconds());
                    if self.files.new_after_save {
                        self.files.new_ready = true;
                        self.files.new_after_save = false;
                    }
                }
                format!("Saved {}", path.display())
            }
            Err(e) => {
                self.files.new_after_save = false;
                format!("Could not save {}: {e}", path.display())
            }
        });
    }
    /// Persist only measurement choices; other unsaved edits remain in memory.
    pub(super) fn autosave_position_points(&mut self) {
        let Some(path) = self.files.project_path.as_ref() else {
            return;
        };
        let result = (|| {
            let mut saved = Project::read(path)?;
            saved.editor.position_point = self.corner;
            saved.editor.position_layer_point = self.position_layer_point;
            write_file(path, saved.to_json()?.as_bytes(), true)
        })();
        if let Err(error) = result {
            self.files.message = Some(format!(
                "Could not autosave measurement points to {}: {error}. Selections remain active; use Save project to save them.",
                path.display()
            ));
        }
    }
    pub(super) fn project_menu(&mut self, ui: &mut egui::Ui) {
        if ui
            .add_enabled(
                !self.device.sending,
                egui::Button::new(tr(ui.ctx(), "ui.new-project-cf8d3c")),
            )
            .clicked()
        {
            self.files.new_requested = true;
            ui.close();
        }
        if let Some(error) = &self.recovery.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        if self.recovery.error.is_some() && ui.button(tr(ui.ctx(), "ui.retry-recovery")).clicked() {
            if self.recovery.restore_failed {
                if let Some(path) = self.recovery.path.clone() {
                    self.restore_recovery(ui.ctx(), path);
                }
            } else {
                self.recovery.error = None;
                self.autosave_recovery(true);
            }
        }
        if let Some(path) = &self.recovery.path {
            ui.small(if self.recovery.error.is_some() {
                "Recovery autosave paused"
            } else {
                "Recovery autosave on · resumes on launch"
            })
            .on_hover_text(path.display().to_string());
        }
        ui.small(self.manual_save_age());
        if ui.button(tr(ui.ctx(), "ui.open-project")).clicked() {
            ui.close();
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Canvas Pilot project", &["json"])
                .pick_file()
            {
                self.files.opening = Some(path);
            }
        }
        if ui
            .button(tr(ui.ctx(), "ui.restore-parameters-from-export"))
            .clicked()
        {
            ui.close();
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    "Exports with Canvas Pilot parameters",
                    &[
                        "svg", "drawio", "xml", "json", "csv", "dxf", "html", "pptx", "figcraft",
                    ],
                )
                .pick_file()
            {
                self.files.opening = Some(path);
            }
        }
        if ui.button(tr(ui.ctx(), "ui.save-project")).clicked() {
            ui.close();
            self.save_project_as();
        }
        ui.small(tr(ui.ctx(), "ui.saves-applied-values-and-reference-images"));
    }
    pub(super) fn export_menu(&mut self, ui: &mut egui::Ui) {
        self.device_scope_picker(ui);
        self.device_apply_button(ui);
        self.device_send_button(ui);
        self.device_load_button(ui);
        ui.separator();
        if ui.button(tr(ui.ctx(), "ui.export-layers")).clicked() {
            self.files.export_open = true;
            ui.close();
        }
    }
    fn export_layer_ids(&self) -> Vec<usize> {
        (0..self.layout.layers().len())
            .filter(|&i| match self.files.export_scope {
                ExportScope::Selected => self.layout.selected_indices().contains(&i),
                ExportScope::All => true,
                ExportScope::Visible => self.layout.visible(i),
                ExportScope::Specific => {
                    self.files.export_layer == Some(self.layout.layer_info(i).id)
                }
            })
            .map(|i| self.layout.layer_info(i).id)
            .collect()
    }
    fn exchange_menu(&mut self, ui: &mut egui::Ui, format: canvas_pilot::interchange::Codec) {
        egui::ComboBox::new("Layers", tr(ui.ctx(), "ui.layers"))
            .selected_text(self.files.export_scope.label())
            .show_ui(ui, |ui| {
                for scope in [
                    ExportScope::Selected,
                    ExportScope::All,
                    ExportScope::Visible,
                    ExportScope::Specific,
                ] {
                    ui.stable_selectable_value(&mut self.files.export_scope, scope, scope.label());
                }
            });
        if self.files.export_scope == ExportScope::Selected {
            self.transfer_layer_picker(ui, "Choose layers…");
        }
        if self.files.export_scope == ExportScope::Specific {
            if !self
                .layout
                .layers()
                .iter()
                .enumerate()
                .any(|(i, _)| Some(self.layout.layer_info(i).id) == self.files.export_layer)
            {
                self.files.export_layer = (0..self.layout.layers().len())
                    .next()
                    .map(|i| self.layout.layer_info(i).id);
            }
            let label = (0..self.layout.layers().len())
                .find(|&i| Some(self.layout.layer_info(i).id) == self.files.export_layer)
                .map(|i| self.layout.layer_label(i))
                .unwrap_or_default();
            egui::ComboBox::new("Layer", tr(ui.ctx(), "ui.layer-ac9ffc"))
                .selected_text(label)
                .show_ui(ui, |ui| {
                    for i in 0..self.layout.layers().len() {
                        ui.stable_selectable_value(
                            &mut self.files.export_layer,
                            Some(self.layout.layer_info(i).id),
                            self.layout.layer_label(i),
                        );
                    }
                });
        }
        let metadata = canvas_pilot::exchange_metadata::supported(format);
        if !metadata {
            self.files.exchange_options.extra_parameters = false;
        }
        if !format.artwork() {
            self.files.exchange_options.placeholders = false;
        }
        let options = &mut self.files.exchange_options;
        let pictures = match (options.background, options.placeholders) {
            (true, true) => "Pictures · background and placeholders",
            (true, false) => "Pictures · background included",
            (false, true) => "Pictures · placeholders included",
            (false, false) => "Pictures · none",
        };
        egui::CollapsingHeader::new(if format.artwork() { pictures } else if options.background { "Background · included" } else { "Background · none" })
            .id_salt("export-pictures").show(ui, |ui| {
                ui.checkbox(&mut options.background, tr(ui.ctx(), "ui.include-background-layer"))
                    .on_hover_text(if format.artwork() { "Use the visible reference images, or a plain background when none is visible." } else { "Include a screen-sized rectangle. This format cannot embed pictures." });
                if format.artwork() {
                    ui.checkbox(&mut options.placeholders, tr(ui.ctx(), "ui.include-placeholder-pictures"))
                        .on_hover_text(tr(ui.ctx(), "ui.show-source-patterns-with-the-current-crop-and-mask-these-are-not-live-captures"));
                }
            });
        egui::CollapsingHeader::new(if options.extra_parameters { "Canvas Pilot settings · included" } else { "Canvas Pilot settings · not included" })
            .id_salt("export-settings").show(ui, |ui| {
                if metadata {
                    ui.checkbox(&mut options.extra_parameters, tr(ui.ctx(), "ui.include-extra-canvas-pilot-parameters-experimental"));
                    ui.small(tr(ui.ctx(), "ui.keep-an-editable-snapshot-of-these-layers-restore-it-from-the-project-menu-other-edit"));
                    ui.label(tr(ui.ctx(), "ui.what-is-saved")).on_hover_text(tr(ui.ctx(), "ui.source-settings-crop-fit-blanking-mappings-inactive-input-proposals-and-workspace-set"));
                } else {
                    ui.small(tr(ui.ctx(), "ui.this-format-cannot-carry-editable-settings-save-a-pilot-json-project-alongside-it"));
                }
            });
        ui.separator();
        let ids = self.export_layer_ids();
        ui.small(format!(
            "{} {} to export",
            ids.len(),
            if ids.len() == 1 { "layer" } else { "layers" }
        ));
        if ui
            .add_enabled(
                !ids.is_empty(),
                egui::Button::new(tr(ui.ctx(), "ui.export-4a4c59")),
            )
            .on_disabled_hover_text("Choose at least one layer to export.")
            .clicked()
        {
            self.export_exchange(
                format,
                &ids,
                if ids.len() == 1 { "layer" } else { "layout" },
            );
        }
    }
    fn export_exchange(
        &mut self,
        format: canvas_pilot::interchange::Codec,
        ids: &[usize],
        name: &str,
    ) {
        let mut options = self.files.exchange_options;
        if !format.artwork() {
            options.placeholders = false;
        }
        if self.files.export_job.is_some() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(format.label(), &[format.extension()])
            .set_file_name(format!("{name}.{}", format.extension()))
            .save_file()
        {
            let project = self.project();
            let ids = ids.to_vec();
            let (tx, rx) = std::sync::mpsc::channel();
            self.files.export_job = Some(rx);
            self.files.export_open = false;
            std::thread::spawn(move || {
                let result = canvas_pilot::interchange::export(&project, &ids, format, options)
                    .map(|bytes| (path, bytes));
                let _ = tx.send(result);
            });
        }
    }
    pub(super) fn project_windows(&mut self, ctx: &egui::Context) {
        if self.files.loading.is_some() {
            egui::Window::new(tr(ctx, "ui.loading-project"))
                .id(egui::Id::new("Loading project"))
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr(
                            ui.ctx(),
                            "ui.reading-project-and-decoding-reference-images",
                        ));
                    });
                    if !self.files.loading_recovery
                        && ui.button(tr(ui.ctx(), "ui.cancel-loading")).clicked()
                    {
                        self.files.loading = None;
                    }
                });
        }
        if let Some(job) = self.files.export_job.take() {
            match job.try_recv() {
                Ok(Ok((path, bytes))) => self.queue_write(path, bytes),
                Ok(Err(error)) => self.files.message = Some(error),
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.files.export_job = Some(job);
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    egui::Window::new(tr(ctx, "ui.exporting-layers"))
                        .id(egui::Id::new("Exporting layers"))
                        .collapsible(false)
                        .show(ctx, |ui| {
                            ui.spinner();
                            ui.label(tr(ui.ctx(), "ui.preparing-the-document"));
                        });
                }
                Err(_) => {
                    self.files.message = Some("The export worker stopped unexpectedly.".into())
                }
            }
        }
        if self.files.export_open {
            let mut open = true;
            egui::Window::new(tr(ctx, "ui.export-layers-c8fcde"))
                .id(egui::Id::new("Export layers"))
                .open(&mut open)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::ComboBox::new("Format", tr(ui.ctx(), "ui.format"))
                        .selected_text(self.files.export_codec.label())
                        .show_ui(ui, |ui| {
                            for codec in canvas_pilot::interchange::Codec::available() {
                                ui.stable_selectable_value(
                                    &mut self.files.export_codec,
                                    codec,
                                    codec.label(),
                                );
                            }
                        });
                    ui.label(self.files.export_codec.help());
                    ui.separator();
                    self.exchange_menu(ui, self.files.export_codec);
                });
            self.files.export_open &= open;
        }
        if let Some(path) = self.files.opening.take() {
            let mut open = true;
            let mut load = false;
            let mut cancel = false;
            egui::Window::new(tr(ctx, "ui.open-project-a77d12")).id(egui::Id::new("Open project?")).collapsible(false).open(&mut open).show(ctx,|ui| {
                ui.label(path.display().to_string());
                ui.label(tr(ui.ctx(), "ui.opening-replaces-the-current-work-undo-history-and-recovery-autosave-embedded-paramet"));
                ui.label(self.manual_save_age());
                ui.horizontal(|ui| {load=ui.button(tr(ui.ctx(), "ui.open-project-5e5eba")).clicked();cancel=ui.button(tr(ui.ctx(), "ui.cancel")).clicked();});
            });
            if load {
                self.start_project_job(ctx, false, move || {
                    let (project, native) =
                        canvas_pilot::interchange::read_project_or_parameters(&path)?;
                    // Never autosave JSON over the source artwork.
                    Ok(Some((project, native.then_some(path), None)))
                });
            } else if open && !cancel {
                self.files.opening = Some(path);
            }
        }
        if let Some((path, contents)) = self.files.writing.take() {
            let mut open = true;
            let mut replace = false;
            let mut cancel = false;
            egui::Window::new(tr(ctx, "ui.replace-existing-file"))
                .id(egui::Id::new("Replace existing file?"))
                .collapsible(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(path.display().to_string());
                    ui.horizontal(|ui| {
                        replace = ui.button(tr(ui.ctx(), "ui.replace-file")).clicked();
                        cancel = ui.button(tr(ui.ctx(), "ui.cancel")).clicked();
                    });
                });
            if replace {
                self.write_project_file(path, contents, true);
            } else if open && !cancel {
                self.files.writing = Some((path, contents));
            } else {
                self.files.new_after_save = false;
            }
        }
        if let Some(message) = self.files.message.take() {
            let mut open = true;
            let mut done = false;
            egui::Window::new(tr(ctx, "ui.project-file"))
                .id(egui::Id::new("Project file"))
                .collapsible(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(&message);
                    done = ui.button(tr(ui.ctx(), "ui.ok")).clicked();
                });
            if open && !done {
                self.files.message = Some(message);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_scope_is_explicit_and_unavailable_options_are_cleared() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.files.export_scope = ExportScope::All;
        let all = app.export_layer_ids();
        assert_eq!(all.len(), app.layout.layers().len());
        app.files.export_scope = ExportScope::Specific;
        app.files.export_layer = Some(all[1]);
        assert_eq!(app.export_layer_ids(), vec![all[1]]);
        app.files.export_layer = Some(usize::MAX);
        assert!(app.export_layer_ids().is_empty());
        app.files.export_scope = ExportScope::Selected;
        let expected: Vec<_> = app
            .layout
            .selected_indices()
            .iter()
            .map(|&i| app.layout.layer_info(i).id)
            .collect();
        assert_eq!(app.export_layer_ids(), expected);
        for format in canvas_pilot::interchange::Codec::available() {
            app.files.exchange_options.extra_parameters = true;
            app.files.exchange_options.placeholders = true;
            let output = ctx.run_ui(egui::RawInput::default(), |root_ui| {
                egui::CentralPanel::default().show(root_ui, |ui| app.exchange_menu(ui, format));
            });
            assert_eq!(
                app.files.exchange_options.extra_parameters,
                canvas_pilot::exchange_metadata::supported(format)
            );
            assert_eq!(app.files.exchange_options.placeholders, format.artwork());
            assert!(
                output.shapes.iter().any(
                    |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Export…")
                )
            );
            assert!(!output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Include placeholder pictures")));
        }
    }

    #[test]
    fn optional_export_groups_expand_without_exporting_or_changing_layers() {
        use super::super::tests::{click_text, frame};
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.files.export_open = true;
        let before = app.project();
        click_text(&mut app, &ctx, "Pictures · none");
        click_text(&mut app, &ctx, "Include placeholder pictures");
        assert!(app.files.exchange_options.placeholders);
        click_text(&mut app, &ctx, "Canvas Pilot settings · not included");
        click_text(
            &mut app,
            &ctx,
            "Include extra Canvas Pilot parameters (experimental)",
        );
        assert!(app.files.exchange_options.extra_parameters);
        frame(&mut app, &ctx, vec![]);
        assert!(app.files.export_job.is_none());
        assert_eq!(app.project(), before);
    }

    #[test]
    fn new_after_save_requires_success_and_does_not_count_autosave() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Editor::default();
        app.files.new_after_save = true;
        app.write_project_file(
            dir.path().join("missing/layout.json"),
            app.project().to_json().unwrap(),
            false,
        );
        assert!(!app.files.new_ready);
        assert!(!app.files.new_after_save);
        assert!(app.files.manual_saved_at.is_none());
        app.files.new_after_save = true;
        app.write_project_file(
            dir.path().join("layout.json"),
            app.project().to_json().unwrap(),
            false,
        );
        assert!(app.files.new_ready);
        assert!(app.files.manual_saved_at.is_some());
        let time = app.files.manual_saved_at;
        app.autosave_position_points();
        assert_eq!(app.files.manual_saved_at, time);
    }

    #[test]
    fn saved_input_proposals_reopen_inactive_and_follow_layer_identity() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let project = Project::from_json(include_str!(
            "../../../docs/interfaces/canvas-pilot-v16.example.json"
        ))
        .unwrap();
        app.device.input_edits.enabled = true;
        app.install_project(&ctx, project.clone()).unwrap();
        assert!(!app.device.input_edits.enabled);
        assert_eq!(app.project().input_proposals, project.input_proposals);
        app.layout.set_device_link(0, None).unwrap();
        assert!(app.project().input_proposals.is_empty());
        app.install_project(&ctx, Project::from_layout(&Layout::default()))
            .unwrap();
        assert!(app.device.input_edits.saved.is_empty());
    }

    #[test]
    fn measurement_clicks_autosave_only_points_and_exports_keep_project_target() {
        use super::super::tests::click_text;
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("layout.json");
        let mut app = Editor::default();
        let saved = app.project();
        app.write_project_file(path.clone(), saved.to_json().unwrap(), false);
        assert_eq!(app.files.project_path.as_ref(), Some(&path));
        app.files.message = None;
        // Other local edits must not be silently saved with the point selection.
        app.layout.nudge(15.0, 30.0).unwrap();
        app.backdrop_title = "Unsaved title".into();
        // Preserve unrelated changes already written by another editor as well.
        let mut external = saved.clone();
        external.editor.backdrop_title = "External title".into();
        write_file(&path, external.to_json().unwrap().as_bytes(), true).unwrap();
        app.sync();
        super::super::tests::open_measurement_menu(&mut app, &ctx);
        super::super::tests::click_measurement_point(&mut app, &ctx, "corner-BottomRight");
        external.editor.position_point = PositionPoint::BottomRight;
        assert_eq!(Project::read(&path).unwrap(), external);
        click_text(&mut app, &ctx, "Use a different measure point");
        external.editor.position_layer_point = Some(PositionPoint::BottomRight);
        assert_eq!(Project::read(&path).unwrap(), external);
        super::super::tests::click_measurement_point(&mut app, &ctx, "layer-point-Center");
        external.editor.position_layer_point = Some(PositionPoint::Center);
        assert_eq!(Project::read(&path).unwrap(), external);
        click_text(&mut app, &ctx, "Use a different measure point");
        external.editor.position_layer_point = None;
        assert_eq!(Project::read(&path).unwrap(), external);
        app.write_project_file(
            dir.path().join("layout.svg"),
            saved.to_svg(false).unwrap(),
            false,
        );
        assert_eq!(app.files.project_path.as_ref(), Some(&path));
        let mut reopened = Editor::default();
        reopened
            .install_project(&ctx, Project::read(&path).unwrap())
            .unwrap();
        assert_eq!(reopened.corner, PositionPoint::BottomRight);
        assert_eq!(reopened.position_layer_point, None);
    }

    #[test]
    fn point_autosave_requires_a_valid_existing_project_and_reports_failure() {
        let mut app = Editor {
            corner: PositionPoint::Center,
            ..Default::default()
        };
        app.autosave_position_points();
        assert!(app.files.message.is_none());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.json");
        std::fs::write(&path, "invalid project").unwrap();
        app.files.project_path = Some(path.clone());
        app.autosave_position_points();
        assert!(
            app.files
                .message
                .as_ref()
                .unwrap()
                .contains("Could not autosave")
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "invalid project");
        assert_eq!(app.corner, PositionPoint::Center);
        std::fs::remove_file(&path).unwrap();
        app.autosave_position_points();
        assert!(!path.exists());
    }

    #[test]
    fn folder_collapse_roundtrip_filters_deleted_folder_names() {
        let ctx = egui::Context::default();
        let mut project = Project::from_layout(&Layout::default());
        project.layers[0].folder = "Memory A".into();
        let mut app = Editor::default();
        app.install_project(&ctx, project).unwrap();
        app.layer_ui
            .collapsed
            .extend(["Memory A".into(), "Deleted".into()]);
        let saved = Project::from_json(&app.project().to_json().unwrap()).unwrap();
        assert_eq!(saved.workspace.collapsed_folders, ["Memory A"]);
        app.layer_ui.collapsed.clear();
        app.install_project(&ctx, saved).unwrap();
        assert_eq!(
            app.layer_ui.collapsed.into_iter().collect::<Vec<_>>(),
            ["Memory A"]
        );
    }

    #[test]
    fn opening_project_restores_resize_setup_but_preserves_active_tool() {
        let ctx = egui::Context::default();
        for mode in [ResizeMode::Same, ResizeMode::Mirror, ResizeMode::Opposite] {
            let mut app = Editor {
                resize_mode: mode,
                grid_tool: Some(100),
                pivot_tool: true,
                navigation_tool: NavigationTool::Pan,
                ..Default::default()
            };
            let saved = app.project();
            app.resize_mode = ResizeMode::Individual;
            app.grid_tool = None;
            app.install_project(&ctx, saved).unwrap();
            assert_eq!(app.resize_mode, mode);
            assert_eq!(app.grid_tool, Some(100));
            assert!(app.pivot_tool);
            assert_eq!(app.navigation_tool, NavigationTool::Pan);
            assert_eq!(app.active_resize_mode(), ResizeMode::Individual);
            app.layout.set_selection([true; 2]);
            app.sync();
            assert_eq!(app.active_resize_mode(), mode);
        }
    }

    #[test]
    fn project_restores_reference_and_editor_and_failed_image_load_keeps_current_work() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 50, 80, 255]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let original_bytes = bytes.into_inner();
        let file = egui::DroppedFile {
            name: "guide.png".into(),
            bytes: Some(original_bytes.clone().into()),
            ..Default::default()
        };
        app.load_reference(&ctx, &file);
        app.wait_reference();
        let reference = app.references.first_mut().unwrap();
        reference.opacity = 0.4;
        reference.visible = false;
        reference.preserve_ratio = true;
        reference.native_size = true;
        reference.offset = [-12.5, 34.25];
        let mut second_file = file.clone();
        second_file.name = "second-guide.png".into();
        app.load_reference(&ctx, &second_file);
        app.wait_reference();
        assert_eq!(app.references.len(), 2);
        app.outline_layers = true;
        app.zoom = 2.5;
        app.pan = Vec2::new(-20.5, 40.0);
        app.corner = PositionPoint::TopLeft;
        app.position_layer_point = Some(PositionPoint::Center);
        app.backdrop_title = "Conference 2027 · Main stage".into();
        app.keep_aspect_ratio = false;
        app.show_pattern = true;
        app.show_toolbar_icon_names = true;
        app.layout
            .set_layer_pivots(&[(0, [-0.25, 0.75]), (1, [0.1, 0.9])])
            .unwrap();
        let project = Project::from_json(&app.project().to_json().unwrap()).unwrap();
        let mut restored = Editor {
            edit_content: true,
            ..Default::default()
        };
        let settings_dir = tempfile::tempdir().unwrap();
        let settings_path = settings_dir.path().join("settings.json");
        restored.settings = settings::Store::at(settings_path.clone()).0;
        restored.device.host = "192.168.10.143".into();
        restored.install_project(&ctx, project.clone()).unwrap();
        assert_eq!(restored.device.host, "192.168.10.143");
        assert_eq!(restored.settings.path.as_ref(), Some(&settings_path));
        assert!(
            !settings_path.exists(),
            "opening a project must not save application defaults"
        );
        assert_eq!(restored.project(), project);
        assert_eq!(restored.pivot_for(0), [-0.25, 0.75]);
        assert_eq!(restored.pivot_for(1), [0.1, 0.9]);
        assert!(!restored.edit_content);
        assert_eq!(restored.references.first().unwrap().bytes, original_bytes);
        let mut broken = project.clone();
        broken.output_resolution.width = 3000;
        broken.references.last_mut().unwrap().image_base64 = STANDARD.encode(b"not an image");
        assert!(restored.install_project(&ctx, broken).is_err());
        assert_eq!(restored.project(), project);
    }
}

#[cfg(test)]
mod confirmation_tests {
    use super::*;
    use crate::gui::tests::{frame, mouse};
    fn click_text(app: &mut Editor, ctx: &egui::Context, text: &str) {
        frame(app, ctx, vec![]);
        let output = frame(app, ctx, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == text => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            })
            .expect("confirmation action rendered");
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(app, ctx, vec![mouse(pos, false)]);
    }
    #[test]
    fn open_and_overwrite_confirmations_preserve_work_on_cancel() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let original = app.project();
        let mut next = original.clone();
        next.output_resolution.width = 2560;
        next.output_resolution.height = 1440;
        let source_dir = tempfile::tempdir().unwrap();
        let source = source_dir.path().join("candidate.midra.json");
        std::fs::write(&source, next.to_json().unwrap()).unwrap();
        app.files.opening = Some(source.clone());
        click_text(&mut app, &ctx, "Cancel");
        assert_eq!(app.project(), original);
        assert!(app.files.opening.is_none());
        app.files.opening = Some(source);
        click_text(&mut app, &ctx, "Open project");
        app.wait_project_job();
        assert_eq!(app.project(), next);
        app.files.message = None;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.json");
        std::fs::write(&path, b"old contents").unwrap();
        app.queue_write(path.clone(), "new contents");
        assert!(app.files.writing.is_some());
        click_text(&mut app, &ctx, "Cancel");
        assert_eq!(std::fs::read(&path).unwrap(), b"old contents");
        app.queue_write(path.clone(), "new contents");
        click_text(&mut app, &ctx, "Replace file");
        assert_eq!(std::fs::read(&path).unwrap(), b"new contents");
    }
}

#[cfg(test)]
mod async_tests {
    use super::*;
    #[test]
    fn loading_is_atomic_cancellable_and_pauses_recovery() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let before = app.project();
        let dir = tempfile::tempdir().unwrap();
        let recovery = dir.path().join("recovery.json");
        std::fs::write(&recovery, b"keep this recovery").unwrap();
        app.recovery.path = Some(recovery.clone());
        let (release, wait) = std::sync::mpsc::channel();
        let next = before.clone();
        app.start_project_job(&ctx, false, move || {
            wait.recv().unwrap();
            Ok(Some((next, None, None)))
        });
        app.poll_project_job();
        assert!(app.files.loading.is_some());
        assert_eq!(app.project(), before);
        app.autosave_recovery(true);
        assert_eq!(std::fs::read(&recovery).unwrap(), b"keep this recovery");
        app.files.loading = None; // Cancel loading drops the only receiver.
        release.send(()).unwrap();
        app.poll_project_job();
        assert_eq!(app.project(), before);

        let mut broken = before.clone();
        broken.references.push(SavedReference {
            name: "Broken".into(),
            image_base64: STANDARD.encode(b"invalid PNG"),
            visible: true,
            opacity: 1.0,
            preserve_ratio: false,
            native_size: false,
            offset: [0.; 2],
        });
        app.start_project_job(&ctx, false, move || Ok(Some((broken, None, None))));
        app.wait_project_job();
        assert_eq!(app.project(), before);
        assert!(
            app.files
                .message
                .as_ref()
                .unwrap()
                .contains("current work kept")
        );

        let mut next = before;
        next.output_resolution.width = 3840;
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 255, 0, 255]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        for name in ["First", "Second"] {
            next.references.push(SavedReference {
                name: name.into(),
                image_base64: STANDARD.encode(bytes.get_ref()),
                visible: name == "First",
                opacity: 0.5,
                preserve_ratio: true,
                native_size: false,
                offset: [10., 20.],
            });
        }
        let expected = next.clone();
        app.start_project_job(&ctx, false, move || {
            Ok(Some((next, Some(PathBuf::from("new.json")), Some(42))))
        });
        app.wait_project_job();
        assert_eq!(app.project(), expected);
        assert_eq!(app.files.project_path, Some(PathBuf::from("new.json")));
        assert_eq!(app.files.manual_saved_at, Some(42));
    }
}
