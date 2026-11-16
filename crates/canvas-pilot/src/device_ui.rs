use super::*;
use canvas_pilot::{
    device,
    device_send::{self, Plan, Sizing, Target},
};
use std::sync::mpsc::{self, Receiver, TryRecvError};

fn linked_layer_label(layout: &canvas_pilot::Layout, id: usize) -> String {
    (0..layout.layers().len())
        .find(|&i| layout.layer_info(i).id == id)
        .map(|i| {
            let info = layout.layer_info(i);
            let folder = if info.folder.is_empty() {
                String::new()
            } else {
                format!(" [{}]", info.folder)
            };
            format!("{}{}", layout.layer_label(i), folder)
        })
        .unwrap_or_else(|| format!("Removed layer {id}"))
}

fn shared_link_warning(
    layout: &canvas_pilot::Layout,
    index: usize,
    link: &canvas_pilot::DeviceLink,
) -> Option<String> {
    let others: Vec<_> = (0..layout.layers().len())
        .filter(|&i| i != index && layout.layer_info(i).device_link.as_ref() == Some(link))
        .map(|i| linked_layer_label(layout, layout.layer_info(i).id))
        .collect();
    (!others.is_empty()).then(|| format!("Screen {} · Layer {} is also linked to: {}. Shared links are allowed; send a folder or selection without overlapping destinations.", link.screen, link.layer, others.join(", ")))
}

fn link_conflicts(
    layout: &canvas_pilot::Layout,
    links: &[(usize, canvas_pilot::DeviceLink)],
) -> Option<String> {
    let mut groups = std::collections::BTreeMap::<(device::Family, &str, &str), Vec<String>>::new();
    for (id, link) in links {
        groups
            .entry((link.family, &link.screen, &link.layer))
            .or_default()
            .push(linked_layer_label(layout, *id));
    }
    let conflicts: Vec<_> = groups
        .into_iter()
        .filter(|(_, layers)| layers.len() > 1)
        .map(|((family, screen, layer), layers)| {
            format!(
                "{family:?} · Screen {screen} · Layer {layer}: {}",
                layers.join(", ")
            )
        })
        .collect();
    (!conflicts.is_empty()).then(|| format!("Cannot send overlapping destinations:\n{}\nChoose one folder, use Only selected layers, or remap/skip the conflicting layers.", conflicts.join("\n")))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) enum TransferScope {
    #[default]
    All,
    Selected,
    Folder(String),
}
impl TransferScope {
    fn indices(&self, layout: &canvas_pilot::Layout) -> Vec<usize> {
        match self {
            Self::All => (0..layout.layers().len()).collect(),
            Self::Selected => layout.selected_indices(),
            Self::Folder(folder) => layout.folder_members(folder),
        }
    }
    fn label(&self, ctx: &egui::Context) -> String {
        match self {
            Self::All => tr(ctx, "ui.all-layers"),
            Self::Selected => tr(ctx, "ui.only-selected-layers"),
            Self::Folder(folder) => rust_i18n::t!(
                "scope.folder",
                locale = localization::locale(ctx),
                name = folder
            )
            .into_owned(),
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
enum Destination {
    #[default]
    Skip,
    New,
    Existing(usize),
}

#[derive(Default)]
pub(super) struct DeviceUi {
    pub(super) input_edits: input_ui::InputEdits,
    pub open: bool,
    pub(super) scope: TransferScope,
    import_closing: bool,
    pub(super) connection_open: bool,
    pub(super) prepare_send: bool,
    target: Target,
    pub(super) allow_program: bool,
    pub(super) autoconnect_at_startup: bool,
    sizing: Sizing,
    pub(super) sending: bool,
    message: String,
    pub(super) host: String,
    pub(super) recent_hosts: Vec<String>,
    pending: Option<Receiver<Result<device::Snapshot, String>>>,
    cancel_read: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub(super) snapshot: Option<device::Snapshot>,
    loading: Option<Vec<(usize, canvas_pilot::DeviceLink)>>,
    screen: usize,
    destinations: Vec<Destination>,
    mapping_scope: Option<(TransferScope, Vec<usize>, String)>,
    error: String,
}
impl DeviceUi {
    pub(super) fn try_startup_connection(&mut self, ctx: &egui::Context) {
        if self.autoconnect_at_startup && !self.host.trim().is_empty() {
            self.fetch(ctx);
        }
    }
    pub(super) fn reset_mappings(&mut self) {
        self.destinations.fill(Destination::Skip);
        self.mapping_scope = None;
        self.loading = None;
    }
    pub(super) fn with_connection_preferences(host: String, allow_program: bool) -> Self {
        Self {
            host,
            allow_program,
            ..Self::default()
        }
    }
    fn scoped_links(
        &self,
        layout: &canvas_pilot::Layout,
    ) -> Vec<(usize, canvas_pilot::DeviceLink)> {
        self.scope
            .indices(layout)
            .into_iter()
            .filter_map(|i| {
                let info = layout.layer_info(i);
                info.device_link.clone().map(|link| (info.id, link))
            })
            .collect()
    }
    pub(super) fn set_scope(&mut self, scope: TransferScope) {
        if self.scope != scope {
            self.scope = scope;
            self.destinations.fill(Destination::Skip);
            self.mapping_scope = None;
            self.message.clear();
            self.error.clear();
        }
    }
    pub(super) fn follow_folder(&mut self, folder: &str) {
        if matches!(self.scope, TransferScope::Folder(_)) {
            self.set_scope(TransferScope::Folder(folder.into()));
        }
    }
    fn linked_plans(&self, layout: &canvas_pilot::Layout) -> Result<Vec<Plan>, String> {
        let snapshot = self.snapshot.as_ref().ok_or("Connect to a device first.")?;
        let links = self.scoped_links(layout);
        if links.is_empty() {
            return Err(
                "No linked layers in this scope. Choose a scope and map its device destinations."
                    .into(),
            );
        }
        if let Some(error) = link_conflicts(layout, &links) {
            return Err(error);
        }
        let mut groups = std::collections::BTreeMap::<String, Vec<(usize, String)>>::new();
        for (id, link) in links {
            if link.family != snapshot.profile.family() {
                return Err("Saved destination belongs to a different device family. Remap it before sending.".into());
            }
            groups
                .entry(link.screen)
                .or_default()
                .push((id, link.layer));
        }
        groups
            .into_iter()
            .map(|(screen, mappings)| {
                let index = snapshot
                    .screens
                    .iter()
                    .position(|s| s.id == screen)
                    .ok_or_else(|| format!("Linked screen {screen} is unavailable."))?;
                let mut plan = Plan::prepare(
                    snapshot,
                    layout,
                    index,
                    &mappings,
                    if self.allow_program {
                        self.target
                    } else {
                        Target::Preview
                    },
                    self.sizing,
                )?;
                self.input_edits
                    .extend_plan(snapshot, &mappings, &mut plan)?;
                Ok(plan)
            })
            .collect()
    }
    fn seed_import_mapping(&mut self, layout: &canvas_pilot::Layout) {
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        let screen = &snapshot.screens[self.screen];
        let indices = self.scope.indices(layout);
        let key = (
            self.scope.clone(),
            indices
                .iter()
                .map(|&i| layout.layer_info(i).id)
                .collect::<Vec<_>>(),
            screen.id.clone(),
        );
        if self.mapping_scope.as_ref() == Some(&key) {
            return;
        }
        if self.mapping_scope.is_some() || self.destinations.len() != screen.layers.len() {
            self.destinations = vec![Destination::Skip; screen.layers.len()];
        }
        for (j, layer) in screen.layers.iter().enumerate() {
            if self.destinations[j] != Destination::Skip || layer.unsupported.is_some() {
                continue;
            }
            let matches = indices
                .iter()
                .copied()
                .filter(|&i| {
                    layout.visible(i)
                        && !layout.locked(i)
                        && layout
                            .layer_info(i)
                            .device_link
                            .as_ref()
                            .is_some_and(|link| {
                                link.family == snapshot.profile.family()
                                    && link.screen == screen.id
                                    && link.layer == layer.id
                            })
                })
                .collect::<Vec<_>>();
            if let [i] = matches.as_slice() {
                self.destinations[j] = Destination::Existing(layout.layer_info(*i).id);
            }
        }
        self.mapping_scope = Some(key);
    }
    pub(super) fn online(&self) -> bool {
        self.snapshot.is_some()
    }
    fn send(&mut self, ctx: &egui::Context, plans: Vec<Plan>) {
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.sending = true;
        self.error.clear();
        self.message.clear();
        let host = self.host.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let mut last = None;
                for (i, plan) in plans.iter().enumerate() {
                    last = Some(device_send::send(&host, plan).map_err(|e| {
                        format!(
                            "Screen {} failed after {i} completed screen sends: {e}",
                            plan.screen
                        )
                    })?);
                }
                last.ok_or_else(|| "No linked layers to send.".into())
            })();
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
    fn cancel_read(&mut self) {
        if let Some(cancel) = self.cancel_read.take() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.pending = None;
        self.loading = None;
    }
    fn fetch(&mut self, ctx: &egui::Context) {
        self.cancel_read();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.cancel_read = Some(cancel.clone());
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.snapshot = None;
        self.destinations.clear();
        self.error.clear();
        self.message.clear();
        let host = self.host.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = device::fetch_cancellable(&host, &cancel);
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
    fn poll(&mut self) {
        let Some(rx) = &self.pending else { return };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                Err("Device reader stopped unexpectedly. Refresh to retry.".into())
            }
        };
        self.pending = None;
        self.cancel_read = None;
        let was_sending = std::mem::take(&mut self.sending);
        match result {
            Ok(snapshot) => {
                if let Ok(address) = device::address(&self.host) {
                    let host = address.to_string();
                    self.recent_hosts.retain(|entry| entry != &host);
                    self.recent_hosts.insert(0, host);
                    self.recent_hosts.truncate(4);
                }
                if was_sending {
                    self.message =
                        "Send complete. Preset update sent; geometry verified by device readback."
                            .into();
                }
                for screen in &snapshot.screens {
                    for layer in &screen.layers {
                        if let Some(warning) = &layer.warning {
                            if !self.message.is_empty() {
                                self.message.push('\n');
                            }
                            self.message.push_str(&format!(
                                "Screen {} · Layer {}: {warning}",
                                screen.id, layer.id
                            ));
                        }
                    }
                }
                self.screen = self.screen.min(snapshot.screens.len() - 1);
                self.destinations =
                    vec![Destination::Skip; snapshot.screens[self.screen].layers.len()];
                self.mapping_scope = None;
                self.snapshot = Some(snapshot);
            }
            Err(error) => {
                self.snapshot = None;
                self.error = error;
                if was_sending {
                    self.prepare_send = true;
                    self.open = true;
                }
            }
        }
    }
}
impl Editor {
    pub(super) fn advanced_input_controls(&mut self, ui: &mut egui::Ui, i: usize) {
        self.device.input_edits.show(
            ui,
            self.device.snapshot.as_ref(),
            self.layout.layer_info(i).device_link.as_ref(),
            self.layout.layer_info(i).id,
        );
    }

    pub(super) fn layer_mapping_controls(&mut self, ui: &mut egui::Ui, i: usize) {
        let original = self.layout.layer_info(i).device_link.clone();
        let mut link = original.clone();
        ui.label(tr(ui.ctx(), "ui.device-screen-layer"));
        ui.small(tr(ui.ctx(), "ui.saved-with-this-layer-linked-sends-and-loads-use-this-screen-and-layer-editing-the-li"));
        if let Some(mapped) = &mut link {
            if let Some(snapshot) = &self.device.snapshot {
                egui::ComboBox::from_id_salt("property-midra-screen")
                    .selected_text(format!("Screen {}", mapped.screen))
                    .show_ui(ui, |ui| {
                        for screen in &snapshot.screens {
                            if ui
                                .selectable_value(
                                    &mut mapped.screen,
                                    screen.id.clone(),
                                    format!("Screen {}", screen.id),
                                )
                                .changed()
                                && {
                                    mapped.family = snapshot.profile.family();
                                    true
                                }
                                && !screen.layers.iter().any(|layer| layer.id == mapped.layer)
                                && let Some(layer) = screen.layers.first()
                            {
                                mapped.family = snapshot.profile.family();
                                mapped.layer = layer.id.clone();
                            }
                        }
                    });
                egui::ComboBox::from_id_salt("property-midra-layer")
                    .selected_text(format!("Layer {}", mapped.layer))
                    .show_ui(ui, |ui| {
                        if let Some(screen) = snapshot
                            .screens
                            .iter()
                            .find(|screen| screen.id == mapped.screen)
                        {
                            for layer in &screen.layers {
                                ui.selectable_value(
                                    &mut mapped.layer,
                                    layer.id.clone(),
                                    format!("Layer {}", layer.id),
                                );
                            }
                        }
                    });
            } else {
                ui.small(tr(
                    ui.ctx(),
                    "ui.offline-choose-the-device-family-and-enter-its-native-screen-layer-ids",
                ));
                egui::ComboBox::from_id_salt("offline-family")
                    .selected_text(format!("{:?}", mapped.family))
                    .show_ui(ui, |ui| {
                        for (family, label, screen) in [
                            (device::Family::Midra4k, "Midra 4K", "1"),
                            (device::Family::Aquilon, "Aquilon / LivePremier", "S1"),
                        ] {
                            if ui
                                .selectable_value(&mut mapped.family, family, label)
                                .changed()
                            {
                                mapped.screen = screen.into();
                            }
                        }
                    });
                let mut prefix = if mapped.screen.starts_with('A') {
                    "A"
                } else {
                    "S"
                };
                if mapped.family == device::Family::Aquilon {
                    egui::ComboBox::from_id_salt("offline-screen-kind")
                        .selected_text(if prefix == "A" { "Auxiliary" } else { "Screen" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut prefix, "S", tr(ui.ctx(), "ui.screen-6e5613"));
                            ui.selectable_value(&mut prefix, "A", tr(ui.ctx(), "ui.auxiliary"));
                        });
                }
                let screen_digits = if mapped.family == device::Family::Aquilon {
                    &mapped.screen[1..]
                } else {
                    &mapped.screen
                };
                let mut screen_number = screen_digits.parse::<u64>().unwrap_or(1);
                let mut layer_number = mapped.layer.parse::<u64>().unwrap_or(1);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut screen_number)
                            .range(1..=9_999_999_999_999_999_u64)
                            .prefix(tr(ui.ctx(), "ui.screen")),
                    );
                    ui.add(
                        egui::DragValue::new(&mut layer_number)
                            .range(1..=9_999_999_999_999_999_u64)
                            .prefix(tr(ui.ctx(), "ui.layer")),
                    );
                });
                mapped.screen = if mapped.family == device::Family::Aquilon {
                    format!("{prefix}{screen_number}")
                } else {
                    screen_number.to_string()
                };
                mapped.layer = layer_number.to_string();
            }
            if ui.button(tr(ui.ctx(), "ui.unmap")).clicked() {
                link = None;
            }
        } else {
            ui.label(tr(
                ui.ctx(),
                "ui.unmapped-excluded-from-linked-sends-and-loads",
            ));
            if ui.button(tr(ui.ctx(), "ui.link-to-device-layer")).clicked() {
                link = self.device.snapshot.as_ref().and_then(|snapshot| {
                    snapshot.screens.iter().find_map(|screen| {
                        screen.layers.first().map(|layer| canvas_pilot::DeviceLink {
                            family: snapshot.profile.family(),
                            screen: screen.id.clone(),
                            layer: layer.id.clone(),
                        })
                    })
                });
                if self.device.snapshot.is_none() {
                    link = Some(canvas_pilot::DeviceLink {
                        family: Default::default(),
                        screen: "1".into(),
                        layer: "1".into(),
                    });
                }
            }
        }
        if let Some(mapped) = &link
            && let Some(warning) = shared_link_warning(&self.layout, i, mapped)
        {
            ui.colored_label(ui.visuals().warn_fg_color, warning);
        }
        if link != original {
            match self.layout.set_device_link(i, link) {
                Ok(()) => {
                    self.device.mapping_scope = None;
                    self.sync();
                }
                Err(error) => {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
            }
        }
    }

    pub(super) fn device_load_button(&mut self, ui: &mut egui::Ui) {
        self.device_load_control(ui, true);
    }
    fn device_load_control(&mut self, ui: &mut egui::Ui, icon: bool) {
        let enabled = self.device.online() && self.device.pending.is_none();
        let tooltip = if self.device.message.is_empty() {
            tr(ui.ctx(), "transfer.read-help")
        } else {
            self.device.message.clone()
        };
        let label = if self.device.loading.is_some() {
            "transfer.loading"
        } else {
            match self.device.scope {
                TransferScope::All => "transfer.load-all",
                TransferScope::Selected => "transfer.load-selected",
                TransferScope::Folder(_) => "transfer.load-folder",
            }
        };
        let label = tr(ui.ctx(), label);
        let response = if icon {
            transfer_button(ui, &label, enabled, false)
        } else {
            ui.add_enabled(enabled, egui::Button::new(label))
        };
        if response.on_hover_text(tooltip).clicked() {
            self.start_scoped_load(ui.ctx());
            ui.close_menu();
        }
    }
    fn start_scoped_load(&mut self, ctx: &egui::Context) {
        let links = self.device.scoped_links(&self.layout);
        if links.is_empty() {
            self.device.error =
                "No linked layers in this scope. Map their destinations first.".into();
            self.device.prepare_send = true;
            self.device.open = true;
            return;
        }
        self.device.fetch(ctx);
        self.device.loading = Some(links);
    }
    pub(super) fn device_scope_picker(&mut self, ui: &mut egui::Ui) {
        let before = self.device.scope.clone();
        egui::ComboBox::from_id_salt("midra-transfer-scope")
            .width(175.0)
            .selected_text(self.device.scope.label(ui.ctx()))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.device.scope, TransferScope::All, tr(ui.ctx(), "ui.all-layers"));
                ui.selectable_value(&mut self.device.scope, TransferScope::Selected, tr(ui.ctx(), "ui.only-selected-layers"));
                ui.separator();
                for folder in self.layout.folders() {
                    ui.selectable_value(&mut self.device.scope, TransferScope::Folder(folder.clone()), format!("Folder: {folder}"));
                }
            }).response.on_hover_text(tr(ui.ctx(), "ui.scope-for-both-device-send-and-load-in-folder-mode-clicking-a-folder-in-the-layer-lis"));
        if before != self.device.scope {
            self.device.destinations.fill(Destination::Skip);
            self.device.mapping_scope = None;
            self.device.error.clear();
            self.device.message.clear();
        }
    }
    fn finish_linked_load(&mut self) {
        if self.device.pending.is_some() {
            return;
        }
        let Some(links) = self.device.loading.take() else {
            return;
        };
        let result = self
            .device
            .snapshot
            .as_ref()
            .ok_or_else(|| self.device.error.clone())
            .and_then(|snapshot| load_linked_layers(&mut self.layout, snapshot, &links));
        match result {
            Ok(()) => {
                self.sync();
                let warnings = std::mem::take(&mut self.device.message);
                self.device.message = format!(
                    "Loaded {} linked Preview layers. Undo restores the previous layout.",
                    links.len()
                );
                if !warnings.is_empty() {
                    self.device.message.push('\n');
                    self.device.message.push_str(&warnings);
                }
            }
            Err(error) => {
                self.device.error = error;
                self.device.prepare_send = true;
                self.device.open = true;
            }
        }
    }
    pub(super) fn open_device_export(&mut self) {
        if let Some(snapshot) = &self.device.snapshot
            && let Some(index) = self
                .device
                .scope
                .indices(&self.layout)
                .iter()
                .find_map(|&i| {
                    let link = self.layout.layer_info(i).device_link.as_ref()?;
                    snapshot
                        .screens
                        .iter()
                        .position(|screen| screen.id == link.screen)
                })
        {
            self.device.screen = index;
        }
        self.device.prepare_send = true;
        self.device.open = true;
    }
    pub(super) fn device_apply_button(&mut self, ui: &mut egui::Ui) {
        if ui
            .button(tr(ui.ctx(), "ui.device-mapping-transfer"))
            .on_hover_text(tr(
                ui.ctx(),
                "ui.open-connection-settings-layer-links-and-send-load-options",
            ))
            .clicked()
        {
            self.open_device_export();
            ui.close_menu();
        }
    }
    pub(super) fn device_send_button(&mut self, ui: &mut egui::Ui) {
        let plans = self.device.linked_plans(&self.layout);
        let conflicts = link_conflicts(&self.layout, &self.device.scoped_links(&self.layout));
        let enabled = self.device.online() && self.device.pending.is_none() && conflicts.is_none();
        let tooltip = if let Some(error) = conflicts {
            error
        } else if self.device.sending {
            "Sending and checking readback…".into()
        } else if !self.device.message.is_empty() {
            self.device.message.clone()
        } else {
            match &plans {
                Ok(plans) => format!(
                    "Send {} linked screen(s) to {:?} using saved layer destinations.",
                    plans.len(),
                    plans[0].target
                ),
                Err(e) => e.clone(),
            }
        };
        let send_label = if self.device.sending {
            tr(ui.ctx(), "transfer.applying")
        } else {
            tr(
                ui.ctx(),
                match self.device.scope {
                    TransferScope::All => "transfer.send-all",
                    TransferScope::Selected => "transfer.send-selected",
                    TransferScope::Folder(_) => "transfer.send-folder",
                },
            )
        };
        if transfer_button(ui, &send_label, enabled, true)
            .on_hover_text(&tooltip)
            .on_disabled_hover_text(&tooltip)
            .clicked()
        {
            match plans {
                Ok(plans) => {
                    self.device.send(ui.ctx(), plans);
                }
                Err(error) => {
                    self.device.error = error;
                    self.device.prepare_send = true;
                    self.device.open = true;
                }
            }
            ui.close_menu();
        }
    }

    fn connection_window(&mut self, ctx: &egui::Context) {
        if !self.device.connection_open {
            return;
        }
        let mut open = true;
        let before = self.device.host.clone();
        egui::Window::new(tr(ctx, "ui.device-connection")).id(egui::Id::new("Device connection")).open(&mut open).collapsible(false)
            .default_width(420.0).show(ctx, |ui| {
                ui.label(tr(ui.ctx(), "ui.device-ip-port"));
                if ui.add_enabled(self.device.pending.is_none(), egui::TextEdit::singleline(&mut self.device.host).hint_text("192.168.10.146:3000")).changed() {
                    self.device.snapshot = None;
                    self.device.input_edits.clear();
                    self.device.reset_mappings();
                    self.device.error.clear();
                    self.device.message.clear();
                }
                ui.add_enabled_ui(self.device.pending.is_none() && !self.device.recent_hosts.is_empty(), |ui| {
                    ui.menu_button(tr(ui.ctx(), "ui.recent-machines"), |ui| {
                        for host in self.device.recent_hosts.clone() {
                            if ui.button(&host).on_hover_text(tr(ui.ctx(), "ui.connect-to-this-machine-read-only")).clicked() {
                                self.device.host = host;
                                self.device.input_edits.clear();
                                self.device.reset_mappings();
                                self.device.fetch(ctx);
                                ui.close_menu();
                            }
                        }
                    }).response.on_hover_text(tr(ui.ctx(), "ui.last-four-successful-connections-choose-one-to-connect"));
                });
                ui.weak(tr(ui.ctx(), "ui.address-saved-automatically-in-application-settings-connect-reads-device-data-it-does"));
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.device.pending.is_none(), egui::Button::new(if self.device.online() { "Refresh connection" } else { "Connect" })).clicked() { self.device.fetch(ctx); }
                    if ui.add_enabled(self.device.online() && self.device.pending.is_none(), egui::Button::new(tr(ui.ctx(), "ui.disconnect"))).clicked() {
                        self.device.snapshot = None;
                        self.device.input_edits.clear();
                        self.device.reset_mappings();
                        self.device.message.clear();
                    }
                });
                if self.device.pending.is_some() {
                    ui.spinner();
                    ui.label(if self.device.sending { "Sending and checking readback…" } else { "Connecting…" });
                    if !self.device.sending && ui.button(tr(ui.ctx(), "ui.cancel-connection")).clicked() { self.device.cancel_read(); }
                }
                if let Some(snapshot) = &self.device.snapshot { ui.label(format!("Online · {}", snapshot.identity)); }
                else { ui.label(tr(ui.ctx(), "ui.offline")); }
                if !self.device.message.is_empty() { ui.label(&self.device.message); }
                if !self.device.error.is_empty() { ui.colored_label(ui.visuals().error_fg_color, &self.device.error); }
                if let Some(error) = &self.settings.error { ui.colored_label(ui.visuals().error_fg_color, error); }
            });
        self.device.connection_open = open;
        if before != self.device.host {
            self.settings.save(&self.application_settings(ctx));
        }
    }
    pub(super) fn device_window(&mut self, ctx: &egui::Context) {
        if !self.device.allow_program {
            self.device.target = Target::Preview;
        }
        let was_pending = self.device.pending.is_some();
        let recent_before = self.device.recent_hosts.clone();
        self.device.poll();
        if self.device.recent_hosts != recent_before {
            self.settings.save(&self.application_settings(ctx));
        }
        self.finish_linked_load();
        if was_pending && self.device.pending.is_none() {
            self.preselect_linked_capture_inputs();
        }
        self.connection_window(ctx);
        if !self.device.open {
            return;
        }
        let mut open = true;
        let mut imported = false;
        let mut send_plan = None;
        egui::Window::new(if self.device.prepare_send { "Device mapping & transfer" } else { "Import layers from device" })
            .enabled(!self.format_draft.as_ref().is_some_and(|d| d.pending(&self.layout)))
            .open(&mut open).default_width(660.0).collapsible(false)
            .max_height((ctx.screen_rect().height() - 100.0).max(200.0)).vscroll(true)
            .show(ctx, |ui| {
                if self.device.online() {
                    ui.horizontal(|ui| {
                        ui.label(tr(ui.ctx(), "ui.apply-to"));
                        self.device_scope_picker(ui);
                    });
                    if self.device.scope == TransferScope::Selected {
                        self.transfer_layer_picker(ui, "Choose selected layers…");
                    }
                }
                let scope_indices = self.device.scope.indices(&self.layout);
                if self.device.online() {
                    ui.weak(format!("{} editor layers in scope · {} saved links. Unlinked layers are skipped by linked send/load.", scope_indices.len(), self.device.scoped_links(&self.layout).len()));
                }
                ui.horizontal(|ui| {
                    if ui.button(tr(ui.ctx(), "ui.connection-settings")).clicked() { self.device.connection_open = true; }
                    if ui.add_enabled(self.device.online() && self.device.pending.is_none(), egui::Button::new(tr(ui.ctx(), "ui.refresh-layers"))).clicked() { self.device.fetch(ctx); }
                });
                if !self.device.online() && self.device.pending.is_none() { ui.label(tr(ui.ctx(), "ui.connect-to-a-device-in-connection-settings-before-transferring-layers")); }
                if self.device.pending.is_some() {
                    ui.horizontal(|ui| { ui.spinner(); ui.label(if self.device.sending { "Sending and checking readback…" } else { "Reading device… editing remains available." }); });
                    if !self.device.sending && ui.button(tr(ui.ctx(), "ui.cancel-read")).clicked() { self.device.cancel_read(); }
                }
                if !self.device.error.is_empty() { ui.colored_label(ui.visuals().error_fg_color, &self.device.error); }
                if !self.device.message.is_empty() { ui.label(&self.device.message); }
                if !self.device.prepare_send { self.device.seed_import_mapping(&self.layout); }
                if let Some(snapshot) = &self.device.snapshot {
                    ui.separator();
                    ui.strong(&snapshot.identity);
                    let previous = self.device.screen;
                    egui::ComboBox::from_id_salt("midra_screen")
                        .selected_text(format!("Screen {}", snapshot.screens[self.device.screen].id))
                        .show_ui(ui, |ui| {
                            for (i, screen) in snapshot.screens.iter().enumerate() { ui.selectable_value(&mut self.device.screen, i, format!("Screen {} · {} × {}", screen.id, screen.resolution[0], screen.resolution[1])); }
                        });
                    let screen = &snapshot.screens[self.device.screen];
                    if previous != self.device.screen {
                        self.device.destinations = vec![Destination::Skip; screen.layers.len()];
                        self.device.mapping_scope = None;
                    }
                    ui.label(format!("Captured Preview bank: {}. Refresh to read subsequent changes.", screen.preview_bank));
                    let matches = screen.resolution == self.layout.formats().output;
                    if !matches { ui.colored_label(ui.visuals().warn_fg_color, format!("Device canvas {} × {}; editor {} × {}. Match output resolution before importing.", screen.resolution[0], screen.resolution[1], self.layout.formats().output[0], self.layout.formats().output[1])); }
                    if self.device.prepare_send {
                        let screen_id = screen.id.clone();
                        let mut targets: std::collections::BTreeMap<_, _> = scope_indices.iter().copied().map(|i| {
                            let info = self.layout.layer_info(i);
                            (info.id, info.device_link.as_ref().filter(|link| link.family == snapshot.profile.family() && link.screen == screen_id).map(|link| link.layer.clone()).unwrap_or_default())
                        }).collect();
                        let previous = targets.clone();
                        let prepared_plan = send_review(ui, &self.layout, &scope_indices, snapshot, self.device.screen, &mut targets,
                            &mut self.device.target, &mut self.device.sizing, self.device.pending.is_some(), self.device.allow_program, &self.device.input_edits);
                        for (id, destination) in targets {
                            if previous.get(&id) != Some(&destination) {
                                let i = (0..self.layout.layers().len()).find(|&i| self.layout.layer_info(i).id == id).unwrap();
                                let link = (!destination.is_empty()).then_some(canvas_pilot::DeviceLink { family: snapshot.profile.family(), screen: screen_id.clone(), layer: destination });
                                if let Err(e) = self.layout.set_device_link(i, link) { self.device.error = e; }
                            }
                        }
                        ui.separator();
                        ui.horizontal(|ui| {
                            self.device_load_control(ui, true);
                            let enabled = prepared_plan.is_some() && self.device.pending.is_none();
                            if transfer_button(ui, &tr(ui.ctx(), "transfer.send-mapped"), enabled, true)
                                .on_hover_text(tr(ui.ctx(), "ui.send-the-mapped-editor-layers-to-the-selected-device-target"))
                                .clicked() {
                                send_plan = prepared_plan;
                            }
                        });
                        return;
                    }
                    ui.horizontal(|ui| {
                        if ui.button(tr(ui.ctx(), "ui.use-saved-links")).clicked() {
                            self.device.destinations.fill(Destination::Skip);
                            self.device.mapping_scope = None;
                        }
                        if ui.add_enabled(self.device.scope != TransferScope::Selected, egui::Button::new(tr(ui.ctx(), "ui.import-unmapped-as-new-layers"))).clicked() {
                            for (i, layer) in screen.layers.iter().enumerate() {
                                if self.device.destinations[i] == Destination::Skip && layer.unsupported.is_none() { self.device.destinations[i] = Destination::New; }
                            }
                        }
                    });
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for (i, layer) in screen.layers.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("Layer {} · {}", layer.id, layer.source));
                                ui.add_enabled_ui(layer.unsupported.is_none(), |ui| {
                                    let destination = &mut self.device.destinations[i];
                                    let label = match *destination {
                                        Destination::Skip => "Do not import".into(),
                                        Destination::New => "New layer".into(),
                                        Destination::Existing(id) => self.layout.layers().iter().enumerate()
                                            .find(|(j, _)| self.layout.layer_info(*j).id == id)
                                            .map(|(j, _)| self.layout.layer_label(j)).unwrap_or("Removed layer — choose again".into()),
                                    };
                                    egui::ComboBox::from_id_salt(("midra_destination", i)).selected_text(label).show_ui(ui, |ui| {
                                        ui.selectable_value(destination, Destination::Skip, "Do not import");
                                        ui.add_enabled_ui(self.device.scope != TransferScope::Selected, |ui| {
                                            ui.selectable_value(destination, Destination::New, "New layer");
                                        });
                                        for &j in &scope_indices {
                                            ui.add_enabled_ui(self.layout.visible(j) && !self.layout.locked(j), |ui| {
                                                ui.selectable_value(destination, Destination::Existing(self.layout.layer_info(j).id), self.layout.layer_label(j));
                                            });
                                        }
                                    });
                                });
                                ui.label(format!("{} × {} at {}, {}", canvas_pilot::view_pixels(layer.geometry.width), canvas_pilot::view_pixels(layer.geometry.height), canvas_pilot::view_pixels(layer.geometry.x), canvas_pilot::view_pixels(layer.geometry.y)));
                            });
                            if let Some(warning) = &layer.warning { ui.colored_label(ui.visuals().warn_fg_color, warning); }
                            if let Some(reason) = &layer.unsupported { ui.weak(reason); }
                        }
                    });
                    ui.weak(tr(ui.ctx(), "ui.supported-layers-import-geometry-and-appearance-midra-explicit-fits-include-crop-and"));
                    let selected: Vec<_> = self.device.destinations.iter().enumerate().filter_map(|(i, dest)| (*dest != Destination::Skip).then_some(i)).collect();
                    ui.label(format!("{} device layers to import", selected.len()));
                    if apply_button(ui, &tr(ui.ctx(), "action.import-layers"), matches && !selected.is_empty()).clicked() {
                        match self.apply_device_import() { Ok(()) => imported = true, Err(e) => self.device.error = e }
                    }
                    ui.weak(tr(ui.ctx(), "ui.choose-an-existing-destination-or-new-layer-beside-each-device-layer-the-scope-limits"));
                }

                ui.separator();
            });
        if !open
            && !self.device.prepare_send
            && self
                .device
                .destinations
                .iter()
                .any(|d| *d != Destination::Skip)
        {
            self.device.import_closing = true;
        }
        if self.device.import_closing {
            open = true;
            match pending_close(ctx, "device-import-close", &self.device.error) {
                Some(true) => {
                    self.device.import_closing = false;
                    match self.apply_device_import() {
                        Ok(()) => imported = true,
                        Err(e) => self.device.error = e,
                    }
                }
                Some(false) => {
                    self.device.import_closing = false;
                    self.device.reset_mappings();
                    open = false;
                }
                None => {}
            }
        }
        self.device.open = open;
        if let Some(plan) = send_plan {
            self.device.send(ctx, vec![plan]);
        }
        if imported {
            self.sync();
            self.device.error.clear();
            self.device.open = false;
            self.device.reset_mappings();
        }
    }
    fn apply_device_import(&mut self) -> Result<(), String> {
        let snapshot = self
            .device
            .snapshot
            .as_ref()
            .ok_or("Connect to a device first.")?;
        let screen = &snapshot.screens[self.device.screen];
        if screen.resolution != self.layout.formats().output {
            return Err("Match output resolution before importing.".into());
        }
        let allowed = self.device.scope.indices(&self.layout);
        for destination in &self.device.destinations {
            if *destination == Destination::New && self.device.scope == TransferScope::Selected {
                return Err(
                    "Only selected layers cannot create new layers. Choose All layers or a folder."
                        .into(),
                );
            }
            if let Destination::Existing(id) = destination
                && !allowed.iter().any(|&i| self.layout.layer_info(i).id == *id)
            {
                return Err("An import destination is outside the current scope. Remap it or choose All layers.".into());
            }
        }
        let selected: Vec<_> = self
            .device
            .destinations
            .iter()
            .enumerate()
            .filter_map(|(i, d)| (*d != Destination::Skip).then_some(i))
            .collect();
        screen.import(&selected)?;
        let values: Vec<_> = selected
            .iter()
            .map(|&i| {
                let target = match self.device.destinations[i] {
                    Destination::Existing(id) => Some(id),
                    _ => None,
                };
                (target, screen.layers[i].geometry, screen.layers[i].framing)
            })
            .collect();
        let links = selected
            .iter()
            .map(|&i| canvas_pilot::DeviceLink {
                family: snapshot.profile.family(),
                screen: screen.id.clone(),
                layer: screen.layers[i].id.clone(),
            })
            .collect::<Vec<_>>();
        let folder = match &self.device.scope {
            TransferScope::Folder(folder) => folder.as_str(),
            _ => "",
        };
        let fits = selected
            .iter()
            .map(|&i| canvas_pilot::formats::LayerAppearance {
                fit: screen.layers[i].fit,
                mask: screen.layers[i].mask,
            })
            .collect::<Vec<_>>();
        self.layout
            .import_linked_device_layers_with_appearance(&values, &links, folder, &fits)?;
        self.preselect_linked_capture_inputs();
        Ok(())
    }
}

// Draw arrows as vectors so the transfer buttons do not depend on symbol fonts.
fn transfer_button(ui: &mut egui::Ui, label: &str, enabled: bool, send: bool) -> egui::Response {
    let palette = theme::palette(ui.ctx());
    let button = egui::Button::new(format!("     {label}")).min_size(ui.spacing().interact_size);
    let button = if send {
        button.fill(if enabled {
            palette.purple
        } else {
            palette.field
        })
    } else {
        button
    };
    let response = ui.add_enabled(enabled, button);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    let center = egui::pos2(response.rect.left() + 10.0, response.rect.center().y);
    let direction = if send { -1.0 } else { 1.0 };
    let stroke = Stroke::new(1.5_f32, if enabled { palette.text } else { palette.muted });
    let tip = center + Vec2::new(0.0, direction * 5.0);
    ui.painter()
        .line_segment([center - Vec2::new(0.0, direction * 5.0), tip], stroke);
    ui.painter().add(egui::Shape::line(
        vec![
            tip + Vec2::new(-4.0, -direction * 4.0),
            tip,
            tip + Vec2::new(4.0, -direction * 4.0),
        ],
        stroke,
    ));
    response
}

#[allow(clippy::too_many_arguments)]
fn send_review(
    ui: &mut egui::Ui,
    layout: &canvas_pilot::Layout,
    indices: &[usize],
    snapshot: &device::Snapshot,
    screen_index: usize,
    targets: &mut std::collections::BTreeMap<usize, String>,
    target_mode: &mut Target,
    sizing: &mut Sizing,
    busy: bool,
    allow_program: bool,
    input_edits: &input_ui::InputEdits,
) -> Option<Plan> {
    let screen = &snapshot.screens[screen_index];
    if !allow_program {
        *target_mode = Target::Preview;
    }
    ui.horizontal(|ui| {
        ui.label(tr(ui.ctx(), "ui.target"));
        ui.selectable_value(target_mode, Target::Preview, "Preview");
        if allow_program {
            ui.selectable_value(target_mode, Target::Program, "Program");
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr(ui.ctx(), "ui.device-container"));
        ui.selectable_value(sizing, Sizing::OutputBox, "Output-sized");
        ui.selectable_value(sizing, Sizing::SourceAspect, "Source aspect");
    });
    let selected = indices;
    targets.retain(|id, _| selected.iter().any(|&i| layout.layer_info(i).id == *id));
    ui.strong(format!(
        "{} editor layers in scope to Screen {}",
        selected.len(),
        screen.id
    ));
    if selected.is_empty() {
        ui.label(tr(
            ui.ctx(),
            "ui.no-layers-in-this-scope-choose-all-layers-select-layers-or-choose-a-folder",
        ));
    }
    for &i in selected {
        let info = layout.layer_info(i);
        let id = info.id;
        if let Some(link) = &info.device_link {
            ui.weak(format!(
                "Saved link: screen {} · layer {}",
                link.screen, link.layer
            ));
        }
        let target = targets.entry(id).or_default();
        ui.separator();
        ui.horizontal(|ui| {
            ui.strong(layout.layer_label(i));
            egui::ComboBox::from_id_salt(("device_send_target", id))
                .selected_text(if target.is_empty() {
                    "Not linked / skip".into()
                } else {
                    format!("Screen {} · Layer {target}", screen.id)
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(target, String::new(), "Not linked / skip");
                    for layer in &screen.layers {
                        ui.selectable_value(
                            target,
                            layer.id.clone(),
                            format!("Screen {} · Layer {}", screen.id, layer.id),
                        );
                    }
                });
        });
        if !target.is_empty()
            && let Some(warning) = shared_link_warning(
                layout,
                i,
                &canvas_pilot::DeviceLink {
                    family: snapshot.profile.family(),
                    screen: screen.id.clone(),
                    layer: target.clone(),
                },
            )
        {
            ui.colored_label(ui.visuals().warn_fg_color, warning);
        }
        if let Some(layer) = screen.layers.iter().find(|layer| layer.id == *target) {
            ui.label(format!(
                "Preview snapshot container: {}",
                container_label(layer.container)
            ));
            if let Some(warning) = &layer.warning {
                ui.colored_label(ui.visuals().warn_fg_color, warning);
            }
            if let Some(reason) = &layer.unsupported {
                ui.weak(reason);
            }
        }
        ui.label(format!(
            "Requested {}: {}",
            if layout.formats().layer_fit[i].is_some() {
                "container"
            } else {
                "visible box"
            },
            container_label(layout.layers()[i])
        ));
        let mask = layout.formats().masks[i];
        if snapshot.profile == device::Profile::Midra4k {
            let g = layout
                .formats()
                .mask_reference(i, layout.layers()[i])
                .unwrap_or(layout.layers()[i]);
            ui.label(format!("Mask ({:?} reference): left {:.1}, right {:.1}, top {:.1}, bottom {:.1} output px (zero clears destination masking)",mask.origin,mask.left*g.width,mask.right*g.width,mask.top*g.height,mask.bottom*g.height));
        }
        if let Some(fit) = layout.formats().layer_fit[i] {
            ui.label(format!("Explicit fit: {}. Editor bounds are the device container; sizing strategy is not applied.", fit.label()));
        } else if let Ok(mapping) = layout.formats().mapping(i, layout.layers()[i]) {
            ui.label(format!(
                "Content zoom: {:.1}% · offset: {:.1}, {:.1}",
                mapping.framing.zoom * 100.0,
                mapping.framing.offset[0],
                mapping.framing.offset[1]
            ));
        }
    }
    ui.separator();
    let mappings: Vec<_> = targets
        .iter()
        .filter(|(_, destination)| !destination.is_empty())
        .map(|(id, destination)| (*id, destination.clone()))
        .collect();
    let links: Vec<_> = mappings
        .iter()
        .map(|(id, layer)| {
            (
                *id,
                canvas_pilot::DeviceLink {
                    family: snapshot.profile.family(),
                    screen: screen.id.clone(),
                    layer: layer.clone(),
                },
            )
        })
        .collect();
    if let Some(error) = link_conflicts(layout, &links) {
        ui.colored_label(ui.visuals().error_fg_color, error);
        return None;
    }
    match Plan::prepare(
        snapshot,
        layout,
        screen_index,
        &mappings,
        *target_mode,
        *sizing,
    )
    .and_then(|mut plan| {
        input_edits.extend_plan(snapshot, &mappings, &mut plan)?;
        Ok(plan)
    }) {
        Ok(plan) => {
            ui.label(format!(
                "{:?} · screen {} · physical bank {} · {} changed fields",
                plan.target,
                plan.screen,
                plan.bank,
                plan.changes.len()
            ));
            ui.collapsing("Exact device changes", |ui| {
                for change in &plan.changes {
                    ui.label(format!(
                        "{}: {} to {}",
                        change.path.join("/"),
                        change.before,
                        change.after
                    ));
                }
            });
            if plan.changes.is_empty() {
                ui.weak(tr(
                    ui.ctx(),
                    "ui.geometry-matches-the-stored-values-send-applies-them-to-the-hardware",
                ));
            }
            return (!busy).then_some(plan);
        }
        Err(error) => {
            ui.colored_label(ui.visuals().warn_fg_color, error);
        }
    }
    None
}

fn load_linked_layers(
    layout: &mut canvas_pilot::Layout,
    snapshot: &device::Snapshot,
    links: &[(usize, canvas_pilot::DeviceLink)],
) -> Result<(), String> {
    let mut fits = Vec::new();
    let mut values = Vec::new();
    for (id, link) in links {
        if link.family != snapshot.profile.family() {
            return Err(
                "Saved destination belongs to a different device family. Remap it before loading."
                    .into(),
            );
        }
        let screen = snapshot
            .screens
            .iter()
            .find(|s| s.id == link.screen)
            .ok_or_else(|| format!("Linked screen {} is unavailable.", link.screen))?;
        if screen.resolution != layout.formats().output {
            return Err(format!(
                "Screen {} resolution does not match the editor output.",
                screen.id
            ));
        }
        let index = screen
            .layers
            .iter()
            .position(|l| l.id == link.layer)
            .ok_or_else(|| {
                format!(
                    "Screen {} layer {} is unavailable.",
                    link.screen, link.layer
                )
            })?;
        screen.import(&[index])?;
        let layer = &screen.layers[index];
        values.push((Some(*id), layer.geometry, layer.framing));
        fits.push(canvas_pilot::formats::LayerAppearance {
            fit: layer.fit,
            mask: layer.mask,
        });
    }
    layout.import_device_layers_with_appearance(&values, &fits)
}

fn container_label(geometry: canvas_pilot::Geometry) -> String {
    let center = geometry.center();
    format!(
        "{:.1} × {:.1} px · center {:.1}, {:.1}",
        geometry.width, geometry.height, center[0], center[1]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn linked_test_snapshot() -> device::Snapshot {
        let layer = serde_json::json!({"position":{"pp":{"posH":500,"posV":300}},"size":{"pp":{"sizeH":640,"sizeV":360}},"source":{"pp":{"input":"INPUT_4"}},"status":{"pp":{"contentWidth":1920,"contentHeight":1080}},"crop":{"pp":{"left":0,"right":0,"top":0,"bottom":0,"aspectOverride":"GLOBAL_SETTING"}},"mask":{"pp":{"left":0,"right":0,"top":0,"bottom":0}}});
        let raw = serde_json::json!({"device": {"preconfig":{"status":{"stateList":{"items":{"CURRENT":{"screenList":{"items":{"1":{"pp":{"layerCount":2}}}}}}}}},"transition":{"screenList":{"items":{"1":{"status":{"pp":{"transition":"AT_DOWN"}}}}}},"screenList":{"items":{"1":{"canvas":{"status":{"size":{"pp":{"sizeH":1920,"sizeV":1080}}}},"presetList":{"items":{"UP":{"liveLayerList":{"items":{"1":layer,"2":layer}}},"DOWN":{"liveLayerList":{"items":{"1":layer,"2":layer}}}}}}}}}});
        device::Snapshot::parse(
            &serde_json::json!({"type":"Eikos 4K","version":{"major":3,"minor":3,"patch":10}}),
            &raw,
        )
        .unwrap()
    }
    #[test]
    fn connection_history_tracks_successes_and_keeps_four_distinct_endpoints() {
        let mut device = DeviceUi::default();
        for port in 3000..3005 {
            device.host = format!("127.0.0.1:{port}");
            let (tx, rx) = mpsc::channel();
            device.pending = Some(rx);
            tx.send(Ok(linked_test_snapshot())).unwrap();
            device.poll();
        }
        assert_eq!(
            device.recent_hosts,
            [
                "127.0.0.1:3004",
                "127.0.0.1:3003",
                "127.0.0.1:3002",
                "127.0.0.1:3001"
            ]
        );
        device.host = " 127.0.0.1:3002 ".into();
        let (tx, rx) = mpsc::channel();
        device.pending = Some(rx);
        tx.send(Ok(linked_test_snapshot())).unwrap();
        device.poll();
        assert_eq!(device.recent_hosts[0], "127.0.0.1:3002");
        assert_eq!(device.recent_hosts.len(), 4);
        let before = device.recent_hosts.clone();
        device.host = "127.0.0.1:9999".into();
        let (tx, rx) = mpsc::channel();
        device.pending = Some(rx);
        tx.send(Err("Offline".into())).unwrap();
        device.poll();
        assert_eq!(device.recent_hosts, before);
        let ctx = egui::Context::default();
        let app = Editor {
            device,
            ..Default::default()
        };
        let saved = app.application_settings(&ctx);
        let restored = Editor::with_settings(&ctx, settings::Store::default(), saved);
        assert_eq!(restored.device.recent_hosts, before);
    }

    #[test]
    fn property_mapping_selects_screen_and_unmaps_without_device_writes() {
        use super::super::tests::click_text;
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let i = app.layout.selected().unwrap();
        let before = app.layout.layers().clone();
        let mut snapshot = linked_test_snapshot();
        let mut second = snapshot.screens[0].clone();
        second.id = "2".into();
        snapshot.screens.push(second);
        app.device.snapshot = Some(snapshot);
        app.property_tab = PropertyTab::Mapping;
        app.format_draft = Some(FormatDraft::new(&app.layout, i));
        click_text(&mut app, &ctx, "Link to device layer");
        assert_eq!(
            app.layout
                .layer_info(i)
                .device_link
                .as_ref()
                .unwrap()
                .screen,
            "1"
        );
        click_text(&mut app, &ctx, "Screen 1");
        click_text(&mut app, &ctx, "Screen 2");
        assert_eq!(
            app.layout
                .layer_info(i)
                .device_link
                .as_ref()
                .unwrap()
                .screen,
            "2"
        );
        assert_eq!(app.device.scoped_links(&app.layout).len(), 1);
        let project = canvas_pilot::project::Project::from_layout(&app.layout);
        click_text(&mut app, &ctx, "Unmap");
        assert!(app.layout.layer_info(i).device_link.is_none());
        assert!(app.device.scoped_links(&app.layout).is_empty());
        assert_eq!(*app.layout.layers(), before);
        assert!(!app.device.sending);
        assert!(app.device.pending.is_none());
        app.layout.undo();
        assert_eq!(
            canvas_pilot::project::Project::from_layout(&app.layout),
            project
        );
    }

    #[test]
    fn closing_import_review_applies_locally_and_never_starts_a_send() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let before = app.layout.layers().clone();
        app.device.snapshot = Some(linked_test_snapshot());
        app.device.destinations = vec![Destination::Existing(1), Destination::Skip];
        app.device.open = true;
        app.device.import_closing = true;
        super::super::tests::click_text(&mut app, &ctx, "Apply");
        assert!(!app.device.open);
        assert!(!app.device.sending);
        assert!(app.device.pending.is_none());
        assert_ne!(*app.layout.layers(), before);
        app.layout.undo();
        assert_eq!(*app.layout.layers(), before);
    }

    #[test]
    fn startup_connection_requires_opt_in_and_an_address_and_never_sends() {
        let ctx = egui::Context::default();
        for (enabled, host, expected) in [
            (false, "invalid-ip", false),
            (true, "   ", false),
            (true, "invalid-ip", true),
        ] {
            // Invalid IP fails before any network access, while exercising the real worker path.
            let mut state = DeviceUi {
                autoconnect_at_startup: enabled,
                host: host.into(),
                ..Default::default()
            };
            state.try_startup_connection(&ctx);
            assert_eq!(state.pending.is_some(), expected);
            assert!(!state.sending);
            assert!(!state.online());
        }
    }
    #[test]
    fn saved_screen_distinguishes_same_numbered_layers_and_restores_picker() {
        let mut app = Editor::default();
        let mut snapshot = linked_test_snapshot();
        let mut second = snapshot.screens[0].clone();
        second.id = "2".into();
        second.layers[0].geometry.x += 200.0;
        snapshot.screens.push(second);
        let mut links = Vec::new();
        for i in 0..2 {
            let link = canvas_pilot::DeviceLink {
                family: Default::default(),
                screen: (i + 1).to_string(),
                layer: "1".into(),
            };
            app.layout.set_device_link(i, Some(link.clone())).unwrap();
            links.push((app.layout.layer_info(i).id, link));
        }
        load_linked_layers(&mut app.layout, &snapshot, &links).unwrap();
        assert_eq!(
            app.layout.layers()[0],
            snapshot.screens[0].layers[0].geometry
        );
        assert_eq!(
            app.layout.layers()[1],
            snapshot.screens[1].layers[0].geometry
        );
        assert_ne!(app.layout.layers()[0], app.layout.layers()[1]);
        app.device.snapshot = Some(snapshot);
        app.device.scope = TransferScope::Selected;
        for i in [1, 0] {
            app.layout.select(Some(i));
            app.open_device_export();
            assert_eq!(app.device.screen, i);
            assert!(app.device.pending.is_none());
        }
    }
    #[test]
    fn linked_load_is_atomic_undoable_and_keeps_links() {
        let snapshot = linked_test_snapshot();
        let mut layout = canvas_pilot::Layout::default();
        let link = canvas_pilot::DeviceLink {
            family: Default::default(),
            screen: "1".into(),
            layer: "2".into(),
        };
        layout.set_device_link(0, Some(link.clone())).unwrap();
        let id = layout.layer_info(0).id;
        let before = layout.layers().clone();
        let bad = canvas_pilot::DeviceLink {
            family: Default::default(),
            screen: "9".into(),
            layer: "1".into(),
        };
        assert!(
            load_linked_layers(&mut layout, &snapshot, &[(id, link.clone()), (id, bad)]).is_err()
        );
        assert_eq!(*layout.layers(), before);
        load_linked_layers(&mut layout, &snapshot, &[(id, link.clone())]).unwrap();
        assert_eq!(layout.layers()[0], snapshot.screens[0].layers[1].geometry);
        assert_eq!(layout.layers()[1], before[1]);
        assert_eq!(layout.layer_info(0).device_link, Some(link.clone()));
        layout.undo();
        assert_eq!(*layout.layers(), before);
        layout.set_locked(0, true);
        assert!(load_linked_layers(&mut layout, &snapshot, &[(id, link)]).is_err());
        assert_eq!(*layout.layers(), before);
    }
    #[test]
    fn linked_load_finishes_from_fresh_read_without_sending() {
        let mut app = Editor::default();
        let link = canvas_pilot::DeviceLink {
            family: Default::default(),
            screen: "1".into(),
            layer: "1".into(),
        };
        app.layout.set_device_link(0, Some(link.clone())).unwrap();
        app.device.loading = Some(vec![(app.layout.layer_info(0).id, link)]);
        let (tx, rx) = mpsc::channel();
        app.device.pending = Some(rx);
        let snapshot = linked_test_snapshot();
        let expected = snapshot.screens[0].layers[0].geometry;
        tx.send(Ok(snapshot)).unwrap();
        app.device.poll();
        app.finish_linked_load();
        assert_eq!(app.layout.layers()[0], expected);
        assert!(app.device.loading.is_none());
        assert!(app.device.pending.is_none());
        assert!(!app.device.sending);
        assert!(!app.device.open);
        assert!(app.device.message.contains("Loaded 1"));
    }
    #[test]
    fn all_linked_plans_include_hidden_locked_unselected_layers_across_screens() {
        let mut snapshot = linked_test_snapshot();
        let second = snapshot.raw["device"]["screenList"]["items"]["1"].clone();
        snapshot.raw["device"]["screenList"]["items"]["2"] = second;
        snapshot.raw["device"]["preconfig"]["status"]["stateList"]["items"]["CURRENT"]["screenList"]
            ["items"]["2"] = serde_json::json!({"pp":{"layerCount":2}});
        let transition = snapshot.raw["device"]["transition"]["screenList"]["items"]["1"].clone();
        snapshot.raw["device"]["transition"]["screenList"]["items"]["2"] = transition;
        let mut screen = snapshot.screens[0].clone();
        screen.id = "2".into();
        snapshot.screens.push(screen);
        let mut state = DeviceUi {
            snapshot: Some(snapshot),
            ..Default::default()
        };
        let mut layout = canvas_pilot::Layout::default();
        for i in 0..2 {
            layout
                .set_device_link(
                    i,
                    Some(canvas_pilot::DeviceLink {
                        family: Default::default(),
                        screen: (i + 1).to_string(),
                        layer: "1".into(),
                    }),
                )
                .unwrap();
        }
        layout.set_visible(0, false);
        layout.set_locked(1, true);
        layout.select(None);
        state.scope = TransferScope::Selected;
        assert!(state.linked_plans(&layout).is_err());
        state.scope = TransferScope::All;
        let plans = state.linked_plans(&layout).unwrap();
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].screen, "1");
        assert_eq!(plans[1].screen, "2");
        assert!(plans.iter().all(|p| !p.changes.is_empty()));
        state.reset_mappings();
        assert_eq!(state.linked_plans(&layout).unwrap().len(), 2);
        layout
            .set_device_link(
                1,
                Some(canvas_pilot::DeviceLink {
                    family: Default::default(),
                    screen: "1".into(),
                    layer: "1".into(),
                }),
            )
            .unwrap();
        assert!(state.linked_plans(&layout).is_err());
        layout.set_device_link(1, None).unwrap();
        assert_eq!(state.linked_plans(&layout).unwrap().len(), 1);
    }
    #[test]
    fn program_destination_is_hidden_and_reset_until_enabled() {
        for allow in [false, true] {
            let ctx = egui::Context::default();
            let layout = canvas_pilot::Layout::default();
            let snapshot = device::Snapshot {
                profile: Default::default(),
                identity: "Test".into(),
                raw: serde_json::Value::Null,
                screens: vec![device::Screen {
                    id: "1".into(),
                    resolution: [1920, 1080],
                    preview_bank: "UP".into(),
                    program_bank: "DOWN".into(),
                    layers: vec![],
                }],
            };
            let mut target = Target::Program;
            let mut targets = Default::default();
            let output = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    send_review(
                        ui,
                        &layout,
                        &TransferScope::All.indices(&layout),
                        &snapshot,
                        0,
                        &mut targets,
                        &mut target,
                        &mut Sizing::OutputBox,
                        false,
                        allow,
                        &input_ui::InputEdits::default(),
                    );
                });
            });
            assert_eq!(
                target,
                if allow {
                    Target::Program
                } else {
                    Target::Preview
                }
            );
            let program_visible = output.shapes.iter().any(|s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.job.text == "Program"));
            assert_eq!(program_visible, allow);
        }
    }
    #[test]
    fn send_all_linked_starts_without_opening_export_window() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        app.device.snapshot = Some(linked_test_snapshot());
        // Empty host ensures the worker cannot contact real hardware.
        app.device.host.clear();
        app.layout
            .set_device_link(
                0,
                Some(canvas_pilot::DeviceLink {
                    family: Default::default(),
                    screen: "1".into(),
                    layer: "1".into(),
                }),
            )
            .unwrap();
        app.layout.select(None);
        let mut pos = egui::Pos2::ZERO;
        for events in [vec![], vec![]] {
            let _ = ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        pos = ui.cursor().min + Vec2::new(30.0, 10.0);
                        app.device_send_button(ui);
                    });
                },
            );
        }
        for pressed in [true, false] {
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![
                        egui::Event::PointerMoved(pos),
                        super::super::tests::mouse(pos, pressed),
                    ],
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.device_send_button(ui));
                },
            );
        }
        assert!(app.device.sending);
        assert!(app.device.pending.is_some());
        assert!(!app.device.open);
        assert!(!app.device.prepare_send);
    }
    #[test]
    fn status_mapping_opens_settings_without_writing_even_when_linked() {
        for (online, linked) in [(false, false), (false, true), (true, false), (true, true)] {
            let ctx = egui::Context::default();
            let mut app = Editor::default();
            if online {
                app.device.snapshot = Some(linked_test_snapshot());
            }
            app.layout.select(Some(0));
            if linked {
                app.layout
                    .set_device_link(
                        0,
                        Some(canvas_pilot::DeviceLink {
                            family: Default::default(),
                            screen: "1".into(),
                            layer: "1".into(),
                        }),
                    )
                    .unwrap();
                if online {
                    assert!(app.device.linked_plans(&app.layout).is_ok());
                }
            }
            let before = app.layout.layers().clone();
            super::super::tests::frame(&mut app, &ctx, vec![]);
            let output = super::super::tests::frame(&mut app, &ctx, vec![]);
            let pos = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::epaint::Shape::Text(t)
                        if t.galley.job.text == "Device mapping & transfer…" =>
                    {
                        Some(t.pos + Vec2::new(8.0, 6.0))
                    }
                    _ => None,
                })
                .expect("status mapping button");
            for pressed in [true, false] {
                super::super::tests::frame(
                    &mut app,
                    &ctx,
                    vec![
                        egui::Event::PointerMoved(pos),
                        super::super::tests::mouse(pos, pressed),
                    ],
                );
            }
            assert!(app.device.open && app.device.prepare_send);
            assert!(app.device.pending.is_none());
            assert_eq!(*app.layout.layers(), before);
            if !online {
                super::super::tests::click_text(&mut app, &ctx, "Connection settings…");
                assert!(app.device.connection_open);
                assert!(app.device.snapshot.is_none());
                assert!(app.device.pending.is_none());
                assert!(!app.device.sending);
            }
        }
    }
    #[test]
    fn send_review_only_maps_selected_layers_and_never_mutates_layout() {
        let ctx = egui::Context::default();
        let mut layout = canvas_pilot::Layout::default();
        layout.select(Some(1));
        let before = layout.layers().clone();
        let selected_id = layout.layer_info(1).id;
        let mut targets = std::collections::BTreeMap::from([
            (layout.layer_info(0).id, "1".into()),
            (selected_id, "2".into()),
        ]);
        let screen = device::Screen {
            id: "1".into(),
            resolution: [1920, 1080],
            preview_bank: "UP".into(),
            program_bank: "DOWN".into(),
            layers: vec![],
        };
        let snapshot = device::Snapshot {
            profile: Default::default(),
            identity: "Eikos".into(),
            screens: vec![screen.clone()],
            raw: serde_json::Value::Null,
        };
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                send_review(
                    ui,
                    &layout,
                    &TransferScope::Selected.indices(&layout),
                    &snapshot,
                    0,
                    &mut targets,
                    &mut Target::Preview,
                    &mut Sizing::OutputBox,
                    false,
                    false,
                    &input_ui::InputEdits::default(),
                )
            });
        });
        assert_eq!(
            targets,
            std::collections::BTreeMap::from([(selected_id, "2".into())])
        );
        assert_eq!(*layout.layers(), before);
        layout.select(None);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                send_review(
                    ui,
                    &layout,
                    &TransferScope::Selected.indices(&layout),
                    &snapshot,
                    0,
                    &mut targets,
                    &mut Target::Preview,
                    &mut Sizing::OutputBox,
                    false,
                    false,
                    &input_ui::InputEdits::default(),
                )
            });
        });
        assert!(targets.is_empty());
    }
    #[test]
    fn successful_read_sets_online_and_failure_clears_it() {
        let mut state = DeviceUi::default();
        assert!(!state.online());
        let (tx, rx) = mpsc::channel();
        state.pending = Some(rx);
        tx.send(Ok(device::Snapshot {
            profile: Default::default(),
            identity: "Eikos".into(),
            raw: serde_json::Value::Null,
            screens: vec![device::Screen {
                id: "1".into(),
                resolution: [1920, 1080],
                preview_bank: "UP".into(),
                program_bank: "DOWN".into(),
                layers: vec![],
            }],
        }))
        .unwrap();
        state.poll();
        assert!(state.online());
        let (tx, rx) = mpsc::channel();
        state.pending = Some(rx);
        tx.send(Err("Unavailable".into())).unwrap();
        state.poll();
        assert!(!state.online());
    }
    #[test]
    fn disconnected_reader_reports_error_and_can_retry() {
        let (tx, rx) = mpsc::channel();
        let mut state = DeviceUi {
            pending: Some(rx),
            ..Default::default()
        };
        state.poll();
        assert!(state.pending.is_some());
        drop(tx);
        state.poll();
        assert!(state.pending.is_none());
        assert!(state.error.contains("Refresh"));
    }
    #[test]
    fn canceled_read_cannot_install_a_late_snapshot() {
        let (tx, rx) = mpsc::channel();
        let mut state = DeviceUi {
            pending: Some(rx),
            ..Default::default()
        };
        state.pending = None;
        assert!(tx.send(Err("late response".into())).is_err());
        state.poll();
        assert!(state.snapshot.is_none());
        assert!(state.error.is_empty());
    }
    #[test]
    fn connection_window_does_not_read_or_modify_on_open() {
        let ctx = egui::Context::default();
        let mut app = Editor::default();
        let before = app.layout.layers().clone();
        app.device.host = "192.168.10.143".into();
        let saved = app.application_settings(&ctx);
        let mut app = Editor::with_settings(&ctx, settings::Store::default(), saved);
        assert_eq!(app.device.host, "192.168.10.143");
        assert!(!app.device.online());
        app.device.connection_open = true;
        super::super::tests::frame(&mut app, &ctx, vec![]);
        assert!(app.device.pending.is_none());
        assert!(!app.device.open);
        app.device.connection_open = false;
        app.device.open = true;
        super::super::tests::frame(&mut app, &ctx, vec![]);
        assert!(!app.device.connection_open);
        assert!(app.device.pending.is_none());
        assert_eq!(*app.layout.layers(), before);
    }
    fn linked_folders() -> Editor {
        let mut app = Editor::default();
        app.device.snapshot = Some(linked_test_snapshot());
        for i in 0..2 {
            app.layout
                .set_device_link(
                    i,
                    Some(canvas_pilot::DeviceLink {
                        family: Default::default(),
                        screen: "1".into(),
                        layer: (i + 1).to_string(),
                    }),
                )
                .unwrap();
            let id = app.layout.layer_info(i).id;
            app.layout
                .assign_folder(&[id], if i == 0 { "Intro" } else { "Talk" })
                .unwrap();
        }
        app
    }

    #[test]
    fn default_scope_ignores_selection_for_send_load_and_import_mapping() {
        let mut app = linked_folders();
        assert_eq!(app.device.scope, TransferScope::All);
        app.layout.select(Some(1));
        assert_eq!(app.device.scoped_links(&app.layout).len(), 2);
        assert_eq!(app.device.linked_plans(&app.layout).unwrap().len(), 1);
        app.device.seed_import_mapping(&app.layout);
        assert!(matches!(
            app.device.destinations[0],
            Destination::Existing(2)
        ));
        assert!(matches!(
            app.device.destinations[1],
            Destination::Existing(1)
        ));
        // Selection never changes explicit All scope or its manual remapping.
        app.device.destinations[0] = Destination::Skip;
        app.layout.select(None);
        app.device.seed_import_mapping(&app.layout);
        assert!(matches!(app.device.destinations[0], Destination::Skip));
        app.device.scope = TransferScope::Selected;
        assert!(app.device.scoped_links(&app.layout).is_empty());
        assert!(app.device.linked_plans(&app.layout).is_err());
        app.layout.select(Some(0));
        app.device.seed_import_mapping(&app.layout);
        assert!(matches!(
            app.device.destinations[0],
            Destination::Existing(2)
        ));
        assert!(matches!(app.device.destinations[1], Destination::Skip));
    }

    #[test]
    fn shared_links_warn_by_name_and_only_overlapping_send_scopes_are_blocked() {
        let mut app = linked_folders();
        let link = app.layout.layer_info(0).device_link.clone().unwrap();
        app.layout.set_device_link(1, Some(link.clone())).unwrap();
        let warning = shared_link_warning(&app.layout, 0, &link).unwrap();
        assert!(warning.contains(&app.layout.layer_label(1)));
        assert!(warning.contains("[Talk]"));
        let error = app.device.linked_plans(&app.layout).unwrap_err();
        for i in 0..2 {
            assert!(error.contains(&app.layout.layer_label(i)));
        }
        assert!(error.contains("Screen 1 · Layer 1"));
        let ctx = egui::Context::default();
        super::super::tests::click_text(&mut app, &ctx, "Send all linked layers");
        assert!(app.device.pending.is_none());
        assert!(!app.device.open);
        let mut targets = std::collections::BTreeMap::from([
            (app.layout.layer_info(0).id, "1".into()),
            (app.layout.layer_info(1).id, "1".into()),
        ]);
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.layer_mapping_controls(ui, 0);
                assert!(
                    send_review(
                        ui,
                        &app.layout,
                        &[0, 1],
                        app.device.snapshot.as_ref().unwrap(),
                        0,
                        &mut targets,
                        &mut Target::Preview,
                        &mut Sizing::OutputBox,
                        false,
                        false,
                        &input_ui::InputEdits::default()
                    )
                    .is_none()
                );
            });
        });
        assert!(
            output
                .shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == warning))
        );
        for i in 0..2 {
            app.layout.select(Some(i));
            app.device.scope = TransferScope::Selected;
            assert!(app.device.linked_plans(&app.layout).is_ok());
        }
        app.layout.set_selection([true; 2]);
        assert!(app.device.linked_plans(&app.layout).is_err());
        let ids: Vec<_> = (0..2).map(|i| app.layout.layer_info(i).id).collect();
        app.layout.assign_folder(&ids, "Combined").unwrap();
        app.device.scope = TransferScope::Folder("Combined".into());
        assert!(app.device.linked_plans(&app.layout).is_err());
        // Same layer number on another screen is a different destination.
        app.layout
            .set_device_link(
                1,
                Some(canvas_pilot::DeviceLink {
                    family: Default::default(),
                    screen: "2".into(),
                    layer: "1".into(),
                }),
            )
            .unwrap();
        assert!(shared_link_warning(&app.layout, 0, &link).is_none());
        assert!(link_conflicts(&app.layout, &app.device.scoped_links(&app.layout)).is_none());
    }

    #[test]
    fn folder_memories_reuse_destinations_without_overwriting_other_links() {
        let mut app = linked_folders();
        let link = app.layout.layer_info(0).device_link.clone();
        app.layout.set_device_link(1, link).unwrap();
        let before = canvas_pilot::project::Project::from_layout(&app.layout);
        assert!(
            app.device
                .linked_plans(&app.layout)
                .unwrap_err()
                .contains("Choose one folder")
        );
        app.device.scope = TransferScope::Folder("Intro".into());
        let first = app.device.linked_plans(&app.layout).unwrap();
        assert_eq!(app.device.scoped_links(&app.layout)[0].0, 2);
        app.device.follow_folder("Talk");
        let second = app.device.linked_plans(&app.layout).unwrap();
        assert_eq!(app.device.scoped_links(&app.layout)[0].0, 1);
        let values = |plan: &Plan| {
            plan.changes
                .iter()
                .map(|change| (change.path.clone(), change.after.clone()))
                .collect::<Vec<_>>()
        };
        assert_ne!(values(&first[0]), values(&second[0]));
        assert_eq!(
            canvas_pilot::project::Project::from_layout(&app.layout),
            before
        );
        app.device.follow_folder("Removed");
        assert!(app.device.scoped_links(&app.layout).is_empty());
        assert!(app.device.linked_plans(&app.layout).is_err());
    }

    #[test]
    fn folder_import_remapping_is_scoped_and_saves_links_with_atomic_undo() {
        let mut app = linked_folders();
        app.device.scope = TransferScope::Folder("Talk".into());
        app.device.seed_import_mapping(&app.layout);
        assert!(matches!(app.device.destinations[0], Destination::Skip));
        assert!(matches!(
            app.device.destinations[1],
            Destination::Existing(1)
        ));
        let before = canvas_pilot::project::Project::from_layout(&app.layout);
        app.device.destinations = vec![Destination::Existing(2), Destination::Skip];
        assert!(
            app.apply_device_import()
                .unwrap_err()
                .contains("outside the current scope")
        );
        assert_eq!(
            canvas_pilot::project::Project::from_layout(&app.layout),
            before
        );
        app.device.destinations = vec![Destination::Existing(1), Destination::New];
        app.apply_device_import().unwrap();
        assert_eq!(
            app.layout.layer_info(1).device_link.as_ref().unwrap().layer,
            "1"
        );
        assert_eq!(app.layout.layer_info(2).folder, "Talk");
        assert_eq!(
            app.layout.layer_info(2).device_link.as_ref().unwrap().layer,
            "2"
        );
        assert_eq!(
            app.layout.layer_info(0),
            &before.layout().unwrap().layer_info(0).clone()
        );
        app.layout.undo();
        assert_eq!(
            canvas_pilot::project::Project::from_layout(&app.layout).layers,
            before.layers
        );
    }

    #[test]
    fn changing_folder_during_read_keeps_the_original_load_destinations() {
        let mut app = linked_folders();
        app.device.scope = TransferScope::Folder("Intro".into());
        let links = app.device.scoped_links(&app.layout);
        let before_other = app.layout.layers()[1];
        app.device.loading = Some(links);
        let (tx, rx) = mpsc::channel();
        app.device.pending = Some(rx);
        app.device.follow_folder("Talk");
        tx.send(Ok(linked_test_snapshot())).unwrap();
        app.device.poll();
        app.finish_linked_load();
        assert_eq!(
            app.layout.layers()[0],
            linked_test_snapshot().screens[0].layers[0].geometry
        );
        assert_eq!(app.layout.layers()[1], before_other);
        assert_eq!(app.device.scope, TransferScope::Folder("Talk".into()));
    }

    #[test]
    fn all_scope_send_review_lists_unselected_layers_and_keeps_manual_destinations() {
        let app = linked_folders();
        let ctx = egui::Context::default();
        let mut targets = std::collections::BTreeMap::from([(1, "2".into()), (2, "1".into())]);
        let before = targets.clone();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                send_review(
                    ui,
                    &app.layout,
                    &TransferScope::All.indices(&app.layout),
                    app.device.snapshot.as_ref().unwrap(),
                    0,
                    &mut targets,
                    &mut Target::Preview,
                    &mut Sizing::OutputBox,
                    false,
                    false,
                    &input_ui::InputEdits::default(),
                );
            });
        });
        assert_eq!(targets, before);
    }
    #[test]
    fn status_scope_picker_changes_transfer_filter_without_changing_selection() {
        let mut app = linked_folders();
        let ctx = egui::Context::default();
        let before = app.layout.selection_mask();
        super::super::tests::click_text(&mut app, &ctx, "All layers");
        super::super::tests::click_text(&mut app, &ctx, "Only selected layers");
        assert_eq!(app.device.scope, TransferScope::Selected);
        assert_eq!(app.layout.selection_mask(), before);
        assert!(app.device.pending.is_none());
        assert!(!app.device.sending);
    }
    #[test]
    fn selected_scope_never_creates_unselected_import_destinations() {
        let mut app = linked_folders();
        app.device.scope = TransferScope::Selected;
        app.device.destinations = vec![Destination::New, Destination::Skip];
        let before = canvas_pilot::project::Project::from_layout(&app.layout);
        assert!(
            app.apply_device_import()
                .unwrap_err()
                .contains("cannot create new layers")
        );
        assert_eq!(
            canvas_pilot::project::Project::from_layout(&app.layout),
            before
        );
    }
    #[test]
    fn aquilon_import_links_load_and_capture_preselection_use_the_device_family() {
        let raw: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/aquilon-6.2.json")).unwrap();
        let identity = serde_json::json!({"type":"AQL Cmax","isLinked":false,"version":{"major":6,"minor":2,"patch":79}});
        let snapshot = device::Snapshot::parse(&identity, &raw).unwrap();
        let mut app = Editor::default();
        app.device.snapshot = Some(snapshot);
        app.device.destinations = vec![
            Destination::New,
            Destination::Skip,
            Destination::Skip,
            Destination::Skip,
            Destination::Skip,
            Destination::Skip,
            Destination::Skip,
        ];
        app.apply_device_import().unwrap();
        let link = app
            .layout
            .layer_info(app.layout.layers().len() - 1)
            .device_link
            .clone()
            .unwrap();
        assert_eq!(link.family, device::Family::Aquilon);
        assert_eq!(link.screen, "S1");
        let mut state = app.device.snapshot.clone().unwrap();
        state.screens[0].layers[0].source = "LIVE_17".into();
        app.device.snapshot = Some(state.clone());
        app.preselect_linked_capture_inputs();
        let id = app.layout.layer_info(app.layout.layers().len() - 1).id;
        let saved = app.project();
        let preview = saved
            .workspace
            .previews
            .iter()
            .find(|p| p.layer_id == id)
            .unwrap();
        assert_eq!(preview.input, 17);
        assert!(!preview.enabled);
        let wrong = canvas_pilot::DeviceLink {
            family: device::Family::Midra4k,
            screen: "1".into(),
            layer: "1".into(),
        };
        let before = app.layout.layers().clone();
        assert!(
            load_linked_layers(&mut app.layout, &state, &[(id, wrong)])
                .unwrap_err()
                .contains("family")
        );
        assert_eq!(app.layout.layers(), &before);
        load_linked_layers(&mut app.layout, &state, &[(id, link)]).unwrap();
    }
}
