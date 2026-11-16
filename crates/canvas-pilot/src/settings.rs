//! Per-user application defaults. Contract: docs/interfaces/canvas-pilot-settings.md.
use crate::theme::Theme;
use serde::{Deserialize, Serialize};
use std::{io::Read, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub version: u32,
    pub language: crate::localization::Language,
    pub midra_host: String,
    pub recent_midra_hosts: Vec<String>,
    pub autoconnect_at_startup: bool,
    pub allow_sending_to_program: bool,
    pub theme: Theme,
    pub recent_themes: Vec<Theme>,
    pub snapping: canvas_pilot::snapping::Snapping,
    pub show_toolbar_icon_names: bool,
    pub keep_aspect_ratio: bool,
    pub show_source_pattern: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 9,
            language: Default::default(),
            autoconnect_at_startup: false,
            allow_sending_to_program: false,
            midra_host: String::new(),
            recent_midra_hosts: Vec::new(),
            theme: Theme::Midra,
            recent_themes: vec![Theme::Midra],
            snapping: Default::default(),
            show_toolbar_icon_names: false,
            keep_aspect_ratio: true,
            show_source_pattern: false,
        }
    }
}

impl Settings {
    fn parse(text: &str) -> Result<Self, String> {
        let mut value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if value["version"] == 1 {
            if value.get("recent_themes").is_some() {
                return Err("Unexpected recent_themes in version 1 settings".into());
            }
            let theme: Theme =
                serde_json::from_value(value["theme"].clone()).map_err(|e| e.to_string())?;
            if !matches!(
                theme,
                Theme::Midra | Theme::FoxifiedDark | Theme::CorporateLight
            ) {
                return Err("Unsupported theme in version 1 settings".into());
            }
            value["recent_themes"] = serde_json::json!([theme]);
            value["version"] = 2.into();
        }
        if value["version"] == 2 {
            if value.get("snapping").is_some() {
                return Err("Unexpected snapping in version 2 settings".into());
            }
            value["snapping"] = serde_json::to_value(canvas_pilot::snapping::Snapping::default())
                .map_err(|e| e.to_string())?;
            for field in ["grid", "show_grid", "grid_divisions"] {
                value["snapping"].as_object_mut().unwrap().remove(field);
            }
            value["version"] = 3.into();
        }
        if value["version"] == 3 {
            if value.get("midra_host").is_some() {
                return Err("Unexpected midra_host in version 3 settings".into());
            }
            value["midra_host"] = "".into();
            value["version"] = 4.into();
        }
        if value["version"] == 4 {
            if value.get("allow_sending_to_program").is_some() {
                return Err("Unexpected allow_sending_to_program in version 4 settings".into());
            }
            value["allow_sending_to_program"] = false.into();
            value["version"] = 5.into();
        }
        if value["version"] == 5 {
            if value.get("autoconnect_at_startup").is_some() {
                return Err("Unexpected autoconnect_at_startup in version 5 settings".into());
            }
            value["autoconnect_at_startup"] = false.into();
            value["version"] = 6.into();
        }
        if value["version"] == 6 {
            if !value["snapping"].is_object() {
                return Err("snapping must be an object".into());
            }
            for field in ["grid", "show_grid", "grid_divisions"] {
                if value["snapping"].get(field).is_some() {
                    return Err(format!("Unexpected {field} in version 6 settings"));
                }
            }
            value["snapping"]["grid"] = false.into();
            value["snapping"]["show_grid"] = true.into();
            value["snapping"]["grid_divisions"] = serde_json::json!([8, 6]);
            value["version"] = 7.into();
        }
        if value["version"] == 7 {
            if value.get("recent_midra_hosts").is_some() {
                return Err("Unexpected recent_midra_hosts in version 7 settings".into());
            }
            value["recent_midra_hosts"] = serde_json::json!([]);
            value["version"] = 8.into();
        }
        if value["version"] == 8 {
            if value.get("language").is_some() {
                return Err("Unexpected language in version 8 settings".into());
            }
            value["language"] = "system".into();
            value["version"] = 9.into();
        }
        let settings: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
        settings.validate()?;
        Ok(settings)
    }
    fn validate(&self) -> Result<(), String> {
        if self.version != 9 {
            return Err(format!("unsupported settings version {}", self.version));
        }
        if self.recent_midra_hosts.len() > 4
            || self.recent_midra_hosts.iter().enumerate().any(|(i, host)| {
                canvas_pilot::device::address(host)
                    .map(|address| address.to_string())
                    .as_ref()
                    != Ok(host)
                    || self.recent_midra_hosts[..i].contains(host)
            })
        {
            return Err(
                "recent_midra_hosts must contain at most four unique canonical IP:port addresses"
                    .into(),
            );
        }
        if self
            .snapping
            .grid_divisions
            .iter()
            .any(|n| !(1..=64).contains(n))
        {
            return Err("Grid divisions must be between 1 and 64 per axis".into());
        }
        if self.recent_themes.is_empty()
            || self.recent_themes.len() > 3
            || self.recent_themes[0] != self.theme
            || self
                .recent_themes
                .iter()
                .enumerate()
                .any(|(i, t)| self.recent_themes[..i].contains(t))
        {
            return Err("recent_themes must contain 1–3 unique themes, current theme first".into());
        }
        Ok(())
    }
}

/// No filesystem access from Default: headless editor tests stay isolated.
#[derive(Default)]
pub struct Store {
    pub path: Option<PathBuf>,
    pub error: Option<String>,
    writable: bool,
}
impl Store {
    pub fn open() -> (Self, Settings) {
        // Keep the original application directory for existing settings and recovery.
        match directories::ProjectDirs::from("org", "beffroi", "midra-layout") {
            Some(dirs) => Self::at(dirs.config_dir().join("settings.json")),
            None => (
                Self {
                    error: Some(
                        "Cannot locate your configuration directory; options will not be saved."
                            .into(),
                    ),
                    ..Self::default()
                },
                Settings::default(),
            ),
        }
    }
    pub(super) fn at(path: PathBuf) -> (Self, Settings) {
        let mut store = Self {
            path: Some(path.clone()),
            error: None,
            writable: true,
        };
        match Self::read(&path) {
            Ok(settings) => (store, settings),
            Err(e) => {
                store.writable = false;
                store.error = Some(format!(
                    "Could not load {}: {e}. Using defaults. Repair or move this file and restart to enable saving; the original file is kept.",
                    path.display()
                ));
                (store, Settings::default())
            }
        }
    }
    fn read(path: &std::path::Path) -> Result<Settings, String> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::default()),
            Err(e) => return Err(e.to_string()),
        };
        let mut text = String::new();
        file.take(65_537)
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        if text.len() > 65_536 {
            return Err("settings exceed 64 KiB".into());
        }
        Settings::parse(&text)
    }

    pub fn save(&mut self, settings: &Settings) {
        if !self.writable {
            return;
        }
        let Some(path) = &self.path else {
            return;
        };
        let result = (|| {
            settings.validate()?;
            let parent = path
                .parent()
                .ok_or("Settings path has no parent directory")?;
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())? + "\n";
            canvas_pilot::project::write_file(path, text.as_bytes(), true)
        })();
        self.error = result.err().map(|e| {
            format!(
                "Could not save {}: {e}. Your options still apply for this session.",
                path.display()
            )
        });
    }
    pub fn can_retry(&self) -> bool {
        self.writable && self.error.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_settings_migrate_and_validate_without_touching_other_preferences() {
        use crate::localization::Language;
        let old = include_str!("../../../docs/interfaces/canvas-pilot-settings-v8.example.json");
        let migrated = Settings::parse(old).unwrap();
        assert_eq!(migrated.language, Language::System);
        assert_eq!(migrated.version, 9);
        let current =
            include_str!("../../../docs/interfaces/canvas-pilot-settings-v9.example.json");
        assert_eq!(Settings::parse(current).unwrap(), migrated);
        for language in Language::ALL {
            let saved = Settings {
                language,
                ..migrated.clone()
            };
            assert_eq!(
                Settings::parse(&serde_json::to_string(&saved).unwrap()).unwrap(),
                saved
            );
        }
        let mut value: serde_json::Value = serde_json::from_str(current).unwrap();
        value["language"] = "unsupported".into();
        assert!(Settings::parse(&value.to_string()).is_err());
        value.as_object_mut().unwrap().remove("language");
        assert!(Settings::parse(&value.to_string()).is_err());
        value["version"] = 8.into();
        value["language"] = "fr".into();
        assert!(Settings::parse(&value.to_string()).is_err());
    }

    #[test]
    fn connection_history_migrates_and_round_trips_strictly() {
        let old = include_str!("../../../docs/interfaces/canvas-pilot-settings-v7.example.json");
        assert!(Settings::parse(old).unwrap().recent_midra_hosts.is_empty());
        let example =
            include_str!("../../../docs/interfaces/canvas-pilot-settings-v8.example.json");
        assert_eq!(Settings::parse(example).unwrap(), Settings::default());
        let mut value: serde_json::Value = serde_json::from_str(example).unwrap();
        value["recent_midra_hosts"] = serde_json::json!(["192.168.10.146:3000", "[::1]:80"]);
        let settings = Settings::parse(&value.to_string()).unwrap();
        assert_eq!(
            Settings::parse(&serde_json::to_string(&settings).unwrap()).unwrap(),
            settings
        );
        for hosts in [
            serde_json::json!(["bad"]),
            serde_json::json!(["127.0.0.1"]),
            serde_json::json!(["127.0.0.1:80", "127.0.0.1:80"]),
            serde_json::json!([
                "127.0.0.1:1",
                "127.0.0.1:2",
                "127.0.0.1:3",
                "127.0.0.1:4",
                "127.0.0.1:5"
            ]),
        ] {
            value["recent_midra_hosts"] = hosts;
            assert!(Settings::parse(&value.to_string()).is_err());
        }
        let mut legacy: serde_json::Value = serde_json::from_str(old).unwrap();
        legacy["recent_midra_hosts"] = serde_json::json!([]);
        assert!(Settings::parse(&legacy.to_string()).is_err());
    }

    #[test]
    fn missing_settings_and_changed_options_roundtrip_without_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app/settings.json");
        let (mut store, mut settings) = Store::at(path.clone());
        assert_eq!(settings, Settings::default());
        assert!(!path.exists());
        settings.theme = Theme::CorporateLight;
        settings.recent_themes = vec![Theme::CorporateLight, Theme::Midra];
        settings.show_toolbar_icon_names = true;
        settings.keep_aspect_ratio = false;
        settings.show_source_pattern = true;
        settings.midra_host = "192.168.10.146:3000".into();
        settings.allow_sending_to_program = true;
        settings.autoconnect_at_startup = true;
        store.save(&settings);
        assert!(store.error.is_none());
        assert_eq!(Store::at(path.clone()).1, settings);
        settings.theme = Theme::FoxifiedDark;
        settings.recent_themes = vec![Theme::FoxifiedDark, Theme::CorporateLight, Theme::Midra];
        store.save(&settings);
        assert_eq!(Store::at(path.clone()).1, settings);
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1
        );
    }
    #[test]
    fn invalid_or_future_settings_are_preserved_and_writes_can_be_retried() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        for text in [
            "broken".to_string(),
            serde_json::to_string(&Settings {
                version: 10,
                ..Default::default()
            })
            .unwrap(),
            "x".repeat(65_537),
        ] {
            std::fs::write(&path, &text).unwrap();
            let (mut store, settings) = Store::at(path.clone());
            assert_eq!(settings, Settings::default());
            assert!(store.error.is_some());
            store.save(&settings);
            assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        }
        let blocker = dir.path().join("blocked");
        let (mut store, settings) = Store::at(blocker.join("settings.json"));
        std::fs::write(&blocker, "occupied").unwrap();
        store.save(&settings);
        assert!(store.can_retry());
        assert_eq!(std::fs::read_to_string(&blocker).unwrap(), "occupied");
        std::fs::remove_file(&blocker).unwrap();
        store.save(&settings);
        assert!(store.error.is_none());
        assert_eq!(Store::at(blocker.join("settings.json")).1, settings);
    }
    #[test]
    fn version_seven_grid_settings_round_trip_and_validate() {
        let example =
            include_str!("../../../docs/interfaces/canvas-pilot-settings-v7.example.json");
        assert_eq!(Settings::parse(example).unwrap(), Settings::default());
        let mut settings = Settings::default();
        settings.snapping.grid = true;
        settings.snapping.show_grid = false;
        settings.snapping.grid_divisions = [12, 9];
        assert_eq!(
            Settings::parse(&serde_json::to_string(&settings).unwrap()).unwrap(),
            settings
        );
        for divisions in [[0, 6], [8, 65]] {
            settings.snapping.grid_divisions = divisions;
            assert!(Settings::parse(&serde_json::to_string(&settings).unwrap()).is_err());
        }
        let mut legacy: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/interfaces/canvas-pilot-settings-v6.example.json"
        ))
        .unwrap();
        legacy["snapping"]["grid"] = true.into();
        assert!(Settings::parse(&legacy.to_string()).is_err());
        legacy["snapping"] = serde_json::Value::Null;
        assert!(Settings::parse(&legacy.to_string()).is_err());
    }
    #[test]
    fn version_six_migrates_with_autoconnect_disabled() {
        let example =
            include_str!("../../../docs/interfaces/canvas-pilot-settings-v6.example.json");
        assert_eq!(Settings::parse(example).unwrap(), Settings::default());
        let mut legacy: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/interfaces/canvas-pilot-settings-v5.example.json"
        ))
        .unwrap();
        legacy["midra_host"] = "192.168.10.143".into();
        let migrated = Settings::parse(&legacy.to_string()).unwrap();
        assert!(!migrated.autoconnect_at_startup);
        assert_eq!(migrated.midra_host, "192.168.10.143");
        legacy["autoconnect_at_startup"] = true.into();
        assert!(Settings::parse(&legacy.to_string()).is_err());
    }
    #[test]
    fn version_five_disables_program_for_legacy_settings() {
        let example =
            include_str!("../../../docs/interfaces/canvas-pilot-settings-v5.example.json");
        assert_eq!(Settings::parse(example).unwrap(), Settings::default());
        let mut legacy: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/interfaces/canvas-pilot-settings-v4.example.json"
        ))
        .unwrap();
        legacy["midra_host"] = "192.168.10.143".into();
        let migrated = Settings::parse(&legacy.to_string()).unwrap();
        assert!(!migrated.allow_sending_to_program);
        assert_eq!(migrated.midra_host, "192.168.10.143");
        legacy["allow_sending_to_program"] = true.into();
        assert!(Settings::parse(&legacy.to_string()).is_err());
    }
    #[test]
    fn version_four_remembers_address_and_migrates_legacy_without_it() {
        let example =
            include_str!("../../../docs/interfaces/canvas-pilot-settings-v4.example.json");
        assert_eq!(Settings::parse(example).unwrap(), Settings::default());
        let mut legacy: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/interfaces/canvas-pilot-settings-v3.example.json"
        ))
        .unwrap();
        legacy["midra_host"] = "192.168.10.143".into();
        assert!(Settings::parse(&legacy.to_string()).is_err());
    }
    #[test]
    fn documented_settings_match_defaults() {
        let example =
            include_str!("../../../docs/interfaces/canvas-pilot-settings-v1.example.json");
        assert_eq!(Settings::parse(example).unwrap(), Settings::default());
    }
    #[test]
    fn version_two_recent_history_is_strict_and_version_one_migrates() {
        let example =
            include_str!("../../../docs/interfaces/canvas-pilot-settings-v2.example.json");
        assert_eq!(Settings::parse(example).unwrap(), Settings::default());
        let legacy = include_str!("../../../docs/interfaces/canvas-pilot-settings-v1.example.json")
            .replace("midra", "foxified_dark");
        let migrated = Settings::parse(&legacy).unwrap();
        assert_eq!(migrated.theme, Theme::FoxifiedDark);
        assert_eq!(migrated.recent_themes, vec![Theme::FoxifiedDark]);
        for recent in [
            serde_json::json!([]),
            serde_json::json!(["midra", "midra"]),
            serde_json::json!(["bluecore"]),
            serde_json::json!(["midra", "bluecore", "hci", "solarized"]),
        ] {
            let mut value: serde_json::Value = serde_json::from_str(example).unwrap();
            value["recent_themes"] = recent;
            assert!(Settings::parse(&value.to_string()).is_err());
        }
    }
    #[test]
    fn version_three_adds_disabled_snapping_to_older_settings() {
        let example =
            include_str!("../../../docs/interfaces/canvas-pilot-settings-v3.example.json");
        assert_eq!(Settings::parse(example).unwrap(), Settings::default());
        let mut settings = Settings::default();
        settings.snapping.enabled = true;
        assert_eq!(
            Settings::parse(&serde_json::to_string(&settings).unwrap()).unwrap(),
            settings
        );
    }
}
