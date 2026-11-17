//! Startup snapshot of optional local software. Detection never launches applications.
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Software {
    LibreOffice,
    Drawio,
    VectorCraft,
}

impl Software {
    pub const ALL: [Self; 3] = [Self::LibreOffice, Self::Drawio, Self::VectorCraft];
    pub fn name(self) -> &'static str {
        match self {
            Self::LibreOffice => "LibreOffice",
            Self::Drawio => "draw.io Desktop",
            Self::VectorCraft => "VectorCraft CLI",
        }
    }
    pub fn environment(self) -> &'static str {
        match self {
            Self::LibreOffice => "CANVAS_PILOT_LIBREOFFICE",
            Self::Drawio => "CANVAS_PILOT_DRAWIO",
            Self::VectorCraft => "MIDRA_VECTORCRAFT_CLI",
        }
    }
    pub fn functionality(self) -> &'static str {
        match self {
            Self::LibreOffice => {
                "Render PowerPoint slides as reference pictures. PPTX geometry import/export and layout-box references work without it."
            }
            Self::Drawio => {
                "Render complete draw.io pages as reference pictures. Diagram geometry import/export and layout-box references work without it."
            }
            Self::VectorCraft => {
                "Import and export Illustrator files, and render Illustrator artboards as reference pictures."
            }
        }
    }
    fn names(self) -> &'static [&'static str] {
        match (self, cfg!(windows)) {
            (Self::LibreOffice, true) => &["soffice.exe"],
            (Self::LibreOffice, false) => &["libreoffice", "soffice"],
            (Self::Drawio, true) => &["draw.io.exe"],
            (Self::Drawio, false) => &["drawio", "draw.io"],
            (Self::VectorCraft, true) => &["vectorcraft-cli.exe"],
            (Self::VectorCraft, false) => &["vectorcraft-cli"],
        }
    }
    fn enabled(self) -> bool {
        self == Self::Drawio
            || cfg!(all(
                feature = "vendor_formats",
                not(feature = "no_proprietary")
            ))
    }
    pub fn integration(self) -> &'static Integration {
        detected()
            .iter()
            .find(|i| i.software == self)
            .expect("all integrations are detected")
    }
    pub fn executable(self) -> Result<&'static Path, String> {
        let integration = self.integration();
        if !integration.enabled {
            return Err(format!(
                "{} integration is disabled in this build.",
                self.name()
            ));
        }
        integration.path.as_deref().ok_or_else(|| format!("{} was not found at startup. See Software integrations for setup, then restart Canvas Pilot.", self.name()))
    }
}

#[derive(Clone, Debug)]
pub struct Integration {
    pub software: Software,
    pub path: Option<PathBuf>,
    pub overridden: bool,
    pub enabled: bool,
}
impl Integration {
    pub fn available(&self) -> bool {
        self.enabled && self.path.is_some()
    }
    pub fn status(&self) -> &'static str {
        if !self.enabled {
            "Disabled in this build"
        } else if self.path.is_some() {
            "Detected"
        } else if self.overridden {
            "Configured executable not found or not executable"
        } else {
            "Not found"
        }
    }
}

pub fn detected() -> &'static [Integration] {
    static DETECTED: OnceLock<Vec<Integration>> = OnceLock::new();
    DETECTED.get_or_init(|| {
        Software::ALL
            .into_iter()
            .map(|software| {
                let configured = std::env::var_os(software.environment()).map(PathBuf::from);
                let mut directories = Vec::new();
                if let Ok(exe) = std::env::current_exe()
                    && let Some(parent) = exe.parent()
                {
                    directories.extend([parent.to_owned(), parent.join("bin")]);
                }
                if let Some(path) = std::env::var_os("PATH") {
                    directories.extend(std::env::split_paths(&path));
                }
                directories.extend(installation_directories(software));
                Integration {
                    software,
                    path: resolve(configured.as_deref(), software.names(), &directories),
                    overridden: configured.is_some(),
                    enabled: software.enabled(),
                }
            })
            .collect()
    })
}

fn executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn resolve(configured: Option<&Path>, names: &[&str], directories: &[PathBuf]) -> Option<PathBuf> {
    let candidates: Vec<_> = if let Some(path) = configured {
        if path.components().count() > 1 || path.is_absolute() {
            vec![path.to_owned()]
        } else {
            directories.iter().map(|dir| dir.join(path)).collect()
        }
    } else {
        directories
            .iter()
            .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
            .collect()
    };
    candidates
        .into_iter()
        .find(|p| executable_file(p))
        .and_then(|p| std::path::absolute(p).ok())
}

fn installation_directories(software: Software) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(target_os = "macos") {
        let suffix = match software {
            Software::LibreOffice => "LibreOffice.app/Contents/MacOS",
            Software::Drawio => "draw.io.app/Contents/MacOS",
            Software::VectorCraft => "VectorCraft.app/Contents/MacOS",
        };
        dirs.push(Path::new("/Applications").join(suffix));
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join("Applications").join(suffix));
        }
    }
    if cfg!(windows) {
        let suffix = match software {
            Software::LibreOffice => "LibreOffice/program",
            Software::Drawio => "draw.io",
            Software::VectorCraft => "VectorCraft",
        };
        for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(root) = std::env::var_os(variable) {
                dirs.push(PathBuf::from(root).join(suffix));
            }
        }
        if let Some(root) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(root).join("Programs").join(suffix));
        }
    }
    dirs
}

/// UI choices: compiled codecs whose required external converter was detected.
pub fn codecs() -> Vec<crate::interchange::Codec> {
    crate::interchange::Codec::available()
        .into_iter()
        .filter(|c| {
            *c != crate::interchange::Codec::Illustrator
                || Software::VectorCraft.integration().available()
        })
        .collect()
}
pub fn geometry_extensions() -> Vec<&'static str> {
    let mut extensions: Vec<_> = codecs().into_iter().map(|c| c.extension()).collect();
    extensions.push("xml");
    extensions
}
pub fn artwork_available(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "pptx" => Software::LibreOffice.integration().available(),
        "drawio" | "xml" => Software::Drawio.integration().available(),
        "ai" => Software::VectorCraft.integration().available(),
        "csv" | "dxf" | "dwg" => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn executable(path: &Path) {
        std::fs::write(path, b"not launched by discovery").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    #[test]
    fn startup_resolution_obeys_override_and_directory_priority() {
        let temp = tempfile::tempdir().unwrap();
        let app = temp.path().join("app");
        let bin = app.join("bin");
        let path = temp.path().join("path with spaces");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&path).unwrap();
        let dirs = [app.clone(), bin.clone(), path.clone()];
        executable(&path.join("tool"));
        assert_eq!(resolve(None, &["tool"], &dirs), Some(path.join("tool")));
        executable(&bin.join("tool"));
        assert_eq!(resolve(None, &["tool"], &dirs), Some(bin.join("tool")));
        executable(&app.join("tool"));
        assert_eq!(resolve(None, &["tool"], &dirs), Some(app.join("tool")));
        assert_eq!(
            resolve(Some(&path.join("tool")), &["tool"], &dirs),
            Some(path.join("tool"))
        );
        assert!(resolve(Some(&path.join("missing")), &["tool"], &dirs).is_none());
        assert_eq!(
            resolve(Some(Path::new("tool")), &["ignored"], &dirs),
            Some(app.join("tool"))
        );
        assert!(
            resolve(None, &["bin"], &dirs).is_none(),
            "directories are not executables"
        );
        #[cfg(unix)]
        {
            std::fs::write(app.join("plain"), b"not executable").unwrap();
            assert!(resolve(None, &["plain"], &dirs).is_none());
        }
    }
    #[test]
    fn integration_states_and_ui_choices_match_detected_capabilities() {
        for software in Software::ALL {
            let mut integration = Integration {
                software,
                path: None,
                overridden: false,
                enabled: true,
            };
            assert_eq!(integration.status(), "Not found");
            assert!(!integration.available());
            integration.overridden = true;
            assert!(integration.status().starts_with("Configured"));
            integration.path = Some("tool".into());
            assert!(integration.available());
            integration.enabled = false;
            assert!(!integration.available());
            assert_eq!(integration.status(), "Disabled in this build");
            assert!(!software.functionality().is_empty());
            assert_eq!(
                software.executable().is_ok(),
                software.integration().available()
            );
        }
        assert_eq!(
            codecs().contains(&crate::interchange::Codec::Illustrator),
            Software::VectorCraft.integration().available()
        );
        assert_eq!(
            artwork_available(Path::new("slides.PPTX")),
            Software::LibreOffice.integration().available()
        );
        assert_eq!(
            artwork_available(Path::new("diagram.xml")),
            Software::Drawio.integration().available()
        );
        assert!(!artwork_available(Path::new("layout.csv")));
        assert!(artwork_available(Path::new("client.pdf")));
    }
}
