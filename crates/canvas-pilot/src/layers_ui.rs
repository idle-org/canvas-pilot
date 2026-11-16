use super::*;

fn order_button(ui: &mut egui::Ui, layer: usize, up: bool, enabled: bool) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let (_, rect) = ui.allocate_space(Vec2::splat(24.0));
        let response = ui.interact(
            rect,
            egui::Id::new(("layer-order", layer, up)),
            egui::Sense::click(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                enabled,
                if up { "Move up" } else { "Move down" },
            )
        });
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
            rect,
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let center = rect.center();
        let direction = if up { -1.0 } else { 1.0 };
        let tip = center + Vec2::new(0.0, 5.0 * direction);
        ui.painter().line_segment(
            [center - Vec2::new(0.0, 5.0 * direction), tip],
            visuals.fg_stroke,
        );
        ui.painter().add(egui::Shape::line(
            vec![
                tip + Vec2::new(-4.0, -4.0 * direction),
                tip,
                tip + Vec2::new(4.0, -4.0 * direction),
            ],
            visuals.fg_stroke,
        ));
        response
    })
    .inner
}

#[derive(Clone, Copy)]
pub(super) struct LayerDrag(pub usize);

#[derive(Clone)]
pub(super) struct FolderDrag(pub String);

pub(super) enum TreeRow {
    Folder(String),
    Layer(usize),
}

#[derive(Default)]
pub(super) struct LayerUi {
    pub open: bool,
    pub(super) collapsed: std::collections::BTreeSet<String>,
    pub(super) folder_rename: Option<(Option<String>, String, Vec<usize>)>,
    folder_move: Option<(String, usize, bool)>,
    last_folder_click: Option<String>,
    assign: Option<(Vec<usize>, String)>,
    pub context_layer: Option<usize>,
    delete_requested: Option<usize>,
    folder_remove: Option<String>,
    pub(super) rename: Option<(usize, String)>,
    pub(super) rename_focus: bool,
    drafts: Vec<(usize, String, String)>,
    error: String,
    closing: bool,
    rename_closing: bool,
    deferred: u8,
    folder_filter: Option<String>,
}
impl LayerUi {
    pub(super) fn sync_order(&mut self, order: &[usize]) {
        if self.drafts.len() == order.len()
            && self.drafts.iter().all(|(id, _, _)| order.contains(id))
        {
            self.drafts
                .sort_by_key(|(id, _, _)| order.iter().position(|current| current == id));
        }
    }
}
impl Editor {
    pub(super) fn layer_actions_menu(&mut self, ui: &mut egui::Ui, id: usize) {
        self.layer_organize_menu(ui, id);
        self.layer_delete_menu(ui, id);
    }
    pub(super) fn layer_organize_menu(&mut self, ui: &mut egui::Ui, id: usize) {
        let Some(i) = (0..self.layout.layers().len()).find(|&i| self.layout.layer_info(i).id == id)
        else {
            return;
        };
        if ui.button("Rename layer…").clicked() {
            self.layer_ui.rename = Some((id, self.layout.layer_info(i).name.clone()));
            self.layer_ui.rename_focus = true;
            ui.close_menu();
        }
        let ids = if self.layout.selection_mask()[i] {
            self.layout
                .selected_indices()
                .into_iter()
                .map(|i| self.layout.layer_info(i).id)
                .collect()
        } else {
            vec![id]
        };
        ui.menu_button("Move to folder", |ui| {
            if ui.button("No folder").clicked() {
                self.layer_ui.assign = Some((ids.clone(), String::new()));
                ui.close_menu();
            }
            for folder in self.layout.folders() {
                if ui.button(&folder).clicked() {
                    self.layer_ui.assign = Some((ids.clone(), folder));
                    ui.close_menu();
                }
            }
            if ui.button("New folder…").clicked() {
                self.layer_ui.folder_rename = Some((None, "New folder".into(), ids.clone()));
                self.layer_ui.rename_focus = true;
                ui.close_menu();
            }
        });
    }
    pub(super) fn layer_delete_menu(&mut self, ui: &mut egui::Ui, id: usize) {
        let Some(i) = (0..self.layout.layers().len()).find(|&i| self.layout.layer_info(i).id == id)
        else {
            return;
        };
        let can_delete =
            self.layout.layers().len() > 1 && self.layout.visible(i) && !self.layout.locked(i);
        if ui
            .add_enabled(can_delete, egui::Button::new("Delete layer"))
            .on_hover_text("Delete this layer. Undo restores it.")
            .on_disabled_hover_text(
                "Keep at least one layer; show and unlock this layer before deleting it.",
            )
            .clicked()
        {
            self.layer_ui.delete_requested = Some(id);
            ui.close_menu();
        }
    }
    fn delete_context_layer(&mut self) {
        let Some(id) = self.layer_ui.delete_requested.take() else {
            return;
        };
        let Some(i) = (0..self.layout.layers().len()).find(|&i| self.layout.layer_info(i).id == id)
        else {
            return;
        };
        if !self.layout.visible(i) || self.layout.locked(i) || self.layout.layers().len() <= 1 {
            return;
        }
        self.layout.select(Some(i));
        match self.layout.remove_selected() {
            Ok(()) => {
                self.align_target = AlignTarget::Selection;
                self.edit_content = false;
                self.format_draft = None;
                self.paste = None;
                self.layer_ui.context_layer = None;
                self.sync();
            }
            Err(error) => self.error = error,
        }
    }
    pub(super) fn reorder_layer(&mut self, id: usize, to: usize) {
        match self.layout.move_layer(id, to) {
            Ok(()) => {
                self.sync();
            }
            Err(e) => self.error = e,
        }
    }
    pub(super) fn names_pending(&self) -> bool {
        self.layer_ui
            .drafts
            .iter()
            .enumerate()
            .any(|(i, (id, name, folder))| {
                i >= self.layout.layers().len()
                    || self.layout.layer_info(i).id != *id
                    || self.layout.layer_info(i).name != name.trim()
                    || self.layout.layer_info(i).folder != folder.trim()
            })
    }
    pub(super) fn manage_layers(&mut self) {
        self.layer_ui.open = true;
        self.layer_ui.drafts = (0..self.layout.layers().len())
            .map(|i| {
                let info = self.layout.layer_info(i);
                (info.id, info.name.clone(), info.folder.clone())
            })
            .collect();
        self.layer_ui.error.clear();
    }
    pub(super) fn add_layer(&mut self) {
        match self.layout.add_layer() {
            Ok(_) => {
                self.sync();
                self.manage_layers();
            }
            Err(error) => self.error = error,
        }
    }
    pub(super) fn remove_layers(&mut self) {
        match self.layout.remove_selected() {
            Ok(()) => {
                self.align_target = AlignTarget::Selection;
                self.edit_content = false;
                self.format_draft = None;
                self.paste = None;
                self.sync();
                self.manage_layers();
            }
            Err(error) => self.error = error,
        }
    }
    fn rename_window(&mut self, ctx: &egui::Context) {
        let Some((id, mut name)) = self.layer_ui.rename.take() else {
            return;
        };
        let Some(i) = (0..self.layout.layers().len()).find(|&i| self.layout.layer_info(i).id == id)
        else {
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        egui::Window::new("Rename layer")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let response = ui.add(egui::TextEdit::singleline(&mut name).char_limit(128));
                if std::mem::take(&mut self.layer_ui.rename_focus) {
                    response.request_focus();
                }
                cancel = ui.input(|input| input.key_pressed(egui::Key::Escape));
                let valid = !name.trim().is_empty();
                apply = valid && ui.input(|input| input.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    apply |= apply_button(
                        ui,
                        "Rename",
                        valid && name.trim() != self.layout.layer_info(i).name,
                    )
                    .clicked();
                    cancel |= ui.button("Cancel").clicked();
                });
            });
        if !open && name.trim() != self.layout.layer_info(i).name {
            self.layer_ui.rename_closing = true;
        }
        if self.layer_ui.rename_closing {
            match pending_close(ctx, "rename-close", &self.error) {
                Some(true) => {
                    apply = true;
                    self.layer_ui.rename_closing = false;
                }
                Some(false) => {
                    cancel = true;
                    self.layer_ui.rename_closing = false;
                }
                None => {}
            }
        }
        if apply {
            let folder = self.layout.layer_info(i).folder.clone();
            match self.layout.rename_layer(i, &name, &folder) {
                Ok(()) => {
                    self.sync();
                    if self.layer_ui.open {
                        self.manage_layers();
                    }
                }
                Err(error) => {
                    self.error = error;
                    self.layer_ui.rename = Some((id, name));
                }
            }
        } else if (open || self.layer_ui.rename_closing) && !cancel {
            self.layer_ui.rename = Some((id, name));
        }
    }
    pub(super) fn layers_window(&mut self, ctx: &egui::Context) {
        self.folder_commands(ctx);
        self.delete_context_layer();
        self.rename_window(ctx);
        if !self.layer_ui.open {
            return;
        }
        let mut open = true;
        let mut add = false;
        let mut remove = false;
        let mut refresh = false;
        let mut sync = false;
        let mut reorder = None;
        let mut commit_names = false;
        egui::Window::new("Layers & folders").enabled(!self.format_draft.as_ref().is_some_and(|d| d.pending(&self.layout))).open(&mut open).default_width(740.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                add = ui.add_enabled(self.layout.layers().len() < 256, egui::Button::new("Add layer")).clicked();
                let count = self.layout.selected_indices().len();
                remove = ui.add_enabled(count > 0 && count < self.layout.layers().len(), egui::Button::new("Remove selected")).clicked();
                ui.weak("Add, remove, rename and reorder support Undo.");
            });
            let stale = self.layer_ui.drafts.len() != self.layout.layers().len() || self.layer_ui.drafts.iter().enumerate().any(|(i, d)| self.layout.layer_info(i).id != d.0);
            if stale {
                ui.label("The layer list changed. Reload this editor before applying names.");
                refresh = ui.button("Reload layer list").clicked();
                return;
            }
            let folders: std::collections::BTreeSet<_> = (0..self.layout.layers().len()).map(|i| self.layout.layer_info(i).folder.clone()).collect();
            egui::ComboBox::from_id_salt("layer-folder-filter").selected_text(self.layer_ui.folder_filter.as_deref().filter(|s| !s.is_empty()).unwrap_or(if self.layer_ui.folder_filter.is_some() { "No folder" } else { "All folders" })).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.layer_ui.folder_filter, None, "All folders");
                for folder in folders { let label = if folder.is_empty() { "No folder" } else { &folder }; ui.selectable_value(&mut self.layer_ui.folder_filter, Some(folder.clone()), label); }
            });
            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                egui::Grid::new("layer-management").striped(true).show(ui, |ui| {
                    ui.label("Order"); ui.label("Select / ID"); ui.label("Name"); ui.label("Folder (optional)"); ui.label("Visibility"); ui.label("Protection"); ui.end_row();
                    for (i, (id, name, folder)) in self.layer_ui.drafts.iter_mut().enumerate().rev() {
                        if self.layer_ui.folder_filter.as_ref().is_some_and(|folder| *folder != self.layout.layer_info(i).folder) { continue; }
                        ui.horizontal(|ui| {
                            let unfiltered = self.layer_ui.folder_filter.is_none();
                            if order_button(ui, *id, true, unfiltered && i + 1 < self.layout.layers().len()).on_hover_text("Move up / bring forward").on_disabled_hover_text("Show all folders to reorder; the top layer cannot move up.").clicked() { reorder = Some((*id, i + 1)); }
                            if order_button(ui, *id, false, unfiltered && i > 0).on_hover_text("Move down / send backward").on_disabled_hover_text("Show all folders to reorder; the bottom layer cannot move down.").clicked() { reorder = Some((*id, i - 1)); }
                        });
                        let selected = self.layout.selection_mask()[i];
                        if ui.selectable_label(selected, format!("{id:02}")).clicked() { self.layout.toggle(i); sync = true; }
                        let name_response = ui.add_sized([170.0,24.0], egui::TextEdit::singleline(name).id(egui::Id::new(("layer-name", *id))).char_limit(128));
                        let folder_response = ui.add_sized([140.0,24.0], egui::TextEdit::singleline(folder).id(egui::Id::new(("layer-folder", *id))).char_limit(128));
                        commit_names |= name_response.lost_focus() || folder_response.lost_focus()
                            || ((name_response.has_focus() || folder_response.has_focus()) && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                        let (_, rect) = ui.allocate_space(Vec2::splat(28.0));
                        if layer_state_icon(ui, rect, ui.id().with(("show", *id)), false, !self.layout.visible(i)).clicked() { self.layout.set_visible(i, !self.layout.visible(i)); sync = true; }
                        let (_, rect) = ui.allocate_space(Vec2::splat(28.0));
                        if layer_state_icon(ui, rect, ui.id().with(("lock", *id)), true, self.layout.locked(i)).clicked() { self.layout.set_locked(i, !self.layout.locked(i)); sync = true; }
                        ui.end_row();
                    }
                });
            });
            ui.weak("Folder names organize the layer list locally; they do not create hardware layers. IDs remain stable when layers are removed.");
            ui.small("Names and folders save on Enter or when leaving the field. Undo restores previous values.");
            if !self.layer_ui.error.is_empty() { ui.colored_label(ui.visuals().error_fg_color, &self.layer_ui.error); }
        });
        if (commit_names || !open || add || remove || refresh || reorder.is_some())
            && self.names_pending()
        {
            match self.layout.rename_layers(&self.layer_ui.drafts) {
                Ok(()) => {
                    self.layer_ui.error.clear();
                    self.sync();
                }
                Err(error) => self.layer_ui.error = error,
            }
        }
        if (!open || add || remove || refresh) && self.names_pending() {
            self.layer_ui.closing = true;
            self.layer_ui.deferred = if add {
                1
            } else if remove {
                2
            } else if refresh {
                3
            } else {
                0
            };
        }
        if self.layer_ui.closing {
            add = false;
            remove = false;
            refresh = false;
            open = true;
            let choice = pending_close(ctx, "names-close", &self.layer_ui.error);
            if let Some(apply) = choice {
                let result = if apply {
                    self.layout.rename_layers(&self.layer_ui.drafts)
                } else {
                    Ok(())
                };
                match result {
                    Ok(()) => {
                        self.layer_ui.closing = false;
                        open = self.layer_ui.deferred != 0;
                        add = self.layer_ui.deferred == 1;
                        remove = self.layer_ui.deferred == 2;
                        refresh = self.layer_ui.deferred == 3;
                        self.layer_ui.error.clear();
                        self.sync();
                    }
                    Err(e) => {
                        self.layer_ui.error = e;
                        self.layer_ui.closing = false;
                    }
                }
            }
        }
        self.layer_ui.open = open;
        if let Some((id, to)) = reorder {
            self.reorder_layer(id, to);
        }
        if sync {
            self.sync();
        }
        if add {
            self.add_layer();
        } else if remove {
            self.remove_layers();
        } else if refresh {
            self.manage_layers();
        }
    }
}

impl Editor {
    pub(super) fn layer_tree_rows(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for i in (0..self.layout.layers().len()).rev() {
            let folder = &self.layout.layer_info(i).folder;
            if folder.is_empty() {
                rows.push(TreeRow::Layer(i));
            } else if seen.insert(folder.clone()) {
                rows.push(TreeRow::Folder(folder.clone()));
                if !self.layer_ui.collapsed.contains(folder) {
                    rows.extend(
                        self.layout
                            .folder_members(folder)
                            .into_iter()
                            .rev()
                            .map(TreeRow::Layer),
                    );
                }
            }
        }
        for folder in self.layout.folders() {
            if seen.insert(folder.clone()) {
                rows.push(TreeRow::Folder(folder));
            }
        }
        rows
    }

    pub(super) fn layer_tree_header(&mut self, ui: &mut egui::Ui, ink: &Ink, width: f32) {
        if toolbar_icon(
            ui,
            Rect::from_min_size(ink.point(width - 92.0, -5.0), Vec2::new(28.0, 24.0)),
            "Create folder",
            ToolbarIcon::AddFolder,
        )
        .clicked()
        {
            self.layer_ui.folder_rename = Some((None, "New folder".into(), Vec::new()));
            self.layer_ui.rename_focus = true;
        }
        let root = ui.interact(
            Rect::from_min_size(ink.point(0.0, -5.0), Vec2::new(60.0, 24.0)),
            egui::Id::new("layer-root"),
            egui::Sense::hover(),
        );
        if root.dnd_hover_payload::<LayerDrag>().is_some() {
            ui.painter().rect_stroke(
                root.rect,
                3.0,
                Stroke::new(1.0_f32, theme::palette(ui.ctx()).cyan),
                StrokeKind::Inside,
            );
        }
        if let Some(payload) = root.dnd_release_payload::<LayerDrag>() {
            self.layer_ui.assign = Some((vec![payload.0], String::new()));
        }
        root.on_hover_text("Drop a layer here to move it out of its folder.");
    }

    pub(super) fn folder_drop_on_layer(&mut self, row: &egui::Response, i: usize) {
        if let Some(payload) = row.dnd_release_payload::<FolderDrag>() {
            let above = row
                .ctx
                .input(|i| i.pointer.interact_pos())
                .is_none_or(|p| p.y < row.rect.center().y);
            self.layer_ui.folder_move =
                Some((payload.0.clone(), self.layout.layer_info(i).id, above));
        }
    }

    pub(super) fn folder_tree_row(
        &mut self,
        ui: &mut egui::Ui,
        ink: &Ink,
        y: f32,
        w: f32,
        folder: &str,
    ) {
        let palette = theme::palette(ui.ctx());
        let members = self.layout.folder_members(folder);
        let mask = self.layout.selection_mask();
        let selected = members.iter().any(|&i| mask[i]);
        let hidden = !members.is_empty() && members.iter().all(|&i| !self.layout.visible(i));
        let locked = !members.is_empty() && members.iter().all(|&i| self.layout.locked(i));
        let color = if selected {
            palette.amber
        } else {
            palette.text
        };
        ink.card(
            0.0,
            y,
            w,
            38.0,
            palette.panel,
            if selected {
                palette.amber
            } else {
                palette.line
            },
        );
        let collapsed = self.layer_ui.collapsed.contains(folder);
        let arrow = ui.interact(
            Rect::from_min_size(ink.point(0.0, y), Vec2::new(28.0, 38.0)),
            egui::Id::new(("folder-expand", folder)),
            egui::Sense::click(),
        );
        let c = arrow.rect.center();
        let offsets = if collapsed {
            [[-3.0, -5.0], [3.0, 0.0], [-3.0, 5.0]]
        } else {
            [[-5.0, -3.0], [0.0, 3.0], [5.0, -3.0]]
        };
        ui.painter().add(egui::Shape::line(
            offsets.map(|[x, y]| c + Vec2::new(x, y)).to_vec(),
            Stroke::new(1.5_f32, color),
        ));
        if arrow
            .on_hover_text(if collapsed {
                "Expand folder"
            } else {
                "Collapse folder"
            })
            .clicked()
        {
            self.layer_ui.last_folder_click = None;
            if collapsed {
                self.layer_ui.collapsed.remove(folder);
            } else {
                self.layer_ui.collapsed.insert(folder.into());
            }
        }
        let row = ui.interact(
            Rect::from_min_size(ink.point(28.0, y), Vec2::new(w - 106.0, 38.0)),
            egui::Id::new(("folder", folder)),
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter().with_clip_rect(row.rect);
        let icon = ink.point(33.0, y + 12.0);
        painter.add(egui::Shape::closed_line(
            vec![
                icon,
                icon + Vec2::new(6.0, 0.0),
                icon + Vec2::new(9.0, 3.0),
                icon + Vec2::new(17.0, 3.0),
                icon + Vec2::new(17.0, 14.0),
                icon + Vec2::new(0.0, 14.0),
            ],
            Stroke::new(1.3_f32, color),
        ));
        painter.text(
            ink.point(57.0, y + 19.0),
            Align2::LEFT_CENTER,
            format!("{folder} ({})", members.len()),
            FontId::proportional(14.0),
            color,
        );
        if row.double_clicked() && self.layer_ui.last_folder_click.as_deref() == Some(folder) {
            self.layer_ui.folder_rename = Some((Some(folder.into()), folder.into(), vec![]));
            self.layer_ui.rename_focus = true;
        }
        if row.clicked() {
            self.layer_ui.last_folder_click = Some(folder.into());
            self.device.follow_folder(folder);
            self.layout.select_folder(folder, click_shift_modifier(ui));
            self.sync();
        }
        if !members.is_empty() {
            row.dnd_set_drag_payload(FolderDrag(folder.into()));
        }
        if row.dnd_hover_payload::<LayerDrag>().is_some()
            || row.dnd_hover_payload::<FolderDrag>().is_some()
        {
            ui.painter().rect_stroke(
                row.rect,
                3.0,
                Stroke::new(2.0_f32, palette.cyan),
                StrokeKind::Inside,
            );
        }
        if row.dnd_hover_payload::<LayerDrag>().is_some()
            && let Some(payload) = row.dnd_release_payload::<LayerDrag>()
        {
            self.layer_ui.assign = Some((vec![payload.0], folder.into()));
            self.layer_ui.collapsed.remove(folder);
        }
        if let Some(payload) = row.dnd_release_payload::<FolderDrag>()
            && !members.is_empty()
        {
            let above = row
                .ctx
                .input(|i| i.pointer.interact_pos())
                .is_none_or(|p| p.y < row.rect.center().y);
            let index = if above {
                *members.last().unwrap()
            } else {
                members[0]
            };
            self.layer_ui.folder_move =
                Some((payload.0.clone(), self.layout.layer_info(index).id, above));
        }
        row.context_menu(|ui| {
            if ui.button("Select folder contents").clicked() {
                self.device.follow_folder(folder);
                self.layout.select_folder(folder, false);
                self.sync();
                ui.close_menu();
            }
            if ui.button("Use folder for device transfers").clicked() {
                self.device
                    .set_scope(device_ui::TransferScope::Folder(folder.into()));
                ui.close_menu();
            }
            if ui.button("Rename folder…").clicked() {
                self.layer_ui.folder_rename = Some((Some(folder.into()), folder.into(), vec![]));
                self.layer_ui.rename_focus = true;
                ui.close_menu();
            }
            for (label, visible, lock) in [
                (
                    if hidden { "Show folder" } else { "Hide folder" },
                    Some(hidden),
                    None,
                ),
                (
                    if locked {
                        "Unlock folder"
                    } else {
                        "Lock folder"
                    },
                    None,
                    Some(!locked),
                ),
            ] {
                if ui.button(label).clicked() {
                    self.layout.set_folder_state(folder, visible, lock);
                    self.sync();
                    ui.close_menu();
                }
            }
            let folders = self.layout.folders();
            let position = folders.iter().position(|name| name == folder).unwrap();
            for (label, target, above) in [
                ("Move folder up", position.checked_sub(1), true),
                (
                    "Move folder down",
                    (position + 1 < folders.len()).then_some(position + 1),
                    false,
                ),
            ] {
                if ui
                    .add_enabled(
                        !members.is_empty()
                            && target.is_some_and(|i| {
                                !self.layout.folder_members(&folders[i]).is_empty()
                            }),
                        egui::Button::new(label),
                    )
                    .clicked()
                {
                    let target_members = self.layout.folder_members(&folders[target.unwrap()]);
                    let index = if above {
                        *target_members.last().unwrap()
                    } else {
                        target_members[0]
                    };
                    self.layer_ui.folder_move =
                        Some((folder.into(), self.layout.layer_info(index).id, above));
                    ui.close_menu();
                }
            }
            if ui
                .button(if members.is_empty() {
                    "Delete empty folder"
                } else {
                    "Ungroup folder"
                })
                .on_hover_text("Keep all layers and move them to the root.")
                .clicked()
            {
                self.layer_ui.folder_remove = Some(folder.into());
                ui.close_menu();
            }
        });
        row.on_hover_text("Click: select contents · Shift-click: toggle selection · double-click: rename · drag: reorder folder");
        for (kind, x, active) in [("lock", w - 72.0, locked), ("visibility", w - 36.0, hidden)] {
            let rect = Rect::from_min_size(ink.point(x, y + 5.0), Vec2::splat(28.0));
            if layer_state_icon(
                ui,
                rect,
                egui::Id::new(("folder-state", kind, folder)),
                kind == "lock",
                active,
            )
            .clicked()
            {
                self.layout.set_folder_state(
                    folder,
                    (kind == "visibility").then_some(active),
                    (kind == "lock").then_some(!active),
                );
                self.sync();
            }
        }
    }

    fn folder_commands(&mut self, ctx: &egui::Context) {
        let result = if let Some(folder) = self.layer_ui.folder_remove.take() {
            self.layer_ui.collapsed.remove(&folder);
            Some(self.layout.ungroup_folder(&folder))
        } else if let Some((folder, id, above)) = self.layer_ui.folder_move.take() {
            Some(self.layout.move_folder(&folder, id, above))
        } else if let Some((ids, folder)) = self.layer_ui.assign.take() {
            Some(self.layout.assign_folder(&ids, &folder))
        } else {
            None
        };
        if let Some(result) = result {
            match result {
                Ok(()) => {
                    self.sync();
                    if self.layer_ui.open {
                        self.manage_layers();
                    }
                }
                Err(error) => self.error = error,
            }
        }
        let Some((old, mut name, ids)) = self.layer_ui.folder_rename.take() else {
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        egui::Window::new(if old.is_some() {
            "Rename folder"
        } else {
            "New folder"
        })
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            let response = ui.add(egui::TextEdit::singleline(&mut name).char_limit(128));
            if std::mem::take(&mut self.layer_ui.rename_focus) {
                response.request_focus();
            }
            let valid = !name.trim().is_empty()
                && (!self
                    .layout
                    .folders()
                    .iter()
                    .any(|folder| folder == name.trim())
                    || old.as_deref() == Some(name.trim()));
            if !valid {
                ui.weak("Use a nonempty, unique folder name.");
            }
            apply = valid && ui.input(|i| i.key_pressed(egui::Key::Enter));
            cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
            ui.horizontal(|ui| {
                apply |= ui
                    .add_enabled(
                        valid,
                        egui::Button::new(if old.is_some() {
                            "Rename"
                        } else {
                            "Create folder"
                        }),
                    )
                    .clicked();
                cancel |= ui.button("Cancel").clicked();
            });
        });
        if apply {
            let result = if let Some(old) = &old {
                self.layout.rename_folder(old, &name)
            } else {
                self.layout.create_folder_with_layers(&name, &ids)
            };
            match result {
                Ok(()) => {
                    if old.as_ref().is_some_and(|old| {
                        self.device.scope == device_ui::TransferScope::Folder(old.clone())
                    }) {
                        self.device.follow_folder(name.trim());
                    }
                    if let Some(old) = old
                        && self.layer_ui.collapsed.remove(&old)
                    {
                        self.layer_ui.collapsed.insert(name.trim().into());
                    }
                    self.sync();
                    if self.layer_ui.open {
                        self.manage_layers();
                    }
                }
                Err(error) => {
                    self.error = error;
                    self.layer_ui.folder_rename = Some((old, name, ids));
                }
            }
        } else if open && !cancel {
            self.layer_ui.folder_rename = Some((old, name, ids));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_layer_list_space_clears_selection() {
        use super::super::tests::{frame, mouse};
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout.assign_folder(&[1, 2], "Video").unwrap();
        for gap in [false, true] {
            click_tree(&mut app, &ctx, egui::Id::new(("folder", "Video")));
            assert_eq!(app.layout.selected_indices(), [0, 1]);
            let rect = ctx
                .read_response(egui::Id::new(("layer", 2_usize)))
                .unwrap()
                .rect;
            let pos = if gap {
                Pos2::new(rect.center().x, rect.bottom() + 3.0)
            } else {
                Pos2::new(rect.left() - 9.0, rect.center().y)
            };
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
            );
            frame(&mut app, &ctx, vec![mouse(pos, false)]);
            assert!(app.layout.selected_indices().is_empty());
        }
        click_tree(&mut app, &ctx, egui::Id::new(("layer", 2_usize)));
        assert_eq!(app.layout.selected_indices().len(), 1);
        assert_eq!(app.layout.layer_info(app.layout.selected().unwrap()).id, 2);
    }

    #[test]
    fn empty_folder_dialog_preserves_selection_and_restores_its_tree_row() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout.set_selection([true, true]);
        let before = app.layout.layers().to_vec();
        app.layer_ui.folder_rename = Some((None, "Empty".into(), vec![]));
        super::super::tests::click_text(&mut app, &ctx, "Create folder");
        assert_eq!(app.layout.layers(), &before);
        assert_eq!(app.layout.selected_indices(), [0, 1]);
        assert!(app.layout.folder_members("Empty").is_empty());
        assert!(
            app.layer_tree_rows()
                .iter()
                .any(|r| matches!(r, TreeRow::Folder(f) if f == "Empty"))
        );
        app.layer_ui.collapsed.insert("Empty".into());
        let saved = app.project();
        app.install_project(&ctx, saved.clone()).unwrap();
        super::super::tests::frame(&mut app, &ctx, vec![]);
        assert_eq!(app.project(), saved);
        assert!(
            ctx.read_response(egui::Id::new(("folder", "Empty")))
                .is_some()
        );
        app.layer_ui.folder_remove = Some("Empty".into());
        super::super::tests::frame(&mut app, &ctx, vec![]);
        assert!(app.layout.folders().is_empty());
        app.layout.undo();
        assert_eq!(app.layout.folders(), ["Empty"]);
    }

    #[test]
    fn both_context_menus_move_the_selection_or_only_an_unselected_target() {
        fn menu_frame(
            app: &mut Editor,
            ctx: &egui::Context,
            viewport: bool,
            id: usize,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 900.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        if viewport {
                            app.layer_ui.context_layer = Some(id);
                            app.layer_context_menu(ui);
                        } else {
                            app.layer_actions_menu(ui, id);
                        }
                    });
                },
            )
        }
        fn click(app: &mut Editor, ctx: &egui::Context, viewport: bool, id: usize, label: &str) {
            menu_frame(app, ctx, viewport, id, vec![]);
            let output = menu_frame(app, ctx, viewport, id, vec![]);
            let point = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => {
                        Some(t.pos + t.galley.size() * 0.5)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing {label}"));
            menu_frame(
                app,
                ctx,
                viewport,
                id,
                vec![
                    egui::Event::PointerMoved(point),
                    super::super::tests::mouse(point, true),
                ],
            );
            menu_frame(
                app,
                ctx,
                viewport,
                id,
                vec![super::super::tests::mouse(point, false)],
            );
        }
        for viewport in [false, true] {
            for selected_target in [false, true] {
                let ctx = egui::Context::default();
                let mut app = Editor::default();
                app.layout.add_layer().unwrap();
                app.layout.set_selection([true, true, false]);
                app.layout.create_folder("Destination").unwrap();
                let id = app
                    .layout
                    .layer_info(if selected_target { 0 } else { 2 })
                    .id;
                click(&mut app, &ctx, viewport, id, "Move to folder");
                click(&mut app, &ctx, viewport, id, "Destination");
                let (ids, folder) = app.layer_ui.assign.take().unwrap();
                app.layout.assign_folder(&ids, &folder).unwrap();
                assert_eq!(
                    app.layout.folder_members("Destination"),
                    if selected_target { vec![0, 1] } else { vec![2] }
                );
                app.layout.undo();
                assert!(app.layout.folder_members("Destination").is_empty());
            }
        }
    }

    #[test]
    fn dragging_top_row_below_bottom_row_sends_it_to_the_back() {
        use super::super::tests::{frame, mouse};
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout.add_layer().unwrap();
        app.sync();
        let before = canvas_pilot::project::Project::from_layout(&app.layout);
        frame(&mut app, &ctx, vec![]);
        let start = ctx
            .read_response(egui::Id::new(("layer", 3_usize)))
            .unwrap()
            .rect
            .center();
        let end = ctx
            .read_response(egui::Id::new(("layer", 2_usize)))
            .unwrap()
            .rect
            .center_bottom()
            - Vec2::new(0.0, 3.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start + Vec2::new(0.0, 12.0))],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        frame(&mut app, &ctx, vec![mouse(end, false)]);
        assert_eq!(
            (0..3)
                .map(|i| app.layout.layer_info(i).id)
                .collect::<Vec<_>>(),
            [3, 2, 1]
        );
        app.layout.undo();
        assert_eq!(
            canvas_pilot::project::Project::from_layout(&app.layout),
            before
        );
    }

    #[test]
    fn manager_commits_names_before_reordering_and_undo_keeps_alignment_identity() {
        use super::super::tests::{frame, mouse};
        let ctx = egui::Context::default();
        let mut app = Editor {
            align_target: AlignTarget::Layer(0),
            ..Default::default()
        };
        app.manage_layers();
        app.layer_ui.drafts[0].1 = "Camera".into();
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        let pos = ctx
            .read_response(egui::Id::new(("layer-order", 1_usize, false)))
            .unwrap()
            .rect
            .center();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(&mut app, &ctx, vec![mouse(pos, false)]);
        assert_eq!(app.layout.layer_info(1).id, 2);
        assert_eq!(app.layout.layer_info(1).name, "Camera");
        assert_eq!(app.layer_ui.drafts[1].1, "Camera");
        assert_eq!(app.align_target, AlignTarget::Layer(1));
        assert!(!app.names_pending());
        assert!(!app.layer_ui.closing);
        assert_eq!(app.layout.layer_info(1).name, "Camera");
        app.layout.undo();
        app.layout.undo();
        app.sync();
        assert_eq!(app.layout.layer_info(0).id, 2);
        assert_eq!(app.layer_ui.drafts[0].0, 2);
        assert_eq!(app.align_target, AlignTarget::Layer(0));
    }

    #[test]
    fn inspector_drag_reorders_once_preserves_selection_and_can_cancel() {
        use super::super::tests::{frame, mouse};
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.layout.add_layer().unwrap();
        app.layout.select(Some(1));
        app.sync();
        let before = canvas_pilot::project::Project::from_layout(&app.layout);
        frame(&mut app, &ctx, vec![]);
        let row = |id| {
            ctx.read_response(egui::Id::new(("layer", id)))
                .unwrap()
                .rect
        };
        let start = row(2_usize).center();
        assert!(row(3_usize).top() < row(1_usize).top());
        assert!(row(1_usize).top() < row(2_usize).top());
        let end = row(3_usize).center_top() + Vec2::new(0.0, 3.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start + Vec2::new(0.0, 12.0))],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        assert_eq!(
            canvas_pilot::project::Project::from_layout(&app.layout),
            before
        );
        frame(&mut app, &ctx, vec![mouse(end, false)]);
        assert_eq!(
            (0..3)
                .map(|i| app.layout.layer_info(i).id)
                .collect::<Vec<_>>(),
            [1, 3, 2]
        );
        assert_eq!(app.layout.layer_info(app.layout.selected().unwrap()).id, 1);
        app.layout.undo();
        app.sync();
        assert_eq!(
            canvas_pilot::project::Project::from_layout(&app.layout),
            before
        );
        frame(&mut app, &ctx, vec![]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
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
        frame(&mut app, &ctx, vec![mouse(end, false)]);
        assert_eq!(
            canvas_pilot::project::Project::from_layout(&app.layout),
            before
        );
        // Releasing outside the list also cancels the reorder.
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        let outside = Pos2::new(500.0, 500.0);
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(outside)]);
        frame(&mut app, &ctx, vec![mouse(outside, false)]);
        assert_eq!(
            canvas_pilot::project::Project::from_layout(&app.layout),
            before
        );
    }

    #[test]
    fn names_and_folders_commit_on_enter_or_focus_loss_and_undo() {
        use super::super::tests::{frame, mouse};
        for enter in [false, true] {
            let ctx = egui::Context::default();
            let mut app = Editor::default();
            app.manage_layers();
            frame(&mut app, &ctx, vec![]);
            let before = app.layout.layer_info(1).clone();
            let field_id =
                egui::Id::new((if enter { "layer-name" } else { "layer-folder" }, before.id));
            ctx.memory_mut(|m| m.request_focus(field_id));
            frame(&mut app, &ctx, vec![]);
            if enter {
                app.layer_ui.drafts[1].1 = "Camera".into();
            } else {
                app.layer_ui.drafts[1].2 = "Cameras".into();
            }
            frame(&mut app, &ctx, vec![]);
            assert_eq!(app.layout.layer_info(1), &before);
            let events = if enter {
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }]
            } else {
                let pos = Pos2::new(20.0, 15.0);
                vec![egui::Event::PointerMoved(pos), mouse(pos, true)]
            };
            let output = frame(&mut app, &ctx, events);
            assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(t) if t.galley.text() == "Apply names & folders")));
            if enter {
                assert_eq!(app.layout.layer_info(1).name, "Camera");
            } else {
                assert_eq!(app.layout.layer_info(1).folder, "Cameras");
            }
            assert!(!app.names_pending());
            app.layout.undo();
            assert_eq!(app.layout.layer_info(1), &before);
        }
    }
    #[test]
    fn closing_batch_names_applies_or_discards_without_losing_identity() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.manage_layers();
        let id = app.layout.layer_info(1).id;
        app.layer_ui.drafts[1].1 = "Camera".into();
        app.layer_ui.closing = true;
        super::super::tests::click_text(&mut app, &ctx, "Apply");
        assert!(!app.layer_ui.open);
        assert_eq!(app.layout.layer_info(1).name, "Camera");
        assert_eq!(app.layout.layer_info(1).id, id);
        app.layout.undo();
        assert_eq!(app.layout.layer_info(1).name, "Presenter");
        app.manage_layers();
        app.layer_ui.drafts[1].1 = "Discarded".into();
        app.layer_ui.closing = true;
        super::super::tests::click_text(&mut app, &ctx, "Discard");
        assert!(!app.layer_ui.open);
        assert_eq!(app.layout.layer_info(1).name, "Presenter");
    }

    #[test]
    fn inspector_sections_follow_the_variable_layer_list() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        for _ in 0..3 {
            app.layout.add_layer().unwrap();
        }
        app.sync();
        super::super::tests::frame(&mut app, &ctx, vec![]);
        let frame = super::super::tests::frame(&mut app, &ctx, vec![]);
        let y = |label: &str| {
            frame
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos.y),
                    _ => None,
                })
                .unwrap()
        };
        assert!(y("MEASURE POSITION FROM") > y("Adjust layer crop") + 60.0);
    }
    #[test]
    fn many_layer_inspector_and_manager_render_after_history_changes() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        for _ in 0..4 {
            app.add_layer();
        }
        let id = app.layout.layer_info(5).id;
        app.layout.rename_layer(5, "Camera", "Video").unwrap();
        app.manage_layers();
        super::super::tests::frame(&mut app, &ctx, vec![]);
        app.remove_layers();
        app.layout.undo();
        app.sync();
        super::super::tests::frame(&mut app, &ctx, vec![]);
        assert_eq!(app.layout.layer_info(5).id, id);
        app.manage_layers();
        assert_eq!(app.layer_ui.drafts[5].1, "Camera");
    }
    fn click_tree(app: &mut Editor, ctx: &egui::Context, id: egui::Id) {
        use super::super::tests::{frame, mouse};
        frame(app, ctx, vec![]);
        frame(app, ctx, vec![]);
        let pos = ctx.read_response(id).unwrap().rect.center();
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(pos), mouse(pos, true)],
        );
        frame(app, ctx, vec![mouse(pos, false)]);
    }

    #[test]
    fn folder_tree_collapses_selects_and_protects_its_contents() {
        use super::super::tests::frame;
        let mut app = Editor::default();
        app.layout.assign_folder(&[1, 2], "Video").unwrap();
        app.layout.select(None);
        let ctx = egui::Context::default();
        assert_eq!(app.layer_tree_rows().len(), 3);
        click_tree(&mut app, &ctx, egui::Id::new(("folder-expand", "Video")));
        assert_eq!(app.layer_tree_rows().len(), 1);
        click_tree(&mut app, &ctx, egui::Id::new(("folder", "Video")));
        assert_eq!(app.layout.selected_indices(), vec![0, 1]);
        click_tree(
            &mut app,
            &ctx,
            egui::Id::new(("folder-state", "visibility", "Video")),
        );
        assert!(!app.layout.visible(0) && !app.layout.visible(1));
        assert!(app.layout.selected_indices().is_empty());
        app.layout.undo();
        assert!(app.layout.visible(0) && app.layout.visible(1));
        click_tree(
            &mut app,
            &ctx,
            egui::Id::new(("folder-state", "lock", "Video")),
        );
        assert!(app.layout.locked(0) && app.layout.locked(1));
        app.layout.undo();
        click_tree(&mut app, &ctx, egui::Id::new(("folder-expand", "Video")));
        assert_eq!(app.layer_tree_rows().len(), 3);
        frame(&mut app, &ctx, vec![]);
        let folder = ctx
            .read_response(egui::Id::new(("folder", "Video")))
            .unwrap()
            .rect;
        let child = ctx
            .read_response(egui::Id::new(("layer", 1_usize)))
            .unwrap()
            .rect;
        assert!(child.top() > folder.bottom());
    }

    fn drag_tree(
        app: &mut Editor,
        ctx: &egui::Context,
        source: egui::Id,
        target: egui::Id,
        below: bool,
    ) {
        use super::super::tests::{frame, mouse};
        frame(app, ctx, vec![]);
        frame(app, ctx, vec![]);
        let start = ctx.read_response(source).unwrap().rect.center();
        let end_rect = ctx.read_response(target).unwrap().rect;
        let end = end_rect.center() + Vec2::new(0.0, if below { 10.0 } else { -10.0 });
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(start), mouse(start, true)],
        );
        frame(
            app,
            ctx,
            vec![egui::Event::PointerMoved(start + Vec2::new(0.0, 12.0))],
        );
        frame(app, ctx, vec![egui::Event::PointerMoved(end)]);
        frame(app, ctx, vec![mouse(end, false)]);
        frame(app, ctx, vec![]);
    }

    #[test]
    fn drag_layers_into_folders_to_root_and_reorder_folder_blocks() {
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        app.layout.assign_folder(&[2], "Video").unwrap();
        drag_tree(
            &mut app,
            &ctx,
            egui::Id::new(("layer", 1_usize)),
            egui::Id::new(("folder", "Video")),
            false,
        );
        assert_eq!(app.layout.folder_members("Video").len(), 2);
        drag_tree(
            &mut app,
            &ctx,
            egui::Id::new(("layer", 1_usize)),
            egui::Id::new("layer-root"),
            false,
        );
        assert_eq!(app.layout.folder_members("Video").len(), 1);
        app.layout.assign_folder(&[1], "Slides").unwrap();
        drag_tree(
            &mut app,
            &ctx,
            egui::Id::new(("folder", "Slides")),
            egui::Id::new(("folder", "Video")),
            true,
        );
        assert_eq!(app.layout.layer_info(0).id, 1);
        assert_eq!(app.layout.folders(), vec!["Video", "Slides"]);
        app.layout.undo();
        assert_eq!(app.layout.layer_info(0).id, 2);
    }

    #[test]
    fn double_click_folder_opens_rename_and_commits_with_undo() {
        use super::super::tests::frame;
        let mut app = Editor::default();
        let ctx = egui::Context::default();
        app.layout.assign_folder(&[1, 2], "Video").unwrap();
        for _ in 0..2 {
            click_tree(&mut app, &ctx, egui::Id::new(("folder", "Video")));
        }
        assert!(app.layer_ui.folder_rename.is_some());
        app.layer_ui.folder_rename.as_mut().unwrap().1 = "Sources".into();
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
        assert!(app.layer_ui.folder_rename.is_none());
        assert_eq!(app.layout.folder_members("Sources"), vec![0, 1]);
        app.layout.undo();
        assert_eq!(app.layout.folder_members("Video"), vec![0, 1]);
    }
    #[test]
    fn folder_selection_changes_device_scope_only_in_folder_mode() {
        for scope in [
            device_ui::TransferScope::All,
            device_ui::TransferScope::Selected,
            device_ui::TransferScope::Folder("Previous".into()),
        ] {
            let mut app = Editor::default();
            let ctx = egui::Context::default();
            app.layout.assign_folder(&[1, 2], "Talk").unwrap();
            app.device.scope = scope.clone();
            click_tree(&mut app, &ctx, egui::Id::new(("folder", "Talk")));
            assert_eq!(
                app.device.scope,
                if matches!(scope, device_ui::TransferScope::Folder(_)) {
                    device_ui::TransferScope::Folder("Talk".into())
                } else {
                    scope
                }
            );
            assert_eq!(app.layout.selected_indices(), vec![0, 1]);
            assert!(!app.device.sending);
        }
    }
}
