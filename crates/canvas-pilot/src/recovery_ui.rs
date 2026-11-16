use super::*;
use canvas_pilot::project::{Project, write_file};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    version: u32,
    #[serde(deserialize_with = "deserialize_project")]
    project: Project,
    project_path: Option<PathBuf>,
    manual_saved_at: Option<u64>,
}
fn deserialize_project<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Project, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Project::from_json(&value.to_string()).map_err(serde::de::Error::custom)
}
#[derive(Default)]
pub(super) struct Recovery {
    pub path: Option<PathBuf>,
    last: Option<String>,
    checked: Option<Instant>,
    pub error: Option<String>,
    pub restore_failed: bool,
}
pub(super) fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
impl Editor {
    pub(super) fn restore_recovery(&mut self, ctx: &egui::Context, path: PathBuf) {
        self.recovery.path = Some(path.clone());
        self.recovery.error = None;
        self.recovery.restore_failed = false;
        self.start_project_job(ctx, true, move || {
            let file = match std::fs::File::open(&path) {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e.to_string()),
            };
            let mut bytes = Vec::new();
            file.take(100 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 100 * 1024 * 1024 {
                return Err("Recovery file exceeds 100 MiB".into());
            }
            let snapshot: Snapshot = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if snapshot.version != 1 {
                return Err("Unsupported recovery version".into());
            }
            // Use the project loader to preserve its validation/migration contract.
            let project = Project::from_json(
                &serde_json::to_string(&snapshot.project).map_err(|e| e.to_string())?,
            )?;
            Ok(Some((
                project,
                snapshot.project_path,
                snapshot.manual_saved_at,
            )))
        });
    }

    pub(super) fn autosave_recovery(&mut self, force: bool) {
        if self.files.loading.is_some()
            || self.recovery.path.is_none()
            || self.recovery.error.is_some()
        {
            return;
        }
        if !force
            && self
                .recovery
                .checked
                .is_some_and(|t| t.elapsed() < Duration::from_secs(2))
        {
            return;
        }
        self.recovery.checked = Some(Instant::now());
        let result: Result<(), String> = (|| {
            let project = self.project();
            project.layout()?;
            let text = serde_json::to_string(&Snapshot {
                version: 1,
                project,
                project_path: self.files.project_path.clone(),
                manual_saved_at: self.files.manual_saved_at,
            })
            .map_err(|e| e.to_string())?;
            if text.len() > 100 * 1024 * 1024 {
                return Err("Recovery snapshot exceeds 100 MiB".into());
            }
            if self.recovery.last.as_ref() == Some(&text) {
                return Ok(());
            }
            let path = self.recovery.path.as_ref().unwrap();
            std::fs::create_dir_all(path.parent().ok_or("Recovery path has no parent")?)
                .map_err(|e| e.to_string())?;
            write_file(path, text.as_bytes(), true)?;
            self.recovery.last = Some(text);
            Ok(())
        })();
        if let Err(error) = result {
            self.recovery.error = Some(format!(
                "Recovery autosave failed: {error}. Use Save project to keep a separate copy."
            ));
            self.files.message = self.recovery.error.clone();
        }
    }
    pub(super) fn manual_save_age(&self) -> String {
        match self.files.manual_saved_at {
            Some(time) => {
                let seconds = now_seconds().saturating_sub(time);
                if seconds < 60 {
                    "Last manual save: less than a minute ago".into()
                } else {
                    format!("Last manual save: {} minutes ago", seconds / 60)
                }
            }
            None => "No recorded manual save for this project".into(),
        }
    }
    pub(super) fn new_project_window(&mut self, ctx: &egui::Context) {
        if self.files.new_ready {
            self.files.new_ready = false;
            match self.install_project(ctx, Project::from_layout(&Layout::default())) {
                Ok(()) => {
                    self.recovery.error = None;
                    self.recovery.restore_failed = false;
                    self.autosave_recovery(true);
                }
                Err(error) => self.files.message = Some(error),
            }
        }
        if !self.files.new_requested {
            return;
        }
        let mut open = true;
        let mut save = false;
        let mut discard = false;
        let mut cancel = false;
        egui::Window::new("New project?").collapsible(false).open(&mut open).show(ctx, |ui| {
            ui.label("Starting a new project replaces the recovery autosave. Current edits will be lost unless saved to a separate project file.");
            ui.label(self.manual_save_age());
            ui.small("Autosave does not count as a manual save. Only applied edits are saved; pending drafts will be discarded.");
            ui.horizontal(|ui| {
                save = ui.button("Save and create new…").clicked();
                discard = ui.button("Discard and create new").clicked();
                cancel = ui.button("Cancel").clicked();
            });
        });
        if !open || cancel {
            self.files.new_requested = false;
        }
        if discard {
            self.files.new_requested = false;
            self.files.new_ready = true;
            ctx.request_repaint();
        }
        if save {
            self.files.new_requested = false;
            self.files.new_after_save = true;
            self.save_project_as();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click_text, frame};
    use super::*;

    #[test]
    fn documented_recovery_example_loads() {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("last_project.json");
        std::fs::write(
            &path,
            include_str!("../../../docs/interfaces/canvas-pilot-recovery-v1.example.json"),
        )
        .unwrap();
        let mut app = Editor::default();
        app.restore_recovery(&ctx, path);
        app.wait_project_job();
        assert!(app.recovery.error.is_none());
        assert_eq!(app.project(), Project::from_layout(&Layout::default()));
    }

    #[test]
    fn recovery_restores_applied_edits_metadata_and_leaves_named_file_alone() {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config/last_project.json");
        let named = dir.path().join("named.json");
        std::fs::write(&named, "leave untouched").unwrap();
        let mut app = Editor::default();
        app.restore_recovery(&ctx, path.clone());
        app.wait_project_job();
        app.files.project_path = Some(named.clone());
        app.files.manual_saved_at = Some(now_seconds() - 180);
        app.layout.nudge(40.0, -20.0).unwrap();
        app.layout
            .set_layer_fit(
                0,
                Some(canvas_pilot::formats::LayerFit::Fullscreen),
                canvas_pilot::formats::Crop {
                    left: 123.0,
                    ..Default::default()
                },
            )
            .unwrap();
        app.layout.set_layer_pivots(&[(1, [0.2, 0.7])]).unwrap();
        app.corner = PositionPoint::BottomRight;
        app.position_layer_point = Some(PositionPoint::Center);
        app.backdrop_title = "Recovered conference".into();
        app.autosave_recovery(true);
        assert!(app.recovery.error.is_none());
        assert_eq!(std::fs::read_to_string(&named).unwrap(), "leave untouched");
        let first = std::fs::read(&path).unwrap();
        app.autosave_recovery(true);
        assert_eq!(std::fs::read(&path).unwrap(), first);
        let mut restored = Editor::default();
        restored.restore_recovery(&ctx, path);
        restored.wait_project_job();
        assert!(restored.recovery.error.is_none());
        assert!(restored.files.message.is_none());
        assert_eq!(restored.project(), app.project());
        assert_eq!(restored.files.project_path, Some(named));
        assert_eq!(restored.files.manual_saved_at, app.files.manual_saved_at);
        assert!(restored.manual_save_age().contains("3 minutes"));
        restored.layout.undo();
        assert_eq!(restored.project(), app.project());
    }

    #[test]
    fn invalid_recovery_is_preserved_and_write_failure_keeps_last_snapshot() {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("last_project.json");
        for text in ["broken", "{\"version\":999}"] {
            std::fs::write(&path, text).unwrap();
            let mut app = Editor::default();
            let before = app.project();
            app.restore_recovery(&ctx, path.clone());
            app.wait_project_job();
            assert!(app.recovery.error.is_some());
            assert!(app.recovery.restore_failed);
            assert_eq!(app.project(), before);
            app.autosave_recovery(true);
            assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        }
        let mut app = Editor::default();
        app.recovery.path = Some(path.join("impossible.json"));
        app.autosave_recovery(true);
        assert!(app.recovery.error.is_some());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "{\"version\":999}");
    }

    #[test]
    fn new_project_cancel_preserves_work_and_discard_replaces_recovery() {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("last_project.json");
        let mut app = Editor::default();
        app.restore_recovery(&ctx, path.clone());
        app.wait_project_job();
        app.layout.nudge(123.0, 0.0).unwrap();
        app.sync();
        app.autosave_recovery(true);
        let before = app.project();
        app.files.new_requested = true;
        click_text(&mut app, &ctx, "Cancel");
        assert_eq!(app.project(), before);
        app.files.new_requested = true;
        click_text(&mut app, &ctx, "Discard and create new");
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.project(), Project::from_layout(&Layout::default()));
        assert!(app.files.manual_saved_at.is_none());
        let mut restored = Editor::default();
        restored.restore_recovery(&ctx, path);
        restored.wait_project_job();
        assert_eq!(restored.project(), app.project());
    }
}
