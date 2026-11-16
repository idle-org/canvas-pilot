//! Explicit device edits and separately saved, inactive input-setting proposals.
use super::*;
use canvas_pilot::{
    device::Snapshot,
    device_send::{Change, Plan},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct InputEdits {
    pub enabled: bool,
    pub saved: Vec<canvas_pilot::input_proposal::InputProposal>,
    message: Option<String>,
    // Only fields actually edited are staged; opening the tab never stages defaults.
    values: BTreeMap<String, Value>,
}

fn input_path(snapshot: &Snapshot, screen: &str, bank: &str, layer: &str) -> Option<String> {
    let input = snapshot.raw.pointer(&format!("/device/screenList/items/{screen}/presetList/items/{bank}/liveLayerList/items/{layer}/source/pp/input"))?.as_str()?;
    let base = format!("/device/inputList/items/{input}");
    let plug = snapshot
        .raw
        .pointer(&format!("{base}/status/pp/plug"))?
        .as_str()?;
    Some(format!("{base}/plugList/items/{plug}/settings"))
}

fn setting_label(value: &str) -> String {
    match value {
        "NATIVE" => "Native".into(),
        "SIGNAL" => "Signal".into(),
        "CUSTOM" => "Custom".into(),
        "LAYER_SETTING" => "Layer setting".into(),
        "CENTERED" => "Centered".into(),
        "FULLSCREEN" => "Full screen".into(),
        "CROPPED" => "Cropped".into(),
        "NONE" => "None".into(),
        "LETTERBOX_1_78" => "Letterbox 1.78:1".into(),
        "LETTERBOX_1_85" => "Letterbox 1.85:1".into(),
        "LETTERBOX_2_35" => "Letterbox 2.35:1".into(),
        "PILLARBOX_1_33" => "Pillarbox 1.33:1".into(),
        _ => value.replace('_', ":"),
    }
}

impl InputEdits {
    pub fn clear(&mut self) {
        self.enabled = false;
        self.values.clear();
        self.message = None;
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: Option<&Snapshot>,
        link: Option<&canvas_pilot::DeviceLink>,
        layer_id: usize,
    ) {
        if let Some(saved) = self.saved.iter().find(|p| p.layer_id == layer_id).cloned() {
            ui.label(format!(
                "Saved proposal: {} · plug {} (inactive until staged)",
                saved.input, saved.plug
            ));
            for (key, value) in &saved.settings {
                ui.small(format!("{key}: {value}"));
            }
            let can_stage =
                self.enabled && snapshot.is_some_and(|s| Self::proposal_matches(&saved, s, link));
            if ui
                .add_enabled(
                    can_stage,
                    egui::Button::new(tr(ui.ctx(), "ui.stage-saved-input-proposal")),
                )
                .clicked()
                && let Some(snapshot) = snapshot
            {
                self.message = Some(match self.stage_proposal(&saved, snapshot, link) {
                    Ok(()) => "Proposal staged. Review mappings before sending.".into(),
                    Err(error) => error,
                });
            }
            ui.small(tr(ui.ctx(), "ui.to-stage-connect-match-this-layer-s-saved-mapping-and-routed-input-plug-then-enable-a"));
            if ui
                .button(tr(ui.ctx(), "ui.remove-saved-input-proposal"))
                .clicked()
            {
                self.saved.retain(|p| p.layer_id != layer_id);
            }
        }
        if let Some(message) = &self.message {
            ui.label(message);
        }
        if snapshot.is_some_and(|s| !s.profile.advanced_inputs()) {
            ui.label(tr(ui.ctx(), "ui.shared-input-corrections-for-this-adapter-are-managed-in-webrcs-layer-crop-and-zoom-r"));
            return;
        }
        ui.label(tr(
            ui.ctx(),
            "ui.advanced-use-only-when-you-need-to-correct-an-input-and-understand-the-effect",
        ));
        ui.label(tr(ui.ctx(), "ui.these-device-controls-map-one-for-one-to-webrcs-advanced-input-settings-content-ratio"));
        ui.label(tr(ui.ctx(), "ui.input-crop-removes-source-edges-before-dynamic-layer-crop-zoom-it-does-not-resize-the"));
        if ui
            .checkbox(
                &mut self.enabled,
                tr(
                    ui.ctx(),
                    "ui.edit-advanced-input-settings-and-include-my-edits-when-sending",
                ),
            )
            .changed()
            && !self.enabled
        {
            self.values.clear();
        }
        ui.small(tr(ui.ctx(), "ui.off-by-default-only-edited-device-fields-are-sent-and-only-with-layers-using-that-inp"));
        let Some((snapshot, base)) = snapshot.and_then(|snapshot| {
            let link = link?;
            let screen = snapshot.screens.iter().find(|s| s.id == link.screen)?;
            input_path(snapshot, &link.screen, &screen.preview_bank, &link.layer)
                .map(|base| (snapshot, base))
        }) else {
            ui.small(tr(ui.ctx(), "ui.connect-and-map-this-layer-to-a-midra-input-source-to-inspect-its-advanced-settings-n"));
            return;
        };
        ui.small(format!(
            "Shared input: {} · plug {} (from linked Preview layer)",
            base.split('/').nth(4).unwrap_or("?"),
            base.split('/').nth(7).unwrap_or("?")
        ));
        ui.add_enabled_ui(self.enabled, |ui| {
            for (suffix, label, options) in [
                (
                    "aspect/pp/signal",
                    "Content ratio",
                    &[
                        "NATIVE", "5_4", "4_3", "16_10", "15_9", "16_9", "21_9", "256_135", "64_27",
                    ][..],
                ),
                (
                    "aspect/pp/transformTo",
                    "Transform to",
                    &[
                        "SIGNAL", "5_4", "4_3", "16_10", "15_9", "16_9", "21_9", "CUSTOM",
                    ][..],
                ),
                (
                    "aspect/pp/layerFill",
                    "Layer fill option",
                    &["LAYER_SETTING", "1_1", "CENTERED", "FULLSCREEN", "CROPPED"][..],
                ),
                (
                    "cropping/control/pp/predefined",
                    "Input crop preset",
                    &[
                        "NONE",
                        "LETTERBOX_1_78",
                        "LETTERBOX_1_85",
                        "LETTERBOX_2_35",
                        "PILLARBOX_1_33",
                    ][..],
                ),
            ] {
                let path = format!("{base}/{suffix}");
                let Some(original) = snapshot.raw.pointer(&path) else {
                    continue;
                };
                let mut value = self
                    .values
                    .get(&path)
                    .unwrap_or(original)
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                let before = value.clone();
                ui.horizontal(|ui| {
                    ui.label(label);
                    egui::ComboBox::from_id_salt(&path)
                        .selected_text(setting_label(&value))
                        .show_ui(ui, |ui| {
                            for option in options {
                                ui.selectable_value(
                                    &mut value,
                                    (*option).into(),
                                    setting_label(option),
                                );
                            }
                        });
                });
                if value != before {
                    self.stage(path, json!(value), original);
                }
            }
            for (suffix, label, min, max, scale) in [
                (
                    "aspect/pp/customRatio",
                    "Custom aspect ratio ",
                    250.,
                    10000.,
                    0.001,
                ),
                (
                    "cropping/control/pp/left",
                    "Input crop left ",
                    0.,
                    8191.,
                    1.,
                ),
                (
                    "cropping/control/pp/right",
                    "Input crop right ",
                    0.,
                    8191.,
                    1.,
                ),
                ("cropping/control/pp/top", "Input crop top ", 0., 8191., 1.),
                (
                    "cropping/control/pp/bottom",
                    "Input crop bottom ",
                    0.,
                    8191.,
                    1.,
                ),
            ] {
                let path = format!("{base}/{suffix}");
                let Some(original) = snapshot.raw.pointer(&path) else {
                    continue;
                };
                let mut value = self
                    .values
                    .get(&path)
                    .unwrap_or(original)
                    .as_f64()
                    .unwrap_or_default()
                    * scale;
                if ui
                    .add(
                        egui::DragValue::new(&mut value)
                            .range(min * scale..=max * scale)
                            .speed(scale)
                            .prefix(label)
                            .max_decimals(if scale == 1. { 0 } else { 3 }),
                    )
                    .changed()
                {
                    self.stage(path, json!((value / scale).round() as i64), original);
                }
            }
        });
        let prefix = format!("{base}/");
        let settings: BTreeMap<_, _> = self
            .values
            .iter()
            .filter_map(|(path, value)| {
                path.strip_prefix(&prefix)
                    .map(|key| (key.to_owned(), value.clone()))
            })
            .collect();
        if ui
            .add_enabled(
                !settings.is_empty(),
                egui::Button::new(tr(
                    ui.ctx(),
                    "ui.save-staged-input-edits-as-a-project-proposal",
                )),
            )
            .clicked()
            && let Some(link) = link
        {
            let proposal = canvas_pilot::input_proposal::InputProposal {
                layer_id,
                device_link: link.clone(),
                input: base.split('/').nth(4).unwrap_or_default().into(),
                plug: base.split('/').nth(7).unwrap_or_default().into(),
                settings,
            };
            if proposal.validate().is_ok() {
                self.saved.retain(|p| p.layer_id != layer_id);
                self.saved.push(proposal);
            }
        }
        ui.small(tr(ui.ctx(), "ui.saving-a-proposal-is-opt-in-project-saves-and-extra-parameter-exports-retain-it-reope"));
        ui.small(tr(ui.ctx(), "ui.edits-wait-for-send-opening-this-tab-or-enabling-the-checkbox-sends-nothing-refresh-l"));
    }
    fn proposal_matches(
        proposal: &canvas_pilot::input_proposal::InputProposal,
        snapshot: &Snapshot,
        link: Option<&canvas_pilot::DeviceLink>,
    ) -> bool {
        proposal.validate().is_ok()
            && snapshot.profile.advanced_inputs()
            && link == Some(&proposal.device_link)
            && snapshot
                .screens
                .iter()
                .find(|s| s.id == proposal.device_link.screen)
                .and_then(|s| {
                    input_path(
                        snapshot,
                        &s.id,
                        &s.preview_bank,
                        &proposal.device_link.layer,
                    )
                })
                .as_deref()
                == Some(proposal.base().as_str())
    }
    fn stage_proposal(
        &mut self,
        proposal: &canvas_pilot::input_proposal::InputProposal,
        snapshot: &Snapshot,
        link: Option<&canvas_pilot::DeviceLink>,
    ) -> Result<(), String> {
        if !self.enabled || !Self::proposal_matches(proposal, snapshot, link) {
            return Err(
                "Enable edits and verify the saved layer, input and plug mapping first.".into(),
            );
        }
        let values: Vec<_> = proposal
            .settings
            .iter()
            .map(|(key, value)| {
                let path = format!("{}/{key}", proposal.base());
                let original = snapshot
                    .raw
                    .pointer(&path)
                    .ok_or("Refresh device: proposal field unavailable.")?
                    .clone();
                Ok((path, value.clone(), original))
            })
            .collect::<Result<_, String>>()?;
        for (path, value, original) in values {
            self.stage(path, value, &original);
        }
        Ok(())
    }
    fn stage(&mut self, path: String, value: Value, original: &Value) {
        if &value == original {
            self.values.remove(&path);
        } else {
            self.values.insert(path, value);
        }
    }
    pub fn extend_plan(
        &self,
        snapshot: &Snapshot,
        mappings: &[(usize, String)],
        plan: &mut Plan,
    ) -> Result<(), String> {
        if !self.enabled || !snapshot.profile.advanced_inputs() {
            return Ok(());
        }
        let inputs: BTreeSet<_> = mappings
            .iter()
            .filter_map(|(_, layer)| input_path(snapshot, &plan.screen, &plan.bank, layer))
            .collect();
        let mut changes = Vec::new();
        for (path, after) in &self.values {
            if !inputs
                .iter()
                .any(|base| path.starts_with(&format!("{base}/")))
            {
                continue;
            }
            let before = snapshot
                .raw
                .pointer(path)
                .ok_or("Refresh the device: a staged input setting is unavailable.")?;
            if before != after {
                changes.push(Change {
                    path: path
                        .trim_start_matches('/')
                        .split('/')
                        .map(str::to_owned)
                        .collect(),
                    before: before.clone(),
                    after: after.clone(),
                });
            }
        }
        changes.append(&mut plan.changes);
        plan.changes = changes;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canvas_pilot::device_send::Target;

    fn fixture() -> Snapshot {
        Snapshot {
            profile: Default::default(),
            identity: "Test".into(),
            screens: vec![],
            raw: json!({"device": {"inputList": {"items": {"INPUT_4": {"status": {"pp": {"plug": "2"}}, "plugList": {"items": {"2": {"settings": {"aspect": {"pp": {"signal": "NATIVE", "transformTo": "SIGNAL", "customRatio": 1333, "layerFill": "LAYER_SETTING"}}, "cropping": {"control": {"pp": {"left": 0, "right": 0, "top": 0, "bottom": 0, "predefined": "NONE", "xUpdate": true}}}}}}}}}}, "screenList": {"items": {"1": {"presetList": {"items": {"UP": {"liveLayerList": {"items": {"1": {"source": {"pp": {"input": "INPUT_4"}}}, "2": {"source": {"pp": {"input": "INPUT_NONE"}}}}}}, "DOWN": {"liveLayerList": {"items": {"1": {"source": {"pp": {"input": "INPUT_NONE"}}}}}}}}}}}}}),
        }
    }
    fn empty_plan() -> Plan {
        Plan {
            profile: Default::default(),
            observed: vec![],
            screen: "1".into(),
            bank: "UP".into(),
            target: Target::Preview,
            changes: vec![],
        }
    }

    #[test]
    fn connected_source_controls_open_read_only_and_unchecking_discards_edits() {
        let mut snapshot = fixture();
        snapshot.screens.push(canvas_pilot::device::Screen {
            id: "1".into(),
            resolution: [1920, 1080],
            preview_bank: "UP".into(),
            program_bank: "DOWN".into(),
            layers: vec![],
        });
        let link = canvas_pilot::DeviceLink {
            family: Default::default(),
            screen: "1".into(),
            layer: "1".into(),
        };
        let ctx = egui::Context::default();
        let mut edits = InputEdits::default();
        let mut draw = |edits: &mut InputEdits, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900., 1000.),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| edits.show(ui, Some(&snapshot), Some(&link), 1));
                },
            )
        };
        draw(&mut edits, vec![]);
        let output = draw(&mut edits, vec![]);
        assert!(!edits.enabled);
        assert!(edits.values.is_empty());
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(t) if t.galley.text() == "Content ratio")));
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(t)
                    if t.galley.text()
                        == "Edit advanced input settings and include my edits when sending" =>
                {
                    Some(t.pos + egui::vec2(10., 5.))
                }
                _ => None,
            })
            .unwrap();
        let click = |edits: &mut InputEdits,
                     draw: &mut dyn FnMut(
            &mut InputEdits,
            Vec<egui::Event>,
        ) -> egui::FullOutput| {
            draw(
                edits,
                vec![
                    egui::Event::PointerMoved(pos),
                    super::super::tests::mouse(pos, true),
                ],
            );
            draw(edits, vec![super::super::tests::mouse(pos, false)]);
        };
        click(&mut edits, &mut draw);
        assert!(edits.enabled);
        assert!(edits.values.is_empty());
        edits.stage("test".into(), json!(40), &json!(0));
        click(&mut edits, &mut draw);
        assert!(!edits.enabled);
        assert!(edits.values.is_empty());
    }

    #[test]
    fn default_enable_only_and_unrelated_layers_never_write_inputs() {
        let snapshot = fixture();
        let mappings = [(1, "1".into())];
        let mut edits = InputEdits::default();
        let mut plan = empty_plan();
        edits.extend_plan(&snapshot, &mappings, &mut plan).unwrap();
        assert!(plan.changes.is_empty());
        edits.enabled = true;
        edits.extend_plan(&snapshot, &mappings, &mut plan).unwrap();
        assert!(plan.changes.is_empty());
        let path = format!(
            "{}/cropping/control/pp/left",
            input_path(&snapshot, "1", "UP", "1").unwrap()
        );
        edits.stage(path.clone(), json!(40), &json!(0));
        edits
            .extend_plan(&snapshot, &[(2, "2".into())], &mut plan)
            .unwrap();
        assert!(plan.changes.is_empty());
        edits.extend_plan(&snapshot, &mappings, &mut plan).unwrap();
        assert_eq!(plan.changes.len(), 1);
        assert_eq!(plan.changes[0].path.join("/"), path.trim_start_matches('/'));
        assert_eq!(plan.changes[0].after, json!(40));
        // Program has a different routed source and must not receive Preview input edits.
        let mut program = empty_plan();
        program.bank = "DOWN".into();
        program.target = Target::Program;
        edits
            .extend_plan(&snapshot, &mappings, &mut program)
            .unwrap();
        assert!(program.changes.is_empty());
        edits.enabled = false;
        let mut disabled = empty_plan();
        edits
            .extend_plan(&snapshot, &mappings, &mut disabled)
            .unwrap();
        assert!(disabled.changes.is_empty());
        edits.clear();
        assert!(edits.values.is_empty());
        assert!(!edits.enabled);
    }

    #[test]
    fn reverted_values_are_not_staged_and_duplicate_routes_are_written_once() {
        let snapshot = fixture();
        let path = format!(
            "{}/aspect/pp/transformTo",
            input_path(&snapshot, "1", "UP", "1").unwrap()
        );
        let mut edits = InputEdits {
            enabled: true,
            ..Default::default()
        };
        edits.stage(path.clone(), json!("16_9"), &json!("SIGNAL"));
        let mut plan = empty_plan();
        edits
            .extend_plan(&snapshot, &[(1, "1".into()), (2, "1".into())], &mut plan)
            .unwrap();
        assert_eq!(plan.changes.len(), 1);
        edits.stage(path, json!("SIGNAL"), &json!("SIGNAL"));
        assert!(edits.values.is_empty());
    }
    #[test]
    fn restored_proposals_are_inert_and_stage_only_after_route_and_field_review() {
        let mut snapshot = fixture();
        snapshot.screens.push(canvas_pilot::device::Screen {
            id: "1".into(),
            resolution: [1920, 1080],
            preview_bank: "UP".into(),
            program_bank: "DOWN".into(),
            layers: vec![],
        });
        let proposal = canvas_pilot::input_proposal::InputProposal {
            layer_id: 1,
            device_link: canvas_pilot::DeviceLink {
                family: Default::default(),
                screen: "1".into(),
                layer: "1".into(),
            },
            input: "INPUT_4".into(),
            plug: "2".into(),
            settings: [
                ("aspect/pp/transformTo".into(), json!("CUSTOM")),
                ("aspect/pp/customRatio".into(), json!(1778)),
            ]
            .into(),
        };
        let mut edits = InputEdits {
            saved: vec![proposal.clone()],
            ..Default::default()
        };
        let link = Some(&proposal.device_link);
        let mut plan = empty_plan();
        edits
            .extend_plan(&snapshot, &[(1, "1".into())], &mut plan)
            .unwrap();
        assert!(plan.changes.is_empty());
        assert!(edits.stage_proposal(&proposal, &snapshot, link).is_err());
        edits.enabled = true;
        edits
            .extend_plan(&snapshot, &[(1, "1".into())], &mut plan)
            .unwrap();
        assert!(
            plan.changes.is_empty(),
            "Enabling alone must not stage saved data"
        );
        let mut wrong = proposal.clone();
        wrong.input = "INPUT_9".into();
        assert!(edits.stage_proposal(&wrong, &snapshot, link).is_err());
        assert!(edits.stage_proposal(&proposal, &snapshot, None).is_err());
        let mut missing = snapshot.clone();
        missing
            .raw
            .pointer_mut("/device/inputList/items/INPUT_4/plugList/items/2/settings/aspect/pp")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("transformTo");
        assert!(edits.stage_proposal(&proposal, &missing, link).is_err());
        assert!(
            edits.values.is_empty(),
            "Missing fields reject staging atomically"
        );
        edits.stage_proposal(&proposal, &snapshot, link).unwrap();
        edits
            .extend_plan(&snapshot, &[(1, "1".into())], &mut plan)
            .unwrap();
        assert_eq!(plan.changes.len(), 2);
        assert!(plan.changes.iter().all(|c| {
            c.path
                .join("/")
                .starts_with("device/inputList/items/INPUT_4/plugList/items/2/settings/aspect/pp/")
        }));
        edits.clear();
        assert!(!edits.enabled && edits.values.is_empty());
        assert_eq!(edits.saved, vec![proposal]);
    }
}
