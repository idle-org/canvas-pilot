use super::*;
#[path = "capture_ui.rs"]
mod capture_ui;
#[path = "device_ui.rs"]
mod device_ui;
#[path = "document_reference_ui.rs"]
mod document_reference_ui;
#[path = "format_ui.rs"]
mod format_ui;
#[path = "inputs_window.rs"]
mod inputs_window;
#[path = "layers_ui.rs"]
mod layers_ui;
#[path = "menu_ui.rs"]
mod menu_ui;
#[path = "pack_browser_ui.rs"]
mod pack_browser_ui;
#[path = "preferences_ui.rs"]
mod preferences_ui;
#[path = "project_ui.rs"]
mod project_ui;
#[path = "ratio_ui.rs"]
mod ratio_ui;
#[path = "recovery_ui.rs"]
mod recovery_ui;
#[path = "reference_ui.rs"]
mod reference_ui;
#[path = "selection_outline.rs"]
mod selection_outline;
#[path = "size_presets_ui.rs"]
mod size_presets_ui;
#[path = "viewport_guides.rs"]
mod viewport_guides;
use canvas_pilot::import::{DetectedShape, ImportDocument, parse_values};
use canvas_pilot::{
    AlignTarget, Alignment, Corner, Geometry, Layout, PositionPoint, PositionReference,
    ResizeHandle, ResizeMode,
};
use format_ui::{FormatDraft, OutputDraft, PropertyTab};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum NavigationTool {
    #[default]
    Select,
    Move,
    Pan,
    Zoom,
}

pub struct Editor {
    device: device_ui::DeviceUi,
    inputs: inputs_window::InputsWindow,
    capture: capture_ui::CaptureUi,
    layer_ui: layers_ui::LayerUi,
    settings: settings::Store,
    ui_scale: f32,
    text_scale: Option<f32>,
    language: localization::Language,
    language_preview: Option<preferences_ui::LanguagePreview>,
    language_pack_action: Option<preferences_ui::PackAction>,
    recovery: recovery_ui::Recovery,
    themes_open: bool,
    preferences_open: bool,
    snapping: canvas_pilot::snapping::Snapping,
    snap_guides: [Option<f64>; 2],
    preferences_section: preferences_ui::Section,
    layout: Layout,
    files: project_ui::FileUi,
    corner: PositionPoint,
    position_layer_point: Option<PositionPoint>,
    fields: [String; 4],
    field_rects: [Rect; 4],
    fields_edited: bool,
    error: String,
    paste: Option<Paste>,
    zoom: f32,
    camera_one_to_one: bool,
    pan: Vec2,
    panning: bool,
    navigation_tool: NavigationTool,
    camera_before: Option<(f32, Vec2)>,
    zoom_drag: Option<(Pos2, Vec2, f32)>,
    marquee: Option<Marquee>,
    align_target: AlignTarget,
    layer_order: Vec<usize>,
    keep_aspect_ratio: bool,
    ratio_edit: ratio_ui::RatioEdit,
    ratio_layer: Option<usize>,
    resize_mode: ResizeMode,
    size_tiles: [u16; 2],
    size_keep_proportions: bool,
    crop_preset_keep_existing: bool,
    grid_tool: Option<u32>,
    pivot_tool: bool,
    pivot_drag: Option<(usize, Layout)>,
    format_draft: Option<FormatDraft>,
    property_tab: PropertyTab,
    output_draft: Option<OutputDraft>,
    edit_content: bool,
    edit_mask: bool,
    alt_mask: bool,
    crop_mask_tab_held: bool,
    crop_mask_enter_held: bool,
    crop_tab_editing: bool,
    show_pattern: bool,
    show_toolbar_icon_names: bool,
    references: Vec<super::reference::Reference>,
    reference_selected: Option<usize>,
    reference_pending: std::collections::VecDeque<
        std::sync::mpsc::Receiver<Result<super::reference::Reference, String>>,
    >,
    document_reference: document_reference_ui::DocumentReferenceUi,
    outline_layers: bool,
    backdrop_title: String,
    reference_open: bool,
    software_integrations_open: bool,
    reference_error: String,
}
#[derive(Clone)]
struct Marquee {
    start: [f64; 2],
    end: [f64; 2],
    base: Vec<bool>,
    previous: Vec<bool>,
}
#[derive(Clone, PartialEq)]
enum ImportScope {
    All,
    Selected,
    Folder(String),
}
impl ImportScope {
    fn label(&self, ctx: &egui::Context) -> String {
        match self {
            Self::All => tr(ctx, "ui.all-layers"),
            Self::Selected => tr(ctx, "ui.selected-layers"),
            Self::Folder(name) => rust_i18n::t!(
                "scope.folder",
                locale = &localization::locale(ctx),
                name = name
            )
            .into_owned(),
        }
    }
}
struct Paste {
    clipboard_entry: bool,
    scope: ImportScope,
    pages: Vec<canvas_pilot::interchange::Page>,
    page_index: usize,
    import_job:
        Option<std::sync::mpsc::Receiver<Result<Vec<canvas_pilot::interchange::Page>, String>>>,
    targets: Vec<usize>,
    target_ids: Vec<usize>,
    available_ids: Vec<usize>,
    text: String,
    shapes: Vec<DetectedShape>,
    mapping: Vec<Option<usize>>,
    ignored: Vec<bool>,
    note: String,
    include_size: bool,
    include_position: bool,
    include_zoom: bool,
    include_crop_fit: bool,
    include_masks: bool,
    include_names: bool,
    include_outline: bool,
    closing: bool,
    error: String,
}
impl Paste {
    fn candidates(&self, layout: &Layout) -> Vec<usize> {
        let indices = if self.scope == ImportScope::All {
            (0..layout.layers().len()).collect()
        } else {
            self.targets.clone()
        };
        indices
            .into_iter()
            .filter(|&i| i < layout.layers().len() && layout.visible(i) && !layout.locked(i))
            .collect()
    }
    fn set_scope(&mut self, scope: ImportScope, layout: &Layout) {
        self.targets = match &scope {
            ImportScope::All => Vec::new(),
            ImportScope::Selected => layout.selected_indices(),
            ImportScope::Folder(folder) => layout.folder_members(folder),
        };
        self.target_ids = self
            .targets
            .iter()
            .map(|&i| layout.layer_info(i).id)
            .collect();
        self.scope = scope;
        // Keep valid explicit choices; only clear mappings outside the new scope.
        let candidates = self.candidates(layout);
        for target in &mut self.mapping {
            if target.is_some_and(|i| !candidates.contains(&i)) {
                *target = None;
            }
        }
    }
    fn set_document(&mut self, doc: ImportDocument) {
        self.shapes = doc.shapes;
        self.note = doc.note;
        self.ignored = self
            .shapes
            .iter()
            .map(|shape| shape.background_only)
            .collect();
        self.reset_mapping();
        self.error.clear();
    }
    fn reset_mapping(&mut self) {
        let mut targets = self.targets.iter();
        self.mapping = self
            .ignored
            .iter()
            .map(|ignored| {
                if *ignored {
                    None
                } else {
                    targets.next().copied()
                }
            })
            .collect();
    }
    fn parse(&mut self) {
        self.import_job = None;
        self.pages.clear();
        match parse_values(&self.text) {
            Ok(doc) => self.set_document(doc),
            Err(e) => {
                self.shapes.clear();
                self.mapping.clear();
                self.ignored.clear();
                self.error = e;
            }
        }
    }
    fn names_available(&self) -> bool {
        self.shapes
            .iter()
            .zip(&self.ignored)
            .any(|(s, ignored)| !ignored && s.name.is_some())
    }
    fn appearance_available(&self) -> bool {
        self.shapes
            .iter()
            .zip(&self.ignored)
            .any(|(s, ignored)| !ignored && s.appearance.is_some())
    }
    fn zoom_available(&self) -> bool {
        self.shapes
            .iter()
            .zip(&self.ignored)
            .any(|(s, ignored)| !ignored && s.zoom.is_some())
    }
    fn geometry(&self, shape: &DetectedShape, before: Geometry) -> Result<Geometry, String> {
        let mut g = shape.paste_geometry(before, self.include_size, self.include_outline)?;
        if !self.include_position {
            g.x = before.x;
            g.y = before.y;
        }
        Ok(g)
    }
    fn apply(&self, layout: &mut Layout) -> Result<(), String> {
        let values = self.values(layout)?;
        let zooms: Vec<_> = if self.include_zoom {
            self.shapes
                .iter()
                .enumerate()
                .filter_map(|(row, s)| {
                    if self.ignored[row] {
                        None
                    } else {
                        s.zoom.map(|zoom| (self.mapping[row].unwrap(), zoom))
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        let appearances: Vec<_> = self
            .shapes
            .iter()
            .enumerate()
            .filter_map(|(row, shape)| {
                if self.ignored[row] {
                    None
                } else {
                    shape.appearance.map(|a| (self.mapping[row].unwrap(), a))
                }
            })
            .collect();
        let names: Vec<_> = self
            .shapes
            .iter()
            .enumerate()
            .filter_map(|(row, s)| {
                if !self.include_names || self.ignored[row] {
                    None
                } else {
                    s.name
                        .clone()
                        .map(|name| (self.mapping[row].unwrap(), name))
                }
            })
            .collect();
        layout.apply_mapped_clipboard_with_names(
            &values,
            &zooms,
            &appearances,
            self.include_crop_fit,
            self.include_masks,
            &names,
        )
    }
    fn destination_count_warning(&self, layout: &Layout) -> Option<String> {
        let count = self.ignored.iter().filter(|ignored| !**ignored).count();
        let available = self.candidates(layout).len();
        (count > available).then(|| format!(
            "Not enough destinations: {available} available, {count} detected ({} excluded). Widen the scope or exclude items.",
            self.shapes.len() - count
        ))
    }
    fn values(&self, layout: &Layout) -> Result<Vec<(usize, Geometry)>, String> {
        if !self.include_size
            && !self.include_position
            && !(self.include_names && self.names_available())
            && !(self.include_zoom && self.zoom_available())
            && !((self.include_crop_fit || self.include_masks) && self.appearance_available())
        {
            return Err(
                "Choose at least one available geometry, appearance or name value to paste.".into(),
            );
        }
        let count = self.ignored.iter().filter(|v| !**v).count();
        if count == 0 {
            return Err("No rectangles to import. Include at least one rectangle.".into());
        }
        if let Some(error) = self.destination_count_warning(layout) {
            return Err(error);
        }
        if self
            .targets
            .iter()
            .zip(&self.target_ids)
            .any(|(&i, &id)| i >= layout.layers().len() || layout.layer_info(i).id != id)
        {
            return Err("Destination layers changed. Close this review and import again.".into());
        }
        let mut values = Vec::new();
        for (row, shape) in self.shapes.iter().enumerate() {
            if self.ignored[row] {
                continue;
            }
            let target =
                self.mapping[row].ok_or("Choose a destination for each detected rectangle.")?;
            if target >= layout.layers().len()
                || self.available_ids.get(target).copied() != Some(layout.layer_info(target).id)
                || !layout.visible(target)
                || layout.locked(target)
            {
                return Err(
                    "Destination unavailable. Choose a visible, unlocked layer from this review."
                        .into(),
                );
            }
            if (self.scope != ImportScope::All && !self.targets.contains(&target))
                || values.iter().any(|(i, _)| *i == target)
            {
                return Err("Each imported rectangle needs a different destination within the chosen scope; choose a different destination for duplicate mappings.".into());
            }
            values.push((target, self.geometry(shape, layout.layers()[target])?));
        }
        Ok(values)
    }
}
impl Default for Editor {
    fn default() -> Self {
        let mut this = Self {
            device: Default::default(),
            inputs: Default::default(),
            capture: Default::default(),
            layer_ui: Default::default(),
            ui_scale: 1.0,
            text_scale: None,
            language: localization::Language::default(),
            language_preview: None,
            language_pack_action: None,
            settings: settings::Store::default(),
            recovery: recovery_ui::Recovery::default(),
            themes_open: false,
            preferences_open: false,
            snapping: Default::default(),
            snap_guides: [None; 2],
            preferences_section: preferences_ui::Section::default(),
            layout: Layout::default(),
            files: project_ui::FileUi::default(),
            corner: PositionPoint::TopRight,
            position_layer_point: None,
            fields: Default::default(),
            field_rects: [Rect::NOTHING; 4],
            fields_edited: false,
            error: String::new(),
            paste: None,
            zoom: 1.0,
            camera_one_to_one: false,
            pan: Vec2::ZERO,
            panning: false,
            navigation_tool: NavigationTool::Select,
            camera_before: None,
            zoom_drag: None,
            marquee: None,
            align_target: AlignTarget::Selection,
            layer_order: Vec::new(),
            keep_aspect_ratio: true,
            ratio_edit: Default::default(),
            ratio_layer: None,
            resize_mode: ResizeMode::Individual,
            size_tiles: [2, 2],
            size_keep_proportions: false,
            crop_preset_keep_existing: false,
            grid_tool: None,
            pivot_tool: false,
            pivot_drag: None,
            format_draft: None,
            property_tab: PropertyTab::default(),
            output_draft: None,
            edit_content: false,
            edit_mask: false,
            alt_mask: false,
            crop_mask_tab_held: false,
            crop_mask_enter_held: false,
            crop_tab_editing: false,
            show_pattern: false,
            show_toolbar_icon_names: false,
            references: Vec::new(),
            reference_selected: None,
            reference_pending: Default::default(),
            document_reference: Default::default(),
            outline_layers: false,
            backdrop_title: canvas_pilot::project::DEFAULT_BACKDROP_TITLE.into(),
            reference_open: false,
            software_integrations_open: false,
            reference_error: String::new(),
        };
        this.sync();
        this
    }
}
fn number(n: f64) -> String {
    canvas_pilot::view_pixels(n)
}
fn bounds(g: Geometry) -> String {
    format!(
        "X {}  ·  Y {}\n{} × {} px",
        number(g.x),
        number(g.y),
        number(g.width),
        number(g.height)
    )
}
impl Editor {
    fn mask_editing(&self) -> bool {
        self.edit_mask || (self.edit_content && self.alt_mask)
    }

    // Handle canvas Enter/Tab before egui focus navigation, leaving text fields alone.
    // Held-key latches are needed: egui-winit leaves repeat detection to egui.
    fn crop_mask_key_input(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        if !input.focused {
            self.crop_mask_tab_held = false;
            self.crop_mask_enter_held = false;
            return;
        }
        let eligible = !self.inputs.open
            && self.navigation_tool == NavigationTool::Select
            && !self.layout.dragging()
            && !self.panning
            && self.zoom_drag.is_none()
            && self.pivot_drag.is_none()
            && self.marquee.is_none()
            && !ctx.input(|i| i.pointer.any_down())
            && !input
                .events
                .iter()
                .any(|e| matches!(e, egui::Event::PointerButton { pressed: true, .. }))
            && !ctx.egui_wants_keyboard_input()
            && ctx.memory(|m| m.focused().is_none())
            && !ctx.any_popup_open()
            && self.output_draft.is_none()
            && self.paste.is_none()
            && !self.preferences_open
            && !self.themes_open
            && !self.reference_open
            && !self.layer_ui.open
            && self.language_preview.is_none()
            && self.language_pack_action.is_none()
            && self.files.loading.is_none()
            && !self.files.new_requested
            && !self.files.new_after_save
            && !self.files.new_ready
            && !(self.fields_edited && self.fields_pending())
            && self
                .layout
                .selected()
                .is_some_and(|i| !self.layout.locked(i) && self.layout.visible(i));
        let tab_eligible = eligible
            && self.edit_content
            && self.format_draft.is_none()
            && self.layout.selected().is_some_and(|i| {
                self.layout.formats().layer_fit[i]
                    .is_none_or(|fit| fit == canvas_pilot::formats::LayerFit::OneToOne)
            });
        let enter_eligible = eligible
            && !self.device.open
            && !self.device.connection_open
            && !self.inputs.open
            && (self.format_draft.is_none()
                || matches!(self.property_tab, PropertyTab::Crop | PropertyTab::Mask));
        input.events.retain(|event| {
            if let egui::Event::Key {
                key: egui::Key::Enter,
                pressed,
                modifiers,
                ..
            } = event
            {
                let was_held = self.crop_mask_enter_held;
                self.crop_mask_enter_held = *pressed;
                if enter_eligible && modifiers.is_none() {
                    if *pressed && !was_held {
                        if self.edit_content {
                            self.finish_crop_edit();
                        } else {
                            self.toggle_crop_edit();
                        }
                    }
                    return false;
                }
            }
            if let egui::Event::Key {
                key: egui::Key::Tab,
                pressed,
                modifiers,
                ..
            } = event
            {
                let was_held = self.crop_mask_tab_held;
                self.crop_mask_tab_held = *pressed;
                if tab_eligible && modifiers.is_none() {
                    if *pressed && !was_held {
                        self.edit_mask = !self.edit_mask;
                        self.alt_mask = false;
                    }
                    return false;
                }
            }
            true
        });
    }

    fn update_alt_mask(&mut self, ctx: &egui::Context) {
        // Keep the active drag's interpretation stable if Alt changes mid-gesture.
        if !self.layout.dragging() {
            self.alt_mask = self.edit_content
                && !self.edit_mask
                && self.navigation_tool == NavigationTool::Select
                && !ctx.egui_wants_keyboard_input()
                && self
                    .layout
                    .selected()
                    .is_some_and(|i| !self.layout.locked(i) && self.layout.visible(i))
                && ctx.input(|i| {
                    i.focused && i.modifiers.alt && !i.modifiers.ctrl && !i.modifiers.command
                });
        }
    }

    fn open_mask_edit(&mut self, i: usize) {
        self.property_tab = PropertyTab::Mask;
        if self
            .format_draft
            .as_ref()
            .is_none_or(|draft| !draft.is_for(i))
        {
            self.format_draft = Some(FormatDraft::new(&self.layout, i));
        }
        self.edit_mask = true;
        self.edit_content = true;
    }

    fn toggle_crop_edit(&mut self) {
        self.edit_mask = false;
        self.layout.finish_drag();
        self.edit_content = !self.edit_content;
    }

    pub(super) fn with_saved_settings(ctx: &egui::Context) -> Self {
        let (store, saved) = settings::Store::open();
        let mut editor = Self::with_settings(ctx, store, saved);
        let startup_error = std::mem::take(&mut editor.error);
        if let Some(path) = editor
            .settings
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.join("last_project.json"))
        {
            editor.restore_recovery(ctx, path);
        }
        if !startup_error.is_empty() {
            if !editor.error.is_empty() {
                editor.error.push('\n');
            }
            editor.error.push_str(&startup_error);
        }
        editor.device.try_startup_connection(ctx);
        editor
    }
    fn with_settings(
        ctx: &egui::Context,
        store: settings::Store,
        saved: settings::Settings,
    ) -> Self {
        let mut theme_errors = store
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|directory| crate::theme_packs::load(ctx, &directory.join("theme-packs")))
            .unwrap_or_default();
        theme::Theme::restore_recent(ctx, saved.recent_themes.clone());
        saved.theme.apply(ctx);
        let pack_errors = store
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|directory| crate::language_packs::load(ctx, &directory.join("language-packs")))
            .unwrap_or_default();
        theme_errors.extend(pack_errors);
        saved.language.apply(ctx);
        preferences_ui::apply_scaling(ctx, saved.ui_scale, saved.text_scale);
        let mut device = device_ui::DeviceUi::with_connection_preferences(
            saved.midra_host,
            saved.allow_sending_to_program,
        );
        device.autoconnect_at_startup = saved.autoconnect_at_startup;
        device.recent_hosts = saved.recent_midra_hosts;
        Self {
            device,
            settings: store,
            error: theme_errors.join("\n"),
            language: saved.language,
            ui_scale: saved.ui_scale,
            text_scale: saved.text_scale,
            snapping: saved.snapping,
            keep_aspect_ratio: saved.keep_aspect_ratio,
            show_pattern: saved.show_source_pattern,
            show_toolbar_icon_names: saved.show_toolbar_icon_names,
            ..Self::default()
        }
    }
    fn application_settings(&self, ctx: &egui::Context) -> settings::Settings {
        settings::Settings {
            ui_scale: self.ui_scale,
            text_scale: self.text_scale,
            language: self
                .language_preview
                .as_ref()
                .map_or_else(|| self.language.clone(), |preview| preview.previous.clone()),
            midra_host: self.device.host.clone(),
            recent_midra_hosts: self.device.recent_hosts.clone(),
            autoconnect_at_startup: self.device.autoconnect_at_startup,
            allow_sending_to_program: self.device.allow_program,
            theme: theme::Theme::current(ctx),
            recent_themes: theme::Theme::recent(ctx),
            snapping: self.snapping,
            keep_aspect_ratio: self.keep_aspect_ratio,
            show_source_pattern: self.show_pattern,
            show_toolbar_icon_names: self.show_toolbar_icon_names,
            ..Default::default()
        }
    }
    fn load_reference(&mut self, ctx: &egui::Context, file: &egui::DroppedFile) {
        let path = file
            .path
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new(&file.name));
        if canvas_pilot::reference_document::is_document(path) {
            if let Err(error) = self.document_reference.enqueue(file.clone()) {
                self.reference_error = error;
            }
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        if self.references.len() + self.reference_pending.len() >= 32 {
            self.reference_error = "Up to 32 reference images can be stored.".into();
            return;
        }
        self.reference_pending.push_back(rx);
        self.reference_error.clear();
        let ctx = ctx.clone();
        let file = file.clone();
        std::thread::spawn(move || {
            let result = super::reference::Reference::load(&ctx, &file);
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
    fn poll_reference(&mut self) {
        let Some(rx) = self.reference_pending.front() else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(_) => Err("Reference reader stopped. Please import again.".into()),
        };
        self.reference_pending.pop_front();
        match result {
            Ok(reference) => self.add_reference(reference),
            Err(error) => self.reference_error = error,
        }
    }
    fn add_reference(&mut self, reference: super::reference::Reference) {
        if self.references.len() >= 32 {
            self.reference_error = "Up to 32 reference images can be stored.".into();
        } else if self.references.iter().map(|r| r.bytes.len()).sum::<usize>()
            + reference.bytes.len()
            > 64 * 1024 * 1024
        {
            self.reference_error = "Reference images exceed the combined 64 MiB limit.".into();
        } else {
            self.references.push(reference);
            self.reference_selected = Some(self.references.len() - 1);
            self.reference_error.clear();
        }
    }
    #[cfg(test)]
    fn wait_reference(&mut self) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !self.reference_pending.is_empty() && std::time::Instant::now() < until {
            self.poll_reference();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(self.reference_pending.is_empty());
    }
    fn import_reference_file(&mut self, ctx: &egui::Context) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter(
                "Pictures and documents",
                &canvas_pilot::reference_document::extensions(),
            )
            .pick_files()
        {
            for path in paths {
                self.load_reference(
                    ctx,
                    &egui::DroppedFile {
                        path: Some(path),
                        ..Default::default()
                    },
                );
            }
        }
    }
    fn reference_window(&mut self, ctx: &egui::Context) {
        let palette = theme::palette(ctx);
        let mut open = self.reference_open;
        egui::Window::new(tr(ctx, "ui.reference-image"))
            .id(egui::Id::new("Reference image"))
            .open(&mut open)
            .default_width(420.0)
            .show(ctx, |ui| {
                if ui.button(tr(ui.ctx(), "ui.import-picture")).clicked() {
                    self.import_reference_file(ctx);
                }
                ui.label("Add pictures, PDF pages or supported documents as references. Drop geometry files here while this window is open to import them as pictures.");
                if !self.reference_pending.is_empty() {
                    ui.spinner();
                    ui.label(tr(
                        ui.ctx(),
                        "ui.loading-image-current-reference-remains-available",
                    ));
                    if ui.button(tr(ui.ctx(), "ui.cancel-image-import")).clicked() {
                        self.reference_pending.clear();
                    }
                }
                if !self.references.is_empty() {
                    let mut index = self.reference_selected.unwrap_or(self.references.len() - 1);
                    egui::ComboBox::from_id_salt("reference-settings-choice")
                        .selected_text(&self.references[index].name)
                        .show_ui(ui, |ui| {
                            for (i, reference) in self.references.iter().enumerate() {
                                ui.stable_selectable_value(&mut index, i, &reference.name);
                            }
                        });
                    self.reference_selected = Some(index);
                }
                ui.checkbox(
                    &mut self.outline_layers,
                    tr(ui.ctx(), "ui.outline-only-layers"),
                );
                let mut remove = false;
                if let Some(reference) = self
                    .reference_selected
                    .and_then(|i| self.references.get_mut(i))
                {
                    ui.label(format!(
                        "{} · {} × {} px",
                        reference.name, reference.size[0], reference.size[1]
                    ));
                    ui.checkbox(&mut reference.visible, tr(ui.ctx(), "ui.show-reference"));
                    ui.add(
                        egui::Slider::new(&mut reference.opacity, 0.0..=1.0)
                            .text(tr(ui.ctx(), "ui.opacity")),
                    );
                    ui.checkbox(
                        &mut reference.native_size,
                        tr(ui.ctx(), "ui.native-pixels-1-image-pixel-1-screen-pixel"),
                    );
                    ui.horizontal(|ui| {
                        ui.label(tr(ui.ctx(), "ui.offset-in-screen-pixels"));
                        ui.add(
                            egui::DragValue::new(&mut reference.offset[0])
                                .prefix(tr(ui.ctx(), "ui.x"))
                                .range(-1_000_000.0..=1_000_000.0)
                                .max_decimals(1),
                        );
                        ui.add(
                            egui::DragValue::new(&mut reference.offset[1])
                                .prefix(tr(ui.ctx(), "ui.y"))
                                .range(-1_000_000.0..=1_000_000.0)
                                .max_decimals(1),
                        );
                    });
                    if ui
                        .add_enabled(
                            reference.offset != [0.0; 2],
                            egui::Button::new(tr(ui.ctx(), "ui.reset-image-offset")),
                        )
                        .clicked()
                    {
                        reference.offset = [0.0; 2];
                    }
                    ui.radio_value(
                        &mut reference.preserve_ratio,
                        false,
                        tr(ui.ctx(), "ui.fill-screen-stretch-if-needed"),
                    );
                    ui.radio_value(
                        &mut reference.preserve_ratio,
                        true,
                        tr(ui.ctx(), "ui.fit-inside-preserve-proportions"),
                    );
                    if let Some((severe, warning)) = reference.warning(self.layout.formats().output)
                    {
                        ui.colored_label(
                            if severe {
                                ui.visuals().error_fg_color
                            } else {
                                palette.amber
                            },
                            warning,
                        );
                    }
                    remove = ui.button(tr(ui.ctx(), "ui.remove-reference")).clicked();
                }
                if remove && let Some(i) = self.reference_selected {
                    self.remove_reference(i);
                }
                if !self.reference_error.is_empty() {
                    ui.colored_label(ui.visuals().error_fg_color, &self.reference_error);
                }
                ui.label(tr(
                    ui.ctx(),
                    "ui.reference-stays-fixed-behind-layers-warnings-never-block-placement",
                ));
            });
        self.reference_open = open;
    }
    fn position_reference(&self) -> PositionReference {
        PositionReference {
            screen: self.corner,
            layer: self.position_layer_point.unwrap_or(self.corner),
        }
    }
    fn sync(&mut self) {
        let ratio_layer = self.layout.selected().map(|i| self.layout.layer_info(i).id);
        if self.ratio_layer != ratio_layer {
            self.ratio_layer = ratio_layer;
            self.ratio_edit = Default::default();
        }
        let order: Vec<_> = (0..self.layout.layers().len())
            .map(|i| self.layout.layer_info(i).id)
            .collect();
        if order != self.layer_order {
            if let AlignTarget::Layer(index) = self.align_target {
                self.align_target = self
                    .layer_order
                    .get(index)
                    .and_then(|id| order.iter().position(|current| current == id))
                    .map(AlignTarget::Layer)
                    .unwrap_or(AlignTarget::Selection);
            }
            self.layer_ui.sync_order(&order);
            self.layer_order = order;
        }
        self.fields_edited = false;
        if let Some(i) = self.layout.selected() {
            let g = self.layout.layers()[i];
            let [x, y] = self
                .position_reference()
                .offsets_on(g, self.layout.formats().output);
            self.fields = [number(x), number(y), number(g.width), number(g.height)];
        }
        self.error = self.layout.gesture_error.clone().unwrap_or_default();
        if self
            .format_draft
            .as_ref()
            .is_some_and(|d| !d.pending(&self.layout))
        {
            self.format_draft = self
                .layout
                .selected()
                .map(|i| FormatDraft::new(&self.layout, i));
        }
    }
    fn request_layer_paste(ctx: &egui::Context) {
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
    }

    fn open_paste(&mut self, text: String) {
        let targets = self.layout.selected_indices();
        let mut p = Paste {
            clipboard_entry: true,
            scope: if targets.is_empty() {
                ImportScope::All
            } else {
                ImportScope::Selected
            },
            pages: Vec::new(),
            page_index: 0,
            import_job: None,
            available_ids: (0..self.layout.layers().len())
                .map(|i| self.layout.layer_info(i).id)
                .collect(),
            target_ids: targets
                .iter()
                .map(|i| self.layout.layer_info(*i).id)
                .collect(),
            targets,
            text,
            shapes: Vec::new(),
            mapping: Vec::new(),
            ignored: Vec::new(),
            note: String::new(),
            include_size: true,
            include_position: true,
            include_zoom: true,
            include_crop_fit: true,
            include_masks: true,
            include_names: false,
            include_outline: false,
            closing: false,
            error: String::new(),
        };
        if !p.text.is_empty() {
            p.parse();
        }
        self.paste = Some(p);
    }
    fn import_geometry_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Layer geometry",
                &canvas_pilot::software::geometry_extensions(),
            )
            .pick_file()
        {
            self.open_paste(String::new());
            if let Some(p) = self.paste.as_mut() {
                Self::read_geometry(&path, p);
            }
        }
    }
    fn read_geometry(path: &std::path::Path, p: &mut Paste) {
        Self::read_geometry_files(
            vec![egui::DroppedFile {
                path: Some(path.to_path_buf()),
                ..Default::default()
            }],
            p,
        );
    }
    fn geometry_drop(file: &egui::DroppedFile) -> bool {
        let path = file
            .path
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new(&file.name));
        path.extension()
            .and_then(|s| s.to_str())
            .is_some_and(|extension| {
                let extension = extension.to_ascii_lowercase();
                canvas_pilot::interchange::extensions().contains(&extension.as_str())
                    || matches!(
                        extension.as_str(),
                        "pptx" | "fig" | "figcraft" | "ai" | "dwg"
                    )
            })
    }
    fn read_geometry_files(files: Vec<egui::DroppedFile>, p: &mut Paste) {
        use std::io::Read;
        let (tx, rx) = std::sync::mpsc::channel();
        p.clipboard_entry = false;
        p.import_job = Some(rx);
        p.pages.clear();
        p.shapes.clear();
        p.mapping.clear();
        p.ignored.clear();
        p.text.clear();
        p.error.clear();
        std::thread::spawn(move || {
            let result = (|| {
                let mut pages = Vec::new();
                let mut remaining = canvas_pilot::exchange::MAX_EXCHANGE_BYTES;
                let multiple = files.len() > 1;
                for file in files {
                    let name = file
                        .path
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or(file.name);
                    let bytes = if let Some(bytes) = file.bytes {
                        if bytes.len() > remaining {
                            return Err("Import exceeds 16 MiB across dropped files.".into());
                        }
                        bytes.to_vec()
                    } else if let Some(path) = file.path {
                        let mut bytes = Vec::new();
                        std::fs::File::open(&path)
                            .map_err(|e| format!("Cannot open geometry file {name}: {e}"))?
                            .take(remaining as u64 + 1)
                            .read_to_end(&mut bytes)
                            .map_err(|e| format!("Cannot read geometry file {name}: {e}"))?;
                        bytes
                    } else {
                        return Err("The dropped geometry file has no path or data.".into());
                    };
                    if bytes.len() > remaining {
                        return Err("Import exceeds 16 MiB across dropped files.".into());
                    }
                    remaining -= bytes.len();
                    let mut imported = canvas_pilot::interchange::import(&bytes, &name)
                        .map_err(|e| format!("{name}: {e}"))?;
                    if multiple {
                        let label = std::path::Path::new(&name)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy();
                        for page in &mut imported {
                            page.name = format!("{label} · {}", page.name);
                        }
                    }
                    pages.extend(imported);
                }
                Ok(pages)
            })();
            let _ = tx.send(result);
        });
    }
    fn fields_pending(&self) -> bool {
        let Some(i) = self.layout.selected() else {
            return false;
        };
        let g = self.layout.layers()[i];
        let [x, y] = self
            .position_reference()
            .offsets_on(g, self.layout.formats().output);
        self.fields
            .iter()
            .zip([x, y, g.width, g.height])
            .any(|(text, value)| {
                text.trim().parse::<f64>().map_or(true, |parsed| {
                    parsed != value && parsed != number(value).parse::<f64>().unwrap()
                })
            })
    }
    fn apply_fields(&mut self) {
        let values: Result<Vec<f64>, _> = self
            .fields
            .iter()
            .map(|s| s.trim().parse::<f64>())
            .collect();
        let result = values
            .map_err(|_| "Enter a number in each field.".to_string())
            .and_then(|mut v| {
                if let Some(i) = self.layout.selected() {
                    let g = self.layout.layers()[i];
                    let [x, y] = self
                        .position_reference()
                        .offsets_on(g, self.layout.formats().output);
                    let original = [x, y, g.width, g.height];
                    for j in 0..4 {
                        if v[j] == number(original[j]).parse::<f64>().unwrap() {
                            v[j] = original[j];
                        }
                    }
                    // Linked numeric fields are displayed rounded, but keep the
                    // original aspect when only one dimension drives the edit.
                    if self.keep_aspect_ratio {
                        if v[2] != g.width && self.fields[3] == number(v[2] * g.height / g.width) {
                            v[3] = v[2] * g.height / g.width;
                        } else if v[3] != g.height
                            && self.fields[2] == number(v[3] * g.width / g.height)
                        {
                            v[2] = v[3] * g.width / g.height;
                        }
                    }
                }
                self.position_reference()
                    .geometry_on(v[0], v[1], v[2], v[3], self.layout.formats().output)
                    .map(|mut geometry| {
                        if let Some(i) = self.layout.selected() {
                            let old = self.layout.layers()[i];
                            let offsets = self
                                .position_reference()
                                .offsets_on(old, self.layout.formats().output);
                            if v[0] == offsets[0] && geometry.width == old.width {
                                geometry.x = old.x;
                            }
                            if v[1] == offsets[1] && geometry.height == old.height {
                                geometry.y = old.y;
                            }
                        }
                        geometry
                    })
            });
        match result {
            Ok(g) => {
                if let Some(i) = self.layout.selected() {
                    match self.layout.apply(i, g) {
                        Ok(()) => self.sync(),
                        Err(e) => self.error = e,
                    }
                }
            }
            Err(e) => self.error = e,
        }
    }
    fn copy(&mut self, ctx: &egui::Context) {
        let entries: Vec<_> = self
            .layout
            .selected_indices()
            .into_iter()
            .map(|i| {
                let g = self.layout.layers()[i];
                let [h, v] = g.center();
                if let Some(mode) = self.layout.formats().layer_fit[i] {
                    return format!("{} — container ({})\nHorizontal: {} px\nVertical: {} px\nWidth: {} px\nHeight: {} px",
                        self.layout.layer_label(i), mode.label(), canvas_pilot::export_pixels(h), canvas_pilot::export_pixels(v),
                        canvas_pilot::export_pixels(g.width), canvas_pilot::export_pixels(g.height));
                }
                format!(
                    "{} — visible box (device container unverified)\nHorizontal: {} px\nVertical: {} px\nWidth: {} px\nHeight: {} px\nCrop zoom: {} %",
                    self.layout.layer_label(i),
                    canvas_pilot::export_pixels(h),
                    canvas_pilot::export_pixels(v),
                    canvas_pilot::export_pixels(g.width),
                    canvas_pilot::export_pixels(g.height),
                    number(self.layout.formats().mapping(i, g).unwrap().framing.zoom * 100.0)
                )
            })
            .collect();
        if !entries.is_empty() {
            match canvas_pilot::clipboard::encode(
                &self.layout,
                &self.layout.selected_indices(),
                &entries.join("\n\n"),
            ) {
                Ok(text) => ctx.copy_text(text),
                Err(error) => self.error = error,
            }
        }
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma)) {
            self.preferences_open = true;
            return;
        }
        if self.inputs.open
            || self.paste.is_some()
            || self.format_draft.is_some()
            || self.output_draft.is_some()
            || self.layer_ui.open
        {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.edit_content = false;
            self.layout.cancel_drag();
            if let Some((zoom, pan)) = self.camera_before.take() {
                self.zoom = zoom;
                self.pan = pan;
            }
            self.zoom_drag = None;
            self.panning = false;
            if let Some((_, before)) = self.pivot_drag.take() {
                self.layout = before;
            }
            if let Some(m) = self.marquee.take() {
                self.layout.set_selection(m.previous);
            }
            self.sync();
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
            return;
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let paste = ctx.input_mut(|i| {
            let mut pasted = None;
            i.events.retain(|e| {
                if let egui::Event::Paste(s) = e {
                    pasted = Some(s.clone());
                    false
                } else {
                    true
                }
            });
            pasted
        });
        if let Some(text) = paste {
            self.open_paste(text);
            return;
        }
        if self.layout.dragging()
            || self.marquee.is_some()
            || self.panning
            || self.zoom_drag.is_some()
            || self.pivot_drag.is_some()
        {
            return;
        }
        if ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy))) {
            self.copy(ctx);
            return;
        }
        let command = ctx.input(|i| i.modifiers.command);
        if command {
            if ctx.input(|i| i.key_pressed(egui::Key::Z)) {
                if ctx.input(|i| i.modifiers.shift) {
                    self.layout.redo();
                } else {
                    self.layout.undo();
                }
                self.sync();
            } else if ctx.input(|i| i.key_pressed(egui::Key::Y)) {
                self.layout.redo();
                self.sync();
            }
            return;
        }
        if ctx.input(|i| i.modifiers == egui::Modifiers::NONE) {
            for (key, tool) in [
                (egui::Key::V, NavigationTool::Select),
                (egui::Key::M, NavigationTool::Move),
                (egui::Key::H, NavigationTool::Pan),
                (egui::Key::Z, NavigationTool::Zoom),
            ] {
                if ctx.input(|i| i.key_pressed(key)) {
                    self.navigation_tool = tool;
                    return;
                }
            }
        }
        let step = if ctx.input(|i| i.modifiers.shift) {
            10.0
        } else {
            1.0
        };
        let mut d = [0.0, 0.0];
        for (key, axis, sign) in [
            (egui::Key::ArrowLeft, 0, -1.0),
            (egui::Key::ArrowRight, 0, 1.0),
            (egui::Key::ArrowUp, 1, -1.0),
            (egui::Key::ArrowDown, 1, 1.0),
        ] {
            if ctx.input(|i| i.key_pressed(key)) {
                d[axis] += sign * step;
            }
        }
        if d != [0.0, 0.0] {
            let result = if self.edit_content
                && let Some(i) = self.layout.selected()
            {
                if self.mask_editing() {
                    self.layout.formats().masks[i]
                        .translated(
                            d,
                            self.layout
                                .formats()
                                .mask_reference(i, self.layout.layers()[i])
                                .unwrap_or(self.layout.layers()[i]),
                        )
                        .and_then(|mask| self.layout.set_mask(i, mask))
                } else if self.layout.formats().layer_fit[i]
                    == Some(canvas_pilot::formats::LayerFit::OneToOne)
                {
                    self.layout.formats().advanced[i]
                        .pan_image(d)
                        .and_then(|crop| {
                            self.layout.set_layer_fit(
                                i,
                                Some(canvas_pilot::formats::LayerFit::OneToOne),
                                crop,
                            )
                        })
                } else {
                    let g = self.layout.layers()[i];
                    self.layout.formats().mapping(i, g).and_then(|m| {
                        let offset = std::array::from_fn(|axis| {
                            (m.framing.offset[axis] + d[axis])
                                .clamp(-m.offset_limits[axis], m.offset_limits[axis])
                        });
                        self.layout.set_framing(
                            i,
                            canvas_pilot::formats::Framing {
                                zoom: m.framing.zoom,
                                offset,
                            },
                        )
                    })
                }
            } else {
                self.layout.nudge(d[0], d[1])
            };
            match result {
                Ok(()) => self.sync(),
                Err(e) => self.error = e,
            }
        }
    }
    fn paste_window(&mut self, ctx: &egui::Context) {
        let palette = theme::palette(ctx);
        let Some(mut p) = self.paste.take() else {
            return;
        };
        if let Some(job) = p.import_job.take() {
            match job.try_recv() {
                Ok(Ok(pages)) => {
                    if let Some(first) = pages.first() {
                        p.set_document(first.document.clone());
                    }
                    p.pages = pages;
                    p.page_index = 0;
                }
                Ok(Err(error)) => p.error = error,
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    p.import_job = Some(job);
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                Err(_) => p.error = "The import worker stopped unexpectedly.".into(),
            }
        }
        let mut close = false;
        let response=egui::Modal::new(egui::Id::new("layer-values-import")).show(ctx,|ui| {
            ui.set_width(600.0);
            ui.heading(tr(ui.ctx(), "ui.import-layer-values"));
            if p.import_job.is_some() { ui.spinner(); ui.label(tr(ui.ctx(), "ui.reading-document")); }
            if p.pages.len() == 1 { ui.label(&p.pages[0].name); }
            if p.pages.len() > 1 {
                let previous = p.page_index;
                egui::ComboBox::new("File / page / frame / slide", tr(ui.ctx(), "ui.file-page-frame-slide")).selected_text(&p.pages[p.page_index].name).show_ui(ui, |ui| {
                    for (index, page) in p.pages.iter().enumerate() { ui.stable_selectable_value(&mut p.page_index, index, &page.name); }
                });
                if previous != p.page_index { p.set_document(p.pages[p.page_index].document.clone()); }
            }
            if p.clipboard_entry && p.shapes.is_empty() && p.import_job.is_none() {
                ui.label(tr(ui.ctx(), "ui.paste-layer-values-to-review-their-geometry-and-destinations"));
                ui.add(egui::TextEdit::multiline(&mut p.text).desired_rows(4).desired_width(f32::INFINITY));
                if ui.button(tr(ui.ctx(), "ui.review-pasted-values")).clicked() { p.parse(); }
            }
            egui::ScrollArea::vertical().id_salt("import-review").max_height((ctx.content_rect().height()-260.0).max(150.0)).show(ui,|ui| {
                if !p.error.is_empty() {ui.colored_label(palette.amber,&p.error);}
                if p.shapes.is_empty() {return;}
                ui.small(format!("{} detected items · review mappings before importing", p.shapes.len()));
                if !p.text.is_empty() {
                    egui::CollapsingHeader::new(tr(ui.ctx(), "ui.clipboard-data")).id_salt("Clipboard data").show(ui, |ui| {
                        egui::ScrollArea::vertical().id_salt("clipboard-preview").max_height(100.0).show(ui, |ui| {
                            ui.add(egui::TextEdit::multiline(&mut p.text).interactive(false).desired_width(f32::INFINITY));
                        });
                    });
                }
                ui.add_space(8.0);
                ui.strong("Destination layers");
                let mut scope = p.scope.clone();
                egui::ComboBox::from_id_salt("import-scope").width(280.0).selected_text(scope.label(ui.ctx())).show_ui(ui, |ui| {
                    ui.stable_selectable_value(&mut scope, ImportScope::All, tr(ui.ctx(), "ui.all-layers"));
                    ui.stable_selectable_value(&mut scope, ImportScope::Selected, tr(ui.ctx(), "ui.selected-layers"));
                    ui.separator();
                    for folder in self.layout.folders() {
                        ui.stable_selectable_value(&mut scope, ImportScope::Folder(folder.clone()), format!("Folder: {folder}"));
                    }
                });
                if scope != p.scope { p.set_scope(scope, &self.layout); }
                if let Some(warning) = p.destination_count_warning(&self.layout) {
                    ui.colored_label(palette.amber, warning);
                }
                ui.small(format!("{} available destinations · hidden and locked layers are excluded", p.candidates(&self.layout).len()));
                ui.small(tr(ui.ctx(), "ui.choose-a-destination-for-each-item-below-other-layers-stay-unchanged"));
                ui.add_space(8.0);
                ui.strong("Values to apply");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut p.include_size,tr(ui.ctx(), "ui.format-width-height"));
                    ui.checkbox(&mut p.include_position,tr(ui.ctx(), "ui.position"));
                    if p.zoom_available() { ui.checkbox(&mut p.include_zoom,tr(ui.ctx(), "ui.crop-zoom")); }
                });
                if p.names_available() {
                    ui.checkbox(&mut p.include_names, tr(ui.ctx(), "ui.rename-destination-layers")).on_hover_text(tr(ui.ctx(), "ui.use-supplied-source-names-unnamed-items-and-destination-folders-stay-unchanged"));
                }
                if p.appearance_available() {
                    let summary = match (p.include_crop_fit, p.include_masks) {
                        (true, true) => "Appearance · crop, fit and blanking",
                        (true, false) => "Appearance · crop and fit",
                        (false, true) => "Appearance · blanking",
                        (false, false) => "Appearance · keep destination",
                    };
                    egui::CollapsingHeader::new(summary).id_salt("paste-appearance").show(ui, |ui| {
                        ui.checkbox(&mut p.include_crop_fit, tr(ui.ctx(), "ui.crop-fit")).on_hover_text(tr(ui.ctx(), "ui.copy-exact-crop-edges-offsets-and-fit-mode-source-dimensions-after-input-crop-must-ma"));
                        ui.checkbox(&mut p.include_masks, tr(ui.ctx(), "ui.blanking")).on_hover_text(tr(ui.ctx(), "ui.copy-the-four-proportional-mask-edges-without-resizing-the-layer"));
                    });
                }
                ui.small(tr(ui.ctx(), "ui.unchecked-values-keep-the-destination-settings-format-changes-may-adjust-existing-cro"));
                if p.shapes.iter().any(|s|s.outline.iter().any(|v|*v>0.0)) {
                    ui.indent("outline-option", |ui| {
                        ui.add_enabled(p.include_size,egui::Checkbox::new(&mut p.include_outline,"Include outline in bounds"));
                    });
                }
                egui::CollapsingHeader::new(tr(ui.ctx(), "ui.import-details")).id_salt("Import details").show(ui, |ui| {
                    ui.label(&p.note);
                    ui.small(if p.appearance_available() { "Source assignments, device links and input settings stay unchanged. Crop edges adapt proportionally using the destination’s Advanced fit; the default is Centered (fit)." } else { "Source format, crop offsets, colors and outline appearance are not imported." });
                });
                if !p.include_size {ui.small(tr(ui.ctx(), "ui.outline-adjustment-requires-format-position-uses-the-source-top-left-when-enabled"));}
                ui.separator();
                ui.strong("Map source items to destination layers");
                ui.small(tr(ui.ctx(), "ui.each-included-item-needs-a-different-destination"));
                let mut reset_mapping=false;
                for row in 0..p.shapes.len() {
                    let shape=p.shapes[row].clone();
                    ui.push_id(row,|ui| {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.strong(format!("{} · {}",row+1,shape.id));
                        });
                        if shape.background_only {
                            ui.small(tr(ui.ctx(), "ui.background-artwork-excluded-from-layer-mapping"));
                        } else {
                            let mut include = !p.ignored[row];
                            if ui.checkbox(&mut include, if shape.background { "Include full-page background" } else { "Include item" }).changed() {
                                p.ignored[row] = !include;
                                reset_mapping = true;
                            }
                        }
                        if !p.ignored[row] {
                            ui.horizontal(|ui| {
                            ui.strong("Destination");
                            egui::ComboBox::from_id_salt("destination").width(330.0).selected_text(p.mapping[row].map(|i| self.layout.layer_label(i)).unwrap_or_else(|| "Choose destination…".into())).show_ui(ui,|ui| {
                                ui.stable_selectable_value(&mut p.mapping[row], None, tr(ui.ctx(), "ui.choose-destination"));
                                let candidates = p.candidates(&self.layout);
                                for target in candidates {
                                    ui.add_enabled_ui(self.layout.visible(target) && !self.layout.locked(target), |ui| {
                                        ui.stable_selectable_value(&mut p.mapping[row],Some(target),self.layout.layer_label(target));
                                    });
                                }
                            });
                            });
                            if p.include_names {
                                if let Some(name) = &shape.name { ui.label(format!("New name: {name}")); }
                                else { ui.small(tr(ui.ctx(), "ui.no-source-name-destination-name-stays-unchanged")); }
                            }
                            egui::CollapsingHeader::new(tr(ui.ctx(), "ui.geometry-before-after")).id_salt("Geometry · before / after").show(ui, |ui| {
                            ui.small(bounds(shape.geometry));
                            if let Some(a) = shape.appearance { ui.small(format!("Fit: {} · crop L/R/T/B: {} / {} / {} / {} px", a.fit.map(|f| f.label()).unwrap_or("Normal crop"), a.crop.left, a.crop.right, a.crop.top, a.crop.bottom)); ui.small(format!("Blanking ({:?}) L/R/T/B: {:.2}% / {:.2}% / {:.2}% / {:.2}%", a.mask.origin, a.mask.left*100., a.mask.right*100., a.mask.top*100., a.mask.bottom*100.)); }
                            if let Some(zoom) = shape.zoom { ui.small(format!("Crop zoom: {:.1}%", zoom * 100.0)); }
                            if let Some(target)=p.mapping[row].filter(|&i| i < self.layout.layers().len()) {
                                let before=self.layout.layers()[target];
                                if let Ok(after)=p.geometry(&shape,before) {
                                    ui.columns(2,|cols| {cols[0].label(tr(ctx, "ui.before"));cols[0].small(bounds(before));cols[1].label(tr(ctx, "ui.after"));cols[1].small(bounds(after));});
                                    if after.outside_on(self.layout.formats().output) {ui.colored_label(palette.amber,"Outside the screen; imported coordinates will be preserved.");}
                                }
                            }
                            });
                        }
                        });
                        ui.add_space(4.0);
                    });
                }
                if reset_mapping {
                    for row in 0..p.mapping.len() {
                        if p.ignored[row] { p.mapping[row] = None; }
                    }
                }
            });
            if !p.shapes.is_empty() {
                let mut preview=self.layout.clone();
                let valid=p.apply(&mut preview);
                if let Err(e)=&valid
                    && p.destination_count_warning(&self.layout).as_ref() != Some(e) {
                    ui.colored_label(palette.amber,e);
                }
                ui.horizontal(|ui| {
                    if apply_button(ui,&tr(ui.ctx(), "action.import-values"), valid.is_ok() && (preview.layers()!=self.layout.layers() || preview.formats()!=self.layout.formats() || (0..preview.layers().len()).any(|i| preview.layer_info(i).name != self.layout.layer_info(i).name))).clicked() {
                        match p.apply(&mut self.layout) {Ok(())=>{self.sync();close=true;},Err(e)=>p.error=e}
                    }
                    if ui.button(tr(ui.ctx(), "ui.cancel")).clicked() {close=true;}
                });
            } else if ui.button(tr(ui.ctx(), "ui.cancel")).clicked() {close=true;}
        });
        if response.should_close() && !close && !p.shapes.is_empty() {
            p.closing = true;
        }
        if p.closing {
            match pending_close(ctx, "paste-close", &p.error) {
                Some(CloseChoice::Apply) => match p.apply(&mut self.layout) {
                    Ok(()) => {
                        self.sync();
                        close = true;
                    }
                    Err(e) => {
                        p.error = e;
                        p.closing = false;
                    }
                },
                Some(CloseChoice::Discard) => close = true,
                Some(CloseChoice::Cancel) => p.closing = false,
                None => {}
            }
        }
        if !close && (!response.should_close() || !p.shapes.is_empty()) {
            self.paste = Some(p);
        }
    }
    fn inspector(&mut self, ui: &mut egui::Ui) {
        let rows = self.layer_tree_rows();
        let layer_row_count = rows.len();
        let input_reference_rows = self.input_reference_count();
        let row_count = layer_row_count
            + if input_reference_rows == 0 {
                0
            } else {
                input_reference_rows + 1
            }
            + 1
            + if self.references.is_empty() {
                0
            } else {
                self.references.len() + 1
            };
        let row_offset = (row_count as f32 - 2.0) * 44.0
            - input_reference_rows as f32 * (44.0 - capture_ui::INPUT_REFERENCE_ROW_HEIGHT);
        let palette = theme::palette(ui.ctx());
        let (r, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 1071.0 + row_offset),
            egui::Sense::hover(),
        );
        let ink = Ink::new(ui, r);
        let w = r.width();
        ink.text(
            0.0,
            2.0,
            &tr(ink.painter.ctx(), "ui.layers-394772"),
            11.0,
            palette.muted,
        );
        if toolbar_icon(
            ui,
            Rect::from_min_size(ink.point(w - 28.0, 0.0), Vec2::new(28.0, 24.0)),
            "Manage layers and folders",
            ToolbarIcon::ManageLayers,
        )
        .clicked()
        {
            self.manage_layers();
        }
        if toolbar_icon(
            ui,
            Rect::from_min_size(ink.point(w - 60.0, 0.0), Vec2::new(28.0, 24.0)),
            "Add reference image",
            ToolbarIcon::AddImage,
        )
        .clicked()
        {
            self.import_reference_file(ui.ctx());
        }
        self.layer_tree_header(ui, &ink, w);
        // Register the list background before its row controls so they keep priority.
        let background = ui.interact(
            Rect::from_min_max(
                ink.point(0.0, 20.0),
                ink.point(w, 28.0 + (layer_row_count + 1) as f32 * 44.0),
            ),
            ui.id().with("layer-list-background"),
            egui::Sense::click(),
        );
        background.context_menu(|ui| {
            self.add_layer_menu(ui);
            if let Some(i) = self.layout.selected() {
                self.layer_organize_menu(ui, self.layout.layer_info(i).id);
            }
        });
        if background.clicked()
            && background.interact_pointer_pos().is_some_and(|pos| {
                !rows.iter().enumerate().any(|(index, row)| {
                    let indent = match row {
                        layers_ui::TreeRow::Layer(i)
                            if !self.layout.layer_info(*i).folder.is_empty() =>
                        {
                            36.0
                        }
                        _ => 0.0,
                    };
                    Rect::from_min_size(
                        ink.point(indent, 28.0 + index as f32 * 44.0),
                        Vec2::new(w - indent, 38.0),
                    )
                    .contains(pos)
                })
            })
        {
            self.layout.select(None);
            self.reference_selected = None;
            self.edit_content = false;
            self.marquee = None;
            self.sync();
        }
        let mut reorder = None;
        for (row_index, tree_row) in rows.into_iter().enumerate() {
            let y = 28.0 + row_index as f32 * 44.0;
            let i = match tree_row {
                layers_ui::TreeRow::Layer(i) => i,
                layers_ui::TreeRow::Folder(folder) => {
                    self.folder_tree_row(ui, &ink, y, w, &folder);
                    continue;
                }
            };
            let indent = if self.layout.layer_info(i).folder.is_empty() {
                0.0
            } else {
                36.0
            };
            if indent > 0.0 {
                self.root_drop_target(
                    ui,
                    Rect::from_min_size(ink.point(0.0, y), Vec2::new(indent - 2.0, 44.0)),
                    egui::Id::new(("layer-root-gutter", self.layout.layer_info(i).id)),
                    false,
                );
            }
            let color = if self.layout.selection_mask()[i] {
                palette.amber
            } else {
                palette.muted
            };
            ink.card(
                indent,
                y,
                w - indent,
                38.0,
                palette.field,
                if self.layout.selection_mask()[i] {
                    color
                } else {
                    palette.line
                },
            );
            ink.text(
                indent + 13.0,
                y + 10.0,
                &self.layout.layer_label(i),
                15.0,
                if self.layout.visible(i) {
                    color
                } else {
                    palette.muted
                },
            );
            let row = ui
                .interact(
                    Rect::from_min_size(ink.point(indent, y), Vec2::new(w - 78.0 - indent, 38.0)),
                    egui::Id::new(("layer", self.layout.layer_info(i).id)),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_text(tr(
                    ui.ctx(),
                    "ui.drag-to-reorder-double-click-to-rename-right-click-for-actions",
                ));
            let layer_id = self.layout.layer_info(i).id;
            row.context_menu(|ui| self.layer_actions_menu(ui, layer_id));
            row.dnd_set_drag_payload(layers_ui::LayerDrag(layer_id));
            if ui.is_enabled() && row.dnd_hover_payload::<layers_ui::LayerDrag>().is_some() {
                let after = ui.input(|input| {
                    input
                        .pointer
                        .interact_pos()
                        .is_some_and(|p| p.y >= row.rect.center().y)
                });
                let y = if after {
                    row.rect.bottom()
                } else {
                    row.rect.top()
                };
                ui.painter().line_segment(
                    [
                        Pos2::new(row.rect.left(), y),
                        Pos2::new(row.rect.right(), y),
                    ],
                    Stroke::new(2.0_f32, palette.cyan),
                );
                if let Some(payload) = row.dnd_release_payload::<layers_ui::LayerDrag>()
                    && let Some(from) = (0..self.layout.layers().len())
                        .find(|&j| self.layout.layer_info(j).id == payload.0)
                {
                    // The UI is top-to-bottom; document indices are back-to-front.
                    let slot = i + usize::from(!after);
                    let to = slot - usize::from(from < slot);
                    reorder = Some((payload.0, self.layout.layer_info(i).folder.clone(), to));
                }
            }
            self.folder_drop_on_layer(&row, i);
            if row.double_clicked() {
                self.layer_ui.rename = Some((
                    self.layout.layer_info(i).id,
                    self.layout.layer_info(i).name.clone(),
                ));
                self.layer_ui.rename_focus = true;
            }
            if row.clicked() {
                if click_shift_modifier(ui) {
                    self.layout.toggle(i);
                } else {
                    self.layout.select(Some(i));
                }
                self.sync();
            }
            for (kind, x, active) in [
                ("lock", w - 72.0, self.layout.locked(i)),
                ("visibility", w - 36.0, !self.layout.visible(i)),
            ] {
                let rect = Rect::from_min_size(ink.point(x, y + 5.0), Vec2::splat(28.0));
                if layer_state_icon(ui, rect, egui::Id::new((kind, i)), kind == "lock", active)
                    .clicked()
                {
                    self.marquee = None;
                    if kind == "lock" {
                        self.layout.set_locked(i, !active);
                    } else {
                        self.layout.set_visible(i, active);
                    }
                    self.sync();
                }
            }
        }
        if let Some((id, folder, to)) = reorder {
            match self.layout.move_layer_to_folder(id, &folder, to) {
                Ok(()) => self.sync(),
                Err(error) => self.error = error,
            }
        }
        self.root_drop_target(
            ui,
            Rect::from_min_size(
                ink.point(0.0, 28.0 + layer_row_count as f32 * 44.0),
                Vec2::new(w, 42.0),
            ),
            egui::Id::new("layer-root-drop"),
            true,
        );
        self.reference_rows(ui, &ink, 28.0 + (layer_row_count + 1) as f32 * 44.0, w);
        let fixed_reference_rows = if self.references.is_empty() {
            0
        } else {
            self.references.len() + 1
        };
        self.input_reference_rows(
            ui,
            &ink,
            28.0 + (layer_row_count + 1 + fixed_reference_rows) as f32 * 44.0,
            w,
        );
        let r = r.translate(Vec2::new(0.0, row_offset));
        let ink = Ink::new(ui, r);
        ink.line([0.0, 130.0], [w, 130.0], palette.line);
        if self.layout.selected_indices().len() > 1 {
            self.alignment_inspector(ui, &ink, w);
            return;
        }
        let Some(i) = self.layout.selected() else {
            if self
                .reference_selected
                .is_some_and(|i| i < self.references.len())
            {
                ink.text(
                    0.0,
                    153.0,
                    &tr(ink.painter.ctx(), "ui.reference-image-f74875"),
                    16.0,
                    palette.purple,
                );
                ink.text(
                    0.0,
                    182.0,
                    &tr(
                        ink.painter.ctx(),
                        "ui.fixed-guide-double-click-its-row-for-settings",
                    ),
                    12.0,
                    palette.muted,
                );
                return;
            }
            ink.text(
                0.0,
                153.0,
                &tr(
                    ink.painter.ctx(),
                    "ui.select-a-layer-or-drag-a-selection-box",
                ),
                16.0,
                palette.text,
            );
            ink.text(
                0.0,
                182.0,
                &tr(ink.painter.ctx(), "ui.shift-click-to-add-or-remove-layers"),
                13.0,
                palette.muted,
            );
            return;
        };
        let g = self.layout.layers()[i];
        ink.text(0.0, 150.0, &self.layout.layer_label(i), 25.0, palette.text);
        ink.text(
            0.0,
            191.0,
            &tr(
                ink.painter.ctx(),
                "ui.layer-resolution-click-for-layer-properties",
            ),
            11.0,
            palette.muted,
        );
        ink.card(0.0, 214.0, w, 34.0, palette.field, palette.line);
        ink.text(
            12.0,
            223.0,
            &format!(
                "{} × {} px  /  {}",
                number(g.width),
                number(g.height),
                ratio_ui::label(g.width / g.height)
            ),
            13.0,
            palette.text,
        );
        if click(ui, &ink, [0.0, 214.0, w, 34.0], "open-formats") {
            self.property_tab = PropertyTab::Layer;
            self.format_draft = Some(FormatDraft::new(&self.layout, i));
        }
        if let Some(mode) = self.layout.formats().layer_fit[i] {
            ink.text(
                0.0,
                251.0,
                &format!("{} · container bounds", mode.label()),
                11.0,
                palette.muted,
            );
        } else if let Ok(m) = self.layout.formats().mapping(i, g) {
            ink.text(
                0.0,
                251.0,
                &format!(
                    "Input {} · content {:.1}% · offset {:.1}, {:.1}",
                    self.layout.formats().source_for[i] + 1,
                    m.framing.zoom * 100.0,
                    m.framing.offset[0],
                    m.framing.offset[1]
                ),
                11.0,
                palette.muted,
            );
        }
        let edit_label = if self.edit_content && self.mask_editing() {
            "crop.done-mask"
        } else if self.edit_content {
            "crop.done"
        } else {
            "crop.adjust"
        };
        let edit_width = if self.edit_content { w - 120.0 } else { w };
        let edit_label_text = tr(ui.ctx(), edit_label);
        ink.button(
            0.0,
            275.0,
            edit_width,
            if self.layout.formats().layer_fit[i].is_some() && !self.edit_content {
                "Adjust edge crop"
            } else {
                &edit_label_text
            },
            self.edit_content,
        );
        if self.edit_content {
            ink.button(
                w - 108.0,
                275.0,
                108.0,
                if self.mask_editing() {
                    "Reset mask"
                } else {
                    "Reset crop"
                },
                false,
            );
            if click(
                ui,
                &ink,
                [w - 108.0, 275.0, 108.0, 34.0],
                "reset-visual-crop",
            ) {
                let result = if self.mask_editing() {
                    self.layout.set_mask(
                        i,
                        canvas_pilot::formats::Mask {
                            origin: self.layout.formats().masks[i].origin,
                            ..Default::default()
                        },
                    )
                } else if let Some(mode) = self.layout.formats().layer_fit[i] {
                    self.layout
                        .set_layer_fit(i, Some(mode), canvas_pilot::formats::Crop::default())
                } else {
                    self.layout
                        .set_framing(i, canvas_pilot::formats::Framing::default())
                };
                match result {
                    Ok(()) => self.sync(),
                    Err(e) => self.error = e,
                }
            }
        }
        if click(
            ui,
            &ink,
            [0.0, 275.0, edit_width, 34.0],
            "edit-layer-content",
        ) {
            if self.mask_editing() {
                self.edit_content = false;
                self.format_draft = None;
            } else {
                self.toggle_crop_edit();
            }
        }
        ui.put(
            Rect::from_min_size(ink.point(0.0, 317.0), Vec2::new(w, 48.0)),
            egui::Label::new(
                egui::RichText::new(tr(
                    ui.ctx(),
                    if self.mask_editing() {
                        "crop.help-mask"
                    } else if self.layout.formats().layer_fit[i].is_some() {
                        "Drag green edges to trim. Image refits on release."
                    } else {
                        "crop.help-normal"
                    },
                ))
                .size(12.0)
                .color(palette.muted),
            )
            .wrap()
            .halign(egui::Align::Min),
        );
        ink.text(
            0.0,
            375.0,
            &tr(ink.painter.ctx(), "ui.measure-position-from"),
            11.0,
            palette.muted,
        );
        let label = rust_i18n::t!(
            "position.from-to",
            locale = &localization::locale(ui.ctx()),
            from = position_point_name(ui.ctx(), self.corner),
            to = position_point_name(ui.ctx(), self.position_layer_point.unwrap_or(self.corner))
        )
        .into_owned();
        let button = ui.put(
            Rect::from_min_size(ink.point(0.0, 394.0), Vec2::new(w, 34.0)),
            egui::Button::new(label),
        );
        let arrow = button.rect.right_center() - Vec2::new(13.0, 0.0);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                arrow + Vec2::new(-4.0, -2.0),
                arrow + Vec2::new(4.0, -2.0),
                arrow + Vec2::new(0.0, 3.0),
            ],
            palette.muted,
            Stroke::NONE,
        ));
        egui::Popup::menu(&button)
            .id(ui.id().with("measurement-points"))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| self.measurement_points_menu(ui));
        let (horizontal, vertical) = match self.corner {
            PositionPoint::Center => ("position.center-x", "position.center-y"),
            PositionPoint::TopLeft => ("position.left", "position.top"),
            PositionPoint::TopRight => ("position.right", "position.top"),
            PositionPoint::BottomLeft => ("position.left", "position.bottom"),
            PositionPoint::BottomRight => ("position.right", "position.bottom"),
        };
        let ink = Ink::new(ui, r.translate(Vec2::new(0.0, 88.0)));
        let half = (w - 12.0) / 2.0;
        let mut enter = false;
        for (index, x, y, title) in [
            (0, 0.0, 368.0, horizontal),
            (1, half + 12.0, 368.0, vertical),
            (2, 0.0, 449.0, "position.width"),
            (3, half + 12.0, 449.0, "position.height"),
        ] {
            ink.text(x, y, &tr(ui.ctx(), title), 12.0, palette.muted);
            let rect = Rect::from_min_size(ink.point(x, y + 23.0), Vec2::new(half, 42.0));
            self.field_rects[index] = rect;
            let response = ui.put(
                rect,
                egui::TextEdit::singleline(&mut self.fields[index])
                    .id(egui::Id::new(("geometry", index)))
                    .font(egui::FontId::proportional(18.0))
                    .margin(Vec2::new(12.0, 10.0)),
            );
            if response.changed() {
                self.fields_edited = true;
            }
            if response.changed()
                && self.keep_aspect_ratio
                && index >= 2
                && let Ok(value) = self.fields[index].parse::<f64>()
                && value.is_finite()
                && value > 0.0
            {
                let other = if index == 2 {
                    value * g.height / g.width
                } else {
                    value * g.width / g.height
                };
                self.fields[if index == 2 { 3 } else { 2 }] = number(other);
            }
            if response.lost_focus()
                && (self.fields_edited || ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                enter = true;
            }
        }
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(Rect::from_min_size(
                ink.point(0.0, 530.0),
                Vec2::new(w, 76.0),
            )),
            |ui| self.resize_ratio_option(ui),
        );
        let ink = Ink::new(ui, r.translate(Vec2::new(0.0, 140.0)));
        let pending = self.fields_pending();
        if enter && pending {
            self.apply_fields();
        }
        if !self.error.is_empty() {
            ui.put(
                Rect::from_min_size(ink.point(0.0, 558.0), Vec2::new(w, 55.0)),
                egui::Label::new(egui::RichText::new(&self.error).color(palette.amber)).wrap(),
            );
        } else if let Some(message) = output_bounds_warning(g, self.layout.formats().output) {
            ui.put(
                Rect::from_min_size(ink.point(0.0, 558.0), Vec2::new(w, 55.0)),
                egui::Label::new(egui::RichText::new(message).size(12.0).color(palette.amber))
                    .wrap(),
            );
        }
        let ink = Ink::new(ui, r.translate(Vec2::new(0.0, 119.0)));
        ink.line([0.0, 640.0], [w, 640.0], palette.line);
        ink.text(
            0.0,
            663.0,
            &tr(ink.painter.ctx(), "ui.visible-box-center-position"),
            11.0,
            palette.cyan,
        );
        let [h, v] = g.center();
        ink.field(0.0, 693.0, half, "Horizontal · from left", &number(h));
        ink.field(half + 12.0, 693.0, half, "Vertical · from top", &number(v));
        ink.text(
            0.0,
            776.0,
            &tr(
                ink.painter.ctx(),
                "ui.layer-properties-crops-provisional-device-mapping",
            ),
            12.0,
            palette.muted,
        );
        ink.button(0.0, 808.0, w, "Copy layer values", true);
        if click(ui, &ink, [0.0, 808.0, w, 34.0], "copy-values") {
            self.copy(ui.ctx());
        }
        ink.button(0.0, 857.0, w, "Paste layer(s) values", false);
        if click(ui, &ink, [0.0, 857.0, w, 34.0], "paste-button") {
            self.open_paste(String::new());
        }
        ink.text(
            0.0,
            912.0,
            &tr(
                ink.painter.ctx(),
                "ui.or-ctrl-cmd-v-with-the-viewport-focused",
            ),
            12.0,
            palette.muted,
        );
    }
    fn alignment_menu(&mut self, ui: &mut egui::Ui) {
        let palette = theme::palette(ui.ctx());
        ui.set_min_width(222.0);
        let single_layer = self.layout.selected_indices().len() == 1;
        let crop_mode = self.edit_content && single_layer;
        if crop_mode {
            ui.label(if self.mask_editing() {
                "ALIGN MASK TO LAYER BOX"
            } else {
                "ALIGN CONTENT TO LAYER BOX"
            });
        } else {
            ui.label(tr(ui.ctx(), "ui.align-to"));
            if self.align_target == AlignTarget::Screen
                || matches!(self.align_target, AlignTarget::Layer(i) if i >= self.layout.layers().len())
            {
                self.align_target = AlignTarget::Selection;
            }
            let default_name = if single_layer {
                "Screen (one selected layer)"
            } else {
                "Selected layers"
            };
            let target_name = |target| match target {
                AlignTarget::Layer(i) => {
                    format!("Layer {}: {}", i + 1, self.layout.layer_info(i).name)
                }
                _ => default_name.to_owned(),
            };
            egui::ComboBox::from_id_salt("alignment-target")
                .width(210.0)
                .selected_text(target_name(self.align_target))
                .show_ui(ui, |ui| {
                    ui.stable_selectable_value(
                        &mut self.align_target,
                        AlignTarget::Selection,
                        default_name,
                    );
                    for i in 0..self.layout.layers().len() {
                        ui.stable_selectable_value(
                            &mut self.align_target,
                            AlignTarget::Layer(i),
                            target_name(AlignTarget::Layer(i)),
                        );
                    }
                });
        }
        ui.add_space(8.0);
        let (grid, _) = ui.allocate_exact_size(Vec2::new(172.0, 128.0), egui::Sense::hover());
        ui.add_enabled_ui(!self.layout.selected_indices().is_empty(), |ui| {
            use Alignment::{Bottom, HorizontalCenter, Left, Right, Top, VerticalCenter};
            for (col, row, title, horizontal, vertical) in [
                (0, 0, "Align top left", Some(Left), Some(Top)),
                (1, 0, "Align top", None, Some(Top)),
                (2, 0, "Align top right", Some(Right), Some(Top)),
                (0, 1, "Align left", Some(Left), None),
                (
                    1,
                    1,
                    "Center both axes",
                    Some(HorizontalCenter),
                    Some(VerticalCenter),
                ),
                (2, 1, "Align right", Some(Right), None),
                (0, 2, "Align bottom left", Some(Left), Some(Bottom)),
                (1, 2, "Align bottom", None, Some(Bottom)),
                (2, 2, "Align bottom right", Some(Right), Some(Bottom)),
                (3, 0, "Center horizontally", Some(HorizontalCenter), None),
                (3, 2, "Center vertically", None, Some(VerticalCenter)),
            ] {
                let rect = Rect::from_min_size(
                    grid.min + Vec2::new(col as f32 * 44.0, row as f32 * 44.0),
                    Vec2::splat(40.0),
                );
                if alignment_icon(ui, rect, title, horizontal, vertical).clicked() {
                    let axes: Vec<_> = [horizontal, vertical].into_iter().flatten().collect();
                    let target = if self.align_target == AlignTarget::Selection && single_layer {
                        AlignTarget::Screen
                    } else {
                        self.align_target
                    };
                    let result = if crop_mode && self.mask_editing() {
                        self.layout
                            .align_mask(self.layout.selected().unwrap(), &axes)
                    } else if crop_mode {
                        self.layout
                            .align_content(self.layout.selected().unwrap(), &axes)
                    } else {
                        self.layout.align_axes(&axes, target)
                    };
                    match result {
                        Ok(()) => {
                            self.sync();
                            ui.close();
                        }
                        Err(e) => self.error = e,
                    }
                }
            }
        });
        ui.small(if crop_mode && self.mask_editing() {
            "Moves the reveal window; keeps the image, mask size and layer box fixed."
        } else if crop_mode {
            "Moves zoomed content; keeps the layer box and zoom fixed."
        } else {
            "Moves selected layers; keeps their sizes."
        });
        if self.layout.selected_indices().is_empty() {
            ui.label(tr(ui.ctx(), "ui.select-layers-to-align"));
        }
        if !self.error.is_empty() {
            ui.colored_label(palette.amber, &self.error);
        }
    }
    fn apply_ratio_target(&mut self) -> Result<(), String> {
        self.ratio_edit.commit()?;
        let Some(ratio) = self.ratio_edit.target else {
            return Ok(());
        };
        if !self.keep_aspect_ratio {
            return Ok(());
        }
        if self.fields_pending() {
            self.apply_fields();
            if self.fields_pending() {
                return Err(self.error.clone());
            }
        }
        let Some(i) = self.layout.selected() else {
            return Ok(());
        };
        let g = self.layout.layers()[i];
        let output = self.layout.formats().output;
        let [x, y] = self.position_reference().offsets_on(g, output);
        let next = self
            .position_reference()
            .geometry_on(x, y, g.width, g.width / ratio, output)?;
        self.layout.apply(i, next)?;
        self.ratio_edit = Default::default();
        self.sync();
        Ok(())
    }
    fn resize_ratio_option(&mut self, ui: &mut egui::Ui) {
        let changed = ui
            .checkbox(
                &mut self.keep_aspect_ratio,
                tr(ui.ctx(), "ui.keep-aspect-ratio-when-resizing"),
            )
            .on_hover_text(tr(
                ui.ctx(),
                "ui.applies-to-layer-sizing-crop-zoom-always-keeps-the-source-proportions",
            ))
            .changed();
        let edited = if let Some(i) = self.layout.selected() {
            let g = self.layout.layers()[i];
            let edited = self.ratio_edit.show(ui, g.width / g.height);
            if !self.keep_aspect_ratio && self.ratio_edit.target.is_some() {
                ui.small(tr(ui.ctx(), "ui.ratio-ready-enable-the-lock-to-resize"));
            }
            edited
        } else {
            false
        };
        if (edited || (changed && self.keep_aspect_ratio))
            && let Err(error) = self.apply_ratio_target()
        {
            self.ratio_edit.error = error;
        }
        if changed {
            let options = self.application_settings(ui.ctx());
            self.settings.save(&options);
        }
    }
    fn measurement_points_menu(&mut self, ui: &mut egui::Ui) {
        let before = (self.corner, self.position_layer_point);
        let mut independent = self.position_layer_point.is_some();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(tr(ui.ctx(), "ui.from-screen"));
                position_point_square(ui, &mut self.corner, "corner");
                ui.label(position_point_name(ui.ctx(), self.corner));
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.label(tr(ui.ctx(), "ui.to-layer"));
                let mut point = self.position_layer_point.unwrap_or(self.corner);
                ui.add_enabled_ui(independent, |ui| {
                    position_point_square(ui, &mut point, "layer-point");
                });
                ui.label(position_point_name(ui.ctx(), point));
                if independent {
                    self.position_layer_point = Some(point);
                }
            });
        });
        if ui
            .checkbox(
                &mut independent,
                tr(ui.ctx(), "ui.use-a-different-measure-point"),
            )
            .changed()
        {
            self.position_layer_point = independent.then_some(self.corner);
        }
        ui.weak(if self.corner == PositionPoint::Center {
            "Offsets: positive right / down, in pixels."
        } else {
            "Offsets: inward from the screen corner, in pixels."
        });
        if before != (self.corner, self.position_layer_point) {
            self.sync();
            self.autosave_position_points();
        }
    }

    fn layer_context_menu(&mut self, ui: &mut egui::Ui) {
        self.add_layer_menu(ui);
        ui.separator();
        self.layer_editing_menu(ui);
        let selected = self.layout.selected_indices();
        if !selected.is_empty() || self.layer_ui.context_layer.is_some() {
            ui.separator();
            ui.menu_button(tr(ui.ctx(), "ui.canvas"), |ui| {
                self.canvas_context_options(ui)
            });
        } else {
            self.canvas_context_options(ui);
        }
    }

    fn layer_editing_menu(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(210.0);
        if let Some(id) = self.layer_ui.context_layer {
            self.layer_organize_menu(ui, id);
            ui.separator();
        }
        let selected = self.layout.selected_indices();
        if !selected.is_empty() {
            ui.menu_button("Crop presets", |ui| {
                if self.crop_preset_buttons(ui) {
                    ui.close();
                }
            });
            if ui.button(tr(ui.ctx(), "ui.copy-layer-values")).clicked() {
                self.copy(ui.ctx());
                ui.close();
            }
            if ui
                .button(tr(ui.ctx(), "ui.paste-layer-values"))
                .on_hover_text(tr(
                    ui.ctx(),
                    "ui.read-the-clipboard-and-review-which-values-and-layers-to-apply",
                ))
                .clicked()
            {
                ui.close();
                Self::request_layer_paste(ui.ctx());
            }
            ui.separator();
        }
        if let Some(i) = self.layout.selected() {
            ui.weak(self.layout.layer_label(i));
            ui.menu_button(tr(ui.ctx(), "ui.crop-mask"), |ui| {
                self.crop_mask_menu(ui, i)
            });
        } else if !selected.is_empty() {
            ui.weak(format!("{} layers selected", selected.len()));
            ui.menu_button(tr(ui.ctx(), "ui.resize-together"), |ui| {
                for (mode, name) in [
                    (ResizeMode::Individual, "Individual layers"),
                    (ResizeMode::Same, "Same handles"),
                    (ResizeMode::Mirror, "Mirror horizontally"),
                    (ResizeMode::Opposite, "Opposite handles"),
                ] {
                    let enabled = matches!(mode, ResizeMode::Individual | ResizeMode::Same)
                        || selected.len() == 2;
                    if ui
                        .add_enabled(
                            enabled,
                            egui::Button::new(name).selected(self.resize_mode == mode),
                        )
                        .clicked()
                    {
                        self.resize_mode = mode;
                        ui.close();
                    }
                }
            });
        }
        if !selected.is_empty() {
            self.resize_ratio_option(ui);
            ui.menu_button(tr(ui.ctx(), "ui.align"), |ui| self.alignment_menu(ui));
            ui.separator();
            if let Some(i) = self.layout.selected() {
                ui.menu_button(tr(ui.ctx(), "ui.properties"), |ui| {
                    let mask_title = tr(ui.ctx(), "ui.mask-properties");
                    for (title, tab) in [
                        ("Source properties…", PropertyTab::Source),
                        ("Layer properties…", PropertyTab::Layer),
                        ("Crop properties…", PropertyTab::Crop),
                        (mask_title.as_str(), PropertyTab::Mask),
                        ("Mapping properties…", PropertyTab::Mapping),
                    ] {
                        if ui.button(title).clicked() {
                            if self.format_draft.as_ref().is_none_or(|d| !d.is_for(i)) {
                                self.format_draft = Some(FormatDraft::new(&self.layout, i));
                            }
                            self.property_tab = tab;
                            ui.close();
                        }
                    }
                });
                ui.menu_button(tr(ui.ctx(), "ui.input-preview-crop-guides"), |ui| {
                    if ui.button(tr(ui.ctx(), "ui.configure-capture")).clicked() {
                        if self.format_draft.as_ref().is_none_or(|d| !d.is_for(i)) {
                            self.format_draft = Some(FormatDraft::new(&self.layout, i));
                        }
                        self.property_tab = PropertyTab::Preview;
                        ui.close();
                    }
                    self.crop_preview_quick_controls(ui, i);
                });
                ui.separator();
            }
            if ui.button(tr(ui.ctx(), "ui.lock-selected-layers")).clicked() {
                for &i in &selected {
                    self.layout.set_locked(i, true);
                }
                self.edit_content = false;
                self.sync();
                ui.close();
            }
            if ui.button(tr(ui.ctx(), "ui.hide-selected-layers")).clicked() {
                for &i in &selected {
                    self.layout.set_visible(i, false);
                }
                self.edit_content = false;
                self.sync();
                ui.close();
            }
        }
        if let Some(id) = self.layer_ui.context_layer {
            self.layer_delete_menu(ui, id);
        }
    }

    fn crop_mask_menu(&mut self, ui: &mut egui::Ui, i: usize) {
        let crop_label = if self.edit_content && !self.mask_editing() {
            "crop.done"
        } else {
            "crop.adjust"
        };
        if ui
            .button(if self.layout.formats().layer_fit[i].is_some() {
                if self.edit_content && !self.mask_editing() { "Finish edge crop" } else { "Adjust edge crop" }.into()
            } else {
                tr(ui.ctx(), crop_label)
            })
            .on_hover_text(if self.layout.formats().layer_fit[i].is_some() {
                "Trim green edges independently. The image refits on release; the container stays fixed.".into()
            } else { tr(ui.ctx(), "ui.change-crop-zoom-and-offset-inside-the-layer-keep-its-visible-box-fixed") })
            .clicked()
        {
            self.toggle_crop_edit();
            ui.close();
        }
        if self.layout.formats().layer_fit[i].is_some()
            && ui.button("Convert to Smart crop")
                .on_hover_text("Keep an exact matching crop; otherwise reset to centered cover framing. Undo restores the advanced fit.")
                .clicked() {
            match self.layout.convert_to_smart_crop(i) {
                Ok(()) => { self.sync(); self.edit_mask = false; self.edit_content = true; },
                Err(e) => self.error = e,
            }
            ui.close();
        }
        if self.layout.formats().layer_fit[i].is_none()
            && ui.button("Convert to Edge crop")
                .on_hover_text("Keep the current picture and crop, then edit its source edges independently. Undo returns to Smart crop.")
                .clicked() {
            match self.layout.convert_to_edge_crop(i) {
                Ok(()) => { self.sync(); self.edit_mask = false; self.edit_content = true; },
                Err(e) => self.error = e,
            }
            ui.close();
        }
        if ui.button(tr(ui.ctx(), "ui.reset-crop")).clicked() {
            let result = if let Some(mode) = self.layout.formats().layer_fit[i] {
                self.layout
                    .set_layer_fit(i, Some(mode), canvas_pilot::formats::Crop::default())
            } else {
                self.layout
                    .set_framing(i, canvas_pilot::formats::Framing::default())
            };
            match result {
                Ok(()) => self.sync(),
                Err(e) => self.error = e,
            }
            ui.close();
        }
        ui.separator();
        if ui.button(tr(ui.ctx(), "ui.adjust-layer-mask")).clicked() {
            self.open_mask_edit(i);
            ui.close();
        }
        if ui.button(tr(ui.ctx(), "ui.reset-mask")).clicked() {
            let mask = canvas_pilot::formats::Mask {
                origin: self.layout.formats().masks[i].origin,
                ..Default::default()
            };
            match self.layout.set_mask(i, mask) {
                Ok(()) => self.sync(),
                Err(error) => self.error = error,
            }
            ui.close();
        }
    }

    fn canvas_context_options(&mut self, ui: &mut egui::Ui) {
        self.snapping_menu(ui);
        ui.menu_button(tr(ui.ctx(), "ui.reference-images"), |ui| {
            self.reference_actions_menu(ui)
        });
        if ui.button(tr(ui.ctx(), "ui.output-format")).clicked() {
            self.output_draft = Some(OutputDraft::new(self.layout.formats().output));
            ui.close();
        }
    }

    fn alignment_inspector(&mut self, ui: &mut egui::Ui, ink: &Ink, w: f32) {
        let palette = theme::palette(ui.ctx());
        ink.text(
            0.0,
            150.0,
            &tr(ink.painter.ctx(), "ui.2-layers-selected"),
            25.0,
            palette.text,
        );
        ink.text(
            0.0,
            187.0,
            &tr(
                ink.painter.ctx(),
                "ui.drag-or-nudge-to-move-the-selection-together",
            ),
            12.0,
            palette.muted,
        );
        ink.text(
            0.0,
            225.0,
            &tr(
                ink.painter.ctx(),
                "ui.use-the-toolbar-alignment-menu-to-align-layers",
            ),
            12.0,
            palette.muted,
        );
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(Rect::from_min_size(
                ink.point(0.0, 254.0),
                Vec2::new(w, 34.0),
            )),
            |ui| {
                egui::ComboBox::from_id_salt("resize-selected-mode")
                    .width(w - 12.0)
                    .selected_text(match self.resize_mode {
                        ResizeMode::Individual => "Resize: individual layer",
                        ResizeMode::Same => "Same handles",
                        ResizeMode::Mirror => "Mirror horizontally",
                        ResizeMode::Opposite => "Opposite handles",
                    })
                    .show_ui(ui, |ui| {
                        ui.stable_selectable_value(
                            &mut self.resize_mode,
                            ResizeMode::Individual,
                            tr(ui.ctx(), "ui.resize-individual-layer"),
                        );
                        ui.stable_selectable_value(
                            &mut self.resize_mode,
                            ResizeMode::Same,
                            tr(ui.ctx(), "ui.same-handles"),
                        );
                        ui.add_enabled_ui(self.layout.selected_indices().len() == 2, |ui| {
                            ui.stable_selectable_value(
                                &mut self.resize_mode,
                                ResizeMode::Mirror,
                                tr(ui.ctx(), "ui.mirror-horizontally-two-layers"),
                            );
                            ui.stable_selectable_value(
                                &mut self.resize_mode,
                                ResizeMode::Opposite,
                                tr(ui.ctx(), "ui.opposite-handles-two-layers"),
                            );
                        });
                    });
            },
        );
        ui.put(Rect::from_min_size(ink.point(0.0, 292.0), Vec2::new(w, 52.0)), egui::Label::new(egui::RichText::new(
            match self.resize_mode {
                ResizeMode::Individual => "Choose a shared resize mode, then drag a selected layer's handle to resize the selection.",
                ResizeMode::Same => "Matching handles, equal proportional scaling. Drag any selected handle. Escape cancels drag.",
                ResizeMode::Mirror => "Left/right mirrored handles, equal proportional scaling. Two layers. Escape cancels drag.",
                ResizeMode::Opposite => "Diagonally opposite corners, equal proportional scaling. Two layers. Escape cancels drag.",
            }
        ).size(12.0).color(palette.muted)).wrap().halign(egui::Align::Min));
        let shifted = Ink {
            painter: ink.painter.clone(),
            origin: ink.origin + Vec2::new(0.0, 42.0),
            scale: ink.scale,
        };
        let ink = &shifted;
        ink.line([0.0, 319.0], [w, 319.0], palette.line);
        ink.text(
            0.0,
            341.0,
            &tr(ink.painter.ctx(), "ui.visible-box-center-positions"),
            11.0,
            palette.cyan,
        );
        for (row, i) in self.layout.selected_indices().into_iter().enumerate() {
            let y = 375.0 + row as f32 * 80.0;
            let [x, cy] = self.layout.layers()[i].center();
            ink.text(0.0, y, &self.layout.layer_label(i), 13.0, palette.muted);
            ink.text(
                0.0,
                y + 25.0,
                &format!("H {}   V {}", number(x), number(cy)),
                19.0,
                palette.text,
            );
        }
        ink.button(0.0, 552.0, w, "Copy selected layer values", true);
        if click(ui, ink, [0.0, 552.0, w, 34.0], "copy-selected") {
            self.copy(ui.ctx());
        }
        ink.text(
            0.0,
            609.0,
            &tr(
                ink.painter.ctx(),
                "ui.numeric-edits-require-one-selected-layer",
            ),
            12.0,
            palette.muted,
        );
        if !self.error.is_empty() {
            ui.put(
                Rect::from_min_size(ink.point(0.0, 640.0), Vec2::new(w, 60.0)),
                egui::Label::new(egui::RichText::new(&self.error).color(palette.amber)).wrap(),
            );
        }
    }
    fn content_hit(&self, pointer: Pos2, origin: Pos2, scale: f32) -> Option<usize> {
        if !self.edit_content {
            return None;
        }
        let i = self.layout.selected()?;
        let to_rect = |g: &Geometry| {
            Rect::from_min_size(
                origin + Vec2::new(g.x as f32, g.y as f32) * scale,
                Vec2::new(g.width as f32, g.height as f32) * scale,
            )
        };
        let rect = to_rect(&self.layout.layers()[i]);
        // The faded preview is behind other visible layers, including locked ones.
        if !rect.contains(pointer)
            && self
                .layout
                .layers()
                .iter()
                .enumerate()
                .any(|(j, g)| j != i && self.layout.visible(j) && to_rect(g).contains(pointer))
        {
            return None;
        }
        if self.mask_editing() {
            return (rect.contains(pointer)
                || self
                    .crop_bounds(origin, scale)
                    .is_some_and(|bounds| bounds.contains(pointer)))
            .then_some(i);
        }
        if self.layout.formats().layer_fit[i].is_some() {
            return (rect.contains(pointer)
                || self
                    .crop_bounds(origin, scale)
                    .is_some_and(|r| r.contains(pointer)))
            .then_some(i);
        }
        format_ui::content_bounds(
            rect,
            self.layout.formats().sources[self.layout.formats().source_for[i]],
            self.layout.formats().advanced[i],
        )
        .filter(|bounds| bounds.contains(pointer))
        .map(|_| i)
    }
    fn crop_bounds(&self, origin: Pos2, scale: f32) -> Option<Rect> {
        if !self.edit_content {
            return None;
        }
        let i = self.layout.selected()?;
        if self.mask_editing() {
            let g = self
                .layout
                .formats()
                .mask_bounds(i, self.layout.layers()[i])
                .ok()?;
            return Some(Rect::from_min_size(
                origin + Vec2::new(g.x as f32, g.y as f32) * scale,
                Vec2::new(g.width as f32, g.height as f32) * scale,
            ));
        }
        if self.layout.formats().layer_fit[i].is_some() {
            let g = self.layout.edge_crop_bounds(i).ok()?;
            return Some(Rect::from_min_size(
                origin + Vec2::new(g.x as f32, g.y as f32) * scale,
                Vec2::new(g.width as f32, g.height as f32) * scale,
            ));
        }
        let g = self.layout.layers()[i];
        let rect = Rect::from_min_size(
            origin + Vec2::new(g.x as f32, g.y as f32) * scale,
            Vec2::new(g.width as f32, g.height as f32) * scale,
        );
        format_ui::content_bounds(
            rect,
            self.layout.formats().sources[self.layout.formats().source_for[i]],
            self.layout.formats().advanced[i],
        )
    }
    fn crop_handle(&self, pointer: Pos2, origin: Pos2, scale: f32) -> Option<ResizeHandle> {
        let rect = self.crop_bounds(origin, scale)?;
        crop_handle_positions(rect)
            .into_iter()
            .find(|(_, p)| Rect::from_center_size(*p, Vec2::splat(14.0)).contains(pointer))
            .map(|(handle, _)| handle)
    }

    fn pivot_for(&self, i: usize) -> [f64; 2] {
        self.layout.layer_info(i).pivot
    }
    fn pivot_hit(&self, pointer: Pos2, origin: Pos2, scale: f32) -> Option<usize> {
        self.layout.selected_indices().into_iter().rev().find(|&i| {
            let g = self.layout.layers()[i];
            let relative = self.pivot_for(i);
            let point = origin
                + Vec2::new(
                    (g.x + relative[0] * g.width) as f32,
                    (g.y + relative[1] * g.height) as f32,
                ) * scale;
            pointer.distance(point) <= 9.0
        })
    }
    fn move_pivot_snapped(
        &mut self,
        i: usize,
        point: [f64; 2],
        modifiers: egui::Modifiers,
        scale: f32,
    ) {
        let mut options = self.snapping;
        if modifiers.alt {
            options.enabled = !options.enabled;
        }
        let (target, guides) = self.layout.snap_pivot(
            i,
            point,
            options,
            6.0 / scale as f64,
            self.effective_grid(modifiers).map(f64::from),
            !modifiers.alt,
        );
        self.move_pivot(i, target);
        self.snap_guides = guides;
    }
    fn move_pivot(&mut self, i: usize, point: [f64; 2]) {
        let g = self.layout.layers()[i];
        let relative = [(point[0] - g.x) / g.width, (point[1] - g.y) / g.height];
        let mode = self.active_resize_mode();
        let targets = if mode == ResizeMode::Individual {
            vec![i]
        } else {
            self.layout.selected_indices()
        };
        let pivots: Vec<_> = targets
            .into_iter()
            .map(|j| {
                let mut pivot = relative;
                if j != i {
                    if matches!(mode, ResizeMode::Mirror | ResizeMode::Opposite) {
                        pivot[0] = 1.0 - pivot[0];
                    }
                    if mode == ResizeMode::Opposite {
                        pivot[1] = 1.0 - pivot[1];
                    }
                }
                (j, pivot)
            })
            .collect();
        let mut next = self
            .pivot_drag
            .as_ref()
            .map_or(&self.layout, |(_, before)| before)
            .clone();
        match next.set_layer_pivots(&pivots) {
            Ok(()) => self.layout = next,
            Err(error) => self.error = error,
        }
    }
    fn resize_handle(
        &self,
        pointer: Pos2,
        origin: Pos2,
        scale: f32,
    ) -> Option<(usize, ResizeHandle)> {
        for i in self.layout.selected_indices().into_iter().rev() {
            let g = self.layout.layers()[i];
            for (corner, [x, y]) in ResizeHandle::positions(g) {
                let center = origin + Vec2::new(x as f32, y as f32) * scale;
                if Rect::from_center_size(center, Vec2::splat(14.0)).contains(pointer) {
                    return Some((i, corner));
                }
            }
        }
        None
    }
    fn effective_grid(&self, modifiers: egui::Modifiers) -> Option<u32> {
        if modifiers.alt {
            return None;
        }
        if modifiers.shift {
            let requested = if modifiers.ctrl { 100 } else { 10 };
            if self.grid_tool == Some(requested) {
                None
            } else {
                Some(requested)
            }
        } else {
            self.grid_tool
        }
    }
    fn effective_pivot(&self, modifiers: egui::Modifiers) -> bool {
        self.pivot_tool ^ (modifiers.ctrl && !modifiers.shift)
    }
    fn drag_with_modifiers(
        &mut self,
        point: [f64; 2],
        modifiers: egui::Modifiers,
        scale: f32,
    ) -> [Option<f64>; 2] {
        self.layout
            .use_resize_pivot(self.effective_pivot(modifiers));
        if let Some(spacing) = self.effective_grid(modifiers) {
            self.layout.drag_to_grid(point, spacing as f64)
        } else {
            let mut snapping = self.snapping;
            if modifiers.alt {
                snapping.enabled = !snapping.enabled;
            }
            self.layout
                .drag_to_snapped(point, snapping, 6.0 / scale as f64)
        }
    }
    fn active_resize_mode(&self) -> ResizeMode {
        if self.layout.selected_indices().len() < 2 {
            ResizeMode::Individual
        } else {
            self.resize_mode
        }
    }
    fn set_view_zoom(
        &mut self,
        zoom: f32,
        world: Vec2,
        pointer: Pos2,
        area: Rect,
        base: f32,
    ) -> (Pos2, f32) {
        self.zoom = zoom.clamp(0.0001, 256.0);
        let scale = base * self.zoom;
        let center = Vec2::new(
            self.layout.formats().output[0] as f32,
            self.layout.formats().output[1] as f32,
        ) / 2.0;
        self.pan = (pointer - area.center()) / scale + center - world;
        (area.center() - center * scale + self.pan * scale, scale)
    }
    fn workspace(&mut self, ui: &mut egui::Ui) {
        self.sync_crop_tab_mode();
        let palette = theme::palette(ui.ctx());
        let r = ui.max_rect();
        let ink = Ink::new(ui, r);
        let output = self.layout.formats().output;
        let [sw, sh] = output.map(|v| v as f32);
        let names = self.show_toolbar_icon_names;
        let (buttons, groups, toolbar_bottom) = toolbar_groups(r.width(), names);
        for group in groups {
            ink.card(
                group.min.x,
                group.min.y,
                group.width(),
                group.height(),
                palette.field,
                palette.line,
            );
        }
        let button = |index: usize| buttons[index].translate(r.min.to_vec2());
        let extra_height = toolbar_bottom - 56.0;
        self.size_preset_toolbar(ui, button(17), names);
        if toolbar_icon(ui, button(13), "Output format", ToolbarIcon::Output).clicked() {
            self.output_draft = Some(OutputDraft::new(output));
        }
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(button(16))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().interact_size.y = 32.0;
                let before = self.snapping;
                let menu = toolbar_menu(ui, "     ", |ui| self.overlay_grid_options(ui));
                menu.response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Overlay grid")
                });
                let center = menu.response.rect.center();
                let color = if self.snapping.show_grid {
                    palette.cyan
                } else {
                    palette.muted
                };
                for n in [-1.0, 0.0, 1.0] {
                    ui.painter().line_segment(
                        [
                            center + Vec2::new(n * 7.0, -7.0),
                            center + Vec2::new(n * 7.0, 7.0),
                        ],
                        Stroke::new(1.0_f32, color),
                    );
                    ui.painter().line_segment(
                        [
                            center + Vec2::new(-7.0, n * 7.0),
                            center + Vec2::new(7.0, n * 7.0),
                        ],
                        Stroke::new(1.0_f32, color),
                    );
                }
                menu.response.on_hover_text(tr(
                    ui.ctx(),
                    "ui.overlay-grid-visibility-subdivisions-and-snapping",
                ));
                if before != self.snapping {
                    self.settings.save(&self.application_settings(ui.ctx()));
                }
            },
        );
        let modifiers = ui.input(|i| i.modifiers);
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(button(11))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().interact_size.y = 32.0;
                let response = toolbar_menu(ui, SNAP_MENU_TITLE, |ui| {
                    ui.label(tr(ui.ctx(), "ui.grid-tools"));
                    for (spacing, label, shortcut) in [(10, "10 px", "Shift"), (100, "100 px", "Ctrl+Shift")] {
                        ui.horizontal(|ui| {
                            let (_, rect) = ui.allocate_space(Vec2::splat(32.0));
                            let selected = self.grid_tool == Some(spacing);
                            let effective = self.effective_grid(modifiers) == Some(spacing);
                            let icon = edit_tool_icon(ui, rect, Some(spacing), label, selected, effective);
                            let text = ui.stable_selectable_label(selected, format!("{label} grid"));
                            let tooltip = format!("Toggle {label} grid. Hold {shortcut} for the same effect; when selected, the modifier temporarily disables it. Alt overrides grids.");
                            if icon.on_hover_text(&tooltip).clicked() || text.on_hover_text(tooltip).clicked() {
                                self.grid_tool = if selected { None } else { Some(spacing) };
                                ui.close();
                            }
                        });
                    }
                    ui.separator();
                    let before = self.snapping;
                    self.snapping_options(ui);
                    if before != self.snapping {
                        self.settings.save(&self.application_settings(ui.ctx()));
                    }
                    if let Some(error) = &self.settings.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                }).response.on_hover_text(tr(ui.ctx(), "ui.snapping-tools-and-options-shift-10-px-ctrl-shift-100-px-alt-invert-snapping"));
                response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Snapping"));
                let chevron = response.rect.right_center() - Vec2::new(8.0, 0.0);
                ui.painter().add(egui::Shape::line(
                    vec![chevron + Vec2::new(-3.0, -2.0), chevron + Vec2::new(0.0, 2.0), chevron + Vec2::new(3.0, -2.0)],
                    Stroke::new(1.5_f32, palette.text),
                ));
                let center = response.rect.left_center() + Vec2::new(17.0, 0.0);
                let color = if self.snapping.enabled || self.effective_grid(modifiers).is_some() {
                    palette.cyan
                } else {
                    palette.muted
                };
                let point = |x, y| center + Vec2::new(x, y);
                // Horseshoe magnet, with distinct pole caps and a rounded lower bend.
                let mut arc = vec![point(-7.0, -8.0), point(-7.0, 1.0)];
                for step in 1..=16 {
                    let angle = std::f32::consts::PI * (1.0 - step as f32 / 16.0);
                    arc.push(point(7.0 * angle.cos(), 1.0 + 7.0 * angle.sin()));
                }
                arc.push(point(7.0, -8.0));
                ui.painter().add(egui::Shape::line(arc, Stroke::new(4.0_f32, color)));
                for x in [-7.0, 7.0] {
                    ui.painter().line_segment([point(x, -8.0), point(x, -4.0)], Stroke::new(4.0_f32, palette.text));
                }
                if names || self.effective_grid(modifiers).is_some() {
                    let caption = match self.effective_grid(modifiers) {
                        Some(10) => "10 px",
                        Some(100) => "100 px",
                        _ => "Snap",
                    };
                    ui.painter().text(response.rect.center_bottom() + Vec2::new(0.0, 3.0), Align2::CENTER_TOP, caption, FontId::proportional(10.0), palette.muted);
                }
            },
        );
        let pivot_rect = button(12);
        if edit_tool_icon(ui, pivot_rect, None, "Pivot", self.pivot_tool, self.effective_pivot(modifiers))
            .on_hover_text(tr(ui.ctx(), "ui.pivot-resize-click-to-toggle-hold-ctrl-for-the-same-effect-if-selected-ctrl-temporari"))
            .clicked()
        {
            self.pivot_tool = !self.pivot_tool;
        }
        if names {
            ui.painter().text(
                pivot_rect.center_bottom() + Vec2::new(0.0, 7.0),
                Align2::CENTER_TOP,
                "Pivot",
                FontId::proportional(11.0),
                palette.muted,
            );
        }
        for (slot, tool, name, icon) in [
            (4, NavigationTool::Select, "Select", ToolbarIcon::Select),
            (5, NavigationTool::Move, "Move", ToolbarIcon::Move),
            (6, NavigationTool::Pan, "Pan", ToolbarIcon::Pan),
            (7, NavigationTool::Zoom, "Zoom", ToolbarIcon::Zoom),
        ] {
            let rect = button(slot);
            if self.navigation_tool == tool {
                ui.painter().rect(
                    rect,
                    4.0,
                    palette.purple,
                    Stroke::new(1.0_f32, palette.cyan),
                    StrokeKind::Inside,
                );
            }
            if toolbar_icon(ui, rect, name, icon).clicked() {
                self.navigation_tool = tool;
            }
            if names {
                ui.painter().text(
                    rect.center_bottom() + Vec2::new(0.0, 7.0),
                    Align2::CENTER_TOP,
                    tr(ui.ctx(), icon.title_key()),
                    FontId::proportional(11.0),
                    palette.muted,
                );
            }
        }
        for (slot, text, icon) in [
            (8, "Fit screen", ToolbarIcon::Screen),
            (9, "Zoom to selection", ToolbarIcon::Selection),
            (2, "Undo", ToolbarIcon::Undo),
            (3, "Redo", ToolbarIcon::Redo),
        ] {
            let rect = button(slot);
            if toolbar_icon(ui, rect, text, icon).clicked() {
                match text {
                    "Fit screen" => self.fit_screen(),
                    "Zoom to selection" => self.fit_selection(),
                    "Undo" => {
                        self.layout.undo();
                        self.sync();
                    }
                    _ => {
                        self.layout.redo();
                        self.sync();
                    }
                }
            }
        }
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(button(10))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().interact_size = Vec2::new(36.0, 32.0);
                ui.spacing_mut().button_padding.x = 11.0;
                let menu = toolbar_menu(ui, "    ", |ui| self.alignment_menu(ui));
                let response = menu
                    .response
                    .on_hover_text(tr(ui.ctx(), "ui.align-selected-layers"));
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        "Align selected layers",
                    )
                });
                let c = response.rect.center();
                let p = ui.painter();
                let color = if response.hovered() || response.has_focus() {
                    palette.cyan
                } else {
                    palette.text
                };
                let stroke = Stroke::new(1.6_f32, color);
                p.line_segment(
                    [c + Vec2::new(-10.0, -9.0), c + Vec2::new(-10.0, 9.0)],
                    stroke,
                );
                p.rect_filled(
                    Rect::from_min_size(c + Vec2::new(-6.0, -7.0), Vec2::new(13.0, 5.0)),
                    0.0,
                    color,
                );
                p.rect_filled(
                    Rect::from_min_size(c + Vec2::new(-6.0, 3.0), Vec2::new(9.0, 5.0)),
                    0.0,
                    color,
                );
            },
        );
        for (slot, exporting) in [(1, false), (18, true)] {
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(button(slot))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    ui.spacing_mut().interact_size = Vec2::new(36.0, 32.0);
                    ui.spacing_mut().button_padding.x = 11.0;
                    let menu = toolbar_menu(ui, "    ", |ui| {
                        if exporting {
                            self.export_menu(ui);
                        } else {
                            self.import_menu(ui);
                        }
                    });
                    let label = tr(ui.ctx(), if exporting { "ui.export" } else { "ui.import" });
                    let response = menu.response.on_hover_text(label.clone());
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label)
                    });
                    paint_transfer_icon(ui, &response, exporting);
                },
            );
        }
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(button(15))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().interact_size.y = 32.0;
                toolbar_menu(ui, &tr(ui.ctx(), "menu.title"), |ui| {
                    self.application_menu(ui)
                });
            },
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(button(0))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().interact_size.y = 32.0;
                toolbar_menu(ui, &tr(ui.ctx(), "ui.project"), |ui| self.project_menu(ui));
            },
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(button(14))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().interact_size = Vec2::new(36.0, 32.0);
                ui.spacing_mut().button_padding.x = 11.0;
                let menu = toolbar_menu(ui, "    ", |ui| {
                    if ui.button(tr(ui.ctx(), "ui.open-all-settings")).clicked() {
                        self.preferences_open = true;
                        ui.close();
                    }
                    ui.separator();
                    self.viewport_options_menu(ui);
                });
                let response = menu
                    .response
                    .on_hover_text(tr(ui.ctx(), "ui.viewport-options"));
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Viewport options")
                });
                let center = response.rect.center();
                let stroke = Stroke::new(
                    1.6_f32,
                    if response.hovered() || response.has_focus() {
                        palette.cyan
                    } else {
                        palette.text
                    },
                );
                let points = (0..32)
                    .map(|i| {
                        let angle = i as f32 * std::f32::consts::TAU / 32.0;
                        let radius = if i % 4 == 0 || i % 4 == 3 { 8.0 } else { 11.0 };
                        center + Vec2::angled(angle) * radius
                    })
                    .collect();
                ui.painter().add(egui::Shape::closed_line(points, stroke));
                ui.painter().circle_stroke(center, 3.5, stroke);
            },
        );
        if names {
            for (slot, label) in [
                (13, "Output"),
                (8, "Fit screen"),
                (9, "Fit selection"),
                (2, "Undo"),
                (3, "Redo"),
                (10, "Align"),
                (1, "Import"),
                (18, "Export"),
                (14, "Options"),
                (16, "Grid"),
            ] {
                ui.painter().text(
                    button(slot).center_bottom() + Vec2::new(0.0, 7.0),
                    egui::Align2::CENTER_TOP,
                    label,
                    egui::FontId::proportional(11.0),
                    palette.muted,
                );
            }
        }
        if self.layout.selected().is_none() {
            self.edit_content = false;
        }
        let (text, color) = if !self.reference_error.is_empty() {
            (self.reference_error.clone(), ui.visuals().error_fg_color)
        } else if let Some((severe, warning)) = self
            .references
            .iter()
            .filter(|r| r.visible)
            .filter_map(|r| r.warning(self.layout.formats().output))
            .max_by_key(|(severe, _)| *severe)
        {
            (
                warning,
                if severe {
                    ui.visuals().error_fg_color
                } else {
                    palette.amber
                },
            )
        } else {
            (
                self.reference_selected
                    .and_then(|i| self.references.get(i))
                    .map(|r| format!("Reference: {} · {} × {} px", r.name, r.size[0], r.size[1]))
                    .unwrap_or_default(),
                palette.muted,
            )
        };
        // Only reserve a status row for an imported image, warning or error.
        let mut canvas_top = 64.0 + extra_height;
        if !text.is_empty() {
            let status = ui.painter().layout(
                text,
                FontId::proportional(12.0),
                color,
                (r.width() - 32.0).max(1.0),
            );
            canvas_top += status.size().y + 8.0;
            ui.painter()
                .galley(ink.point(16.0, 64.0 + extra_height), status, color);
        }
        // Artwork uses half-screen coordinates at twice the camera scale. Since
        // the output stays centered, allow the label's overhang on both sides.
        let label_margin = SCREEN_LABEL_TOP * 4.0 * artwork_scale(output);
        // Help is optional: reclaim its footer when width is tight, or when it
        // would reduce the fitted output's scale. Toolbar tooltips remain available.
        let width_fitted_height = (r.width() - 40.0) / sw * (sh + label_margin) + 24.0;
        let show_hints =
            r.width() >= 760.0 && r.height() - canvas_top - 48.0 >= width_fitted_height;
        let area = Rect::from_min_max(
            r.min + Vec2::new(8.0, canvas_top),
            r.max - Vec2::new(8.0, if show_hints { 48.0 } else { 8.0 }),
        );
        let base = ((area.width() - 24.0) / sw).min((area.height() - 24.0) / (sh + label_margin));
        if self.camera_one_to_one {
            self.zoom = (1.0 / (base * ui.ctx().pixels_per_point())).clamp(0.0001, 256.0);
            self.camera_one_to_one = false;
        }
        let mut scale = base * self.zoom;
        let mut origin = area.center() - Vec2::new(sw / 2.0, sh / 2.0) * scale + self.pan * scale;
        let response = ui.interact(area, egui::Id::new("canvas"), egui::Sense::click_and_drag());
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let temporary_pan = ui.input(|i| {
            i.key_down(egui::Key::Space) || i.pointer.button_down(egui::PointerButton::Middle)
        });
        let edit_pointer = self.navigation_tool == NavigationTool::Select && !temporary_pan;
        let move_pointer = self.navigation_tool == NavigationTool::Move && !temporary_pan;
        if move_pointer && response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
        }

        if response.hovered()
            && !self.layout.dragging()
            && !self.panning
            && self.marquee.is_none()
            && self.zoom_drag.is_none()
            && self.pivot_drag.is_none()
        {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0
                && let Some(p) = pointer
            {
                let world = (p - origin) / scale;
                (origin, scale) =
                    self.set_view_zoom(self.zoom * (scroll * 0.002).exp(), world, p, area, base);
            }
        }
        if edit_pointer
            && response.hovered()
            && pointer.is_some_and(|p| self.content_hit(p, origin, scale).is_some())
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if edit_pointer
            && self.edit_content
            && response.hovered()
            && let Some(corner) = pointer.and_then(|p| self.crop_handle(p, origin, scale))
        {
            ui.ctx().set_cursor_icon(match corner {
                ResizeHandle::Corner(Corner::TopLeft | Corner::BottomRight) => {
                    egui::CursorIcon::ResizeNwSe
                }
                ResizeHandle::Corner(_) => egui::CursorIcon::ResizeNeSw,
                ResizeHandle::Left | ResizeHandle::Right => egui::CursorIcon::ResizeHorizontal,
                ResizeHandle::Top | ResizeHandle::Bottom => egui::CursorIcon::ResizeVertical,
            });
        }
        if edit_pointer
            && !self.edit_content
            && response.hovered()
            && let Some(p) = pointer
            && let Some((_, corner)) = self.resize_handle(p, origin, scale)
        {
            ui.ctx().set_cursor_icon(match corner {
                ResizeHandle::Corner(Corner::TopLeft | Corner::BottomRight) => {
                    egui::CursorIcon::ResizeNwSe
                }
                ResizeHandle::Corner(_) => egui::CursorIcon::ResizeNeSw,
                ResizeHandle::Left | ResizeHandle::Right => egui::CursorIcon::ResizeHorizontal,
                ResizeHandle::Top | ResizeHandle::Bottom => egui::CursorIcon::ResizeVertical,
            });
        }
        let pivot_visible = edit_pointer && self.effective_pivot(ui.input(|i| i.modifiers));
        let pivot_hover = if pivot_visible {
            pointer.and_then(|p| self.pivot_hit(p, origin, scale))
        } else {
            None
        };
        if response.hovered() && pivot_hover.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
        }
        if response.hovered() && !edit_pointer {
            ui.ctx().set_cursor_icon(
                if temporary_pan || self.navigation_tool == NavigationTool::Pan {
                    if self.panning {
                        egui::CursorIcon::Grabbing
                    } else {
                        egui::CursorIcon::Grab
                    }
                } else if ui.input(|i| i.modifiers.alt) {
                    egui::CursorIcon::ZoomOut
                } else {
                    egui::CursorIcon::ZoomIn
                },
            );
        }
        if response.drag_started_by(egui::PointerButton::Primary)
            || response.drag_started_by(egui::PointerButton::Middle)
        {
            ui.ctx().memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
            self.panning = temporary_pan || self.navigation_tool == NavigationTool::Pan;
            if self.panning || self.navigation_tool == NavigationTool::Zoom {
                self.camera_before = Some((self.zoom, self.pan));
            }
            if !self.panning
                && self.navigation_tool == NavigationTool::Zoom
                && let Some(p) = ui.input(|i| i.pointer.press_origin())
            {
                self.zoom_drag = Some((p, (p - origin) / scale, self.zoom));
            }
            if !self.panning
                && move_pointer
                && let Some(p) = ui.input(|i| i.pointer.press_origin())
            {
                let p = (p - origin) / scale;
                let point = [p.x as f64, p.y as f64];
                if self.edit_content {
                    if let Some(i) = self.layout.selected() {
                        if self.mask_editing() {
                            self.layout.start_mask_drag(i, point, None);
                        } else {
                            self.layout.start_content_drag(i, point);
                        }
                    }
                } else {
                    self.layout.start_selection_drag(point);
                }
            }
            if !self.panning
                && edit_pointer
                && let Some(p) = ui.input(|i| i.pointer.press_origin())
            {
                let pivot = if pivot_visible {
                    self.pivot_hit(p, origin, scale)
                } else {
                    None
                };
                let handle = self.resize_handle(p, origin, scale);
                let content = self.content_hit(p, origin, scale);
                let crop_handle = self.crop_handle(p, origin, scale);
                let p = (p - origin) / scale;
                let point = [p.x as f64, p.y as f64];
                let shift = ui.input(|i| i.modifiers.shift);
                if let Some(i) = pivot {
                    self.pivot_drag = Some((i, self.layout.clone()));
                } else if self.edit_content {
                    if let Some(corner) = crop_handle {
                        if let Some(i) = self.layout.selected() {
                            if self.mask_editing() {
                                self.layout.start_mask_drag(i, point, Some(corner));
                            } else {
                                self.layout.start_content_resize(i, corner, point);
                            }
                        }
                    } else if let Some(i) = content {
                        if self.mask_editing() {
                            self.layout.start_mask_drag(i, point, None);
                        } else {
                            self.layout.start_content_drag(i, point);
                        }
                    }
                } else if let Some((i, corner)) = handle {
                    if let Err(e) = self.layout.start_selected_resize(
                        i,
                        corner,
                        point,
                        self.keep_aspect_ratio,
                        self.active_resize_mode(),
                    ) {
                        self.layout.gesture_error = Some(e);
                    }
                } else if self.layout.hit(point).is_none() {
                    let previous = self.layout.selection_mask();
                    let base = if shift {
                        previous.clone()
                    } else {
                        vec![false; self.layout.layers().len()]
                    };
                    self.marquee = Some(Marquee {
                        start: point,
                        end: point,
                        base: base.clone(),
                        previous,
                    });
                    self.layout.set_selection(base);
                } else {
                    self.layout.start_drag(point);
                }
                self.sync();
            }
        }
        self.snap_guides = [None; 2];
        let modifiers = ui.input(|i| i.modifiers);
        if (response.dragged() || response.drag_stopped())
            && let Some((start, world, zoom)) = self.zoom_drag
            && let Some(p) = pointer
        {
            (origin, scale) = self.set_view_zoom(
                zoom * ((p.x - start.x) * 0.01).exp(),
                world,
                start,
                area,
                base,
            );
        }
        if response.clicked()
            && self.navigation_tool == NavigationTool::Zoom
            && !temporary_pan
            && let Some(p) = pointer
        {
            let world = (p - origin) / scale;
            let factor = if modifiers.alt { 1.0 / 1.25 } else { 1.25 };
            (origin, scale) = self.set_view_zoom(self.zoom * factor, world, p, area, base);
        }
        if response.dragged() {
            if self.panning {
                self.pan += response.drag_delta() / scale;
                origin += response.drag_delta();
            } else if (edit_pointer || move_pointer)
                && let Some(p) = pointer
            {
                let p = (p - origin) / scale;
                let point = [p.x as f64, p.y as f64];
                if let Some(i) = self.pivot_drag.as_ref().map(|(i, _)| *i) {
                    self.move_pivot_snapped(i, point, modifiers, scale);
                } else if let Some(m) = self.marquee.as_mut() {
                    m.end = point;
                    self.layout.select_region(m.start, point, &m.base);
                } else {
                    self.snap_guides = self.drag_with_modifiers(point, modifiers, scale);
                }
                self.sync();
            }
        }
        if response.drag_stopped() {
            if (self.layout.dragging() || self.marquee.is_some() || self.pivot_drag.is_some())
                && let Some(p) = pointer
            {
                let p = (p - origin) / scale;
                let point = [p.x as f64, p.y as f64];
                if let Some(i) = self.pivot_drag.as_ref().map(|(i, _)| *i) {
                    self.move_pivot_snapped(i, point, modifiers, scale);
                } else if let Some(m) = self.marquee.as_mut() {
                    m.end = point;
                    self.layout.select_region(m.start, point, &m.base);
                } else {
                    self.snap_guides = self.drag_with_modifiers(point, modifiers, scale);
                }
                self.sync();
            }
            self.layout.finish_drag();
            self.pivot_drag = None;
            self.marquee = None;
            self.panning = false;
            self.zoom_drag = None;
            self.camera_before = None;
        }
        let finish_crop = self.edit_content
            && edit_pointer
            && pivot_hover.is_none()
            && !click_shift_modifier(ui)
            && response.double_clicked_by(egui::PointerButton::Primary);
        if finish_crop {
            self.edit_content = false;
        }
        if response.clicked()
            && edit_pointer
            && self.edit_content
            && pointer.is_some_and(|p| {
                self.content_hit(p, origin, scale).is_none()
                    && self.crop_handle(p, origin, scale).is_none()
            })
        {
            self.edit_content = false;
        }
        if response.double_clicked()
            && let Some(i) = pivot_hover
        {
            self.move_pivot(i, self.layout.layers()[i].center());
        }
        if response.clicked()
            && edit_pointer
            && pivot_hover.is_none()
            && !finish_crop
            && !self.edit_content
            && let Some(p) = pointer
        {
            let handle = self.resize_handle(p, origin, scale);
            let p = (p - origin) / scale;
            let hit = handle
                .map(|(i, _)| i)
                .or_else(|| self.layout.hit([p.x as f64, p.y as f64]));
            if click_shift_modifier(ui) {
                if let Some(i) = hit {
                    self.layout.toggle(i);
                }
            } else {
                if self.resize_mode == ResizeMode::Individual
                    || hit.is_none_or(|i| !self.layout.selection_mask()[i])
                {
                    self.layout.select(hit);
                }
                if response.double_clicked_by(egui::PointerButton::Primary) && hit.is_some() {
                    self.layout.select(hit);
                    self.toggle_crop_edit();
                }
            }
            self.sync();
            ui.ctx().memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
        }
        if response.secondary_clicked()
            && let Some(p) = pointer
        {
            let point = (p - origin) / scale;
            self.layer_ui.context_layer = self
                .content_hit(p, origin, scale)
                .or_else(|| self.layout.hit([point.x as f64, point.y as f64]))
                .map(|i| self.layout.layer_info(i).id);
        }
        if response.secondary_clicked()
            && edit_pointer
            && let Some(p) = pointer
        {
            let point = (p - origin) / scale;
            let hit = self
                .content_hit(p, origin, scale)
                .or_else(|| self.layout.hit([point.x as f64, point.y as f64]));
            if hit.is_none_or(|i| !self.layout.selection_mask()[i]) {
                self.layout.select(hit);
                self.edit_content = false;
                self.sync();
            }
        }
        egui::Popup::context_menu(&response)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| self.layer_context_menu(ui));
        let screen = Ink {
            painter: ui.painter_at(area),
            origin,
            scale: scale * 2.0,
        };
        if !self.references.is_empty() {
            screen.painter.rect_filled(
                Rect::from_min_size(origin, Vec2::new(sw, sh) * scale),
                0.0,
                palette.canvas,
            );
            for reference in &self.references {
                reference.paint(
                    &screen.painter,
                    Rect::from_min_size(origin, Vec2::new(sw, sh) * scale),
                    output,
                );
            }
        } else {
            screen_artwork(&screen, &self.backdrop_title, output);
        }
        if self.snapping.show_grid {
            let bounds = Rect::from_min_size(origin, Vec2::new(sw, sh) * scale);
            let painter = screen
                .painter
                .with_clip_rect(screen.painter.clip_rect().intersect(bounds));
            for (axis, dimension) in output.into_iter().enumerate() {
                for coordinate in self.snapping.grid_guides(dimension, axis) {
                    let endpoints = if axis == 0 {
                        [
                            origin + Vec2::new(coordinate as f32 * scale, 0.0),
                            origin + Vec2::new(coordinate as f32 * scale, sh * scale),
                        ]
                    } else {
                        [
                            origin + Vec2::new(0.0, coordinate as f32 * scale),
                            origin + Vec2::new(sw * scale, coordinate as f32 * scale),
                        ]
                    };
                    painter.line_segment(endpoints, Stroke::new(1.0_f32, palette.grid));
                }
            }
        }
        screen_labels(&screen, output);
        if self.references.is_empty() {
            screen_reference_hint(&screen, output);
        }
        if self.edit_content
            && self.mask_editing()
            && let Some(i) = self.layout.selected()
            && self.layout.formats().masks[i].origin == canvas_pilot::formats::MaskOrigin::Source
            && let Ok(reference) = self
                .layout
                .formats()
                .mask_reference(i, self.layout.layers()[i])
        {
            let rect = Rect::from_min_size(
                origin + Vec2::new(reference.x as f32, reference.y as f32) * scale,
                Vec2::new(reference.width as f32, reference.height as f32) * scale,
            );
            let layer = self.layout.layers()[i];
            let layer_rect = Rect::from_min_size(
                origin + Vec2::new(layer.x as f32, layer.y as f32) * scale,
                Vec2::new(layer.width as f32, layer.height as f32) * scale,
            );
            // Inside the layer, the masked-area helper owns the ghost opacity.
            for strip in format_ui::outside_strips(rect, layer_rect) {
                let clip = strip.intersect(screen.painter.clip_rect());
                if !clip.is_positive() {
                    continue;
                }
                let mut ghost = screen.painter.with_clip_rect(clip);
                ghost.multiply_opacity(0.3);
                format_ui::paint_source(
                    &ghost,
                    rect,
                    self.layout.formats().sources[self.layout.formats().source_for[i]],
                    Default::default(),
                    self.layer_preview(i),
                );
            }
        }
        if self.edit_content
            && !self.mask_editing()
            && let Some(i) = self.layout.selected()
        {
            let g = self.layout.layers()[i];
            let rect = Rect::from_min_size(
                origin + Vec2::new(g.x as f32, g.y as f32) * scale,
                Vec2::new(g.width as f32, g.height as f32) * scale,
            );
            if let Some(fit) = self.layout.formats().layer_fit[i] {
                if let Ok(image) = self.layout.formats().advanced[i]
                    .remaining(self.layout.formats().input(i).unwrap())
                    .and_then(|retained| fit.image(retained, g))
                {
                    let image = Rect::from_min_size(
                        origin + Vec2::new(image.x as f32, image.y as f32) * scale,
                        Vec2::new(image.width as f32, image.height as f32) * scale,
                    );
                    // Do not stack the crop ghost over the masked-area ghost inside
                    // the layer. Keep the usable-source guide outside it and in the reveal.
                    let reveal = format_ui::layer_mask_rect(rect, &self.layout, i)
                        .unwrap_or(Rect::NOTHING)
                        .intersect(rect);
                    for clip in format_ui::outside_strips(screen.painter.clip_rect(), rect)
                        .into_iter()
                        .chain(std::iter::once(reveal))
                    {
                        let clip = clip.intersect(screen.painter.clip_rect());
                        if !clip.is_positive() {
                            continue;
                        }
                        format_ui::paint_one_to_one_helper(
                            &screen.painter.with_clip_rect(clip),
                            image,
                            self.layout.formats().sources[self.layout.formats().source_for[i]],
                            self.layout.formats().advanced[i],
                            self.layer_preview(i),
                        );
                    }
                }
            } else {
                format_ui::paint_content_overflow(
                    &screen.painter,
                    rect,
                    self.layout.formats().sources[self.layout.formats().source_for[i]],
                    self.layout.formats().advanced[i],
                    self.layer_preview(i),
                );
            }
        }
        for i in self.layout.viewport_order() {
            let g = &self.layout.layers()[i];
            if !self.layout.visible(i) {
                continue;
            }
            let color = if self.layout.selection_mask()[i] {
                palette.amber
            } else {
                palette.muted
            };
            let rect = Rect::from_min_size(
                origin + Vec2::new(g.x as f32, g.y as f32) * scale,
                Vec2::new(g.width as f32, g.height as f32) * scale,
            );
            let p = &screen.painter;
            let border = if self.layout.selection_mask()[i] && !self.edit_content {
                Stroke::NONE
            } else {
                Stroke::new(1.0, color)
            };
            p.rect(
                rect,
                0.0,
                if self.outline_layers
                    || self.layout.formats().layer_fit[i].is_some()
                    || self.layout.formats().masks[i] != Default::default()
                {
                    Color32::TRANSPARENT
                } else if self.references.iter().any(|r| r.visible) {
                    Color32::from_rgba_unmultiplied(
                        palette.layer.r(),
                        palette.layer.g(),
                        palette.layer.b(),
                        35,
                    )
                } else {
                    palette.layer
                },
                border,
                StrokeKind::Inside,
            );
            let show_guides = self.edit_content
                && ((self.show_pattern && !self.outline_layers)
                    || self.layout.selected() == Some(i));
            let preview = self.layer_preview(i);
            let show_guides = show_guides
                || (self.layout.formats().layer_fit[i].is_some()
                    && self.layout.selected() == Some(i));
            if show_guides
                || preview.is_some()
                || self.layout.formats().masks[i] != Default::default()
            {
                if self.edit_content && self.layout.selected() == Some(i) {
                    format_ui::paint_mask_helper(p, rect, &self.layout, i, preview);
                }
                format_ui::paint_layer_source(
                    p,
                    rect,
                    &self.layout,
                    i,
                    preview.map(|(texture, opacity)| {
                        (texture, if show_guides { opacity } else { 0.0 })
                    }),
                );
                self.paint_preview_warning(p, rect, i);
                p.rect_stroke(rect, 0.0, border, StrokeKind::Inside);
            }
            if rect.width() > 85.0 && rect.height() > 40.0 {
                let clipped = p.with_clip_rect(area.intersect(rect.shrink(5.0)));
                clipped.text(
                    rect.min + Vec2::splat(12.0),
                    Align2::LEFT_TOP,
                    self.layout.layer_label(i),
                    FontId::proportional(13.0),
                    color,
                );
                if rect.height() > 75.0 {
                    clipped.text(
                        rect.left_bottom() + Vec2::new(12.0, -20.0),
                        Align2::LEFT_BOTTOM,
                        format!(
                            "{} × {} px · {}",
                            number(g.width),
                            number(g.height),
                            ratio_ui::label(g.width / g.height)
                        ),
                        FontId::proportional(12.0),
                        palette.text,
                    );
                }
            }
        }
        viewport_guides::paint(&screen.painter, area, origin, scale, output);
        if !self.edit_content {
            for i in self.layout.selected_indices() {
                let g = self.layout.layers()[i];
                let rect = Rect::from_min_size(
                    origin + Vec2::new(g.x as f32, g.y as f32) * scale,
                    Vec2::new(g.width as f32, g.height as f32) * scale,
                );
                let p = &screen.painter;
                let color = palette.amber;
                let covers = selection_outline::covers(&self.layout, i, origin, scale);
                selection_outline::paint(p, rect, &covers, Stroke::new(1.0, color));
                for corner in [
                    rect.left_top(),
                    rect.right_top(),
                    rect.left_bottom(),
                    rect.right_bottom(),
                    rect.center_top(),
                    rect.center_bottom(),
                    rect.left_center(),
                    rect.right_center(),
                ] {
                    p.rect(
                        Rect::from_center_size(corner, Vec2::splat(7.0)),
                        1.0,
                        palette.panel,
                        Stroke::new(1.0_f32, color),
                        StrokeKind::Inside,
                    );
                }
                p.line_segment(
                    [
                        rect.center() - Vec2::new(7.0, 0.0),
                        rect.center() + Vec2::new(7.0, 0.0),
                    ],
                    Stroke::new(1.0_f32, color),
                );
                p.line_segment(
                    [
                        rect.center() - Vec2::new(0.0, 7.0),
                        rect.center() + Vec2::new(0.0, 7.0),
                    ],
                    Stroke::new(1.0_f32, color),
                );
            }
        }
        if let Some(rect) = self.crop_bounds(origin, scale) {
            let p = &screen.painter;
            let helper_color = if self.mask_editing() {
                Color32::from_rgb(222, 108, 220)
            } else if self
                .layout
                .selected()
                .is_some_and(|i| self.layout.formats().layer_fit[i].is_some())
            {
                Color32::from_rgb(100, 220, 130)
            } else {
                palette.cyan
            };
            if !self.mask_editing()
                && self.layout.dragging()
                && let Some(i) = self.layout.selected()
                && self.layout.formats().layer_fit[i].is_some()
            {
                let g = self.layout.layers()[i];
                let container = Rect::from_min_size(
                    origin + Vec2::new(g.x as f32, g.y as f32) * scale,
                    Vec2::new(g.width as f32, g.height as f32) * scale,
                );
                for strip in format_ui::outside_strips(container, rect) {
                    p.rect_filled(strip, 0.0, Color32::from_black_alpha(110));
                }
            }
            let covers = selection_outline::covers(
                &self.layout,
                self.layout.selected().unwrap(),
                origin,
                scale,
            );
            selection_outline::paint(p, rect, &covers, Stroke::new(1.5, helper_color));
            for (_, corner) in crop_handle_positions(rect) {
                p.rect(
                    Rect::from_center_size(corner, Vec2::splat(9.0)),
                    1.0,
                    palette.panel,
                    Stroke::new(1.5_f32, helper_color),
                    StrokeKind::Inside,
                );
            }
        }
        if pivot_visible || self.pivot_drag.is_some() {
            for i in self.layout.selected_indices() {
                let g = self.layout.layers()[i];
                let relative = self.pivot_for(i);
                let p = origin
                    + Vec2::new(
                        (g.x + relative[0] * g.width) as f32,
                        (g.y + relative[1] * g.height) as f32,
                    ) * scale;
                let stroke = Stroke::new(1.5_f32, palette.cyan);
                screen.painter.circle(p, 6.0, palette.panel, stroke);
                screen
                    .painter
                    .line_segment([p - Vec2::new(10.0, 0.0), p + Vec2::new(10.0, 0.0)], stroke);
                screen
                    .painter
                    .line_segment([p - Vec2::new(0.0, 10.0), p + Vec2::new(0.0, 10.0)], stroke);
            }
        }
        for (axis, guide) in self.snap_guides.iter().enumerate() {
            if let Some(value) = guide {
                let position = if axis == 0 { origin.x } else { origin.y } + *value as f32 * scale;
                let points = if axis == 0 {
                    [
                        Pos2::new(position, area.top()),
                        Pos2::new(position, area.bottom()),
                    ]
                } else {
                    [
                        Pos2::new(area.left(), position),
                        Pos2::new(area.right(), position),
                    ]
                };
                screen
                    .painter
                    .line_segment(points, Stroke::new(1.5_f32, palette.cyan));
            }
        }
        if let Some(m) = &self.marquee {
            let from = origin + Vec2::new(m.start[0] as f32, m.start[1] as f32) * scale;
            let to = origin + Vec2::new(m.end[0] as f32, m.end[1] as f32) * scale;
            screen.painter.rect(
                Rect::from_two_pos(from, to),
                0.0,
                Color32::from_rgba_unmultiplied(
                    palette.amber.r(),
                    palette.amber.g(),
                    palette.amber.b(),
                    25,
                ),
                Stroke::new(1.0_f32, palette.amber),
                StrokeKind::Inside,
            );
        }
        if let Some(i) = self.layout.selected() {
            let g = self.layout.layers()[i];
            if !g.outside_on(self.layout.formats().output) {
                let [dx, dy] = self
                    .position_reference()
                    .offsets_on(g, self.layout.formats().output);
                let reference = self.position_reference();
                let point = reference.layer.point_on(g);
                let from = reference.screen_point(output);
                let edge = origin + Vec2::new(point[0] as f32, point[1] as f32) * scale;
                let boundary = origin + Vec2::new(from[0] as f32, from[1] as f32) * scale;
                let guide = &screen.painter;
                let stroke = Stroke::new(1.0_f32, palette.amber);
                guide.line_segment([edge, Pos2::new(boundary.x, edge.y)], stroke);
                guide.line_segment([edge, Pos2::new(edge.x, boundary.y)], stroke);
                guide.text(
                    Pos2::new((edge.x + boundary.x) / 2.0, edge.y + 6.0),
                    Align2::CENTER_TOP,
                    format!("{dx:.1} px"),
                    FontId::proportional(11.0),
                    palette.amber,
                );
                guide.text(
                    Pos2::new(edge.x + 6.0, (edge.y + boundary.y) / 2.0),
                    Align2::LEFT_CENTER,
                    format!("{dy:.1} px"),
                    FontId::proportional(11.0),
                    palette.amber,
                );
            }
        }
        if show_hints {
            ink.text(
            24.0,
            r.height() - 40.0,
            if move_pointer { "Move tool · Drag anywhere to move the current selection · V: Select · H: Pan" } else if self.navigation_tool == NavigationTool::Pan { "Pan tool · Drag to move the view · V: Select · Z: Zoom · Space/middle-drag pans from any tool" } else if self.navigation_tool == NavigationTool::Zoom { "Zoom tool · Click to zoom in · Alt-click to zoom out · Drag right/left to zoom · V: Select · H: Pan" } else if self.edit_content && self.mask_editing() { "Mask · Enter: finish · Tab: crop · Drag edges to blank · Drag inside layer or use arrows to move window · Shift+arrows: 10 px · Escape cancels" } else if self.edit_content && self.layout.selected().is_some_and(|i| self.layout.formats().layer_fit[i] == Some(canvas_pilot::formats::LayerFit::OneToOne)) { "1:1 edge crop · Tab: mask · Green edges refit on release · Drag picture or use arrows to reposition · Enter, double-click or click outside to finish" } else if self.edit_content && self.layout.selected().is_some_and(|i| self.layout.formats().layer_fit[i].is_some()) { "Edge crop · Drag green edges to trim · Image refits on release · Enter/double-click: finish · Escape: cancel" } else if pivot_visible { "Pivot resize · Drag pivot to position · Double-click to center · Ctrl temporarily toggles pivot" } else if self.edit_content { "Crop mode · Tab: mask · Hold Alt/Option for temporary mask · Drag crop handles to zoom content · Drag image to reposition · Enter, double-click or click outside to finish" } else { match self.active_resize_mode() {
                ResizeMode::Individual => "Corners resize · Side handles keep opposite edge fixed · Shift-click selects",
                ResizeMode::Same => "Same handles · Matching handles scale selected layers proportionally · Escape cancels drag",
                ResizeMode::Mirror => "Mirror horizontally · Left/right mirrored handles · Escape cancels drag",
                ResizeMode::Opposite => "Opposite handles · Opposite corners or edges · Escape cancels drag",
            } },
            12.0,
            palette.muted,
        );
        }
        let missing =
            self.layout.layers().iter().enumerate().any(|(i, g)| {
                self.layout.visible(i) && layer_outside_view(*g, origin, scale, area)
            });
        if show_hints {
            ink.text(
            24.0,
            r.height() - 22.0,
            if missing {
                "Some layers are outside the view · Select them in the list, then Zoom to selection"
            } else {
                "Scroll to zoom · Space-drag to pan · Reference image stays fixed"
            },
            12.0,
            if missing {
                palette.amber
            } else {
                palette.muted
            },
        );
        } else if missing {
            let warning = ui.painter().layout(
                "Some layers are outside the view · Select them in the list, then Zoom to selection".into(),
                FontId::proportional(12.0), palette.amber, (area.width() - 24.0).max(1.0),
            );
            let pos = Pos2::new(area.left() + 12.0, area.bottom() - warning.size().y - 8.0);
            let painter = ui.painter_at(area);
            painter.rect_filled(
                Rect::from_min_size(pos, warning.size()).expand(4.0),
                3.0,
                palette.panel,
            );
            painter.galley(pos, warning, palette.amber);
        }
    }
}
fn layer_outside_view(g: Geometry, origin: Pos2, scale: f32, view: Rect) -> bool {
    let rect = Rect::from_min_size(
        origin + Vec2::new(g.x as f32, g.y as f32) * scale,
        Vec2::new(g.width as f32, g.height as f32) * scale,
    );
    !rect.intersect(view).is_positive()
}

fn output_bounds_warning(g: Geometry, output: [u32; 2]) -> Option<&'static str> {
    if !g.outside_on(output) {
        None
    } else if g.x + g.width <= 0.0
        || g.y + g.height <= 0.0
        || g.x >= output[0] as f64
        || g.y >= output[1] as f64
    {
        Some(
            "Layer is entirely outside the output screen. If unintended, use alignment tools or edit its position.",
        )
    } else {
        Some(
            "Layer is partially outside the output screen. If unintended, use alignment tools or edit its position.",
        )
    }
}

fn crop_handle_positions(rect: Rect) -> [(ResizeHandle, Pos2); 8] {
    ResizeHandle::positions(Geometry {
        x: rect.min.x as f64,
        y: rect.min.y as f64,
        width: rect.width() as f64,
        height: rect.height() as f64,
    })
    .map(|(handle, [x, y])| (handle, Pos2::new(x as f32, y as f32)))
}

// Use the modifiers attached to the click, even if Shift was released later
// in the same input frame.
fn click_shift_modifier(ui: &egui::Ui) -> bool {
    ui.input(|input| {
        input
            .events
            .iter()
            .rev()
            .find_map(|event| match event {
                egui::Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers,
                    ..
                } => Some(modifiers.shift),
                _ => None,
            })
            .unwrap_or(input.modifiers.shift)
    })
}
fn position_point_name(ctx: &egui::Context, point: PositionPoint) -> String {
    tr(
        ctx,
        match point {
            PositionPoint::TopLeft => "anchor.top-left",
            PositionPoint::TopRight => "anchor.top-right",
            PositionPoint::BottomLeft => "anchor.bottom-left",
            PositionPoint::BottomRight => "anchor.bottom-right",
            PositionPoint::Center => "anchor.center",
        },
    )
}

fn position_point_square(ui: &mut egui::Ui, point: &mut PositionPoint, prefix: &str) {
    let palette = theme::palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(104.0, 76.0), egui::Sense::hover());
    let ink = Ink::new(ui, rect);
    ink.card(5.0, 9.0, 94.0, 58.0, palette.field, palette.line);
    for (x, y, candidate) in [
        (13.0, 17.0, PositionPoint::TopLeft),
        (89.0, 17.0, PositionPoint::TopRight),
        (13.0, 59.0, PositionPoint::BottomLeft),
        (89.0, 59.0, PositionPoint::BottomRight),
        (51.0, 38.0, PositionPoint::Center),
    ] {
        ink.rect(
            x - 3.0,
            y - 3.0,
            6.0,
            6.0,
            if *point == candidate {
                palette.cyan
            } else {
                palette.muted
            },
        );
        let response = ui
            .interact(
                Rect::from_center_size(ink.point(x, y), Vec2::splat(20.0)),
                egui::Id::new(format!("{prefix}-{candidate:?}")),
                egui::Sense::click(),
            )
            .on_hover_text(position_point_name(ui.ctx(), candidate));
        if response.clicked() {
            *point = candidate;
        }
    }
}

fn click(
    ui: &egui::Ui,
    ink: &Ink,
    b: [f32; 4],
    id: impl std::hash::Hash + std::fmt::Debug,
) -> bool {
    let clicked = ui
        .interact(
            Rect::from_min_size(ink.point(b[0], b[1]), Vec2::new(b[2], b[3])),
            egui::Id::new(id),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked();
    if clicked {
        ui.ctx().memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
    }
    clicked
}
impl Editor {
    fn ui(&mut self, root_ui: &mut egui::Ui) {
        let ctx = root_ui.ctx().clone();
        self.update_alt_mask(&ctx);
        self.main_ui(root_ui);
        self.language_confirmation(&ctx);
        self.language_pack_confirmation(&ctx);
    }
    fn main_ui(&mut self, root_ui: &mut egui::Ui) {
        let ctx = &root_ui.ctx().clone();
        if self.fields_edited
            && self.fields_pending()
            && !self.layout.dragging()
            && ctx.input(|input| {
                input.pointer.any_pressed()
                    && input
                        .pointer
                        .interact_pos()
                        .is_some_and(|p| !self.field_rects.iter().any(|r| r.contains(p)))
            })
        {
            self.apply_fields();
        }
        self.poll_project_job();
        let blocked = self.files.loading.is_some()
            || self.files.new_requested
            || self.files.new_after_save
            || self.files.new_ready
            || (self.fields_edited && self.fields_pending())
            || self
                .format_draft
                .as_ref()
                .is_some_and(|d| d.pending(&self.layout))
            || self
                .output_draft
                .as_ref()
                .is_some_and(|d| d.pending(&self.layout))
            || (self.layer_ui.open && self.names_pending())
            || self.layer_ui.rename.is_some()
            || self.layer_ui.folder_rename.is_some();
        self.poll_reference();
        self.software_integrations_window(ctx);
        if let Some(reference) = self.document_reference.show(ctx) {
            self.add_reference(reference);
        }
        let palette = theme::palette(ctx);
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if self.files.loading.is_none() {
            let mut geometry = Vec::new();
            for file in &dropped {
                if Self::geometry_drop(file) && !self.reference_open {
                    geometry.push(file.clone());
                } else {
                    self.load_reference(ctx, file);
                }
            }
            if !geometry.is_empty() {
                if self.paste.is_none() {
                    self.open_paste(String::new());
                }
                Self::read_geometry_files(geometry, self.paste.as_mut().unwrap());
            }
        }
        if !blocked {
            self.shortcuts(ctx);
        }
        egui::Panel::top("header")
            .exact_size(44.0)
            .frame(egui::Frame::NONE.fill(palette.panel))
            .show(root_ui, |ui| {
                if blocked {
                    ui.disable();
                }
                let r = ui.max_rect();
                let ink = Ink::new(ui, r);
                ink.text(
                    24.0,
                    11.0,
                    &tr(ink.painter.ctx(), "ui.midra-layout"),
                    18.0,
                    palette.text,
                );
                ink.line([206.0, 13.0], [206.0, 31.0], palette.line);
                ink.text(
                    222.0,
                    15.0,
                    &tr(ink.painter.ctx(), "ui.layer-positioning-helper"),
                    13.0,
                    palette.muted,
                );
                ink.painter.text(
                    ink.point(r.width() - 24.0, 22.0),
                    egui::Align2::RIGHT_CENTER,
                    "EIKOS 4K",
                    egui::FontId::proportional(12.0),
                    palette.cyan,
                );
            });
        egui::Panel::bottom("status")
            .exact_size(34.0)
            .frame(egui::Frame::NONE.fill(palette.panel))
            .show(root_ui, |ui| {
                if blocked {
                    ui.disable();
                }
                let row = ui.max_rect().shrink2(Vec2::new(18.0, 0.0));
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(row)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        ui.spacing_mut().interact_size.y = 24.0;
                        let online = self.device.online();
                        let status_color = if online { palette.cyan } else { palette.muted };
                        let (dot, _) =
                            ui.allocate_exact_size(Vec2::splat(6.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot.center(), 3.0, status_color);
                        ui.label(
                            egui::RichText::new(if online { "ONLINE" } else { "OFFLINE" })
                                .size(11.0)
                                .color(status_color),
                        );
                        self.device_apply_button(ui);
                        self.device_load_button(ui);
                        self.device_send_button(ui);
                        self.device_scope_picker(ui);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!(
                                        "Screen pixels / {} × {}",
                                        self.layout.formats().output[0],
                                        self.layout.formats().output[1]
                                    ))
                                    .size(12.0)
                                    .color(palette.muted),
                                )
                                .truncate(),
                            );
                        });
                    },
                );
            });
        egui::Panel::right("inspector")
            .exact_size(392.0)
            .resizable(false)
            .frame(egui::Frame::NONE.fill(palette.panel).inner_margin(24.0))
            .show(root_ui, |ui| {
                if blocked {
                    ui.disable();
                }
                egui::ScrollArea::vertical()
                    .scroll_source(egui::scroll_area::ScrollSource {
                        drag: egui::scroll_area::DragScroll::Never,
                        ..Default::default()
                    })
                    .show(ui, |ui| self.inspector(ui));
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(palette.bg))
            .show(root_ui, |ui| {
                if blocked {
                    ui.disable();
                }
                self.workspace(ui)
            });
        if self.files.loading.is_some() {
            self.project_windows(ctx);
            return;
        }
        self.paste_window(ctx);
        self.reference_window(ctx);
        self.output_window(ctx);
        self.formats_window(ctx);
        self.capture_window(ctx);
        self.project_windows(ctx);
        self.new_project_window(ctx);
        self.preferences_window(ctx);
        self.device_window(ctx);
        self.inputs_window(ctx);
        self.layers_window(ctx);
        self.theme_window(ctx);
    }
}

impl eframe::App for Editor {
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        self.crop_mask_key_input(ctx, input);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.ui(ui);
        if !ctx.input(|i| i.pointer.any_down())
            && !self.layout.dragging()
            && self.pivot_drag.is_none()
        {
            self.autosave_recovery(false);
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(2));
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.layout.cancel_drag();
        if let Some((_, before)) = self.pivot_drag.take() {
            self.layout = before;
        }
        self.autosave_recovery(true);
    }
}

fn alignment_icon(
    ui: &mut egui::Ui,
    rect: Rect,
    title: &str,
    horizontal: Option<Alignment>,
    vertical: Option<Alignment>,
) -> egui::Response {
    let palette = theme::palette(ui.ctx());
    let response = ui.interact(rect, ui.id().with(title), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), title)
    });
    let p = ui.painter();
    p.rect(
        rect,
        3.0,
        if response.is_pointer_button_down_on() {
            palette.purple
        } else if response.hovered() {
            palette.panel
        } else {
            palette.field
        },
        Stroke::new(1.0_f32, palette.line),
        StrokeKind::Inside,
    );
    let c = rect.center();
    let stroke = Stroke::new(
        1.6_f32,
        if response.hovered() || response.has_focus() {
            palette.cyan
        } else {
            palette.text
        },
    );
    let x = match horizontal {
        Some(Alignment::Left) => -6.0,
        Some(Alignment::Right) => 6.0,
        _ => 0.0,
    };
    let y = match vertical {
        Some(Alignment::Top) => -6.0,
        Some(Alignment::Bottom) => 6.0,
        _ => 0.0,
    };
    // The standalone centering icons indicate the direction of movement.
    let (horizontal, vertical) = match (horizontal, vertical) {
        (Some(Alignment::HorizontalCenter), None) => (None, Some(Alignment::VerticalCenter)),
        (None, Some(Alignment::VerticalCenter)) => (Some(Alignment::HorizontalCenter), None),
        axes => axes,
    };
    for (axis, value) in [(0, horizontal), (1, vertical)] {
        if let Some(value) = value {
            let offset = match value {
                Alignment::Left | Alignment::Top => -12.0,
                Alignment::Right | Alignment::Bottom => 12.0,
                _ => 0.0,
            };
            let ends = if axis == 0 {
                [c + Vec2::new(offset, -12.0), c + Vec2::new(offset, 12.0)]
            } else {
                [c + Vec2::new(-12.0, offset), c + Vec2::new(12.0, offset)]
            };
            p.line_segment(ends, stroke);
        }
    }
    p.rect(
        Rect::from_center_size(c + Vec2::new(x, y), Vec2::splat(8.0)),
        0.0,
        palette.field,
        stroke,
        StrokeKind::Inside,
    );
    response.on_hover_text(title)
}

// Closing a draft is a separate decision from sending anything to a device.
#[derive(Clone, Copy, PartialEq)]
enum CloseChoice {
    Apply,
    Discard,
    Cancel,
}
fn pending_close(ctx: &egui::Context, id: &str, error: &str) -> Option<CloseChoice> {
    egui::Modal::new(egui::Id::new(id))
        .show(ctx, |ui| {
            ui.heading(tr(ui.ctx(), "ui.would-you-like-to-apply-changes"));
            if !error.is_empty() {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.horizontal(|ui| {
                if apply_button(ui, &tr(ui.ctx(), "action.apply"), true).clicked() {
                    Some(CloseChoice::Apply)
                } else if ui.button(tr(ui.ctx(), "ui.discard")).clicked() {
                    Some(CloseChoice::Discard)
                } else if ui.button(tr(ui.ctx(), "ui.cancel")).clicked() {
                    Some(CloseChoice::Cancel)
                } else {
                    None
                }
            })
            .inner
        })
        .inner
}

fn apply_button(ui: &mut egui::Ui, label: &str, pending: bool) -> egui::Response {
    let palette = theme::palette(ui.ctx());
    let button = egui::Button::new(label)
        .min_size(ui.spacing().interact_size)
        .fill(if pending {
            palette.purple
        } else {
            palette.field
        });
    ui.add_enabled(pending, button)
}

#[derive(Clone, Copy)]
enum ToolbarIcon {
    AddLayer,
    AddFolder,
    AddImage,
    ManageLayers,
    Select,
    Move,
    Pan,
    Zoom,
    Output,
    Screen,
    Selection,
    Undo,
    Redo,
}

// Keep related tools together when wrapping; reserve caption space per group.
fn toolbar_groups(width: f32, captions: bool) -> ([Rect; 19], Vec<Rect>, f32) {
    let mut buttons = [Rect::NOTHING; 19];
    let mut groups = Vec::new();
    let mut x = 24.0;
    let mut y = 20.0;
    let height = if captions { 58.0 } else { 36.0 };
    for slots in [
        &[15, 0, 1, 18][..],
        &[2, 3],
        &[4, 5],
        &[6, 7, 8, 9],
        &[10, 11, 16, 12],
        &[13, 14],
        &[17],
    ] {
        let cell_width = |slot| match slot {
            0 | 15 => 68.0,
            11 => 56.0,
            8 | 9 | 17 if captions => 80.0,
            _ if captions => 56.0,
            1 | 10 | 14 | 18 => 44.0,
            17 => 34.0,
            _ => 36.0,
        };
        let group_width = slots.iter().map(|&slot| cell_width(slot)).sum::<f32>() + 8.0;
        if x > 24.0 && x + group_width > width - 16.0 {
            x = 24.0;
            y += height + 8.0;
        }
        groups.push(Rect::from_min_size(
            Pos2::new(x, y),
            Vec2::new(group_width, 36.0),
        ));
        let mut cursor = x + 4.0;
        for &slot in slots {
            let cell = cell_width(slot);
            let button_width = match slot {
                0 | 15 => 64.0,
                11 => 52.0,
                1 | 10 | 14 | 18 => 40.0,
                _ => 32.0,
            };
            buttons[slot] = Rect::from_min_size(
                Pos2::new(cursor + (cell - button_width) / 2.0, y + 2.0),
                Vec2::new(button_width, 32.0),
            );
            cursor += cell;
        }
        x += group_width + 10.0;
    }
    (buttons, groups, y + height)
}

// Inward/outward arrows share the same open box for a recognizable pair.
fn paint_transfer_icon(ui: &egui::Ui, response: &egui::Response, exporting: bool) {
    let palette = theme::palette(ui.ctx());
    let color = if response.hovered() || response.has_focus() {
        palette.cyan
    } else {
        palette.text
    };
    let stroke = Stroke::new(1.8, color);
    let c = response.rect.center();
    let p = ui.painter();
    p.add(egui::Shape::line(
        vec![
            c + Vec2::new(-1.0, -5.0),
            c + Vec2::new(-1.0, -10.0),
            c + Vec2::new(10.0, -10.0),
            c + Vec2::new(10.0, 10.0),
            c + Vec2::new(-1.0, 10.0),
            c + Vec2::new(-1.0, 5.0),
        ],
        stroke,
    ));
    p.line_segment([c + Vec2::new(-11.0, 0.0), c + Vec2::new(5.0, 0.0)], stroke);
    let (tip, tail) = if exporting { (-11.0, -6.0) } else { (5.0, 0.0) };
    p.add(egui::Shape::line(
        vec![
            c + Vec2::new(tail, -5.0),
            c + Vec2::new(tip, 0.0),
            c + Vec2::new(tail, 5.0),
        ],
        stroke,
    ));
}

impl ToolbarIcon {
    fn title_key(self) -> &'static str {
        match self {
            Self::AddLayer => "ui.add-layer",
            Self::AddFolder => "toolbar.addfolder",
            Self::AddImage => "toolbar.addimage",
            Self::ManageLayers => "toolbar.managelayers",
            Self::Select => "toolbar.select",
            Self::Move => "toolbar.move",
            Self::Pan => "toolbar.pan",
            Self::Zoom => "toolbar.zoom",
            Self::Output => "toolbar.output",
            Self::Screen => "toolbar.screen",
            Self::Selection => "toolbar.selection",
            Self::Undo => "toolbar.undo",
            Self::Redo => "toolbar.redo",
        }
    }
}

const SNAP_MENU_TITLE: &str = "          ";

// Match the original toolbar's dark surfaces without changing popup controls.
fn toolbar_menu<R>(
    ui: &mut egui::Ui,
    title: &str,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    let palette = theme::palette(ui.ctx());
    let original = ui.style().clone();
    let widgets = &mut ui.style_mut().visuals.widgets;
    for (visuals, fill, border) in [
        (&mut widgets.inactive, palette.field, palette.line),
        (
            &mut widgets.hovered,
            original.visuals.widgets.hovered.bg_fill,
            palette.cyan,
        ),
        (&mut widgets.active, palette.purple, palette.cyan),
        (&mut widgets.open, palette.panel, palette.cyan),
    ] {
        visuals.bg_fill = fill;
        visuals.weak_bg_fill = fill;
        visuals.bg_stroke = if border == palette.line {
            Stroke::NONE
        } else {
            Stroke::new(1.0_f32, border)
        };
        visuals.corner_radius = egui::CornerRadius::same(4);
        visuals.fg_stroke.color = if border == palette.cyan {
            palette.cyan
        } else {
            palette.text
        };
    }
    let (response, inner) = egui::containers::menu::MenuButton::new(title)
        .config(
            egui::containers::menu::MenuConfig::new()
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
        )
        .ui(ui, |ui| {
            ui.set_style(original.clone());
            // The toolbar's 34 px hit targets should not inflate dropdown rows.
            let spacing = ui.spacing_mut();
            spacing.interact_size.y = 22.0;
            spacing.button_padding.y = 2.0;
            spacing.item_spacing.y = 4.0;
            contents(ui)
        });
    ui.set_style(original);
    egui::InnerResponse {
        response,
        inner: inner.map(|response| response.inner),
    }
}

fn layer_state_icon(
    ui: &mut egui::Ui,
    rect: Rect,
    id: egui::Id,
    lock: bool,
    active: bool,
) -> egui::Response {
    let label = match (lock, active) {
        (true, true) => "Unlock layer",
        (true, false) => "Lock layer",
        (false, true) => "Show layer",
        (false, false) => "Hide layer",
    };
    state_icon(ui, rect, id, lock, active, label)
}

fn state_icon(
    ui: &mut egui::Ui,
    rect: Rect,
    id: egui::Id,
    lock: bool,
    active: bool,
    label: &str,
) -> egui::Response {
    let palette = theme::palette(ui.ctx());
    let response = ui.interact(rect, id, egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let painter = ui.painter();
    if active || response.hovered() {
        painter.rect_filled(
            rect,
            4.0,
            if active {
                palette.purple
            } else {
                palette.panel
            },
        );
    }
    let stroke = Stroke::new(
        1.5_f32,
        if active || response.hovered() {
            palette.cyan
        } else {
            palette.text
        },
    );
    let c = rect.center();
    let pt = |x, y| c + Vec2::new(x, y);
    if lock {
        painter.rect_stroke(
            Rect::from_min_max(pt(-6.0, -1.0), pt(6.0, 8.0)),
            2.0,
            stroke,
            StrokeKind::Inside,
        );
        let points = if active {
            vec![
                pt(-4.0, -1.0),
                pt(-4.0, -6.0),
                pt(0.0, -8.0),
                pt(4.0, -6.0),
                pt(4.0, -1.0),
            ]
        } else {
            vec![pt(-4.0, -1.0), pt(-4.0, -6.0), pt(0.0, -8.0), pt(4.0, -6.0)]
        };
        painter.add(egui::Shape::line(points, stroke));
        painter.circle_filled(pt(0.0, 3.0), 1.5, stroke.color);
    } else {
        painter.add(egui::Shape::closed_line(
            vec![
                pt(-9.0, 0.0),
                pt(-4.0, -5.0),
                pt(4.0, -5.0),
                pt(9.0, 0.0),
                pt(4.0, 5.0),
                pt(-4.0, 5.0),
            ],
            stroke,
        ));
        painter.circle_stroke(c, 2.5, stroke);
        if active {
            painter.line_segment([pt(-9.0, 8.0), pt(9.0, -8.0)], stroke);
        }
    }
    response.on_hover_text(label)
}

fn edit_tool_icon(
    ui: &mut egui::Ui,
    rect: Rect,
    grid: Option<u32>,
    label: &str,
    selected: bool,
    effective: bool,
) -> egui::Response {
    let palette = theme::palette(ui.ctx());
    let response = ui.interact(
        rect,
        egui::Id::new(("edit-tool", label)),
        egui::Sense::click(),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let p = ui.painter();
    p.rect(
        rect,
        4.0,
        if effective {
            palette.purple
        } else {
            palette.field
        },
        Stroke::new(1.0_f32, if selected { palette.cyan } else { palette.line }),
        StrokeKind::Inside,
    );
    let c = rect.center();
    let stroke = Stroke::new(
        1.3_f32,
        if effective || response.hovered() {
            palette.cyan
        } else {
            palette.text
        },
    );
    if let Some(spacing) = grid {
        for offset in [-7.0, 0.0, 7.0] {
            p.line_segment(
                [c + Vec2::new(offset, -12.0), c + Vec2::new(offset, 2.0)],
                stroke,
            );
            p.line_segment(
                [
                    c + Vec2::new(-7.0, offset - 5.0),
                    c + Vec2::new(7.0, offset - 5.0),
                ],
                stroke,
            );
        }
        p.text(
            c + Vec2::new(0.0, 3.0),
            Align2::CENTER_TOP,
            spacing.to_string(),
            FontId::proportional(10.0),
            stroke.color,
        );
    } else {
        p.circle_stroke(c, 7.0, stroke);
        p.line_segment([c - Vec2::new(11.0, 0.0), c + Vec2::new(11.0, 0.0)], stroke);
        p.line_segment([c - Vec2::new(0.0, 11.0), c + Vec2::new(0.0, 11.0)], stroke);
        p.circle_filled(c, 2.0, stroke.color);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn toolbar_icon(ui: &mut egui::Ui, rect: Rect, label: &str, icon: ToolbarIcon) -> egui::Response {
    let palette = theme::palette(ui.ctx());
    let id = if matches!(
        icon,
        ToolbarIcon::Select | ToolbarIcon::Move | ToolbarIcon::Pan | ToolbarIcon::Zoom
    ) {
        egui::Id::new(("navigation-tool", label))
    } else {
        ui.id().with(("toolbar", label))
    };
    let response = ui.interact(rect, id, egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            tr(ui.ctx(), icon.title_key()),
        )
    });
    let painter = ui.painter();
    let focused = response.hovered() || response.has_focus();
    if focused || response.is_pointer_button_down_on() {
        painter.rect_filled(
            rect.shrink(1.0),
            4.0,
            if response.is_pointer_button_down_on() {
                palette.purple
            } else {
                palette.panel
            },
        );
    }
    let stroke = Stroke::new(1.6_f32, if focused { palette.cyan } else { palette.text });
    let c = rect.center();
    let point = |x, y| c + Vec2::new(x, y);
    match icon {
        ToolbarIcon::AddLayer => {
            painter.rect_stroke(
                Rect::from_min_max(point(-9.0, -8.0), point(5.0, 5.0)),
                1.0,
                stroke,
                StrokeKind::Inside,
            );
            painter.line_segment([point(-6.0, 8.0), point(0.0, 8.0)], stroke);
            painter.line_segment(
                [point(6.0, 2.0), point(6.0, 12.0)],
                Stroke::new(1.6, palette.cyan),
            );
            painter.line_segment(
                [point(1.0, 7.0), point(11.0, 7.0)],
                Stroke::new(1.6, palette.cyan),
            );
        }
        ToolbarIcon::AddFolder | ToolbarIcon::AddImage => {
            if matches!(icon, ToolbarIcon::AddFolder) {
                painter.add(egui::Shape::closed_line(
                    vec![
                        point(-10.0, 7.0),
                        point(-10.0, -8.0),
                        point(-3.0, -8.0),
                        point(0.0, -5.0),
                        point(9.0, -5.0),
                        point(9.0, 7.0),
                    ],
                    stroke,
                ));
            } else {
                painter.rect_stroke(
                    Rect::from_min_max(point(-10.0, -8.0), point(9.0, 7.0)),
                    2.0,
                    stroke,
                    StrokeKind::Inside,
                );
                painter.circle_filled(point(3.0, -3.0), 1.8, stroke.color);
                painter.add(egui::Shape::line(
                    vec![point(-8.0, 4.0), point(-3.0, -1.0), point(2.0, 4.0)],
                    stroke,
                ));
            }
            painter.circle_filled(point(7.0, 5.0), 5.5, palette.panel);
            let accent = Stroke::new(1.6_f32, palette.cyan);
            painter.line_segment([point(3.0, 5.0), point(11.0, 5.0)], accent);
            painter.line_segment([point(7.0, 1.0), point(7.0, 9.0)], accent);
        }
        ToolbarIcon::ManageLayers => {
            for (y, knob) in [(-6.0, -3.0), (0.0, 4.0), (6.0, -1.0)] {
                painter.line_segment([point(-9.0, y), point(9.0, y)], stroke);
                painter.circle_filled(point(knob, y), 2.5, palette.panel);
                painter.circle_stroke(point(knob, y), 2.5, Stroke::new(1.6_f32, palette.cyan));
            }
        }

        ToolbarIcon::Select => {
            painter.add(egui::Shape::closed_line(
                vec![
                    point(-7.0, -11.0),
                    point(-7.0, 10.0),
                    point(-1.0, 4.0),
                    point(4.0, 12.0),
                    point(8.0, 9.0),
                    point(3.0, 2.0),
                    point(11.0, 1.0),
                ],
                stroke,
            ));
        }
        ToolbarIcon::Move => {
            for axis in [Vec2::X, Vec2::Y] {
                painter.line_segment([c - axis * 11.0, c + axis * 11.0], stroke);
                let side = Vec2::new(-axis.y, axis.x);
                for sign in [-1.0, 1.0] {
                    let tip = c + axis * 11.0 * sign;
                    for direction in [-1.0, 1.0] {
                        painter.line_segment(
                            [tip, tip - axis * 4.0 * sign + side * 4.0 * direction],
                            stroke,
                        );
                    }
                }
            }
        }
        ToolbarIcon::Pan => {
            painter.add(egui::Shape::line(
                vec![
                    point(-8.0, 0.0),
                    point(-11.0, -3.0),
                    point(-13.0, -1.0),
                    point(-7.0, 9.0),
                    point(5.0, 9.0),
                    point(9.0, 2.0),
                    point(9.0, -7.0),
                    point(5.0, -7.0),
                    point(5.0, -11.0),
                    point(1.0, -11.0),
                    point(1.0, -12.0),
                    point(-3.0, -12.0),
                    point(-3.0, -9.0),
                    point(-7.0, -9.0),
                    point(-8.0, 0.0),
                ],
                stroke,
            ));
        }
        ToolbarIcon::Zoom => {
            painter.circle_stroke(c - Vec2::splat(3.0), 7.0, stroke);
            painter.line_segment([point(2.0, 2.0), point(11.0, 11.0)], stroke);
            painter.line_segment([point(-7.0, -3.0), point(1.0, -3.0)], stroke);
            painter.line_segment([point(-3.0, -7.0), point(-3.0, 1.0)], stroke);
        }

        ToolbarIcon::Output => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::new(22.0, 18.0)),
                2.0,
                stroke,
                StrokeKind::Inside,
            );
            for (y, knob) in [(-4.0, -3.0), (4.0, 3.0)] {
                painter.line_segment([point(-7.0, y), point(7.0, y)], stroke);
                painter.circle_filled(point(knob, y), 2.5, palette.field);
                painter.circle_stroke(point(knob, y), 2.5, stroke);
            }
        }
        ToolbarIcon::Screen => {
            painter.rect_stroke(
                Rect::from_center_size(point(0.0, -2.0), Vec2::new(20.0, 14.0)),
                2.0,
                stroke,
                StrokeKind::Inside,
            );
            painter.line_segment([point(0.0, 5.0), point(0.0, 9.0)], stroke);
            painter.line_segment([point(-5.0, 9.0), point(5.0, 9.0)], stroke);
        }
        ToolbarIcon::Selection => {
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                painter.add(egui::Shape::line(
                    vec![
                        point(x * 5.0, y * 10.0),
                        point(x * 10.0, y * 10.0),
                        point(x * 10.0, y * 5.0),
                    ],
                    stroke,
                ));
            }
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::new(8.0, 6.0)),
                0.0,
                stroke,
                StrokeKind::Inside,
            );
        }
        ToolbarIcon::Undo | ToolbarIcon::Redo => {
            let sign = if matches!(icon, ToolbarIcon::Undo) {
                1.0
            } else {
                -1.0
            };
            let points = [
                (-8.0, -4.0),
                (-3.0, -7.0),
                (3.0, -7.0),
                (7.0, -4.0),
                (9.0, 0.0),
                (8.0, 5.0),
                (4.0, 8.0),
                (0.0, 8.0),
            ];
            painter.add(egui::Shape::line(
                points
                    .into_iter()
                    .map(|(x, y)| point(x * sign, y))
                    .collect(),
                stroke,
            ));
            painter.add(egui::Shape::line(
                vec![
                    point(-8.0 * sign, -10.0),
                    point(-8.0 * sign, -4.0),
                    point(-2.0 * sign, -4.0),
                ],
                stroke,
            ));
        }
    }
    response.on_hover_text(match icon {
        ToolbarIcon::AddLayer => "Add a layer",
        ToolbarIcon::AddFolder => "Create an empty folder",
        ToolbarIcon::AddImage => "Add reference image",
        ToolbarIcon::ManageLayers => "Manage layers and folders",
        ToolbarIcon::Select => "Select (V) — select, move and resize layers; drag empty space to select. Grid and Pivot toggles still apply.",
        ToolbarIcon::Move => "Move (M) — drag anywhere to move the current selection without selecting, deselecting or resizing. In crop mode, moves content only.",
        ToolbarIcon::Pan => "Pan (H) — drag to move the view. Hold Space or middle-drag to pan temporarily from any tool.",
        ToolbarIcon::Zoom => "Zoom (Z) — click to zoom in, Alt-click to zoom out; drag right/left to zoom around the starting point. Scroll zoom works from any tool.",
        ToolbarIcon::Output => "Output format — screen ratio and resolution",
        ToolbarIcon::Screen => "Fit screen — show the entire output",
        ToolbarIcon::Selection => "Zoom to selection — frame the selected layers",
        ToolbarIcon::Undo => "Undo (Ctrl/Cmd+Z)",
        ToolbarIcon::Redo => "Redo (Ctrl/Cmd+Shift+Z)",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn frame(
        app: &mut Editor,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        // Interaction tests address final widget positions, not animation frames.
        ctx.global_style_mut(|style| style.animation_time = 0.0);
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 940.0))),
            events,
            ..Default::default()
        };
        app.crop_mask_key_input(ctx, &mut input);
        ctx.run_ui(input, |ui| app.ui(ui))
    }
    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }
    pub(super) fn cancel_pending_close(app: &mut Editor, ctx: &egui::Context) {
        frame(app, ctx, vec![]);
        let output = frame(app, ctx, vec![]);
        // The underlying editor can also have a Cancel button. Click the modal.
        let pos = output
            .shapes
            .iter()
            .rev()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "Cancel" => {
                    Some(t.pos + t.galley.size() / 2.)
                }
                _ => None,
            })
            .expect("Close-confirmation Cancel button");
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(app, ctx, vec![mouse(pos, false)]);
    }
    pub(super) fn click_text(app: &mut Editor, ctx: &egui::Context, text: &str) {
        frame(app, ctx, vec![]);
        let output = frame(app, ctx, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t)
                    if t.galley.job.text == text
                        || (!text.trim().is_empty() && t.galley.job.text.trim() == text.trim()) =>
                {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Button not rendered: {text}"));
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(app, ctx, vec![mouse(pos, false)]);
    }
    #[test]
    fn source_mask_guide_can_be_hit_outside_layer() {
        let mut app = Editor::default();
        app.layout.select(Some(0));
        app.layout
            .set_framing(
                0,
                canvas_pilot::formats::Framing {
                    zoom: 2.0,
                    offset: [0.0; 2],
                },
            )
            .unwrap();
        app.layout
            .set_mask(
                0,
                canvas_pilot::formats::Mask {
                    origin: canvas_pilot::formats::MaskOrigin::Source,
                    ..Default::default()
                },
            )
            .unwrap();
        app.edit_content = true;
        app.edit_mask = true;
        let g = app.layout.layers()[0];
        let guide = app.crop_bounds(Pos2::ZERO, 1.0).unwrap();
        assert!(guide.width() > g.width as f32);
        let point = guide.left_center() + Vec2::new(1.0, 0.0);
        assert!(point.x < g.x as f32);
        assert_eq!(app.content_hit(point, Pos2::ZERO, 1.0), Some(0));
        let ctx = egui::Context::default();
        app.layout
            .set_mask(
                0,
                canvas_pilot::formats::Mask {
                    origin: canvas_pilot::formats::MaskOrigin::Source,
                    left: 0.2,
                    ..Default::default()
                },
            )
            .unwrap();
        frame(&mut app, &ctx, vec![]);
        let reset = ctx
            .read_response(egui::Id::new("reset-visual-crop"))
            .unwrap()
            .rect
            .center();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(reset), mouse(reset, true)],
        );
        frame(&mut app, &ctx, vec![mouse(reset, false)]);
        assert!(app.layout.formats().masks[0].is_empty());
        assert_eq!(
            app.layout.formats().masks[0].origin,
            canvas_pilot::formats::MaskOrigin::Source
        );
    }

    #[test]
    fn pivot_snap_modifiers_and_linked_markers_preserve_geometry() {
        let mut app = Editor {
            resize_mode: ResizeMode::Mirror,
            pivot_tool: true,
            grid_tool: Some(100),
            ..Default::default()
        };
        app.layout.set_selection([true; 2]);
        app.sync();
        let before = app.layout.layers().clone();
        app.move_pivot_snapped(0, [122.0, 361.0], egui::Modifiers::NONE, 1.0);
        assert_eq!(app.pivot_for(0), [0.0, 0.5]);
        assert_eq!(app.pivot_for(1), [1.0, 0.5]);
        assert_eq!(app.snap_guides, [Some(120.0), Some(360.0)]);
        app.move_pivot_snapped(0, [833.0, 653.0], egui::Modifiers::NONE, 1.0);
        assert_eq!(app.snap_guides, [Some(800.0), Some(700.0)]);
        app.move_pivot_snapped(
            0,
            [833.0, 653.0],
            egui::Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            },
            1.0,
        );
        assert_eq!(app.snap_guides, [None; 2]);
        app.snapping.enabled = true;
        app.move_pivot_snapped(
            0,
            [122.0, 361.0],
            egui::Modifiers {
                alt: true,
                ..Default::default()
            },
            1.0,
        );
        assert_eq!(app.snap_guides, [None; 2]);
        assert_ne!(app.pivot_for(0), [0.0, 0.5]);
        assert_eq!(app.layout.layers(), &before);
    }

    #[test]
    fn pivot_snap_survives_pointer_release_and_escape_restores_placement() {
        let ctx = egui::Context::default();
        let mut app = Editor {
            pivot_tool: true,
            ..Default::default()
        };
        let before = app.layout.layers().clone();
        let output = frame(&mut app, &ctx, vec![]);
        let rect = layer_rects(&output)[1];
        let start = rect.center();
        let end = rect.left_top() + Vec2::new(2.0, 1.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        assert_eq!(app.pivot_for(1), [0.0; 2]);
        frame(&mut app, &ctx, vec![mouse(end, false)]);
        assert_eq!(app.pivot_for(1), [0.0; 2]);
        let end2 = rect.right_center() - Vec2::new(2.0, 1.0);
        frame(&mut app, &ctx, vec![mouse(end, true)]);
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end2)]);
        assert_eq!(app.pivot_for(1), [1.0, 0.5]);
        frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
        frame(&mut app, &ctx, vec![mouse(end2, false)]);
        assert_eq!(app.pivot_for(1), [0.0; 2]);
        assert_eq!(app.layout.layers(), &before);
    }

    #[test]
    fn optional_layer_point_changes_measurements_only_and_center_survives_size_edits() {
        let ctx = egui::Context::default();
        let mut app = Editor {
            corner: PositionPoint::TopLeft,
            ..Default::default()
        };
        app.sync();
        let before = app.layout.layers().clone();
        open_measurement_menu(&mut app, &ctx);
        let from = ctx.read_response(egui::Id::new("corner-Center")).unwrap();
        let to = ctx
            .read_response(egui::Id::new("layer-point-Center"))
            .unwrap();
        assert_eq!(from.rect.center().y, to.rect.center().y);
        assert!(from.rect.right() < to.rect.left());
        assert!(!to.enabled());
        click_text(&mut app, &ctx, "Use a different measure point");
        click_measurement_point(&mut app, &ctx, "layer-point-Center");
        assert_eq!(app.position_layer_point, Some(PositionPoint::Center));
        assert_eq!(app.layout.layers(), &before);
        let center = before[1].center();
        assert_eq!(app.fields[0], number(center[0]));
        assert_eq!(app.fields[1], number(center[1]));
        app.fields[2] = "400".into();
        app.apply_fields();
        assert_eq!(app.layout.layers()[1].center(), center);
        let resized = app.layout.layers().clone();
        click_text(&mut app, &ctx, "Use a different measure point");
        assert!(app.position_layer_point.is_none());
        assert_eq!(app.layout.layers(), &resized);
        assert_eq!(app.fields[0], number(resized[1].x));
    }

    #[test]
    fn interaction_footer_collapses_and_returns_without_hiding_output_labels_or_warnings() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        for (size, hints) in [
            (Vec2::new(1440.0, 940.0), true),
            (Vec2::new(1440.0, 720.0), false),
            (Vec2::new(1000.0, 940.0), false),
            (Vec2::new(1440.0, 940.0), true),
        ] {
            let _ = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ui| app.ui(ui),
            );
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ui| app.ui(ui),
            );
            for prefix in ["Corners resize", "Scroll to zoom"] {
                assert_eq!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().starts_with(prefix))), hints);
            }
            let canvas = ctx.read_response(egui::Id::new("canvas")).unwrap().rect;
            assert_eq!(
                canvas.bottom(),
                size.y - 34.0 - if hints { 48.0 } else { 8.0 }
            );
            for label in ["SCREEN 1", "1920 × 1080 px"] {
                let shape = output
                    .shapes
                    .iter()
                    .find(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == label))
                    .unwrap();
                let egui::Shape::Text(text) = &shape.shape else {
                    unreachable!()
                };
                assert!(
                    shape
                        .clip_rect
                        .contains_rect(Rect::from_min_size(text.pos, text.galley.size()))
                );
            }
        }
        app.pan = Vec2::new(10000.0, 0.0);
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 720.0))),
                ..Default::default()
            },
            |ui| app.ui(ui),
        );
        assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().starts_with("Some layers are outside the view"))));
    }

    #[test]
    fn overlay_grid_remains_over_reference_images_and_can_be_hidden() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let before = app.layout.layers().clone();
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::new(16, 9)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        for reference in [false, true] {
            if reference {
                app.load_reference(
                    &ctx,
                    &egui::DroppedFile {
                        name: "grid-guide.png".into(),
                        bytes: Some(png.get_ref().clone().into()),
                        ..Default::default()
                    },
                );
                app.wait_reference();
            }
            for divisions in [[8, 6], [3, 4]] {
                app.snapping.grid_divisions = divisions;
                app.snapping.show_grid = true;
                let visible = frame(&mut app, &ctx, vec![]);
                let color = theme::palette(&ctx).grid;
                let lines = |out: &egui::FullOutput| {
                    out.shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::LineSegment { stroke, .. } if stroke.color == color)).count()
                };
                let count = lines(&visible);
                app.snapping.show_grid = false;
                assert_eq!(
                    count - lines(&frame(&mut app, &ctx, vec![])),
                    (divisions[0] + divisions[1] + 2) as usize
                );
                if reference {
                    let image = visible.shapes.iter().position(|shape| matches!(&shape.shape, egui::Shape::Mesh(mesh) if mesh.texture_id == app.references[0].texture.id())).unwrap();
                    let grid = visible.shapes.iter().position(|shape| matches!(&shape.shape, egui::Shape::LineSegment { stroke, .. } if stroke.color == color)).unwrap();
                    assert!(grid > image);
                }
            }
        }
        click_text(&mut app, &ctx, "     ");
        click_text(&mut app, &ctx, "Show overlay grid");
        assert!(app.snapping.show_grid);
        assert_eq!(app.layout.layers(), &before);
    }

    #[test]
    fn fit_screen_keeps_output_labels_inside_the_viewport() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(16, 9, image::Rgba([50, 90, 120, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        for size in [
            Vec2::new(1000.0, 720.0),
            Vec2::new(1440.0, 940.0),
            Vec2::new(2000.0, 720.0),
        ] {
            for (captions, reference) in
                [(false, false), (true, false), (false, true), (true, true)]
            {
                let ctx = egui::Context::default();
                let mut app = Editor {
                    show_toolbar_icon_names: captions,
                    zoom: 3.0,
                    pan: Vec2::new(100.0, -200.0),
                    ..Default::default()
                };
                if reference {
                    app.load_reference(
                        &ctx,
                        &egui::DroppedFile {
                            name: "reference.png".into(),
                            bytes: Some(png.get_ref().clone().into()),
                            ..Default::default()
                        },
                    );
                    app.wait_reference();
                    assert_eq!(app.references.len(), 1);
                }
                app.fit_screen();
                let output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ui| app.ui(ui),
                );
                for label in ["SCREEN 1", "1920 × 1080 px"] {
                    let shape = output.shapes.iter().find(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == label)).unwrap();
                    let egui::Shape::Text(text) = &shape.shape else {
                        unreachable!()
                    };
                    let bounds = Rect::from_min_size(text.pos, text.galley.size());
                    assert!(
                        shape.clip_rect.contains_rect(bounds),
                        "{label} at {size:?}: {bounds:?} clipped by {:?}",
                        shape.clip_rect
                    );
                }
            }
        }
    }

    #[test]
    fn example_title_renders_custom_and_blank_text_at_different_output_sizes() {
        for output in [
            [1920, 1080],
            [1280, 720],
            [3840, 2160],
            [1080, 1920],
            [1024, 1024],
            [3840, 1080],
        ] {
            let ctx = egui::Context::default();
            let mut app = Editor::default();
            app.layout.set_output(output).unwrap();
            let contains = |frame: &egui::FullOutput, text: &str| {
                frame
                    .shapes
                    .iter()
                    .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == text))
            };
            assert!(contains(
                &frame(&mut app, &ctx, vec![]),
                "Example Conference Name"
            ));
            let before = app.layout.layers().clone();
            app.backdrop_title = "My conference".into();
            let custom = frame(&mut app, &ctx, vec![]);
            assert!(contains(&custom, "My conference"));
            assert!(!contains(&custom, "Example Conference Name"));
            for label in [
                "Ideas worth sharing.".to_string(),
                "SCREEN 1".to_string(),
                format!("{} × {} px", output[0], output[1]),
            ] {
                let shape = custom
                    .shapes
                    .iter()
                    .find(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == label))
                    .expect("example artwork remains visible");
                let egui::Shape::Text(text) = &shape.shape else {
                    unreachable!()
                };
                assert!(
                    shape
                        .clip_rect
                        .contains_rect(Rect::from_min_size(text.pos, text.galley.size())),
                    "clipped {label} at {output:?}"
                );
            }
            app.backdrop_title.clear();
            assert!(!contains(&frame(&mut app, &ctx, vec![]), "My conference"));
            assert_eq!(app.layout.layers(), &before);
        }
    }

    #[test]
    fn move_tool_preserves_selection_and_never_resizes() {
        for target in 0..3 {
            let ctx = egui::Context::default();
            let mut app = Editor {
                navigation_tool: NavigationTool::Move,
                pivot_tool: true,
                ..Default::default()
            };
            let before = app.layout.layers().clone();
            let selection = app.layout.selection_mask();
            let output = frame(&mut app, &ctx, vec![]);
            let rects = layer_rects(&output);
            let start = match target {
                0 => rects[0].center(),       // unselected layer
                1 => rects[1].right_bottom(), // former resize handle
                _ => {
                    ctx.read_response(egui::Id::new("canvas"))
                        .unwrap()
                        .rect
                        .left_top()
                        + Vec2::splat(20.0)
                }
            };
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(start), mouse(start, true)],
            );
            frame(&mut app, &ctx, vec![mouse(start, false)]);
            assert_eq!(app.layout.selection_mask(), selection);
            let end = start + Vec2::new(25.0, 15.0);
            frame(&mut app, &ctx, vec![mouse(start, true)]);
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
            frame(&mut app, &ctx, vec![mouse(end, false)]);
            assert_eq!(app.layout.selection_mask(), selection);
            assert_eq!(app.layout.layers()[0], before[0]);
            let after = app.layout.layers()[1];
            assert_ne!(after.x, before[1].x);
            assert_eq!(
                [after.width, after.height],
                [before[1].width, before[1].height]
            );
            app.layout.undo();
            assert_eq!(app.layout.layers(), &before);
        }
    }

    #[test]
    fn navigation_icons_are_exclusive_and_keep_grid_pivot_choices() {
        let ctx = egui::Context::default();
        let mut app = Editor {
            grid_tool: Some(10),
            pivot_tool: true,
            ..Default::default()
        };
        let before = app.layout.layers().clone();
        for (label, tool) in [
            ("Pan", NavigationTool::Pan),
            ("Zoom", NavigationTool::Zoom),
            ("Select", NavigationTool::Select),
            ("Move", NavigationTool::Move),
        ] {
            frame(&mut app, &ctx, vec![]);
            let pos = ctx
                .read_response(egui::Id::new(("navigation-tool", label)))
                .unwrap()
                .rect
                .center();
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
            assert_eq!(app.navigation_tool, tool);
            assert_eq!(app.grid_tool, Some(10));
            assert!(app.pivot_tool);
        }
        assert_eq!(app.layout.layers(), &before);
        for (keycode, tool) in [
            (egui::Key::H, NavigationTool::Pan),
            (egui::Key::Z, NavigationTool::Zoom),
            (egui::Key::V, NavigationTool::Select),
        ] {
            frame(&mut app, &ctx, vec![key(keycode)]);
            assert_eq!(app.navigation_tool, tool);
        }
        ctx.memory_mut(|m| m.request_focus(egui::Id::new(("geometry", 0_usize))));
        frame(&mut app, &ctx, vec![key(egui::Key::H)]);
        assert_eq!(app.navigation_tool, NavigationTool::Select);
    }

    #[test]
    fn zoom_tool_anchors_click_and_drag_without_editing_layers_and_escape_restores_camera() {
        let ctx = egui::Context::default();
        let mut app = Editor {
            navigation_tool: NavigationTool::Zoom,
            edit_content: true,
            ..Default::default()
        };
        let before = app.layout.layers().clone();
        let formats = app.layout.formats().clone();
        let selection = app.layout.selection_mask();
        let output = frame(&mut app, &ctx, vec![]);
        let start = layer_rects(&output)[1].center();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        let output = frame(&mut app, &ctx, vec![mouse(start, false)]);
        assert!((app.zoom - 1.25).abs() < 1e-6);
        assert!(layer_rects(&output)[1].center().distance(start) < 0.001);
        let alt = egui::Modifiers {
            alt: true,
            ..Default::default()
        };
        frame_modifiers(&mut app, &ctx, alt, vec![mouse(start, true)]);
        frame_modifiers(&mut app, &ctx, alt, vec![mouse(start, false)]);
        assert!((app.zoom - 1.0).abs() < 1e-6);
        let camera = (app.zoom, app.pan);
        let end = start + Vec2::new(35.0, 0.0);
        frame(&mut app, &ctx, vec![mouse(start, true)]);
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        assert!(app.zoom > camera.0);
        frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
        frame(&mut app, &ctx, vec![mouse(end, false)]);
        assert_eq!((app.zoom, app.pan), camera);
        assert_eq!(app.layout.layers(), &before);
        assert_eq!(app.layout.formats(), &formats);
        assert_eq!(app.layout.selection_mask(), selection);
    }

    #[test]
    fn pan_tool_and_temporary_pan_only_change_the_view() {
        for tool in [
            NavigationTool::Pan,
            NavigationTool::Select,
            NavigationTool::Zoom,
        ] {
            let ctx = egui::Context::default();
            let mut app = Editor {
                navigation_tool: tool,
                ..Default::default()
            };
            let before = app.layout.layers().clone();
            let selection = app.layout.selection_mask();
            let output = frame(&mut app, &ctx, vec![]);
            let start = layer_rects(&output)[1].center();
            let end = start + Vec2::new(30.0, 20.0);
            if tool != NavigationTool::Pan {
                frame(&mut app, &ctx, vec![key(egui::Key::Space)]);
            }
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(start), mouse(start, true)],
            );
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
            frame(&mut app, &ctx, vec![mouse(end, false)]);
            assert!(app.pan.x > 0.0 && app.pan.y > 0.0);
            assert_eq!(app.zoom, 1.0);
            assert_eq!(app.navigation_tool, tool);
            assert_eq!(app.layout.layers(), &before);
            assert_eq!(app.layout.selection_mask(), selection);
        }
    }

    #[test]
    fn manual_ratio_keeps_width_and_position_reference_and_is_undoable() {
        for point in [
            PositionPoint::TopLeft,
            PositionPoint::TopRight,
            PositionPoint::BottomLeft,
            PositionPoint::BottomRight,
            PositionPoint::Center,
        ] {
            let mut app = Editor {
                corner: point,
                ..Default::default()
            };
            app.sync();
            let before = app.layout.layers().clone();
            let formats = app.layout.formats().clone();
            let offsets = point.offsets_on(before[1], app.layout.formats().output);
            app.ratio_edit.text = "4:3".into();
            app.ratio_edit.dirty = true;
            app.apply_ratio_target().unwrap();
            let after = app.layout.layers()[1];
            assert_eq!(after.width, before[1].width);
            assert!((after.width / after.height - 4.0 / 3.0).abs() < 1e-9);
            assert_eq!(
                point.offsets_on(after, app.layout.formats().output),
                offsets
            );
            assert_eq!(app.layout.layers()[0], before[0]);
            app.layout.undo();
            assert_eq!(app.layout.layers(), &before);
            assert_eq!(app.layout.formats(), &formats);
        }
    }

    #[test]
    fn unlocked_ratio_waits_for_lock_and_invalid_input_preserves_geometry() {
        let mut app = Editor {
            keep_aspect_ratio: false,
            ..Default::default()
        };
        let before = app.layout.layers().clone();
        app.ratio_edit.text = "1.85".into();
        app.ratio_edit.dirty = true;
        app.apply_ratio_target().unwrap();
        assert_eq!(app.layout.layers(), &before);
        assert_eq!(app.ratio_edit.target, Some(1.85));
        app.keep_aspect_ratio = true;
        app.apply_ratio_target().unwrap();
        assert!((app.layout.layers()[1].width / app.layout.layers()[1].height - 1.85).abs() < 1e-9);
        let resized = app.layout.layers().clone();
        for text in ["0:3", "4:0", "-1", "NaN", "inf", "4:3:2", "1e-100", ""] {
            app.ratio_edit.text = text.into();
            app.ratio_edit.dirty = true;
            assert!(app.apply_ratio_target().is_err(), "{text}");
            assert_eq!(app.layout.layers(), &resized);
        }
        app.layout.select(Some(0));
        app.sync();
        assert!(!app.ratio_edit.dirty);
        assert!(app.ratio_edit.target.is_none());
    }

    #[test]
    fn ratio_edit_enter_commits_from_the_inspector() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let before = app.layout.layers().clone();
        click_text(&mut app, &ctx, "16:9");
        app.ratio_edit.text = "4/3".into();
        app.ratio_edit.dirty = true;
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert!((app.layout.layers()[1].height - before[1].width * 0.75).abs() < 1e-8);
        assert_eq!(app.layout.layers()[1].width, before[1].width);
    }

    #[test]
    fn toolbar_groups_wrap_without_splitting_or_overlapping_controls() {
        for width in [600.0, 720.0, 1000.0, 1440.0] {
            for captions in [false, true] {
                let (buttons, groups, bottom) = toolbar_groups(width, captions);
                for (i, group) in groups.iter().enumerate() {
                    assert!(group.right() <= width - 16.0);
                    assert!(group.bottom() <= bottom);
                    assert!(groups[..i].iter().all(|other| !other.intersects(*group)));
                }
                for (i, button) in buttons.iter().enumerate() {
                    assert!(groups.iter().any(|group| group.contains_rect(*button)));
                    assert!(buttons[..i].iter().all(|other| !other.intersects(*button)));
                }
                for slots in [
                    &[15, 0, 1, 18][..],
                    &[2, 3],
                    &[4, 5],
                    &[6, 7, 8, 9],
                    &[10, 11, 16, 12],
                    &[13, 14],
                    &[17],
                ] {
                    assert!(
                        slots
                            .windows(2)
                            .all(|pair| buttons[pair[0]].top() == buttons[pair[1]].top()
                                && buttons[pair[0]].right() < buttons[pair[1]].left())
                    );
                }
            }
        }
    }

    #[test]
    fn tool_icons_fit_above_the_canvas_with_captions_and_at_minimum_window_size() {
        for size in [Vec2::new(1000.0, 720.0), Vec2::new(1440.0, 940.0)] {
            for captions in [false, true] {
                let ctx = egui::Context::default();
                let mut app = Editor {
                    show_toolbar_icon_names: captions,
                    ..Default::default()
                };
                for _ in 0..2 {
                    let _ = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                            ..Default::default()
                        },
                        |ui| app.ui(ui),
                    );
                }
                let canvas = ctx.read_response(egui::Id::new("canvas")).unwrap().rect;
                let mut seen: Vec<Rect> = Vec::new();
                for (kind, label) in [
                    ("edit-tool", "Pivot"),
                    ("navigation-tool", "Select"),
                    ("navigation-tool", "Move"),
                    ("navigation-tool", "Pan"),
                    ("navigation-tool", "Zoom"),
                ] {
                    let rect = ctx
                        .read_response(egui::Id::new((kind, label)))
                        .unwrap()
                        .rect;
                    assert!(rect.right() <= canvas.right());
                    assert!(rect.bottom() + if captions { 18.0 } else { 0.0 } < canvas.top());
                    assert!(seen.iter().all(|before| !before.intersects(rect)));
                    seen.push(rect);
                }
            }
        }
    }

    #[test]
    fn persistent_tools_and_modifiers_combine_without_changing_latched_state() {
        for grid in [None, Some(10), Some(100)] {
            for pivot in [false, true] {
                let app = Editor {
                    grid_tool: grid,
                    pivot_tool: pivot,
                    ..Default::default()
                };
                for shift in [false, true] {
                    for ctrl in [false, true] {
                        for alt in [false, true] {
                            let modifiers = egui::Modifiers {
                                shift,
                                ctrl,
                                alt,
                                ..Default::default()
                            };
                            let requested = if ctrl { 100 } else { 10 };
                            let expected_grid = if alt || (shift && grid == Some(requested)) {
                                None
                            } else if shift {
                                Some(requested)
                            } else {
                                grid
                            };
                            assert_eq!(app.effective_grid(modifiers), expected_grid);
                            assert_eq!(
                                app.effective_pivot(modifiers),
                                if ctrl && !shift { !pivot } else { pivot }
                            );
                            assert_eq!(app.grid_tool, grid);
                            assert_eq!(app.pivot_tool, pivot);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn grid_and_pivot_icons_toggle_independently_and_do_not_edit_layers() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let before = app.layout.layers().clone();
        let formats = app.layout.formats().clone();
        for (label, grid, pivot) in [
            ("10 px", Some(10), false),
            ("Pivot", Some(10), true),
            ("100 px", Some(100), true),
            ("100 px", None, true),
            ("Pivot", None, false),
        ] {
            if label != "Pivot" {
                click_text(&mut app, &ctx, SNAP_MENU_TITLE);
            }
            frame(&mut app, &ctx, vec![]);
            let pos = ctx
                .read_response(egui::Id::new(("edit-tool", label)))
                .unwrap()
                .rect
                .center();
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
            assert_eq!(app.grid_tool, grid);
            assert_eq!(app.pivot_tool, pivot);
        }
        assert_eq!(app.layout.layers(), &before);
        assert_eq!(app.layout.formats(), &formats);
    }

    #[test]
    fn latched_grid_and_pivot_resize_together_and_modifier_release_restores_grid() {
        for spacing in [10, 100] {
            let mut app = Editor {
                grid_tool: Some(spacing),
                pivot_tool: true,
                ..Default::default()
            };
            let before = app.layout.layers().clone();
            let original = before[1];
            app.layout
                .start_handle_resize(1, ResizeHandle::Right, [1800.0, 360.0], true);
            app.layout.set_resize_pivot([0.25, 0.75]);
            let fixed = [
                original.x + original.width * 0.25,
                original.y + original.height * 0.75,
            ];
            let guides = app.drag_with_modifiers([1837.0, 360.0], egui::Modifiers::NONE, 1.0);
            let target = (1837.0 / spacing as f64).round() * spacing as f64;
            assert_eq!(guides[0], Some(target));
            let g = app.layout.layers()[1];
            assert!((g.x + g.width - target).abs() < 1e-7);
            assert!((g.x + g.width * 0.25 - fixed[0]).abs() < 1e-7);
            assert!((g.y + g.height * 0.75 - fixed[1]).abs() < 1e-7);
            app.layout.cancel_drag();
            app.layout.start_drag([1160.0, 180.0]);
            let modifiers = egui::Modifiers {
                shift: true,
                ctrl: spacing == 100,
                ..Default::default()
            };
            app.drag_with_modifiers([1173.0, 217.0], modifiers, 1.0);
            assert_eq!(app.layout.layers()[1].x, 1173.0);
            app.drag_with_modifiers([1173.0, 217.0], egui::Modifiers::NONE, 1.0);
            assert_eq!(
                app.layout.layers()[1].x,
                (1173.0 / spacing as f64).round() * spacing as f64
            );
            app.layout.finish_drag();
            app.layout.undo();
            assert_eq!(app.layout.layers(), &before);
            assert_eq!(app.grid_tool, Some(spacing));
            assert!(app.pivot_tool);
        }
    }

    #[test]
    fn linked_pivot_markers_reflect_and_can_be_moved_from_either_layer() {
        for mode in [ResizeMode::Same, ResizeMode::Mirror, ResizeMode::Opposite] {
            let mut app = Editor {
                resize_mode: mode,
                ..Default::default()
            };
            app.layout.set_selection([true; 2]);
            app.sync();
            let before = app.layout.layers().clone();
            app.move_pivot(
                0,
                [
                    before[0].x + before[0].width * 0.2,
                    before[0].y + before[0].height * 0.7,
                ],
            );
            let other = app.pivot_for(1);
            assert_eq!(
                other,
                [
                    if mode == ResizeMode::Same { 0.2 } else { 0.8 },
                    if mode == ResizeMode::Opposite {
                        1.0 - 0.7
                    } else {
                        0.7
                    }
                ]
            );
            let g = before[1];
            app.move_pivot(1, [g.x + g.width * 0.25, g.y + g.height * 0.75]);
            assert_eq!(app.pivot_for(1), [0.25, 0.75]);
            assert_eq!(app.layout.layers(), &before);
            let saved = [app.pivot_for(0), app.pivot_for(1)];
            app.layout.select(Some(0));
            app.sync();
            assert_eq!([app.pivot_for(0), app.pivot_for(1)], saved);
        }
    }

    #[test]
    fn ctrl_drags_pivot_then_resizes_around_it_without_editing_geometry_during_placement() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let ctrl = egui::Modifiers {
            ctrl: true,
            ..Default::default()
        };
        let before = app.layout.layers().clone();
        let output = frame(&mut app, &ctx, vec![]);
        let rect = layer_rects(&output)[1];
        let start = rect.center();
        let end = rect.min + rect.size() * 0.25;
        frame_modifiers(
            &mut app,
            &ctx,
            ctrl,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame_modifiers(&mut app, &ctx, ctrl, vec![egui::Event::PointerMoved(end)]);
        frame_modifiers(&mut app, &ctx, ctrl, vec![mouse(end, false)]);
        assert_eq!(app.layout.layers(), &before);
        assert!((app.pivot_for(1)[0] - 0.25).abs() < 1e-6);
        assert!((app.pivot_for(1)[1] - 0.25).abs() < 1e-6);
        let relative = app.pivot_for(1);
        // Escape restores placement without adding a document history entry.
        let moved = end + Vec2::new(18.0, 12.0);
        frame_modifiers(
            &mut app,
            &ctx,
            ctrl,
            vec![egui::Event::PointerMoved(end), mouse(end, true)],
        );
        frame_modifiers(&mut app, &ctx, ctrl, vec![egui::Event::PointerMoved(moved)]);
        assert_ne!(app.pivot_for(1), relative);
        frame_modifiers(&mut app, &ctx, ctrl, vec![key(egui::Key::Escape)]);
        frame_modifiers(&mut app, &ctx, ctrl, vec![mouse(moved, false)]);
        assert_eq!(app.pivot_for(1), relative);
        assert_eq!(app.layout.layers(), &before);

        let fixed = [
            before[1].x + relative[0] * before[1].width,
            before[1].y + relative[1] * before[1].height,
        ];
        let start = rect.right_center();
        let end = start + Vec2::new(30.0, 0.0);
        frame_modifiers(
            &mut app,
            &ctx,
            ctrl,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame_modifiers(&mut app, &ctx, ctrl, vec![egui::Event::PointerMoved(end)]);
        frame_modifiers(&mut app, &ctx, ctrl, vec![mouse(end, false)]);
        let after = app.layout.layers()[1];
        assert!(after.width > before[1].width);
        assert!((after.x + relative[0] * after.width - fixed[0]).abs() < 1e-6);
        assert!((after.y + relative[1] * after.height - fixed[1]).abs() < 1e-6);
        app.layout.undo();
        assert_eq!(app.layout.layers(), &before);
        // Undoing resize retains the previously committed per-layer pivot.
        assert_eq!(app.pivot_for(1), relative);
        app.layout.undo();
        assert_eq!(app.pivot_for(1), [0.5; 2]);
        app.layout.redo();
        assert_eq!(app.pivot_for(1), relative);
    }

    #[test]
    fn alt_temporarily_inverts_move_and_resize_snapping_and_overrides_grids() {
        for enabled in [false, true] {
            for resize in [false, true] {
                for shift in [false, true] {
                    for ctrl in [false, true] {
                        let mut app = Editor::default();
                        app.snapping.enabled = enabled;
                        app.snapping.pixels = false;
                        app.snapping.layer_edges = true;
                        let before = app.layout.layers().clone();
                        let saved = app.snapping;
                        let point = if resize {
                            app.layout.start_handle_resize(
                                1,
                                ResizeHandle::Right,
                                [1800.0, 360.0],
                                false,
                            );
                            [1918.0, 360.0]
                        } else {
                            app.layout.start_drag([1160.0, 180.0]);
                            [1278.0, 180.0]
                        };
                        let guides = app.drag_with_modifiers(
                            point,
                            egui::Modifiers {
                                alt: true,
                                shift,
                                ctrl,
                                ..Default::default()
                            },
                            0.75,
                        );
                        assert_eq!(guides[0], if enabled { None } else { Some(1920.0) });
                        let g = app.layout.layers()[1];
                        assert_eq!(g.x + g.width, if enabled { 1918.0 } else { 1920.0 });
                        assert_eq!(app.snapping, saved);
                        // Releasing Alt restores the stored setting without a pointer move.
                        let guides = app.drag_with_modifiers(point, egui::Modifiers::NONE, 0.75);
                        assert_eq!(guides[0], if enabled { Some(1920.0) } else { None });
                        let g = app.layout.layers()[1];
                        assert_eq!(g.x + g.width, if enabled { 1920.0 } else { 1918.0 });
                        app.layout.finish_drag();
                        app.layout.undo();
                        assert_eq!(app.layout.layers(), &before);
                        assert_eq!(app.snapping, saved);
                    }
                }
            }
        }
    }

    #[test]
    fn quick_and_context_snapping_menus_save_the_same_options_without_editing_layers() {
        for context in [false, true] {
            let ctx = egui::Context::default();
            let mut app = Editor::default();
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("settings.json");
            app.settings = settings::Store::at(path.clone()).0;
            app.layout.select(None);
            let before = app.layout.layers().clone();
            let formats = app.layout.formats().clone();
            frame(&mut app, &ctx, vec![]);
            let pos = if context {
                Pos2::new(400.0, 700.0)
            } else {
                toolbar_test_position(&app, &ctx, 14)
            };
            let button = if context {
                egui::PointerButton::Secondary
            } else {
                egui::PointerButton::Primary
            };
            let event = |pressed| egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), event(true)],
            );
            frame(&mut app, &ctx, vec![event(false)]);
            // Submenus open on hover. Let their placement settle before clicking
            // an item; clicking the submenu itself can toggle it shut after hover.
            let output = frame(&mut app, &ctx, vec![]);
            let snapping = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::epaint::Shape::Text(t) if t.galley.job.text == "Snapping" => {
                        Some(t.pos + t.galley.size() / 2.0)
                    }
                    _ => None,
                })
                .unwrap();
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(snapping)]);
            click_text(&mut app, &ctx, "Snap movement & resizing");
            assert!(app.snapping.enabled);
            for label in [
                "Pixel edges",
                "Screen edges, center & quarters",
                "Layer borders",
            ] {
                click_text(&mut app, &ctx, label);
            }
            assert!(
                !app.snapping.pixels && !app.snapping.screen_edges && !app.snapping.layer_edges
            );
            let saved = settings::Store::at(path).1;
            assert_eq!(saved.snapping, app.snapping);
            assert_eq!(app.layout.layers(), &before);
            assert_eq!(app.layout.formats(), &formats);
        }
    }

    #[test]
    fn mirror_resize_snaps_at_contact_through_release_and_alt_bypasses() {
        for zoom in [0.75, 1.0] {
            let ctx = egui::Context::default();
            let mut app = Editor {
                zoom,
                resize_mode: ResizeMode::Mirror,
                ..Default::default()
            };
            app.layout.set_selection([true; 2]);
            app.snapping.enabled = true;
            app.sync();
            let before = app.layout.layers().clone();
            let output = frame(&mut app, &ctx, vec![]);
            let rect = layer_rects(&output)[0];
            let scale = rect.width() / before[0].width as f32;
            let start = rect.right_center();
            let end = start + Vec2::new(198.0 * scale, 0.0);
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(start), mouse(start, true)],
            );
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
            assert!(
                (app.layout.layers()[0].x + app.layout.layers()[0].width
                    - app.layout.layers()[1].x)
                    .abs()
                    < 1e-6
            );
            assert_eq!(app.snap_guides[0], Some(960.0));
            frame_modifiers(
                &mut app,
                &ctx,
                egui::Modifiers {
                    alt: true,
                    ..Default::default()
                },
                vec![],
            );
            assert!(
                (app.layout.layers()[0].x + app.layout.layers()[0].width
                    - app.layout.layers()[1].x)
                    .abs()
                    > 1.0
            );
            frame(&mut app, &ctx, vec![mouse(end, false)]);
            assert!(
                (app.layout.layers()[0].x + app.layout.layers()[0].width
                    - app.layout.layers()[1].x)
                    .abs()
                    < 1e-6
            );
            app.layout.undo();
            assert_eq!(app.layout.layers(), &before);
        }
    }

    #[test]
    fn viewport_visibility_and_output_overflow_are_independent() {
        let inside = Geometry {
            x: 100.0,
            y: 100.0,
            width: 200.0,
            height: 100.0,
        };
        let partial = Geometry {
            x: -100.0,
            ..inside
        };
        let outside = Geometry {
            x: -200.0,
            ..inside
        };
        let view = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
        assert!(output_bounds_warning(inside, [1920, 1080]).is_none());
        assert!(
            output_bounds_warning(partial, [1920, 1080])
                .unwrap()
                .contains("partially")
        );
        assert!(
            output_bounds_warning(outside, [1920, 1080])
                .unwrap()
                .contains("entirely")
        );
        assert!(!layer_outside_view(partial, Pos2::ZERO, 1.0, view));
        assert!(layer_outside_view(outside, Pos2::ZERO, 1.0, view));
        // A pan can hide an in-bounds layer or reveal an out-of-bounds layer.
        assert!(layer_outside_view(
            inside,
            Pos2::new(2000.0, 0.0),
            1.0,
            view
        ));
        assert!(!layer_outside_view(
            outside,
            Pos2::new(500.0, 0.0),
            0.5,
            view
        ));
        let right = Geometry {
            x: 1920.0,
            ..inside
        };
        let bottom = Geometry {
            y: 1080.0,
            ..inside
        };
        for g in [right, bottom] {
            assert!(layer_outside_view(g, Pos2::ZERO, 1.0, view));
            assert!(
                output_bounds_warning(g, [1920, 1080])
                    .unwrap()
                    .contains("entirely")
            );
        }
    }

    #[test]
    fn viewport_warning_only_counts_visible_fully_missing_layers() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let warning = |out: &egui::FullOutput| {
            out.shapes.iter().any(|shape| matches!(
            &shape.shape, egui::Shape::Text(text) if text.galley.text().starts_with("Some layers are outside the view")
        ))
        };
        assert!(!warning(&frame(&mut app, &ctx, vec![])));
        let g = Geometry {
            x: -10000.0,
            ..app.layout.layers()[0]
        };
        app.layout.apply(0, g).unwrap();
        assert!(warning(&frame(&mut app, &ctx, vec![])));
        app.layout.set_visible(0, false);
        assert!(!warning(&frame(&mut app, &ctx, vec![])));
        app.pan = Vec2::new(10000.0, 0.0);
        assert!(warning(&frame(&mut app, &ctx, vec![])));
    }

    pub(super) fn open_measurement_menu(app: &mut Editor, ctx: &egui::Context) {
        frame(app, ctx, vec![]);
        let output = frame(app, ctx, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "MEASURE POSITION FROM" => {
                    Some(t.pos + Vec2::new(46.0, 34.0))
                }
                _ => None,
            })
            .unwrap();
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(app, ctx, vec![mouse(pos, false)]);
        frame(app, ctx, vec![]);
    }
    pub(super) fn click_measurement_point(app: &mut Editor, ctx: &egui::Context, id: &str) {
        frame(app, ctx, vec![]);
        let pos = ctx.read_response(egui::Id::new(id)).unwrap().rect.center();
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(app, ctx, vec![mouse(pos, false)]);
    }

    #[test]
    fn center_marker_switches_reference_without_moving_and_resizes_about_center() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        let before = app.layout.layers().clone();
        open_measurement_menu(&mut app, &ctx);
        click_measurement_point(&mut app, &ctx, "corner-Center");
        assert_eq!(app.corner, PositionPoint::Center);
        assert_eq!(app.layout.layers(), &before);
        let i = app.layout.selected().unwrap();
        let center = before[i].center();
        app.keep_aspect_ratio = false;
        app.fields[2] = "400".into();
        app.apply_fields();
        assert_eq!(app.layout.layers()[i].width, 400.0);
        assert_eq!(app.layout.layers()[i].center(), center);
    }

    #[test]
    fn leaving_numeric_fields_commits_and_invalid_edits_stay_inline() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        frame(&mut app, &ctx, vec![]);
        app.fields[0] = "200".into();
        app.fields_edited = true;
        let pos = Pos2::new(20.0, 15.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(&mut app, &ctx, vec![mouse(pos, false)]);
        assert_eq!(
            app.corner
                .offsets_on(app.layout.layers()[1], app.layout.formats().output)[0],
            200.0
        );
        assert!(!app.fields_pending());
        app.fields[0] = "invalid".into();
        app.fields_edited = true;
        let before = app.layout.layers().clone();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(&mut app, &ctx, vec![mouse(pos, false)]);
        assert!(!app.error.is_empty());
        assert_eq!(*app.layout.layers(), before);
        assert_eq!(app.fields[0], "invalid");
    }

    #[test]
    fn paste_review_rejects_replaced_layer_with_same_list_index() {
        let mut app = Editor::default();
        app.open_paste(include_str!("../tests/fixtures/drawio-rectangle.txt").into());
        app.layout.remove_selected().unwrap();
        app.layout.add_layer().unwrap();
        assert_eq!(app.layout.selected_indices(), vec![1]);
        assert!(
            app.paste
                .as_ref()
                .unwrap()
                .values(&app.layout)
                .unwrap_err()
                .contains("Destination layers changed")
        );
    }
    #[test]
    #[ignore = "manual viewport timing; run with --release --ignored --nocapture"]
    fn viewport_capacity_timing() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        for _ in 2..256 {
            app.layout.add_layer().unwrap();
        }
        let before = app.layout.layers().clone();
        let mut timings = Vec::new();
        for n in 0..120 {
            let now = std::time::Instant::now();
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(Pos2::new(
                    200.0 + (n % 20) as f32,
                    300.0,
                ))],
            );
            if n >= 20 {
                timings.push(now.elapsed().as_secs_f64() * 1000.0);
            }
        }
        timings.sort_by(f64::total_cmp);
        println!(
            "256 layers, 1440×940 headless egui: p50 {:.2} ms, p95 {:.2} ms; excludes GPU presentation",
            timings[50], timings[95]
        );
        assert_eq!(*app.layout.layers(), before);
    }
    #[test]
    fn source_tiles_only_render_during_crop_editing_even_with_saved_pattern_enabled() {
        for show_pattern in [false, true] {
            let ctx = egui::Context::default();
            let mut app = Editor {
                show_pattern,
                ..Default::default()
            };
            let colors = theme::palette(&ctx).pattern;
            let tiles = |output: &egui::FullOutput| {
                output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Rect(rect) if colors.contains(&rect.fill))
            })
            };
            assert!(!tiles(&frame(&mut app, &ctx, vec![])));
            app.edit_content = true;
            assert!(tiles(&frame(&mut app, &ctx, vec![])));
            app.edit_content = false;
            assert!(!tiles(&frame(&mut app, &ctx, vec![])));
        }
    }
    #[test]
    fn double_click_toggles_crop_mode_without_changing_geometry() {
        for target in [Some(0), Some(1), None] {
            let mut app = Editor::default();
            let ctx = egui::Context::default();
            let output = frame(&mut app, &ctx, vec![]);
            let before = app.layout.layers().clone();
            let formats = app.layout.formats().clone();
            let pos = target
                .map(|i| layer_rects(&output)[i].center())
                .unwrap_or(Pos2::new(500.0, 700.0));
            for _ in 0..2 {
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
                );
                frame(&mut app, &ctx, vec![mouse(pos, false)]);
            }
            assert_eq!(app.edit_content, target.is_some());
            assert_eq!(app.layout.selected(), target);
            assert_eq!(*app.layout.layers(), before);
            assert_eq!(*app.layout.formats(), formats);
            if target.is_some() {
                // Separate gestures, then double-click again to finish and again to reopen.
                for expected in [false, true] {
                    for _ in 0..40 {
                        frame(&mut app, &ctx, vec![]);
                    }
                    for _ in 0..2 {
                        frame(
                            &mut app,
                            &ctx,
                            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
                        );
                        frame(&mut app, &ctx, vec![mouse(pos, false)]);
                    }
                    assert_eq!(app.edit_content, expected);
                    assert_eq!(app.layout.selected(), target);
                    assert_eq!(*app.layout.layers(), before);
                    assert_eq!(*app.layout.formats(), formats);
                }
            }
            frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
            assert!(!app.edit_content);
        }
    }
    fn alt_frame(app: &mut Editor, ctx: &egui::Context, alt: bool, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 940.0))),
            modifiers: if alt {
                egui::Modifiers::ALT
            } else {
                egui::Modifiers::NONE
            },
            events,
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |ui| app.ui(ui));
    }

    #[test]
    fn holding_alt_in_crop_mode_temporarily_edits_mask_without_opening_properties() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let output = frame(&mut app, &ctx, vec![]);
        let pos = layer_rects(&output)[1].center();
        for _ in 0..2 {
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
        }
        assert!(app.edit_content);
        let geometry = app.layout.layers().clone();
        let before = app.layout.formats().clone();
        alt_frame(&mut app, &ctx, true, vec![]);
        assert!(app.mask_editing());
        assert!(!app.edit_mask);
        assert!(app.format_draft.is_none());
        assert_eq!(*app.layout.formats(), before);
        alt_frame(&mut app, &ctx, false, vec![]);
        assert!(!app.mask_editing());
        assert!(app.edit_content);

        app.layout
            .set_mask(
                1,
                canvas_pilot::formats::Mask {
                    origin: Default::default(),
                    left: 0.1,
                    right: 0.1,
                    top: 0.1,
                    bottom: 0.1,
                },
            )
            .unwrap();
        let before = app.layout.formats().clone();
        alt_frame(&mut app, &ctx, true, vec![key(egui::Key::ArrowRight)]);
        assert_ne!(app.layout.formats().masks, before.masks);
        assert_eq!(app.layout.formats().advanced, before.advanced);
        assert_eq!(*app.layout.layers(), geometry);
        alt_frame(&mut app, &ctx, false, vec![]);
        assert!(!app.mask_editing());
        assert!(app.edit_content);
    }

    #[test]
    fn advanced_crop_handles_stage_until_pointer_release() {
        use canvas_pilot::formats::{Crop, LayerFit};
        for fit in [LayerFit::Centered, LayerFit::Cropped, LayerFit::Fullscreen] {
            let mut app = Editor::default();
            let ctx = egui::Context::default();
            let rect = layer_rects(&frame(&mut app, &ctx, vec![]))[1];
            app.layout.select(Some(1));
            app.layout
                .set_layer_fit(1, Some(fit), Crop::default())
                .unwrap();
            app.edit_content = true;
            frame(&mut app, &ctx, vec![]);
            let before = app.project();
            let g = app.layout.layers()[1];
            let image = app.layout.edge_crop_bounds(1).unwrap();
            let k = rect.width() / g.width as f32;
            let start = rect.min
                + Vec2::new(
                    (image.x - g.x) as f32,
                    (image.y + image.height / 2.0 - g.y) as f32,
                ) * k;
            let end = start + Vec2::new(15.0, 0.0);
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(start), mouse(start, true)],
            );
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
            assert!(app.layout.dragging(), "{fit:?}");
            assert_eq!(app.project(), before);
            assert!(app.layout.edge_crop_bounds(1).unwrap().x > image.x);
            frame(&mut app, &ctx, vec![mouse(end, false)]);
            assert!(app.layout.formats().advanced[1].left > 0.0);
            assert_eq!(app.layout.layers()[1], g);
            app.layout.undo();
            assert_eq!(app.project(), before);
        }
    }

    #[test]
    fn advanced_fit_shortcuts_open_edge_crop_without_changing_fit_or_geometry() {
        use canvas_pilot::formats::LayerFit;
        for fit in [LayerFit::Centered, LayerFit::Cropped, LayerFit::Fullscreen] {
            for keyboard in [false, true] {
                let ctx = egui::Context::default();
                let mut app = Editor::default();
                let output = frame(&mut app, &ctx, vec![]);
                let pos = layer_rects(&output)[1].center();
                app.layout.select(Some(1));
                app.layout
                    .set_layer_fit(1, Some(fit), Default::default())
                    .unwrap();
                let before = app.project();
                frame(&mut app, &ctx, vec![]);
                if keyboard {
                    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
                } else {
                    for _ in 0..2 {
                        frame(
                            &mut app,
                            &ctx,
                            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
                        );
                        frame(&mut app, &ctx, vec![mouse(pos, false)]);
                    }
                }
                assert!(app.edit_content, "{fit:?}, keyboard={keyboard}");
                assert!(app.format_draft.is_none());
                assert_eq!(app.project(), before);
            }
        }
    }

    #[test]
    fn enter_opens_crop_and_release_is_required_before_closing() {
        for fit in [None, Some(canvas_pilot::formats::LayerFit::OneToOne)] {
            let ctx = egui::Context::default();
            let mut app = Editor::default();
            app.layout.select(Some(1));
            app.layout
                .set_layer_fit(1, fit, Default::default())
                .unwrap();
            let before = app.project();
            let mut input = egui::RawInput {
                events: vec![key(egui::Key::Enter)],
                ..Default::default()
            };
            app.crop_mask_key_input(&ctx, &mut input);
            assert!(app.edit_content);
            assert!(input.events.is_empty());
            input.events = vec![key(egui::Key::Enter)];
            app.crop_mask_key_input(&ctx, &mut input);
            assert!(app.edit_content);
            let mut release = key(egui::Key::Enter);
            if let egui::Event::Key { pressed, .. } = &mut release {
                *pressed = false;
            }
            input.events = vec![release, key(egui::Key::Enter)];
            app.crop_mask_key_input(&ctx, &mut input);
            assert!(!app.edit_content);
            assert_eq!(app.project(), before);
        }
    }

    #[test]
    fn enter_does_not_open_crop_while_typing_or_in_input_settings() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout.select(Some(1));
        let field = egui::Id::new("typing-field");
        ctx.memory_mut(|m| m.request_focus(field));
        let mut input = egui::RawInput {
            events: vec![key(egui::Key::Enter)],
            ..Default::default()
        };
        app.crop_mask_key_input(&ctx, &mut input);
        assert!(!app.edit_content);
        assert_eq!(input.events.len(), 1);
        ctx.memory_mut(|m| m.surrender_focus(field));
        app.crop_mask_enter_held = false;
        app.inputs.open = true;
        app.crop_mask_key_input(&ctx, &mut input);
        assert!(!app.edit_content);
        assert_eq!(input.events.len(), 1);
    }

    #[test]
    fn enter_finishes_crop_and_mask_without_changing_live_edits_or_undo() {
        for mask in [false, true] {
            for properties in [false, true] {
                let ctx = egui::Context::default();
                let mut app = Editor::default();
                app.layout.select(Some(1));
                let before = app.layout.formats().clone();
                if mask {
                    app.layout
                        .set_mask(
                            1,
                            canvas_pilot::formats::Mask {
                                left: 0.1,
                                ..Default::default()
                            },
                        )
                        .unwrap();
                } else {
                    app.layout
                        .set_framing(
                            1,
                            canvas_pilot::formats::Framing {
                                zoom: 1.2,
                                ..Default::default()
                            },
                        )
                        .unwrap();
                }
                app.edit_content = true;
                app.edit_mask = mask;
                if properties {
                    app.property_tab = if mask {
                        PropertyTab::Mask
                    } else {
                        PropertyTab::Crop
                    };
                    app.format_draft = Some(FormatDraft::new(&app.layout, 1));
                }
                frame(&mut app, &ctx, vec![]);
                let applied = app.project();
                frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
                assert!(!app.edit_content);
                assert!(!app.edit_mask);
                assert!(app.format_draft.is_none());
                assert_eq!(app.project(), applied);
                app.layout.undo();
                assert_eq!(*app.layout.formats(), before);
            }
        }
    }

    #[test]
    fn enter_leaves_numeric_input_and_active_drags_alone() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout.select(Some(1));
        app.edit_content = true;
        let field = egui::Id::new("numeric-crop-field");
        ctx.memory_mut(|m| m.request_focus(field));
        let mut input = egui::RawInput {
            events: vec![key(egui::Key::Enter)],
            ..Default::default()
        };
        app.crop_mask_key_input(&ctx, &mut input);
        assert!(app.edit_content);
        assert_eq!(input.events.len(), 1);
        ctx.memory_mut(|m| m.surrender_focus(field));
        // A repeat after field submission must not also finish the helper.
        app.crop_mask_key_input(&ctx, &mut input);
        assert!(app.edit_content);
        app.crop_mask_enter_held = false;
        app.layout.start_mask_drag(1, [0.0; 2], None);
        let mut input = egui::RawInput {
            events: vec![key(egui::Key::Enter)],
            ..Default::default()
        };
        app.crop_mask_key_input(&ctx, &mut input);
        assert!(app.edit_content);
        assert!(app.layout.dragging());
        assert_eq!(input.events.len(), 1);
        app.layout.cancel_drag();
    }

    #[test]
    fn tab_toggles_crop_and_mask_without_focus_or_document_changes() {
        for fit in [None, Some(canvas_pilot::formats::LayerFit::OneToOne)] {
            let ctx = egui::Context::default();
            let mut app = Editor::default();
            app.layout.select(Some(1));
            app.layout
                .set_layer_fit(1, fit, Default::default())
                .unwrap();
            app.edit_content = true;
            frame(&mut app, &ctx, vec![]);
            let before = app.project();
            frame(&mut app, &ctx, vec![key(egui::Key::Tab)]);
            assert!(app.mask_editing());
            assert!(ctx.memory(|m| m.focused().is_none()));
            frame(&mut app, &ctx, vec![key(egui::Key::Tab)]); // native repeat is initially unmarked
            assert!(app.mask_editing());
            let release = egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            frame(&mut app, &ctx, vec![release]);
            frame(&mut app, &ctx, vec![key(egui::Key::Tab)]);
            assert!(!app.mask_editing());
            assert!(app.edit_content);
            assert_eq!(app.project(), before);
            assert!(ctx.memory(|m| m.focused().is_none()));
        }
    }

    #[test]
    fn tab_keeps_navigation_outside_canvas_editing_and_during_drags() {
        for case in 0..9 {
            let ctx = egui::Context::default();
            let mut app = Editor::default();
            app.layout.select(Some(1));
            app.edit_content = true;
            match case {
                0 => app.edit_content = false,
                1 => app.navigation_tool = NavigationTool::Pan,
                2 => app.preferences_open = true,
                3 => app.format_draft = Some(FormatDraft::new(&app.layout, 1)),
                4 => ctx.memory_mut(|m| m.request_focus(egui::Id::new("field"))),
                5 => app.layout.set_locked(1, true),
                6 => app.layout.start_mask_drag(1, [0.0; 2], None),
                7 => {
                    app.layout
                        .set_layer_fit(
                            1,
                            Some(canvas_pilot::formats::LayerFit::Centered),
                            Default::default(),
                        )
                        .unwrap();
                }
                _ => app.navigation_tool = NavigationTool::Zoom,
            }
            let mut input = egui::RawInput {
                events: vec![key(egui::Key::Tab)],
                ..Default::default()
            };
            app.crop_mask_key_input(&ctx, &mut input);
            assert!(!app.mask_editing(), "case {case}");
            assert_eq!(input.events.len(), 1, "case {case}");
        }
        let ctx = egui::Context::default();
        let mut app = Editor {
            edit_content: true,
            ..Default::default()
        };
        for modifiers in [
            egui::Modifiers::SHIFT,
            egui::Modifiers::CTRL,
            egui::Modifiers::ALT,
            egui::Modifiers::COMMAND,
        ] {
            app.crop_mask_tab_held = false;
            let mut input = egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Tab,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
                ..Default::default()
            };
            app.crop_mask_key_input(&ctx, &mut input);
            assert!(!app.mask_editing());
            assert_eq!(input.events.len(), 1);
        }
        app.crop_mask_key_input(
            &ctx,
            &mut egui::RawInput {
                focused: false,
                ..Default::default()
            },
        );
        assert!(!app.crop_mask_tab_held);
    }

    #[test]
    fn alt_mask_respects_modes_and_keeps_drags_stable() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout.select(Some(1));
        alt_frame(&mut app, &ctx, true, vec![]);
        assert!(!app.mask_editing());
        app.edit_content = true;
        for tool in [NavigationTool::Pan, NavigationTool::Zoom] {
            app.navigation_tool = tool;
            alt_frame(&mut app, &ctx, true, vec![]);
            assert!(!app.mask_editing());
        }
        app.navigation_tool = NavigationTool::Select;
        app.layout.set_locked(1, true);
        alt_frame(&mut app, &ctx, true, vec![]);
        assert!(!app.mask_editing());
        app.layout.set_locked(1, false);
        app.layout.select(Some(1));
        app.edit_content = true;
        alt_frame(&mut app, &ctx, true, vec![]);
        assert!(app.mask_editing());
        app.layout
            .start_mask_drag(1, app.layout.layers()[1].center(), None);
        alt_frame(&mut app, &ctx, false, vec![]);
        assert!(app.mask_editing());
        app.layout.finish_drag();
        alt_frame(&mut app, &ctx, false, vec![]);
        assert!(!app.mask_editing());
        app.edit_mask = true;
        alt_frame(&mut app, &ctx, true, vec![]);
        alt_frame(&mut app, &ctx, false, vec![]);
        assert!(app.mask_editing());
    }

    #[test]
    fn double_click_does_not_edit_a_locked_layer() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let output = frame(&mut app, &ctx, vec![]);
        let pos = layer_rects(&output)[1].center();
        app.layout.set_locked(1, true);
        for _ in 0..2 {
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
        }
        assert!(!app.edit_content);
    }
    #[test]
    fn content_mode_drags_only_crop_and_camera_zoom_keeps_document_unchanged() {
        let mut app = Editor {
            edit_content: true,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.layout
            .set_framing(
                1,
                canvas_pilot::formats::Framing {
                    zoom: 2.0,
                    offset: [0.0; 2],
                },
            )
            .unwrap();
        let before = app.layout.formats().clone();
        let boxes = app.layout.layers().clone();
        let output = frame(&mut app, &ctx, vec![]);
        let start = layer_rects(&output)[1].center();
        let end = start + Vec2::new(30.0, 20.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        frame(&mut app, &ctx, vec![mouse(end, false)]);
        let framing = app.layout.formats().mapping(1, boxes[1]).unwrap().framing;
        assert!(framing.offset[0] > 0.0 && framing.offset[1] > 0.0);
        assert_eq!(*app.layout.layers(), boxes);
        app.layout.undo();
        assert_eq!(*app.layout.formats(), before);
        frame(&mut app, &ctx, vec![key(egui::Key::ArrowRight)]);
        assert_eq!(*app.layout.layers(), boxes);
        assert!(
            (app.layout
                .formats()
                .mapping(1, boxes[1])
                .unwrap()
                .framing
                .offset[0]
                - 1.0)
                .abs()
                < 1e-8
        );
        let unchanged = app.layout.formats().clone();
        app.zoom = 2.0;
        frame(&mut app, &ctx, vec![]);
        assert_eq!(*app.layout.formats(), unchanged);
        assert_eq!(*app.layout.layers(), boxes);
        frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
        assert!(!app.edit_content);
    }
    #[test]
    fn viewport_options_menu_toggles_preferences_without_changing_layout() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let before = app.layout.layers().clone();
        let formats = app.layout.formats().clone();
        frame(&mut app, &ctx, vec![]);
        let pos = toolbar_test_position(&app, &ctx, 14);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(&mut app, &ctx, vec![mouse(pos, false)]);
        for label in [
            "Keep aspect ratio when resizing",
            "Show source patterns on other layers while cropping",
            "Show toolbar icon names",
        ] {
            let output = frame(&mut app, &ctx, vec![]);
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
                .expect("viewport setting rendered");
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
        }
        assert!(!app.keep_aspect_ratio);
        assert!(app.show_pattern);
        assert!(app.show_toolbar_icon_names);
        assert_eq!(*app.layout.layers(), before);
        assert_eq!(*app.layout.formats(), formats);
    }

    #[test]
    fn visual_reset_crop_centers_at_minimum_zoom_and_is_undoable() {
        let mut app = Editor {
            edit_content: true,
            ..Default::default()
        };
        app.layout
            .set_framing(
                1,
                canvas_pilot::formats::Framing {
                    zoom: 2.0,
                    offset: [30.0, -20.0],
                },
            )
            .unwrap();
        let before = app.layout.formats().clone();
        let boxes = app.layout.layers().clone();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        let pos = ctx
            .read_response(egui::Id::new("reset-visual-crop"))
            .unwrap()
            .rect
            .center();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(&mut app, &ctx, vec![mouse(pos, false)]);
        let framing = app.layout.formats().mapping(1, boxes[1]).unwrap().framing;
        assert!((framing.zoom - 1.0).abs() < 1e-8);
        assert!(framing.offset.iter().all(|v| v.abs() < 1e-8));
        assert_eq!(*app.layout.layers(), boxes);
        assert_eq!(app.layout.formats().sources, before.sources);
        assert!(app.edit_content);
        app.layout.undo();
        assert_eq!(*app.layout.formats(), before);
    }

    #[test]
    fn mask_helper_drags_entire_layer_and_handles_with_arrows_and_cancel() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let rect = layer_rects(&frame(&mut app, &ctx, vec![]))[1];
        let mask = canvas_pilot::formats::Mask {
            origin: Default::default(),
            left: 0.25,
            right: 0.25,
            top: 0.25,
            bottom: 0.25,
        };
        app.layout.set_mask(1, mask).unwrap();
        app.edit_content = true;
        app.edit_mask = true;
        let before = app.layout.formats().advanced.clone();
        let boxes = app.layout.layers().clone();
        let start = rect.min + Vec2::splat(15.0);
        let end = start + Vec2::new(12.0, 8.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        frame(&mut app, &ctx, vec![mouse(end, false)]);
        let moved = app.layout.formats().masks[1];
        assert!(moved.left > mask.left && moved.top > mask.top);
        frame(&mut app, &ctx, vec![key(egui::Key::ArrowRight)]);
        assert!(
            (app.layout.formats().masks[1].left - moved.left - 1.0 / boxes[1].width).abs() < 1e-8
        );
        let m = app.layout.formats().masks[1];
        let edge = Pos2::new(
            rect.left() + rect.width() * m.left as f32,
            rect.top() + rect.height() * (m.top + (1.0 - m.top - m.bottom) / 2.0) as f32,
        );
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(edge), mouse(edge, true)],
        );
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(edge + Vec2::new(20.0, 0.0))],
        );
        assert!(app.layout.formats().masks[1].left > m.left);
        frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
        assert_eq!(app.layout.formats().masks[1], m);
        assert_eq!(app.layout.formats().advanced, before);
        assert_eq!(*app.layout.layers(), boxes);
    }

    #[test]
    fn one_to_one_crop_moves_from_empty_layer_margins_and_arrow_keys() {
        use canvas_pilot::formats::{Crop, LayerFit};
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let output = frame(&mut app, &ctx, vec![]);
        let rect = layer_rects(&output)[1];
        let crop = Crop {
            left: 800.0,
            right: 800.0,
            top: 450.0,
            bottom: 450.0,
        };
        app.layout
            .set_layer_fit(1, Some(LayerFit::OneToOne), crop)
            .unwrap();
        app.edit_content = true;
        frame(&mut app, &ctx, vec![]);
        let boxes = app.layout.layers().clone();
        // The retained image is 320×180 in a 640×360 layer. Start in its empty margin.
        let start = Pos2::new(rect.left() + rect.width() * 0.1, rect.center().y);
        let end = start + Vec2::new(20.0, 10.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        frame(&mut app, &ctx, vec![mouse(end, false)]);
        assert!(app.edit_content);
        assert_eq!(app.layout.selected(), Some(1));
        assert_eq!(*app.layout.layers(), boxes);
        let moved = app.layout.formats().advanced[1];
        assert!(moved.left < crop.left && moved.top < crop.top);
        assert!((moved.left + moved.right - 1600.0).abs() < 1e-7);
        assert!((moved.top + moved.bottom - 900.0).abs() < 1e-7);
        frame(&mut app, &ctx, vec![key(egui::Key::ArrowRight)]);
        let nudged = app.layout.formats().advanced[1];
        assert!((nudged.left - (moved.left - 1.0)).abs() < 1e-7);
        assert!((nudged.right - (moved.right + 1.0)).abs() < 1e-7);
        assert_eq!(*app.layout.layers(), boxes);
        app.layout.undo();
        assert_eq!(app.layout.formats().advanced[1], moved);
        app.layout.undo();
        assert_eq!(app.layout.formats().advanced[1], crop);
    }

    #[test]
    fn double_click_one_to_one_opens_crop_helper_and_handles_trim_without_zoom() {
        use canvas_pilot::formats::{Crop, LayerFit};
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let output = frame(&mut app, &ctx, vec![]);
        let rect = layer_rects(&output)[1];
        app.layout
            .set_layer_fit(
                1,
                Some(LayerFit::OneToOne),
                Crop {
                    left: 640.0,
                    right: 640.0,
                    top: 360.0,
                    bottom: 360.0,
                },
            )
            .unwrap();
        frame(&mut app, &ctx, vec![]);
        let center = rect.center();
        for _ in 0..2 {
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(center), mouse(center, true)],
            );
            frame(&mut app, &ctx, vec![mouse(center, false)]);
        }
        assert!(app.edit_content);
        assert!(app.format_draft.is_none());
        let before = app.layout.layers().clone();
        let crop = app.layout.formats().advanced[1];
        let start = rect.left_center();
        let end = start + Vec2::new(20.0, 0.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        frame(&mut app, &ctx, vec![mouse(end, false)]);
        assert!(app.edit_content);
        assert_eq!(*app.layout.layers(), before);
        let changed = app.layout.formats().advanced[1];
        assert!(changed.left > crop.left);
        assert_eq!(
            [changed.right, changed.top, changed.bottom],
            [crop.right, crop.top, crop.bottom]
        );
        assert_eq!(
            app.layout.formats().mapping(1, before[1]).unwrap().scale,
            1.0
        );
        frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
        assert!(!app.edit_content);
        app.layout.undo();
        assert_eq!(app.layout.formats().advanced[1], crop);
    }

    #[test]
    fn crop_handle_gesture_zooms_content_without_resizing_layer() {
        for (zoom, handle_index, pivot_tool) in [0.6, 1.0]
            .into_iter()
            .flat_map(|zoom| (0..8).flat_map(move |i| [false, true].map(|pivot| (zoom, i, pivot))))
        {
            let mut app = Editor {
                edit_content: true,
                pivot_tool,
                keep_aspect_ratio: false,
                zoom,
                ..Default::default()
            };
            let ctx = egui::Context::default();
            let before = app.layout.layers().clone();
            let formats = app.layout.formats().clone();
            let output = frame(&mut app, &ctx, vec![]);
            let rect = layer_rects(&output)[1];
            let (_, start) = crop_handle_positions(rect)[handle_index];
            let direction = start - rect.center();
            let end = start + Vec2::new(direction.x.signum() * 20.0, direction.y.signum() * 15.0);
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(start), mouse(start, true)],
            );
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
            frame(&mut app, &ctx, vec![mouse(end, false)]);
            assert!(app.edit_content);
            assert_eq!(*app.layout.layers(), before);
            assert!(
                app.layout
                    .formats()
                    .mapping(1, before[1])
                    .unwrap()
                    .framing
                    .zoom
                    > 1.0
            );
            if pivot_tool {
                let mapping = app.layout.formats().mapping(1, before[1]).unwrap();
                assert!(mapping.framing.offset.iter().all(|v| v.abs() < 1e-5));
            }
            app.layout.undo();
            assert_eq!(*app.layout.formats(), formats);
        }
    }

    #[test]
    fn clicking_outside_crop_exits_and_selects_normally_without_changing_geometry() {
        for target in [Some(0), None] {
            let mut app = Editor {
                edit_content: true,
                ..Default::default()
            };
            let ctx = egui::Context::default();
            let output = frame(&mut app, &ctx, vec![]);
            let before = app.layout.layers().clone();
            let formats = app.layout.formats().clone();
            let pos = target
                .map(|i| layer_rects(&output)[i].center())
                .unwrap_or(Pos2::new(500.0, 700.0));
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
            assert!(!app.edit_content);
            assert_eq!(app.layout.selected(), target);
            assert_eq!(*app.layout.layers(), before);
            assert_eq!(*app.layout.formats(), formats);
        }
    }

    #[test]
    fn clicking_visible_or_faded_crop_content_keeps_editing() {
        for outside_box in [false, true] {
            let mut app = Editor {
                edit_content: true,
                ..Default::default()
            };
            app.layout
                .set_framing(
                    1,
                    canvas_pilot::formats::Framing {
                        zoom: 2.0,
                        offset: [0.0; 2],
                    },
                )
                .unwrap();
            let ctx = egui::Context::default();
            let output = frame(&mut app, &ctx, vec![]);
            let rect = layer_rects(&output)[1];
            let pos = if outside_box {
                rect.center_top() - Vec2::new(0.0, 15.0)
            } else {
                rect.center()
            };
            let formats = app.layout.formats().clone();
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
            assert!(app.edit_content);
            assert_eq!(app.layout.selected(), Some(1));
            assert_eq!(*app.layout.formats(), formats);
        }
    }

    #[test]
    fn faded_content_can_be_grabbed_outside_box_at_different_camera_scales() {
        for zoom in [0.6, 1.0] {
            let mut app = Editor {
                edit_content: true,
                zoom,
                pan: Vec2::new(-30.0, 20.0),
                ..Default::default()
            };
            app.layout
                .set_framing(
                    1,
                    canvas_pilot::formats::Framing {
                        zoom: 2.0,
                        offset: [0.0; 2],
                    },
                )
                .unwrap();
            let ctx = egui::Context::default();
            let before = app.layout.layers().clone();
            let formats = app.layout.formats().clone();
            let output = frame(&mut app, &ctx, vec![]);
            let rect = layer_rects(&output)[1];
            let start = rect.center_top() - Vec2::new(0.0, 15.0);
            assert!(!rect.contains(start));
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(start), mouse(start, true)],
            );
            let end = start + Vec2::new(10.0, 10.0);
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
            frame(&mut app, &ctx, vec![mouse(end, false)]);
            assert_eq!(*app.layout.layers(), before);
            assert_ne!(*app.layout.formats(), formats);
            assert_eq!(app.layout.selected(), Some(1));
            app.layout.undo();
            assert_eq!(*app.layout.formats(), formats);
        }
    }
    #[test]
    fn content_hit_excludes_empty_space_and_preview_covered_by_another_layer() {
        let mut app = Editor {
            edit_content: true,
            ..Default::default()
        };
        app.layout
            .set_framing(
                1,
                canvas_pilot::formats::Framing {
                    zoom: 4.0,
                    offset: [0.0; 2],
                },
            )
            .unwrap();
        let origin = Pos2::ZERO;
        let covered = Pos2::new(700.0, 300.0);
        assert_eq!(app.content_hit(covered, origin, 1.0), None);
        app.layout.set_visible(0, false);
        assert_eq!(app.content_hit(covered, origin, 1.0), Some(1));
        assert_eq!(
            app.content_hit(Pos2::new(1480.0, -500.0), origin, 1.0),
            None
        );
        app.edit_content = false;
        assert_eq!(app.content_hit(covered, origin, 1.0), None);
    }
    #[test]
    fn context_paste_requests_clipboard_and_reviews_values_before_applying() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let before = app.layout.layers().clone();
        let output = frame(&mut app, &ctx, vec![]);
        let pos = layer_rects(&output)[0].center();
        let secondary = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), secondary(true)],
        );
        frame(&mut app, &ctx, vec![secondary(false)]);
        let output = frame(&mut app, &ctx, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "Paste layer values" => {
                    Some(t.pos + t.galley.size() / 2.)
                }
                _ => None,
            })
            .expect("context paste action");
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        let output = frame(&mut app, &ctx, vec![mouse(pos, false)]);
        assert!(output.viewport_output.values().any(|v| {
            v.commands
                .iter()
                .any(|c| matches!(c, egui::ViewportCommand::RequestPaste))
        }));
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Paste(
                include_str!("../tests/fixtures/drawio-rectangle.txt").into(),
            )],
        );
        let paste = app.paste.as_ref().expect("paste mapping review");
        assert!(paste.error.is_empty());
        assert_eq!(paste.shapes.len(), 1);
        assert_eq!(*app.layout.layers(), before);
    }

    #[test]
    fn export_toolbar_opens_existing_review_without_editing_the_project() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        let before = app.project();
        let pos = toolbar_test_position(&app, &ctx, 18);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(&mut app, &ctx, vec![mouse(pos, false)]);
        click_text(&mut app, &ctx, "Export layers…");
        let output = frame(&mut app, &ctx, vec![]);
        assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.job.text == "Export layers")));
        assert_eq!(app.project(), before);
    }

    #[test]
    fn import_menu_requests_clipboard_and_opens_layer_review() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        let mut requested = false;
        for label in ["Import", "Clipboard values"] {
            let output = frame(&mut app, &ctx, vec![]);
            let pos = if label == "Import" {
                toolbar_test_position(&app, &ctx, 1)
            } else {
                output
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::epaint::Shape::Text(t) if t.galley.job.text == label => {
                            Some(t.pos + Vec2::new(10.0, 6.0))
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            let output = frame(&mut app, &ctx, vec![mouse(pos, false)]);
            requested |= output.viewport_output.values().any(|v| {
                v.commands
                    .iter()
                    .any(|c| matches!(c, egui::ViewportCommand::RequestPaste))
            });
        }
        assert!(requested);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Paste(
                include_str!("../tests/fixtures/drawio-rectangle.txt").into(),
            )],
        );
        let p = app.paste.as_ref().unwrap();
        assert!(p.error.is_empty());
        assert_eq!(p.shapes.len(), 1);
    }
    #[test]
    fn paste_format_position_and_zoom_are_independent_and_undo_together() {
        for size in [false, true] {
            for position in [false, true] {
                for zoom in [false, true] {
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
                    let before = app.layout.layers().clone();
                    let formats = app.layout.formats().clone();
                    app.open_paste("Source — visible box (device container unverified)\nHorizontal: 500 px\nVertical: 600 px\nWidth: 300 px\nHeight: 250 px\nCrop zoom: 300 %".into());
                    let p = app.paste.as_mut().unwrap();
                    assert!(p.error.is_empty(), "{}", p.error);
                    p.include_size = size;
                    p.include_position = position;
                    p.include_zoom = zoom;
                    let result = p.apply(&mut app.layout);
                    if !size && !position && !zoom {
                        assert!(result.unwrap_err().contains("Choose at least one"));
                    } else {
                        result.unwrap();
                        let g = app.layout.layers()[1];
                        assert_eq!(
                            [g.x, g.y],
                            if position {
                                [350.0, 475.0]
                            } else {
                                [before[1].x, before[1].y]
                            }
                        );
                        assert_eq!(
                            [g.width, g.height],
                            if size {
                                [300.0, 250.0]
                            } else {
                                [before[1].width, before[1].height]
                            }
                        );
                        assert!(
                            (app.layout.formats().mapping(1, g).unwrap().framing.zoom
                                - if zoom { 3.0 } else { 2.0 })
                            .abs()
                                < 1e-8
                        );
                        app.layout.undo();
                    }
                    assert_eq!(*app.layout.layers(), before);
                    assert_eq!(*app.layout.formats(), formats);
                }
            }
        }
        let mut app = Editor::default();
        app.open_paste(include_str!("../tests/fixtures/drawio-rectangle.txt").into());
        let p = app.paste.as_mut().unwrap();
        assert!(!p.zoom_available());
        p.include_size = false;
        p.include_position = false;
        assert!(p.apply(&mut app.layout).is_err());
    }
    #[test]
    fn native_copy_paste_round_trip_supports_single_and_multiple_layers() {
        for selection in [[false, true], [true, true]] {
            let mut app = Editor::default();
            app.layout.set_selection(selection);
            let ctx = egui::Context::default();
            frame(&mut app, &ctx, vec![]);
            let before = app.layout.layers().clone();
            let output = frame(&mut app, &ctx, vec![egui::Event::Copy]);
            let text = output
                .platform_output
                .commands
                .iter()
                .find_map(|c| match c {
                    egui::OutputCommand::CopyText(text) => Some(text.clone()),
                    _ => None,
                })
                .unwrap();
            app.layout.nudge(50.0, 30.0).unwrap();
            let moved = app.layout.layers().clone();
            frame(&mut app, &ctx, vec![egui::Event::Paste(text)]);
            let p = app.paste.as_ref().unwrap();
            assert!(p.error.is_empty(), "{}", p.error);
            let values = p.values(&app.layout).unwrap();
            assert_eq!(values.len(), selection.iter().filter(|v| **v).count());
            app.layout.apply_import(&values).unwrap();
            assert_eq!(*app.layout.layers(), before);
            app.layout.undo();
            assert_eq!(*app.layout.layers(), moved);
        }
    }
    #[test]
    fn rounded_fields_do_not_quantize_unchanged_values_and_copy_uses_pixels() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let g = Geometry {
            x: 12.36,
            y: 20.51,
            width: 300.33,
            height: 250.58,
        };
        app.layout.apply(1, g).unwrap();
        app.sync();
        assert_eq!(app.fields[2], "300.3");
        assert_eq!(app.fields[3], "250.6");
        app.apply_fields();
        assert_eq!(app.layout.layers()[1], g);
        let output = ctx.run_ui(egui::RawInput::default(), |ui| app.copy(ui.ctx()));
        assert!(output.platform_output.commands.iter().any(|c|matches!(c,egui::OutputCommand::CopyText(text) if text.contains("Width: 300 px") && text.contains("Height: 251 px"))));
        assert_eq!(app.layout.layers()[1], g);
    }
    #[test]
    fn mixed_named_and_unnamed_imports_leave_unnamed_destinations_alone() {
        let mut app = Editor::default();
        app.layout.select(Some(0));
        app.open_paste(r#"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500"><rect id="Presenter" width="100" height="50"/><rect width="100" height="50"/></svg>"#.into());
        let p = app.paste.as_mut().unwrap();
        p.set_scope(ImportScope::All, &app.layout);
        p.mapping = vec![Some(0), Some(1)];
        p.include_size = false;
        p.include_position = false;
        p.include_names = true;
        let before = canvas_pilot::project::Project::from_layout(&app.layout);
        p.apply(&mut app.layout).unwrap();
        assert_eq!(app.layout.layer_info(0).name, "Presenter");
        assert_eq!(app.layout.layer_info(1).name, before.layers[1].name);
        app.layout.undo();
        assert_eq!(
            canvas_pilot::project::Project::from_layout(&app.layout),
            before
        );
        p.ignored[0] = true;
        assert!(!p.names_available());
        assert!(p.apply(&mut app.layout).is_err());
    }
    #[test]
    fn paste_review_renames_only_when_requested_and_supports_name_only_undo() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout
            .rename_layer(0, "Presenter", "Source folder")
            .unwrap();
        app.layout
            .rename_layer(1, "Old name", "Destination folder")
            .unwrap();
        app.layout.select(Some(1));
        let before = canvas_pilot::project::Project::from_layout(&app.layout);
        let text = canvas_pilot::clipboard::encode(&app.layout, &[0], "Summary").unwrap();
        app.open_paste(text);
        let p = app.paste.as_mut().unwrap();
        assert!(p.names_available());
        assert!(!p.include_names);
        p.include_size = false;
        p.include_position = false;
        p.include_crop_fit = false;
        p.include_masks = false;
        assert!(p.apply(&mut app.layout).is_err());
        frame(&mut app, &ctx, vec![]);
        click_text(&mut app, &ctx, "Rename destination layers");
        assert!(app.paste.as_ref().unwrap().include_names);
        click_text(&mut app, &ctx, "Import mapped values");
        assert!(app.paste.is_none());
        assert_eq!(app.layout.layer_info(1).name, "Presenter");
        assert_eq!(app.layout.layer_info(1).folder, "Destination folder");
        assert_eq!(app.layout.layers()[1], before.layers[1].bounds);
        app.layout.undo();
        assert_eq!(
            canvas_pilot::project::Project::from_layout(&app.layout),
            before
        );
    }
    #[test]
    fn copy_and_review_preserve_all_fit_modes_and_allow_mask_only_paste() {
        use canvas_pilot::formats::{Crop, LayerFit, Mask};
        for fit in [
            None,
            Some(LayerFit::OneToOne),
            Some(LayerFit::Centered),
            Some(LayerFit::Fullscreen),
            Some(LayerFit::Cropped),
        ] {
            let ctx = egui::Context::default();
            let mut app = Editor::default();
            app.layout.select(Some(0));
            app.layout
                .set_layer_fit(
                    0,
                    fit,
                    Crop {
                        left: 40.25,
                        right: 60.5,
                        top: 15.125,
                        bottom: 25.25,
                    },
                )
                .unwrap();
            app.layout
                .set_mask(
                    0,
                    Mask {
                        origin: Default::default(),
                        left: 0.1,
                        right: 0.2,
                        top: 0.05,
                        bottom: 0.15,
                    },
                )
                .unwrap();
            let output = ctx.run_ui(egui::RawInput::default(), |ui| app.copy(ui.ctx()));
            let text = output
                .platform_output
                .commands
                .iter()
                .find_map(|c| match c {
                    egui::OutputCommand::CopyText(text) => Some(text.clone()),
                    _ => None,
                })
                .unwrap();
            app.layout
                .set_layer_fit(1, fit, Default::default())
                .unwrap();
            app.layout.select(Some(1));
            app.open_paste(text.clone());
            let p = app.paste.take().unwrap();
            assert!(p.appearance_available());
            assert!(!p.zoom_available());
            p.apply(&mut app.layout).unwrap();
            assert_eq!(app.layout.formats().layer_fit[1], fit);
            assert_eq!(
                app.layout.formats().advanced[1],
                app.layout.formats().advanced[0]
            );
            assert_eq!(app.layout.formats().masks[1], app.layout.formats().masks[0]);
            app.layout.undo();
            let before = app.project();
            app.open_paste(text);
            let p = app.paste.as_mut().unwrap();
            p.include_position = false;
            p.include_size = false;
            click_text(&mut app, &ctx, "Appearance · crop, fit and blanking");
            click_text(&mut app, &ctx, "Crop & fit");
            assert!(!app.paste.as_ref().unwrap().include_crop_fit);
            click_text(&mut app, &ctx, "Import mapped values");
            assert!(app.paste.is_none());
            assert_eq!(app.layout.layers()[1], before.layers[1].bounds);
            assert_eq!(
                app.layout.formats().advanced[1],
                before.layers[1].advanced_crop
            );
            assert_eq!(
                app.layout.formats().layer_fit[1],
                before.layers[1].layer_fit
            );
            assert_eq!(app.layout.formats().masks[1], app.layout.formats().masks[0]);
            app.layout.undo();
            assert_eq!(app.project(), before);
        }
    }

    #[test]
    fn resize_choice_survives_deselection_and_single_layer_gestures() {
        for mode in [ResizeMode::Same, ResizeMode::Mirror, ResizeMode::Opposite] {
            let ctx = egui::Context::default();
            let mut app = Editor {
                resize_mode: mode,
                ..Default::default()
            };
            app.layout.select(None);
            app.sync();
            frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
            assert_eq!(app.resize_mode, mode);
            app.layout.select(Some(1));
            app.sync();
            assert_eq!(app.active_resize_mode(), ResizeMode::Individual);
            let before = app.layout.layers().clone();
            let output = frame(&mut app, &ctx, vec![]);
            let start = layer_rects(&output)[1].right_center();
            let end = start + Vec2::new(25.0, 0.0);
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(start), mouse(start, true)],
            );
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
            frame(&mut app, &ctx, vec![mouse(end, false)]);
            assert_eq!(app.layout.layers()[0], before[0]);
            assert!(app.layout.layers()[1].width > before[1].width);
            assert_eq!(app.resize_mode, mode);
            app.layout.undo();
            app.sync();
            assert_eq!(*app.layout.layers(), before);
            app.layout.redo();
            app.sync();
            assert_eq!(app.resize_mode, mode);
            app.layout.set_selection([true; 2]);
            app.sync();
            assert_eq!(app.active_resize_mode(), mode);
            // Pair-only modes remain selected but refuse three-layer resizing.
            app.layout.add_layer().unwrap();
            app.layout.set_selection([true; 3]);
            app.sync();
            let result = app.layout.start_selected_resize(
                0,
                ResizeHandle::Right,
                [0.0, 0.0],
                true,
                app.active_resize_mode(),
            );
            assert_eq!(result.is_ok(), mode == ResizeMode::Same);
            app.layout.cancel_drag();
            assert_eq!(app.resize_mode, mode);
        }
    }

    #[test]
    fn selected_resize_mode_control_keeps_selection_during_handle_drag() {
        for (title, mode) in [
            ("Same handles", ResizeMode::Same),
            ("Mirror horizontally (two layers)", ResizeMode::Mirror),
            ("Opposite handles (two layers)", ResizeMode::Opposite),
        ] {
            let mut app = Editor::default();
            app.layout.set_selection([true; 2]);
            let ctx = egui::Context::default();
            frame(&mut app, &ctx, vec![]);
            for text in ["Resize: individual layer", title] {
                let output = frame(&mut app, &ctx, vec![]);
                let pos = output
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::epaint::Shape::Text(t) if t.galley.job.text == text => {
                            Some(t.pos + Vec2::new(12.0, 6.0))
                        }
                        _ => None,
                    })
                    .unwrap();
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
                );
                frame(&mut app, &ctx, vec![mouse(pos, false)]);
            }
            assert_eq!(app.resize_mode, mode);
            let before = app.layout.layers().clone();
            let output = frame(&mut app, &ctx, vec![]);
            let start = layer_rects(&output)[1].right_center();
            let end = start + Vec2::new(20.0, 0.0);
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(start), mouse(start, true)],
            );
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
            frame(&mut app, &ctx, vec![mouse(end, false)]);
            assert_eq!(app.layout.selection_mask(), [true; 2]);
            assert!(
                app.layout
                    .layers()
                    .iter()
                    .zip(before.clone())
                    .all(|(g, old)| g.width > old.width)
            );
            app.layout.undo();
            assert_eq!(*app.layout.layers(), before);
            frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
            assert_eq!(app.resize_mode, mode);
        }
    }
    #[test]
    fn side_handle_hit_regions_resize_at_multiple_zoom_levels() {
        for zoom in [0.6, 1.0] {
            for side in 0..4 {
                let mut app = Editor {
                    zoom,
                    keep_aspect_ratio: false,
                    ..Default::default()
                };
                let ctx = egui::Context::default();
                let output = frame(&mut app, &ctx, vec![]);
                let rect = layer_rects(&output)[1];
                let before = app.layout.layers()[1];
                let (start, delta) = match side {
                    0 => (rect.center_top(), Vec2::new(0.0, -20.0)),
                    1 => (rect.center_bottom(), Vec2::new(0.0, 20.0)),
                    2 => (rect.left_center(), Vec2::new(-20.0, 0.0)),
                    _ => (rect.right_center(), Vec2::new(20.0, 0.0)),
                };
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::PointerMoved(start), mouse(start, true)],
                );
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::PointerMoved(start + delta)],
                );
                frame(&mut app, &ctx, vec![mouse(start + delta, false)]);
                let after = app.layout.layers()[1];
                match side {
                    0 => assert_eq!(after.y + after.height, before.y + before.height),
                    1 => assert_eq!(after.y, before.y),
                    2 => assert_eq!(after.x + after.width, before.x + before.width),
                    _ => assert_eq!(after.x, before.x),
                }
                if side < 2 {
                    assert!(after.height > before.height);
                    assert_eq!(after.width, before.width);
                } else {
                    assert!(after.width > before.width);
                    assert_eq!(after.height, before.height);
                }
                app.layout.undo();
                assert_eq!(app.layout.layers()[1], before);
            }
        }
    }
    #[test]
    fn corner_handles_resize_at_different_zoom_levels_and_escape_restores() {
        for zoom in [0.6, 1.0] {
            for keep_aspect_ratio in [true, false] {
                let mut app = Editor {
                    zoom,
                    keep_aspect_ratio,
                    ..Default::default()
                };
                let ctx = egui::Context::default();
                let output = frame(&mut app, &ctx, vec![]);
                let before = app.layout.layers().clone();
                let start = layer_rects(&output)[1].right_bottom();
                let end = start + Vec2::new(35.0, 5.0);
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::PointerMoved(start), mouse(start, true)],
                );
                frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
                frame(&mut app, &ctx, vec![mouse(end, false)]);
                let resized = app.layout.layers()[1];
                assert!(resized.width > before[1].width);
                assert_eq!([resized.x, resized.y], [before[1].x, before[1].y]);
                assert_eq!(app.layout.layers()[0], before[0]);
                let ratio_changed =
                    (resized.width / resized.height - before[1].width / before[1].height).abs()
                        > 1e-8;
                assert_eq!(ratio_changed, !keep_aspect_ratio);
                app.layout.undo();
                assert_eq!(*app.layout.layers(), before);
                let output = frame(&mut app, &ctx, vec![]);
                let start = layer_rects(&output)[1].left_top();
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::PointerMoved(start), mouse(start, true)],
                );
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::PointerMoved(start - Vec2::splat(20.0))],
                );
                frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
                assert_eq!(*app.layout.layers(), before);
            }
        }
    }
    #[test]
    fn dropped_geometry_files_use_the_regular_review_without_changing_layers() {
        use canvas_pilot::interchange::Codec;
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let before = app.layout.layers().clone();
        let project = canvas_pilot::project::Project::from_layout(&app.layout);
        let ids: Vec<_> = project.layers.iter().map(|l| l.id).collect();
        let dir = tempfile::tempdir().unwrap();
        for codec in Codec::available()
            .into_iter()
            .filter(|c| *c != Codec::Illustrator)
        {
            let bytes =
                canvas_pilot::interchange::export(&project, &ids, codec, Default::default())
                    .unwrap();
            let name = format!("example.{}", codec.extension().to_ascii_uppercase());
            for path_backed in [false, true] {
                let file = if path_backed {
                    let path = dir.path().join(&name);
                    std::fs::write(&path, &bytes).unwrap();
                    egui::DroppedFile {
                        path: Some(path),
                        ..Default::default()
                    }
                } else {
                    egui::DroppedFile {
                        name: name.clone(),
                        bytes: Some(bytes.clone().into()),
                        ..Default::default()
                    }
                };
                let _ = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::new(1440.0, 940.0),
                        )),
                        dropped_files: vec![file],
                        ..Default::default()
                    },
                    |ui| app.ui(ui),
                );
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while app.paste.as_ref().unwrap().import_job.is_some()
                    && std::time::Instant::now() < deadline
                {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    frame(&mut app, &ctx, vec![]);
                }
                let paste = app.paste.as_ref().unwrap();
                assert!(paste.import_job.is_none());
                assert!(paste.error.is_empty(), "{codec:?}: {}", paste.error);
                assert_eq!(
                    paste.shapes.iter().filter(|s| !s.background_only).count(),
                    2
                );
                assert_eq!(app.layout.layers(), &before);
                assert!(app.references.is_empty());
                app.paste = None;
            }
        }
        for extension in ["pptx", "fig", "figcraft", "ai", "xml"] {
            assert!(Editor::geometry_drop(&egui::DroppedFile {
                name: format!("test.{extension}"),
                ..Default::default()
            }));
        }
        assert!(!Editor::geometry_drop(&egui::DroppedFile {
            name: "picture.png".into(),
            ..Default::default()
        }));
    }

    #[test]
    fn geometry_drop_batches_and_failures_keep_imports_atomic() {
        let mut app = Editor::default();
        let before = app.layout.layers().clone();
        let project = canvas_pilot::project::Project::from_layout(&app.layout);
        let ids: Vec<_> = project.layers.iter().map(|l| l.id).collect();
        let bytes = canvas_pilot::interchange::export(
            &project,
            &ids,
            canvas_pilot::interchange::Codec::Svg,
            Default::default(),
        )
        .unwrap();
        let file = |name: &str, bytes: Vec<u8>| egui::DroppedFile {
            name: name.into(),
            bytes: Some(bytes.into()),
            ..Default::default()
        };
        app.open_paste(String::new());
        let paste = app.paste.as_mut().unwrap();
        Editor::read_geometry_files(
            vec![
                file("first.svg", bytes.clone()),
                file("second.svg", bytes.clone()),
            ],
            paste,
        );
        let pages = paste
            .import_job
            .take()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap()
            .unwrap();
        assert_eq!(pages.len(), 2);
        assert!(pages[0].name.starts_with("first.svg"));
        assert!(pages[1].name.starts_with("second.svg"));
        Editor::read_geometry_files(
            vec![file("good.svg", bytes), file("bad.svg", b"broken".to_vec())],
            paste,
        );
        let error = paste
            .import_job
            .take()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap()
            .unwrap_err();
        assert!(error.contains("bad.svg"));
        assert!(paste.shapes.is_empty());
        assert!(paste.mapping.is_empty());
        Editor::read_geometry_files(
            vec![file(
                "huge.svg",
                vec![0; canvas_pilot::exchange::MAX_EXCHANGE_BYTES + 1],
            )],
            paste,
        );
        assert!(
            paste
                .import_job
                .take()
                .unwrap()
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap()
                .unwrap_err()
                .contains("16 MiB")
        );
        assert_eq!(app.layout.layers(), &before);
    }

    #[test]
    fn dropped_reference_warns_without_editing_layers_and_failed_replace_keeps_image() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let before = app.layout.layers().clone();
        let pixels = image::RgbaImage::from_pixel(16, 10, image::Rgba([50, 90, 120, 255]));
        let mut bytes = std::io::Cursor::new(Vec::new());
        pixels
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 940.0))),
                dropped_files: vec![egui::DroppedFile {
                    name: "reference.png".into(),
                    bytes: Some(bytes.into_inner().into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
            |ui| app.ui(ui),
        );
        app.wait_reference();
        assert!(
            app.references
                .first()
                .unwrap()
                .warning([1920, 1080])
                .unwrap()
                .0
        );
        assert_eq!(*app.layout.layers(), before);
        let output = frame(&mut app, &ctx, vec![]);
        assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.job.text.contains("SEVERE WARNING"))));
        let texture = app.references.first().unwrap().texture.id();
        app.load_reference(
            &ctx,
            &egui::DroppedFile {
                bytes: Some(Vec::from(b"broken").into()),
                ..Default::default()
            },
        );
        app.wait_reference();
        assert!(!app.reference_error.is_empty());
        assert_eq!(app.references.first().unwrap().texture.id(), texture);
        app.layout.nudge(1.0, 0.0).unwrap();
        assert_eq!(app.layout.layers()[1].x, before[1].x + 1.0);
        app.layout.undo();
        assert_eq!(*app.layout.layers(), before);
        assert_eq!(app.references.first().unwrap().texture.id(), texture);
    }
    #[test]
    fn paste_without_selection_maps_directly_from_rectangle_dropdown() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        app.layout.select(None);
        let before = app.layout.layers().clone();
        app.open_paste(include_str!("../tests/fixtures/drawio-rectangle.txt").into());
        assert!(app.paste.as_ref().unwrap().targets.is_empty());
        fn text_position(output: &egui::FullOutput, label: &str) -> Pos2 {
            output
                .shapes
                .iter()
                .rev()
                .find_map(|s| match &s.shape {
                    egui::epaint::Shape::Text(t) if t.galley.job.text == label => {
                        Some(t.pos + Vec2::new(12.0, 6.0))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing label {label}"))
        }
        frame(&mut app, &ctx, vec![]);
        let output = frame(&mut app, &ctx, vec![]);
        let pos = text_position(&output, "Choose destination…");
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(&mut app, &ctx, vec![mouse(pos, false)]);
        let output = frame(&mut app, &ctx, vec![]);
        let pos = text_position(&output, &app.layout.layer_label(0));
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(&mut app, &ctx, vec![mouse(pos, false)]);
        let paste = app.paste.as_ref().unwrap();
        assert!(paste.targets.is_empty());
        assert_eq!(paste.mapping, vec![Some(0)]);
        assert_eq!(*app.layout.layers(), before);
        paste.apply(&mut app.layout).unwrap();
        assert_eq!(app.layout.layers()[0].width, 945.0);
        assert_eq!(app.layout.layers()[1], before[1]);
        assert!(app.layout.selected_indices().is_empty());
        app.layout.undo();
        assert_eq!(*app.layout.layers(), before);
    }
    #[test]
    fn import_review_hides_acquisition_and_collapses_clipboard_details() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let source = include_str!("../tests/fixtures/drawio-rectangle.txt");
        app.open_paste(source.into());
        fn texts(output: &egui::FullOutput) -> Vec<String> {
            output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::epaint::Shape::Text(t) => Some(t.galley.job.text.clone()),
                    _ => None,
                })
                .collect()
        }
        frame(&mut app, &ctx, vec![]);
        let labels = texts(&frame(&mut app, &ctx, vec![]));
        for label in [
            "Clipboard data",
            "Destination layers",
            "Destination",
            "Import mapped values",
        ] {
            assert!(labels.iter().any(|t| t == label), "Missing {label}");
        }
        for label in [
            "Import file…",
            "Read geometry",
            "Review pasted values",
            source,
            "Before",
            "After",
        ] {
            assert!(!labels.iter().any(|t| t == label), "Unexpected {label}");
        }
        click_text(&mut app, &ctx, "Clipboard data");
        for _ in 0..20 {
            frame(&mut app, &ctx, vec![]);
        }
        assert!(
            texts(&frame(&mut app, &ctx, vec![]))
                .iter()
                .any(|t| t == source)
        );
        app.open_paste("invalid".into());
        frame(&mut app, &ctx, vec![]);
        assert!(texts(&frame(&mut app, &ctx, vec![])).contains(&"Review pasted values".into()));
    }

    #[test]
    fn import_scope_filters_destinations_without_changing_selection() {
        let mut app = Editor::default();
        let selection = app.layout.selected_indices();
        let id = app.layout.layer_info(0).id;
        app.layout.move_layer_to_folder(id, "Memory A", 0).unwrap();
        app.open_paste(include_str!("../tests/fixtures/drawio-rectangle.txt").into());
        let p = app.paste.as_mut().unwrap();
        p.set_scope(ImportScope::Folder("Memory A".into()), &app.layout);
        assert_eq!(p.candidates(&app.layout), vec![0]);
        assert_eq!(p.mapping, vec![None]);
        p.mapping[0] = Some(0);
        p.apply(&mut app.layout).unwrap();
        assert_eq!(app.layout.selected_indices(), selection);
        assert_eq!(app.layout.layers()[0].width, 945.0);
        p.set_scope(ImportScope::All, &app.layout);
        assert_eq!(p.mapping, vec![Some(0)]);
        p.set_scope(ImportScope::Selected, &app.layout);
        assert_eq!(p.mapping, vec![None]);
        p.set_scope(ImportScope::Folder("Empty".into()), &app.layout);
        assert!(p.candidates(&app.layout).is_empty());
        assert!(p.values(&app.layout).is_err());
    }
    #[test]
    fn fewer_pasted_rectangles_can_map_to_either_selected_layer() {
        for target in 0..2 {
            let mut app = Editor::default();
            app.layout.set_selection([true; 2]);
            let before = app.layout.layers().clone();
            let formats = app.layout.formats().clone();
            app.open_paste(include_str!("../tests/fixtures/drawio-rectangle.txt").into());
            let paste = app.paste.as_mut().unwrap();
            paste.mapping[0] = Some(target);
            paste.shapes[0].zoom = Some(2.0);
            paste.apply(&mut app.layout).unwrap();
            assert_eq!(app.layout.layers()[target].width, 945.0);
            assert_eq!(app.layout.layers()[1 - target], before[1 - target]);
            assert_eq!(
                app.layout.formats().advanced[1 - target],
                formats.advanced[1 - target]
            );
            assert_eq!(app.layout.selected_indices(), vec![0, 1]);
            app.layout.undo();
            assert_eq!(*app.layout.layers(), before);
            assert_eq!(*app.layout.formats(), formats);
            app.layout.redo();
            assert_eq!(app.layout.layers()[target].width, 945.0);
        }
    }
    #[test]
    fn paste_opens_review_without_editing_and_escape_asks_before_discarding() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        let before = app.layout.layers().clone();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Paste(
                include_str!("../tests/fixtures/drawio-rectangle.txt").into(),
            )],
        );
        assert_eq!(*app.layout.layers(), before);
        let p = app.paste.as_ref().unwrap();
        assert_eq!(p.targets, vec![1]);
        assert_eq!(p.shapes[0].geometry.width, 945.0);
        frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
        assert!(app.paste.as_ref().unwrap().closing);
        assert_eq!(*app.layout.layers(), before);
        cancel_pending_close(&mut app, &ctx);
        let p = app.paste.as_ref().unwrap();
        assert!(!p.closing);
        assert_eq!(p.targets, vec![1]);
        assert_eq!(p.shapes[0].geometry.width, 945.0);
        assert_eq!(*app.layout.layers(), before);
    }
    #[test]
    fn exchange_background_is_automatically_excluded_from_mapping() {
        use canvas_pilot::exchange::{self, Format, Options};
        for format in [Format::Svg, Format::Drawio] {
            let mut app = Editor::default();
            app.layout.set_selection([true; 2]);
            let project = canvas_pilot::project::Project::from_layout(&app.layout);
            let ids = project.layers.iter().map(|l| l.id).collect::<Vec<_>>();
            let text = exchange::export(
                &project,
                &ids,
                format,
                Options {
                    extra_parameters: false,
                    background: true,
                    placeholders: true,
                },
            )
            .unwrap();
            app.open_paste(text);
            let paste = app.paste.as_ref().unwrap();
            assert_eq!(paste.ignored, vec![true, false, false]);
            assert_eq!(paste.mapping, vec![None, Some(0), Some(1)]);
            assert_eq!(paste.values(&app.layout).unwrap().len(), 2);
        }
    }

    #[test]
    fn import_review_excludes_background_and_applies_reversed_mapping_atomically() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        app.layout.set_selection([true; 2]);
        let before = app.layout.layers().clone();
        app.open_paste(r#"<svg width="1920" height="1080"><rect width="1920" height="1080"/><rect id="a" x="12" y="30" width="400" height="300"/><rect id="b" x="800" y="50" width="500" height="400"/></svg>"#.into());
        let p = app.paste.as_mut().unwrap();
        assert!(
            p.values(&app.layout)
                .unwrap_err()
                .contains("2 available, 3 detected")
        );
        p.ignored[0] = true;
        p.reset_mapping();
        p.mapping[1] = Some(1);
        p.mapping[2] = Some(1);
        assert!(p.values(&app.layout).is_err());
        p.mapping[2] = Some(0);
        let expected = [p.shapes[2].geometry, p.shapes[1].geometry];
        assert_eq!(*app.layout.layers(), before);
        frame(&mut app, &ctx, vec![]);
        let output = frame(&mut app, &ctx, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) if t.galley.job.text == "Import mapped values" => {
                    Some(t.pos + Vec2::new(20.0, 6.0))
                }
                _ => None,
            })
            .unwrap();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(&mut app, &ctx, vec![mouse(pos, false)]);
        assert!(app.paste.is_none());
        assert_eq!(*app.layout.layers(), expected);
        app.layout.undo();
        assert_eq!(*app.layout.layers(), before);
    }
    #[test]
    fn numeric_focus_keeps_paste_out_of_importer_and_arrows_out_of_geometry() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        ctx.memory_mut(|m| m.request_focus(egui::Id::new(("geometry", 0_usize))));
        frame(&mut app, &ctx, vec![]);
        assert!(ctx.egui_wants_keyboard_input());
        let before = app.layout.layers().clone();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Paste("123".into()), key(egui::Key::ArrowLeft)],
        );
        assert!(app.paste.is_none());
        assert_eq!(*app.layout.layers(), before);
    }
    #[test]
    fn invalid_numeric_edits_preserve_document_and_valid_edits_are_reversible() {
        let mut app = Editor::default();
        let before = app.layout.layers().clone();
        app.fields = ["0".into(), "0".into(), "NaN".into(), "300".into()];
        app.apply_fields();
        assert!(!app.error.is_empty());
        assert_eq!(*app.layout.layers(), before);
        app.fields = ["120".into(), "180".into(), "945".into(), "531.58".into()];
        app.apply_fields();
        assert!(app.error.is_empty());
        assert_eq!(app.layout.layers()[1].width, 945.0);
        app.layout.undo();
        assert_eq!(*app.layout.layers(), before);
    }
    #[test]
    fn paste_without_selection_and_invalid_xml_do_not_edit() {
        let mut app = Editor::default();
        let before = app.layout.layers().clone();
        app.layout.select(None);
        app.open_paste("invalid".into());
        assert!(app.paste.as_ref().unwrap().targets.is_empty());
        assert!(!app.paste.as_ref().unwrap().error.is_empty());
        app.layout.select(Some(0));
        app.open_paste("invalid".into());
        assert!(app.paste.as_ref().unwrap().shapes.is_empty());
        assert!(!app.paste.as_ref().unwrap().error.is_empty());
        assert_eq!(*app.layout.layers(), before);
    }
    pub(super) fn mouse(p: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    #[test]
    fn modal_apply_button_imports_exact_sample_and_undo_restores() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        let before = app.layout.layers().clone();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Paste(
                include_str!("../tests/fixtures/drawio-rectangle.txt").into(),
            )],
        );
        let output = frame(&mut app, &ctx, vec![]);
        let position = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) if t.galley.job.text == "Import mapped values" => {
                    Some(t.pos + Vec2::new(30.0, 6.0))
                }
                _ => None,
            })
            .expect("Apply button rendered");
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(position), mouse(position, true)],
        );
        frame(&mut app, &ctx, vec![mouse(position, false)]);
        assert!(app.paste.is_none());
        assert_eq!(app.layout.layers()[1].x, 12.0);
        assert_eq!(app.layout.layers()[1].height, 531.58);
        assert_eq!(app.layout.layers()[0], before[0]);
        app.layout.undo();
        assert_eq!(*app.layout.layers(), before);
    }
    fn frame_modifiers(
        app: &mut Editor,
        ctx: &egui::Context,
        modifiers: egui::Modifiers,
        mut events: Vec<egui::Event>,
    ) {
        for event in &mut events {
            if let egui::Event::PointerButton {
                modifiers: current, ..
            } = event
            {
                *current = modifiers;
            }
        }
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 940.0))),
                modifiers,
                events,
                ..Default::default()
            },
            |ui| app.ui(ui),
        );
    }
    #[test]
    fn modifier_grids_move_and_resize_through_release_at_different_zoom_levels() {
        for resize in [false, true] {
            for enabled in [false, true] {
                for (ctrl, spacing) in [(false, 10.0), (true, 100.0)] {
                    for zoom in [0.75, 1.0] {
                        let ctx = egui::Context::default();
                        let mut app = Editor {
                            zoom,
                            ..Default::default()
                        };
                        app.snapping.enabled = enabled;
                        let before = app.layout.layers().clone();
                        let output = frame(&mut app, &ctx, vec![]);
                        let rect = layer_rects(&output)[1];
                        let scale = rect.width() / before[1].width as f32;
                        let start = if resize {
                            rect.right_center()
                        } else {
                            rect.center()
                        };
                        let end = start + Vec2::new(37.0, 23.0) * scale;
                        let modifiers = egui::Modifiers {
                            shift: true,
                            ctrl,
                            command: ctrl,
                            ..Default::default()
                        };
                        for events in [
                            vec![egui::Event::PointerMoved(start), mouse(start, true)],
                            vec![egui::Event::PointerMoved(end)],
                            vec![mouse(end, false)],
                        ] {
                            frame_modifiers(&mut app, &ctx, modifiers, events);
                        }
                        let next = app.layout.layers()[1];
                        if resize {
                            assert!(
                                (next.x + next.width
                                    - ((before[1].x + before[1].width + 37.0) / spacing).round()
                                        * spacing)
                                    .abs()
                                    < 0.01
                            );
                            assert_eq!(next.x, before[1].x);
                        } else {
                            assert!(
                                (next.x - ((before[1].x + 37.0) / spacing).round() * spacing).abs()
                                    < 0.01
                            );
                            assert!(
                                (next.y - ((before[1].y + 23.0) / spacing).round() * spacing).abs()
                                    < 0.01
                            );
                        }
                        assert_eq!(app.layout.layers()[0], before[0]);
                        assert_eq!(app.snapping.enabled, enabled);
                        assert!(app.marquee.is_none());
                        app.layout.undo();
                        assert_eq!(*app.layout.layers(), before);
                    }
                }
            }
        }
    }

    #[test]
    fn grid_release_uses_current_modifiers_and_alt_bypasses_both_grids() {
        for alt in [false, true] {
            let ctx = egui::Context::default();
            let mut app = Editor::default();
            app.snapping.enabled = true;
            let before = app.layout.layers().clone();
            let output = frame(&mut app, &ctx, vec![]);
            let rect = layer_rects(&output)[1];
            let scale = rect.width() / before[1].width as f32;
            let start = rect.center();
            let end = start + Vec2::new(13.0, 37.0) * scale;
            let shift = egui::Modifiers {
                shift: true,
                ..Default::default()
            };
            frame_modifiers(
                &mut app,
                &ctx,
                shift,
                vec![egui::Event::PointerMoved(start), mouse(start, true)],
            );
            frame_modifiers(&mut app, &ctx, shift, vec![egui::Event::PointerMoved(end)]);
            assert_eq!(app.layout.layers()[1].x, 1170.0);
            assert_eq!(app.layout.layers()[1].y, 220.0);
            frame_modifiers(
                &mut app,
                &ctx,
                egui::Modifiers {
                    ctrl: true,
                    alt,
                    ..shift
                },
                vec![mouse(end, false)],
            );
            let g = app.layout.layers()[1];
            assert!((g.x - if alt { 1173.0 } else { 1200.0 }).abs() < 0.01);
            assert!((g.y - if alt { 217.0 } else { 200.0 }).abs() < 0.01);
            app.layout.undo();
            assert_eq!(*app.layout.layers(), before);
        }
    }
    #[test]
    fn source_tab_opens_shared_input_settings_without_local_precrop_controls() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout.select(Some(0));
        app.format_draft = Some(FormatDraft::new(&app.layout, 0));
        app.property_tab = PropertyTab::Source;
        let before = app.project();
        click_text(&mut app, &ctx, "Open shared input settings…");
        assert!(app.inputs.open);
        assert_eq!(app.project(), before);
    }
    #[test]
    fn snapped_move_and_resize_stay_snapped_on_release_and_undo_as_one_edit() {
        for resize in [false, true] {
            for zoom in [0.75, 1.0] {
                let ctx = egui::Context::default();
                let mut app = Editor {
                    zoom,
                    keep_aspect_ratio: false,
                    ..Default::default()
                };
                app.snapping.enabled = true;
                app.snapping.pixels = false;
                app.snapping.layer_edges = true;
                let before = app.layout.layers().clone();
                let output = frame(&mut app, &ctx, vec![]);
                let rect = layer_rects(&output)[1];
                let scale = rect.width() / before[1].width as f32;
                let start = if resize {
                    rect.right_center()
                } else {
                    rect.center()
                };
                let delta = if resize {
                    1920.0 - before[1].x - before[1].width
                } else {
                    -before[1].x
                };
                let end = start + Vec2::new(delta as f32 * scale - 3.0, 0.0);
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::PointerMoved(start), mouse(start, true)],
                );
                frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
                let snapped = app.layout.layers().clone();
                let edge = if resize {
                    snapped[1].x + snapped[1].width
                } else {
                    snapped[1].x
                };
                assert!(
                    (edge - if resize { 1920.0 } else { 0.0 }).abs() < 0.001,
                    "resize={resize} zoom={zoom} edge={edge} start={start:?} end={end:?}"
                );
                frame(&mut app, &ctx, vec![mouse(end, false)]);
                assert_eq!(
                    *app.layout.layers(),
                    snapped,
                    "release must preserve snapping"
                );
                frame(&mut app, &ctx, vec![]);
                assert_eq!(*app.layout.layers(), snapped);
                app.layout.undo();
                assert_eq!(*app.layout.layers(), before);
                app.layout.redo();
                assert_eq!(*app.layout.layers(), snapped);
            }
        }
    }
    #[test]
    fn pointer_drag_is_one_edit_and_does_not_move_other_layer() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        let before = app.layout.layers().clone();
        let start = Pos2::new(700.0, 420.0);
        let end = start + Vec2::new(40.0, 20.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        frame(&mut app, &ctx, vec![mouse(end, false)]);
        assert_eq!(app.layout.layers()[0], before[0]);
        assert!(app.layout.layers()[1].x > before[1].x);
        assert!(app.layout.layers()[1].y > before[1].y);
        app.layout.undo();
        assert_eq!(*app.layout.layers(), before);
    }
    #[test]
    fn list_selection_then_paste_targets_that_layer() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        let p = ctx
            .read_response(egui::Id::new(("layer", 2_usize)))
            .unwrap()
            .rect
            .center();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(p), mouse(p, true)],
        );
        frame(&mut app, &ctx, vec![mouse(p, false)]);
        assert_eq!(app.layout.selected(), Some(0));
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Paste(
                include_str!("../tests/fixtures/drawio-rectangle.txt").into(),
            )],
        );
        assert_eq!(app.paste.as_ref().unwrap().targets, vec![0]);
    }
    #[test]
    fn zoom_changes_camera_without_geometry_or_undo_edits() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        let before = app.layout.layers().clone();
        frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(Pos2::new(700.0, 420.0)),
                egui::Event::MouseWheel {
                    phase: egui::TouchPhase::Move,
                    unit: egui::MouseWheelUnit::Point,
                    delta: Vec2::new(0.0, 100.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert!(app.zoom > 1.0);
        assert_eq!(*app.layout.layers(), before);
        app.layout.undo();
        assert_eq!(*app.layout.layers(), before);
    }
    #[test]
    fn outline_checkbox_changes_preview_then_applies_as_one_undo_step() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        let before = app.layout.layers().clone();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Paste(
                include_str!("../tests/fixtures/drawio-rectangle.txt").into(),
            )],
        );
        assert!(!app.paste.as_ref().unwrap().include_outline);
        for text in ["Include outline in bounds", "Import mapped values"] {
            let output = frame(&mut app, &ctx, vec![]);
            let pos = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::epaint::Shape::Text(t) if t.galley.job.text == text => {
                        Some(t.pos + Vec2::new(20.0, 6.0))
                    }
                    _ => None,
                })
                .expect("paste control rendered");
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
            if text == "Include outline in bounds" {
                assert!(app.paste.as_ref().unwrap().include_outline);
                assert_eq!(*app.layout.layers(), before);
            }
        }
        assert!(app.paste.is_none());
        assert_eq!(
            app.layout.layers()[1],
            Geometry {
                x: 9.5,
                y: 268.5,
                width: 950.0,
                height: 536.58
            }
        );
        app.layout.undo();
        assert_eq!(*app.layout.layers(), before);
    }
    fn frame_shift(
        app: &mut Editor,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 940.0))),
                events,
                modifiers: egui::Modifiers::SHIFT,
                ..Default::default()
            },
            |ui| app.ui(ui),
        )
    }
    fn click_shift(app: &mut Editor, ctx: &egui::Context, p: Pos2) {
        let event = |pressed| egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::SHIFT,
        };
        frame_shift(app, ctx, vec![egui::Event::PointerMoved(p), event(true)]);
        frame(app, ctx, vec![event(false)]); // Shift released by the end of this frame.
    }
    fn layer_rects(output: &egui::FullOutput) -> Vec<Rect> {
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(r) if r.fill == Color32::from_rgb(23, 59, 80) => {
                    Some(r.rect)
                }
                _ => None,
            })
            .collect()
    }
    #[test]
    fn double_click_layer_name_renames_and_supports_cancel_and_undo() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        let pos = ctx
            .read_response(egui::Id::new(("layer", 1_usize)))
            .unwrap()
            .rect
            .center();
        for _ in 0..2 {
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
        }
        assert!(app.layer_ui.rename.is_some());
        let original = app.layout.layer_info(1).name.clone();
        app.layer_ui.rename.as_mut().unwrap().1 = "Main slides".into();
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert!(app.layer_ui.rename.is_none());
        assert_eq!(app.layout.layer_info(1).name, "Main slides");
        app.layout.undo();
        assert_eq!(app.layout.layer_info(1).name, original);
        app.layer_ui.rename = Some((app.layout.layer_info(1).id, "Canceled".into()));
        frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
        assert!(app.layer_ui.rename.is_none());
        assert_eq!(app.layout.layer_info(1).name, original);
    }
    #[test]
    fn layer_buttons_protect_selection_and_toggle_drawing() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![]);
        let before = app.layout.layers().clone();
        for (kind, expected_count) in [
            ("lock", 2),
            ("visibility", 1),
            ("visibility", 2),
            ("lock", 2),
        ] {
            let rect = ctx
                .read_response(egui::Id::new((kind, 1_usize)))
                .unwrap()
                .rect;
            click_shift(&mut app, &ctx, rect.center());
            let output = frame(&mut app, &ctx, vec![]);
            assert_eq!(layer_rects(&output).len(), expected_count);
            assert_eq!(app.layout.selected(), None);
            assert_eq!(*app.layout.layers(), before);
        }
        assert!(!app.layout.locked(1));
        assert!(app.layout.visible(1));
        let row = ctx
            .read_response(egui::Id::new(("layer", 1_usize)))
            .unwrap()
            .rect;
        click_shift(&mut app, &ctx, row.center());
        assert_eq!(app.layout.selected(), Some(1));
    }
    #[test]
    fn position_pending_state_preserves_precision_and_clears_after_apply_or_revert() {
        let mut app = Editor::default();
        let mut g = app.layout.layers()[1];
        g.width = 640.12345;
        app.layout.apply(1, g).unwrap();
        app.sync();
        let original = app.fields.clone();
        assert!(!app.fields_pending());
        app.fields[2] = "640.1000".into();
        assert!(!app.fields_pending());
        app.fields[0] = "invalid".into();
        assert!(app.fields_pending());
        app.fields = original.clone();
        assert!(!app.fields_pending());
        app.fields[0] = "150".into();
        assert!(app.fields_pending());
        app.apply_fields();
        assert!(!app.fields_pending());
        assert_eq!(app.layout.layers()[1].width, g.width);
        app.layout.undo();
        app.sync();
        assert!(!app.fields_pending());
        assert_eq!(app.fields, original);
    }

    #[test]
    fn crop_menu_renders_translations_in_both_editing_states() {
        for language in localization::Language::ALL {
            let ctx = egui::Context::default();
            language.apply(&ctx);
            for editing in [false, true] {
                let mut app = Editor {
                    edit_content: editing,
                    ..Default::default()
                };
                assert!(app.layout.selected().is_some());
                let output = ctx.run_ui(egui::RawInput::default(), |root_ui| {
                    egui::CentralPanel::default().show(root_ui, |ui| {
                        app.crop_mask_menu(ui, app.layout.selected().unwrap())
                    });
                });
                let expected = tr(&ctx, if editing { "crop.done" } else { "crop.adjust" });
                let labels: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => Some(text.galley.text()),
                        _ => None,
                    })
                    .collect();
                assert!(
                    labels.contains(&expected.as_str()),
                    "{language:?}: missing {expected}"
                );
                assert!(
                    !labels.iter().any(|label| label.starts_with("crop.")),
                    "raw key in {language:?} menu"
                );
                assert!(labels.contains(&"Convert to Edge crop"));
                assert!(!labels.contains(&"Convert to Smart crop"));
            }
        }
    }

    #[test]
    fn viewport_and_list_context_actions_target_the_clicked_layer() {
        for list in [false, true] {
            for rename in [false, true] {
                let ctx = egui::Context::default();
                let mut app = Editor {
                    navigation_tool: NavigationTool::Move,
                    ..Default::default()
                };
                let before = app.layout.layers().clone();
                let id = app.layout.layer_info(0).id;
                frame(&mut app, &ctx, vec![]);
                let output = frame(&mut app, &ctx, vec![]);
                let pos = if list {
                    ctx.read_response(egui::Id::new(("layer", id)))
                        .unwrap()
                        .rect
                        .center()
                } else {
                    layer_rects(&output)[0].center()
                };
                let secondary = |pressed| egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                };
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::PointerMoved(pos), secondary(true)],
                );
                frame(&mut app, &ctx, vec![secondary(false)]);
                let output = frame(&mut app, &ctx, vec![]);
                assert_eq!(app.layout.selected(), Some(1));
                if list {
                    assert!(!output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Source properties…")));
                }
                click_text(
                    &mut app,
                    &ctx,
                    if rename {
                        "Rename layer…"
                    } else {
                        "Delete layer"
                    },
                );
                if rename {
                    assert_eq!(app.layer_ui.rename.as_ref().unwrap().0, id);
                    app.layer_ui.rename.as_mut().unwrap().1 = "Renamed target".into();
                    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
                    assert_eq!(app.layout.layer_info(0).name, "Renamed target");
                    assert_eq!(app.layout.layers(), &before);
                } else {
                    assert_eq!(app.layout.layers().len(), 1);
                    assert_eq!(app.layout.layers()[0], before[1]);
                    assert!(!app.layer_ui.open);
                }
                app.layout.undo();
                assert_eq!(app.layout.layers(), &before);
                assert_eq!(app.layout.layer_info(0).id, id);
            }
        }
    }

    #[test]
    fn grouped_crop_mask_resets_preserve_other_values_and_undo() {
        use canvas_pilot::formats::{Crop, Framing, LayerFit, Mask, MaskOrigin};
        for origin in [MaskOrigin::Layer, MaskOrigin::Source] {
            for mode in std::iter::once(None).chain(LayerFit::ALL.into_iter().map(Some)) {
                for reset_mask in [true, false] {
                    let mut app = Editor::default();
                    app.layout.select(Some(0));
                    let ctx = egui::Context::default();
                    let output = frame(&mut app, &ctx, vec![]);
                    let pos = layer_rects(&output)[0].center();
                    app.layout
                        .set_layer_fit(
                            0,
                            mode,
                            Crop {
                                left: 100.0,
                                right: 100.0,
                                top: 50.0,
                                bottom: 50.0,
                            },
                        )
                        .unwrap();
                    if mode.is_none() {
                        app.layout
                            .set_framing(
                                0,
                                Framing {
                                    zoom: 1.5,
                                    offset: [0.0; 2],
                                },
                            )
                            .unwrap();
                    }
                    let mask = Mask {
                        origin,
                        left: 0.1,
                        right: 0.2,
                        top: 0.1,
                        bottom: 0.05,
                    };
                    app.layout.set_mask(0, mask).unwrap();
                    app.sync();
                    let before = app.project();
                    let crop = app.layout.formats().advanced[0];
                    frame(&mut app, &ctx, vec![]);
                    let secondary = |pressed| egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Secondary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    };
                    frame(
                        &mut app,
                        &ctx,
                        vec![egui::Event::PointerMoved(pos), secondary(true)],
                    );
                    frame(&mut app, &ctx, vec![secondary(false)]);
                    let output = frame(&mut app, &ctx, vec![]);
                    assert_eq!(app.layout.selected(), Some(0));
                    let labels: Vec<_> = output
                        .shapes
                        .iter()
                        .filter_map(|s| {
                            if let egui::Shape::Text(t) = &s.shape {
                                Some(t.galley.text())
                            } else {
                                None
                            }
                        })
                        .collect();
                    assert!(labels.contains(&"Crop & mask") && labels.contains(&"Canvas"));
                    assert!(
                        !labels.contains(&"Reset crop")
                            && !labels.contains(&"Reset mask")
                            && !labels.contains(&"Output format…")
                    );
                    click_text(&mut app, &ctx, "Crop & mask");
                    click_text(
                        &mut app,
                        &ctx,
                        if reset_mask {
                            "Reset mask"
                        } else {
                            "Reset crop"
                        },
                    );
                    assert_eq!(app.layout.formats().layer_fit[0], mode);
                    if reset_mask {
                        assert_eq!(
                            app.layout.formats().masks[0],
                            Mask {
                                origin,
                                ..Mask::default()
                            }
                        );
                        assert_eq!(app.layout.formats().advanced[0], crop);
                    } else {
                        assert_eq!(app.layout.formats().masks[0], mask);
                        assert_eq!(app.layout.formats().advanced[0], Crop::default());
                    }
                    assert_eq!(app.project().inputs, before.inputs);
                    assert_eq!(app.project().layers[1], before.layers[1]);
                    assert_eq!(app.project().layers[0].bounds, before.layers[0].bounds);
                    assert!(!app.device.sending);
                    app.layout.undo();
                    assert_eq!(app.project(), before);
                }
            }
        }
    }

    #[test]
    fn canvas_options_are_grouped_only_when_a_layer_is_selected() {
        for selected in [false, true] {
            let mut app = Editor::default();
            if !selected {
                app.layout.select(None);
            }
            let ctx = egui::Context::default();
            let output = ctx.run_ui(egui::RawInput::default(), |root_ui| {
                egui::CentralPanel::default().show(root_ui, |ui| app.layer_context_menu(ui));
            });
            let labels: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        Some(t.galley.text())
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(labels.contains(&"Canvas"), selected);
            assert_eq!(
                labels.contains(&tr(&ctx, "ui.output-format").as_str()),
                !selected
            );
            assert_eq!(labels.contains(&"Crop & mask"), selected);
        }
    }

    #[test]
    fn canvas_context_menu_selects_target_preserves_groups_and_opens_properties() {
        for (grouped, title, tab) in [
            (false, "Source properties…", PropertyTab::Source),
            (false, "Layer properties…", PropertyTab::Layer),
            (false, "Crop properties…", PropertyTab::Crop),
            (false, "Mask properties…", PropertyTab::Mask),
            (false, "Mapping properties…", PropertyTab::Mapping),
            (false, "Configure capture…", PropertyTab::Preview),
            (true, "Lock selected layers", PropertyTab::Layer),
        ] {
            let mut app = Editor::default();
            if grouped {
                app.layout.set_selection([true; 2]);
            }
            let before = app.layout.layers().clone();
            let ctx = egui::Context::default();
            let output = frame(&mut app, &ctx, vec![]);
            let pos = layer_rects(&output)[0].center();
            let secondary = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), secondary(true)],
            );
            frame(&mut app, &ctx, vec![secondary(false)]);
            let output = frame(&mut app, &ctx, vec![]);
            assert_eq!(app.layout.selection_mask(), [true, grouped]);
            assert_eq!(*app.layout.layers(), before);
            let output = if !grouped {
                let submenu = if tab == PropertyTab::Preview {
                    "Input preview & crop guides"
                } else {
                    "Properties"
                };
                let pos = output
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t) if t.galley.text() == submenu => {
                            Some(t.pos + t.galley.size() / 2.0)
                        }
                        _ => None,
                    })
                    .expect("property submenu");
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
                );
                frame(&mut app, &ctx, vec![mouse(pos, false)]);
                frame(&mut app, &ctx, vec![])
            } else {
                output
            };
            let pos = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == title => {
                        Some(t.pos + t.galley.size() / 2.0)
                    }
                    _ => None,
                })
                .expect("context menu action");
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
            if grouped {
                assert!(app.layout.locked(0) && app.layout.locked(1));
                assert!(app.layout.selected_indices().is_empty());
            } else {
                assert!(app.format_draft.is_some());
                assert_eq!(app.property_tab, tab);
            }
            assert_eq!(*app.layout.layers(), before);
        }
    }

    #[test]
    fn list_top_matches_paint_order_and_selection_does_not_raise_layers() {
        let mut app = Editor::default();
        let mut g = app.layout.layers()[0];
        g.x += 100.0;
        app.layout.apply(1, g).unwrap();
        app.layout.select(Some(0));
        let before = app.layout.layers().clone();
        let ctx = egui::Context::default();
        let output = frame(&mut app, &ctx, vec![]);
        let rects = layer_rects(&output);
        assert!(rects[1].left() > rects[0].left());
        let top_row = ctx
            .read_response(egui::Id::new(("layer", 1_usize)))
            .unwrap()
            .rect;
        let bottom_row = ctx
            .read_response(egui::Id::new(("layer", 2_usize)))
            .unwrap()
            .rect;
        assert!(top_row.top() < bottom_row.top());
        let overlap = rects[0].intersect(rects[1]).center();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(overlap), mouse(overlap, true)],
        );
        frame(&mut app, &ctx, vec![mouse(overlap, false)]);
        assert_eq!(app.layout.selected(), Some(1));
        assert_eq!(*app.layout.layers(), before);
        app.layout.select(Some(1));
        let output = frame(&mut app, &ctx, vec![]);
        let rects = layer_rects(&output);
        assert!(rects[1].left() > rects[0].left());
    }

    #[test]
    fn shift_click_toggles_canvas_and_list_selection() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let output = frame(&mut app, &ctx, vec![]);
        let rects = layer_rects(&output);
        assert_eq!(rects.len(), 2);
        click_shift(&mut app, &ctx, rects[0].center());
        assert_eq!(app.layout.selected_indices(), vec![0, 1]);
        click_shift(&mut app, &ctx, rects[1].center());
        assert_eq!(app.layout.selected(), Some(0));
        let second = ctx
            .read_response(egui::Id::new(("layer", 1_usize)))
            .unwrap()
            .rect
            .center();
        click_shift(&mut app, &ctx, second);
        assert_eq!(app.layout.selected_indices(), vec![0, 1]);
        let row = ctx
            .read_response(egui::Id::new(("layer", 2_usize)))
            .unwrap()
            .rect;
        click_shift(&mut app, &ctx, row.center());
        assert_eq!(app.layout.selection_mask(), [false, true]);
    }
    #[test]
    fn marquee_selects_both_at_different_camera_transforms_without_moving_them() {
        for zoom in [0.6, 1.0] {
            let mut app = Editor {
                zoom,
                pan: Vec2::new(10.0, -8.0),
                ..Default::default()
            };
            let ctx = egui::Context::default();
            let output = frame(&mut app, &ctx, vec![]);
            let before = app.layout.layers().clone();
            let rects = layer_rects(&output);
            let area = rects[0].union(rects[1]).expand(10.0);
            let start = area.left_top();
            let end = area.right_bottom();
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(start), mouse(start, true)],
            );
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
            assert!(app.marquee.is_some());
            frame(&mut app, &ctx, vec![mouse(end, false)]);
            assert_eq!(app.layout.selected_indices(), vec![0, 1]);
            assert!(app.marquee.is_none());
            assert_eq!(*app.layout.layers(), before);
        }
    }
    #[test]
    fn escape_cancels_selection_box_and_restores_previous_selection() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        let output = frame(&mut app, &ctx, vec![]);
        let rects = layer_rects(&output);
        let area = rects[0].union(rects[1]).expand(10.0);
        let start = area.left_top();
        let end = area.right_bottom();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        assert_eq!(app.layout.selected_indices(), vec![0, 1]);
        frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
        frame(&mut app, &ctx, vec![mouse(end, false)]);
        assert_eq!(app.layout.selected(), Some(1));
        assert!(app.marquee.is_none());
    }
    fn toolbar_test_position(app: &Editor, ctx: &egui::Context, slot: usize) -> Pos2 {
        let canvas = ctx.read_response(egui::Id::new("canvas")).unwrap().rect;
        let (buttons, _, bottom) =
            toolbar_groups(canvas.width() + 16.0, app.show_toolbar_icon_names);
        buttons[slot].center() + Vec2::new(canvas.left() - 8.0, canvas.top() - bottom - 8.0)
    }

    fn open_alignment_test_menu(app: &mut Editor, ctx: &egui::Context) -> egui::FullOutput {
        frame(app, ctx, vec![]);
        let pos = toolbar_test_position(app, ctx, 10);
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(app, ctx, vec![mouse(pos, false)]);
        frame(app, ctx, vec![])
    }
    fn alignment_test_position(output: &egui::FullOutput, title: &str) -> Pos2 {
        let rects: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(r) if r.rect.size() == Vec2::splat(40.0) => Some(r.rect),
                _ => None,
            })
            .collect();
        rects[if title == "Align top" { 1 } else { 7 }].center()
    }
    #[test]
    fn crop_alignment_grid_moves_content_without_moving_box_or_changing_zoom() {
        for (index, x, y) in [
            (0, Some(1.0), Some(1.0)),
            (1, None, Some(1.0)),
            (2, Some(-1.0), Some(1.0)),
            (3, Some(1.0), None),
            (4, Some(0.0), Some(0.0)),
            (5, Some(-1.0), None),
            (6, Some(1.0), Some(-1.0)),
            (7, None, Some(-1.0)),
            (8, Some(-1.0), Some(-1.0)),
            (9, Some(0.0), None),
            (10, None, Some(0.0)),
        ] {
            let mut app = Editor {
                edit_content: true,
                align_target: AlignTarget::Layer(0),
                ..Default::default()
            };
            app.layout
                .set_framing(
                    1,
                    canvas_pilot::formats::Framing {
                        zoom: 2.5,
                        offset: [30.0, -20.0],
                    },
                )
                .unwrap();
            let boxes = app.layout.layers().clone();
            let formats = app.layout.formats().clone();
            let mapping = formats.mapping(1, boxes[1]).unwrap();
            let ctx = egui::Context::default();
            let output = open_alignment_test_menu(&mut app, &ctx);
            assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "ALIGN CONTENT TO LAYER BOX")));
            let pos = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Rect(r) if r.rect.size() == Vec2::splat(40.0) => {
                        Some(r.rect.center())
                    }
                    _ => None,
                })
                .nth(index)
                .unwrap();
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
            let actual = app.layout.formats().mapping(1, boxes[1]).unwrap().framing;
            let expected = [
                x.map_or(30.0, |v| v * mapping.offset_limits[0]),
                y.map_or(-20.0, |v| v * mapping.offset_limits[1]),
            ];
            for (actual_offset, expected_offset) in actual.offset.iter().zip(expected) {
                assert!(
                    (actual_offset - expected_offset).abs() < 1e-7,
                    "button {index}"
                );
            }
            assert!((actual.zoom - 2.5).abs() < 1e-7);
            assert_eq!(app.layout.layers(), &boxes);
            assert_eq!(app.layout.formats().sources, formats.sources);
            assert_eq!(app.layout.formats().advanced[0], formats.advanced[0]);
            assert!(app.edit_content);
            assert_eq!(app.align_target, AlignTarget::Layer(0));
            let aligned = app.layout.formats().clone();
            app.layout.undo();
            assert_eq!(app.layout.formats(), &formats);
            app.layout.redo();
            assert_eq!(app.layout.formats(), &aligned);
        }
    }

    #[test]
    fn single_layer_alignment_grid_uses_current_screen_bounds() {
        for (index, x, y) in [
            (0, Some(0.0), Some(0.0)),
            (1, None, Some(0.0)),
            (2, Some(1920.0), Some(0.0)),
            (3, Some(0.0), None),
            (4, Some(960.0), Some(540.0)),
            (5, Some(1920.0), None),
            (6, Some(0.0), Some(1080.0)),
            (7, None, Some(1080.0)),
            (8, Some(1920.0), Some(1080.0)),
            (9, Some(960.0), None),
            (10, None, Some(540.0)),
        ] {
            let mut app = Editor::default();
            app.layout.set_output([2560, 1440]).unwrap();
            let before = app.layout.layers().clone();
            let formats = app.layout.formats().clone();
            let ctx = egui::Context::default();
            let output = open_alignment_test_menu(&mut app, &ctx);
            assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.job.text == "Screen (one selected layer)")));
            let pos = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::epaint::Shape::Rect(r) if r.rect.size() == Vec2::splat(40.0) => {
                        Some(r.rect.center())
                    }
                    _ => None,
                })
                .nth(index)
                .unwrap();
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
            let g = app.layout.layers()[1];
            assert_eq!(g.x, x.unwrap_or(before[1].x));
            assert_eq!(g.y, y.unwrap_or(before[1].y));
            assert_eq!([g.width, g.height], [before[1].width, before[1].height]);
            assert_eq!(app.layout.layers()[0], before[0]);
            assert_eq!(*app.layout.formats(), formats);
            app.layout.undo();
            assert_eq!(*app.layout.layers(), before);
        }
    }
    #[test]
    fn top_and_bottom_buttons_align_rendered_edges_for_every_target() {
        for target in [
            AlignTarget::Selection,
            AlignTarget::Layer(0),
            AlignTarget::Layer(1),
        ] {
            for (title, expected_y) in [
                (
                    "Align top",
                    match target {
                        AlignTarget::Selection | AlignTarget::Layer(0) => [60.0, 60.0],
                        AlignTarget::Screen => [0.0, 0.0],
                        AlignTarget::Layer(_) => [180.0, 180.0],
                    },
                ),
                (
                    "Align bottom",
                    match target {
                        AlignTarget::Selection | AlignTarget::Layer(1) => [340.0, 180.0],
                        AlignTarget::Screen => [880.0, 720.0],
                        AlignTarget::Layer(_) => [60.0, -100.0],
                    },
                ),
            ] {
                let mut app = Editor::default();
                app.layout
                    .apply(
                        0,
                        Geometry {
                            x: 90.0,
                            y: 60.0,
                            width: 300.0,
                            height: 200.0,
                        },
                    )
                    .unwrap();
                app.layout.set_selection([true; 2]);
                app.align_target = target;
                let before = app.layout.layers().clone();
                let ctx = egui::Context::default();
                let output = open_alignment_test_menu(&mut app, &ctx);
                let pos = alignment_test_position(&output, title);
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
                );
                frame(&mut app, &ctx, vec![mouse(pos, false)]);
                let output = frame(&mut app, &ctx, vec![]);
                let rects = layer_rects(&output);
                assert_eq!(rects.len(), 2);
                let edges = if title == "Align top" {
                    [rects[0].top(), rects[1].top()]
                } else {
                    [rects[0].bottom(), rects[1].bottom()]
                };
                assert!((edges[0] - edges[1]).abs() < 0.001, "{title}: {target:?}");
                for i in 0..2 {
                    assert_eq!(
                        app.layout.layers()[i].y,
                        expected_y[i],
                        "{title}: {target:?}"
                    );
                    assert_eq!(app.layout.layers()[i].x, before[i].x);
                    assert_eq!(app.layout.layers()[i].height, before[i].height);
                }
                app.layout.undo();
                assert_eq!(*app.layout.layers(), before);
            }
        }
    }
    #[test]
    fn alignment_button_updates_selected_layers_and_undo_restores() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        app.layout
            .apply(
                0,
                Geometry {
                    x: 90.0,
                    y: 60.0,
                    width: 300.0,
                    height: 200.0,
                },
            )
            .unwrap();
        app.layout.set_selection([true; 2]);
        let before = app.layout.layers().clone();
        let output = open_alignment_test_menu(&mut app, &ctx);
        let pos = alignment_test_position(&output, "Align top");
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(&mut app, &ctx, vec![mouse(pos, false)]);
        assert_eq!(app.layout.layers()[1].y, 60.0);
        assert_eq!(app.layout.layers()[1].x, before[1].x);
        assert_eq!(app.layout.layers()[0], before[0]);
        app.layout.undo();
        assert_eq!(*app.layout.layers(), before);
        app.open_paste(include_str!("../tests/fixtures/drawio-rectangle.txt").into());
        let paste = app.paste.as_ref().unwrap();
        assert_eq!(paste.values(&app.layout).unwrap().len(), 1);
    }
}
