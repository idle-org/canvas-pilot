//! Runtime palette packs. Contract: docs/interfaces/theme-packs.md.
use crate::theme::Palette;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAX_ARCHIVE: u64 = 2 * 1024 * 1024;
const MAX_EXPANDED: u64 = 4 * 1024 * 1024;
const MAX_PACKS: usize = 32;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    id: String,
    revision: u64,
    themes: Vec<Description>,
    /// Attribution and license notices travel with the pack.
    #[serde(default)]
    notices: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Description {
    pub id: String,
    pub name: String,
    pub light: bool,
    pub colors: Colors,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Colors {
    pub canvas: String,
    pub banner: String,
    pub grid: String,
    pub layer: String,
    pub bg: String,
    pub panel: String,
    pub field: String,
    pub line: String,
    pub text: String,
    pub muted: String,
    pub cyan: String,
    pub amber: String,
    pub purple: String,
    pub pattern: [String; 2],
}
fn color(text: &str) -> Result<egui::Color32, String> {
    if text.len() != 7
        || !text.starts_with('#')
        || !text[1..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(format!("Expected opaque #RRGGBB color: {text}"));
    }
    let value = u32::from_str_radix(&text[1..], 16).map_err(|e| e.to_string())?;
    Ok(egui::Color32::from_rgb(
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    ))
}
impl Colors {
    pub fn palette(&self) -> Result<Palette, String> {
        Ok(Palette {
            canvas: color(&self.canvas)?,
            banner: color(&self.banner)?,
            grid: color(&self.grid)?,
            layer: color(&self.layer)?,
            bg: color(&self.bg)?,
            panel: color(&self.panel)?,
            field: color(&self.field)?,
            line: color(&self.line)?,
            text: color(&self.text)?,
            muted: color(&self.muted)?,
            cyan: color(&self.cyan)?,
            amber: color(&self.amber)?,
            purple: color(&self.purple)?,
            pattern: [color(&self.pattern[0])?, color(&self.pattern[1])?],
        })
    }
}
#[derive(Clone)]
struct Pack {
    manifest: Manifest,
}
#[derive(Clone, Debug)]
pub struct Installed {
    pub id: String,
    pub revision: u64,
    pub themes: Vec<Description>,
    pub notices: String,
    path: PathBuf,
    bytes: std::sync::Arc<[u8]>,
}
pub fn installed(ctx: &egui::Context) -> Vec<Installed> {
    ctx.data(|data| data.get_temp::<Vec<Installed>>(egui::Id::new("theme-packs")))
        .unwrap_or_default()
}
pub fn valid_choice(id: &str) -> bool {
    id.split_once('/').is_some_and(|(pack, theme)| {
        crate::language_packs::valid_id(pack) && crate::language_packs::valid_id(theme)
    })
}
pub fn find(ctx: &egui::Context, id: &str) -> Option<Description> {
    let (pack, theme) = id.split_once('/')?;
    installed(ctx)
        .into_iter()
        .find(|p| p.id == pack)?
        .themes
        .into_iter()
        .find(|t| t.id == theme)
}
pub fn choices(ctx: &egui::Context) -> Vec<crate::theme::Theme> {
    installed(ctx)
        .into_iter()
        .flat_map(|p| {
            p.themes
                .into_iter()
                .map(move |t| crate::theme::Theme::Pack(format!("{}/{}", p.id, t.id)))
        })
        .collect()
}
fn register(ctx: &egui::Context, pack: &Pack, path: &Path, bytes: &[u8]) {
    let mut packs = installed(ctx);
    packs.retain(|p| p.id != pack.manifest.id);
    packs.push(Installed {
        id: pack.manifest.id.clone(),
        revision: pack.manifest.revision,
        themes: pack.manifest.themes.clone(),
        notices: pack.manifest.notices.clone(),
        path: path.into(),
        bytes: bytes.into(),
    });
    packs.sort_by(|a, b| a.id.cmp(&b.id));
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("theme-packs"), packs));
}
fn parse(bytes: &[u8]) -> Result<Pack, String> {
    if bytes.len() as u64 > MAX_ARCHIVE {
        return Err("Theme pack exceeds 2 MiB".into());
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if zip.len() != 1 {
        return Err("Expected only manifest.json in the theme pack".into());
    }
    let mut entry = zip.by_index(0).map_err(|e| e.to_string())?;
    if entry.name() != "manifest.json"
        || entry.is_dir()
        || entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
    {
        return Err("Expected a regular manifest.json at the ZIP root".into());
    }
    let mut data = Vec::new();
    (&mut entry)
        .take(MAX_EXPANDED + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > MAX_EXPANDED {
        return Err("Expanded theme pack exceeds 4 MiB".into());
    }
    let manifest: Manifest = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
    if manifest.version != 1
        || manifest.revision == 0
        || !crate::language_packs::valid_id(&manifest.id)
        || manifest.themes.is_empty()
        || manifest.themes.len() > 16
    {
        return Err("Invalid theme-pack manifest or unsupported version".into());
    }
    if manifest.notices.len() > 65536
        || manifest
            .notices
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r')
    {
        return Err("Invalid attribution notices".into());
    }
    let mut ids = BTreeSet::new();
    for theme in &manifest.themes {
        if !crate::language_packs::valid_id(&theme.id)
            || !ids.insert(&theme.id)
            || theme.name.trim().is_empty()
            || theme.name.len() > 160
            || theme.name.chars().any(char::is_control)
        {
            return Err("Invalid or duplicate theme identifier/name".into());
        }
        theme.colors.palette()?;
    }
    Ok(Pack { manifest })
}
fn read(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_ARCHIVE + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}
fn check_collisions(ctx: &egui::Context, pack: &Pack, replacing: bool) -> Result<(), String> {
    if !replacing && installed(ctx).iter().any(|p| p.id == pack.manifest.id) {
        return Err("Pack identifier is already installed".into());
    }
    Ok(())
}
/// Load once at application startup; invalid files do not prevent startup.
pub fn load(ctx: &egui::Context, directory: &Path) -> Vec<String> {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("theme-packs"), Vec::<Installed>::new()));
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return vec![],
        Err(error) => return vec![error.to_string()],
    };
    let mut paths = Vec::new();
    let mut errors = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) if entry.path().extension().is_some_and(|ext| ext == "zip") => {
                paths.push(entry.path())
            }
            Ok(_) => (),
            Err(error) => errors.push(error.to_string()),
        }
        if paths.len() > MAX_PACKS {
            errors.push("Too many theme packs (maximum 32)".into());
            break;
        }
    }
    paths.sort();
    for path in paths.into_iter().take(MAX_PACKS) {
        let result = read(&path).and_then(|bytes| {
            let pack = parse(&bytes)?;
            check_collisions(ctx, &pack, false)?;
            register(ctx, &pack, &path, &bytes);
            Ok(())
        });
        if let Err(error) = result {
            errors.push(format!("{}: {error}", path.display()));
        }
    }
    errors
}
/// Validated bytes and the exact destination reviewed by the user. Never re-read the source on confirmation.
pub struct Prepared {
    pack: Pack,
    bytes: Vec<u8>,
    path: PathBuf,
    previous: Option<Vec<u8>>,
    pub previous_revision: Option<u64>,
}
impl Prepared {
    pub fn id(&self) -> &str {
        &self.pack.manifest.id
    }
    pub fn revision(&self) -> u64 {
        self.pack.manifest.revision
    }
    pub fn needs_confirmation(&self) -> bool {
        self.previous.is_some()
            && self
                .previous_revision
                .is_none_or(|old| self.revision() <= old)
    }
}
fn existing_bytes(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match read(path) {
        Ok(bytes) if bytes.len() as u64 <= MAX_ARCHIVE => Ok(Some(bytes)),
        Ok(_) => Err("Existing pack exceeds 2 MiB; remove it manually before replacing it".into()),
        Err(_error) if !path.exists() => Ok(None),
        Err(error) => Err(error),
    }
}
pub fn prepare(ctx: &egui::Context, directory: &Path, source: &Path) -> Result<Prepared, String> {
    let bytes = read(source)?;
    let pack = parse(&bytes)?;
    check_collisions(ctx, &pack, true)?;
    let path = installed(ctx)
        .into_iter()
        .find(|p| p.id == pack.manifest.id)
        .map_or_else(
            || directory.join(format!("{}.zip", pack.manifest.id)),
            |p| p.path,
        );
    let previous = existing_bytes(&path)?;
    if let Some(old) = previous.as_ref().and_then(|bytes| parse(bytes).ok())
        && old.manifest.id != pack.manifest.id
    {
        return Err("Destination belongs to a different pack; remove that pack first".into());
    }
    let previous_revision = previous
        .as_ref()
        .and_then(|bytes| parse(bytes).ok())
        .filter(|old| old.manifest.id == pack.manifest.id)
        .map(|old| old.manifest.revision);
    Ok(Prepared {
        pack,
        bytes,
        path,
        previous,
        previous_revision,
    })
}
pub fn commit(ctx: &egui::Context, pending: Prepared, overwrite: bool) -> Result<(), String> {
    if pending.needs_confirmation() && !overwrite {
        return Err("Replacing this pack requires overwrite confirmation".into());
    }
    check_collisions(ctx, &pending.pack, true)?;
    if existing_bytes(&pending.path)? != pending.previous {
        return Err("Installed pack changed during review; choose the ZIP again".into());
    }
    let directory = pending.path.parent().ok_or("Missing pack directory")?;
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    if pending.previous.is_none()
        && std::fs::read_dir(directory)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "zip"))
            .count()
            >= MAX_PACKS
    {
        return Err("Too many theme packs (maximum 32)".into());
    }
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
    file.write_all(&pending.bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    if pending.previous.is_some() {
        file.persist(&pending.path).map_err(|e| e.to_string())?;
    } else {
        file.persist_noclobber(&pending.path)
            .map_err(|e| e.to_string())?;
    }
    register(ctx, &pending.pack, &pending.path, &pending.bytes);
    Ok(())
}
pub fn remove(ctx: &egui::Context, pack: &Installed) -> Result<(), String> {
    if read(&pack.path)?.as_slice() != pack.bytes.as_ref() {
        return Err("Installed pack changed; restart before removing it".into());
    }
    std::fs::remove_file(&pack.path).map_err(|e| e.to_string())?;
    let mut packs = installed(ctx);
    packs.retain(|p| p.id != pack.id);
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("theme-packs"), packs));
    Ok(())
}
#[cfg(test)]
pub fn install(ctx: &egui::Context, directory: &Path, source: &Path) -> Result<(), String> {
    commit(ctx, prepare(ctx, directory, source)?, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    const EXAMPLES: [(&str, &[u8], &str); 4] = [
        (
            "tokyo-night",
            include_bytes!("../../../docs/theme-packs/tokyo-night.zip"),
            include_str!("../../../docs/theme-packs/tokyo-night/manifest.json"),
        ),
        (
            "catppuccin",
            include_bytes!("../../../docs/theme-packs/catppuccin.zip"),
            include_str!("../../../docs/theme-packs/catppuccin/manifest.json"),
        ),
        (
            "detroit-become-human",
            include_bytes!("../../../docs/theme-packs/detroit-become-human.zip"),
            include_str!("../../../docs/theme-packs/detroit-become-human/manifest.json"),
        ),
        (
            "minecraft",
            include_bytes!("../../../docs/theme-packs/minecraft.zip"),
            include_str!("../../../docs/theme-packs/minecraft/manifest.json"),
        ),
    ];
    fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, bytes) in files {
            zip.start_file(
                *name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }
    fn modified(change: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
        let mut manifest: serde_json::Value = serde_json::from_str(EXAMPLES[0].2).unwrap();
        change(&mut manifest);
        zip(&[("manifest.json", &serde_json::to_vec(&manifest).unwrap())])
    }
    #[test]
    fn examples_install_restart_preserve_bytes_and_have_legible_text() {
        fn luminance(c: egui::Color32) -> f64 {
            let channel = |v: u8| {
                let v = v as f64 / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
        }
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir().unwrap();
        let packs = dir.path().join("packs");
        for (id, bytes, source) in EXAMPLES {
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
            let mut text = String::new();
            archive
                .by_name("manifest.json")
                .unwrap()
                .read_to_string(&mut text)
                .unwrap();
            assert_eq!(text, source);
            let path = dir.path().join("source.zip");
            std::fs::write(&path, bytes).unwrap();
            install(&ctx, &packs, &path).unwrap();
            assert_eq!(
                std::fs::read(packs.join(format!("{id}.zip"))).unwrap(),
                bytes
            );
            let pack = parse(bytes).unwrap();
            for t in pack.manifest.themes {
                let choice = Theme::Pack(format!("{id}/{}", t.id));
                choice.select(&ctx);
                assert_eq!(Theme::current(&ctx), choice);
                assert_eq!(choice.display_name(&ctx), t.name);
                assert_eq!(ctx.global_style().visuals.dark_mode, !t.light);
                let p = choice.resolved_palette(&ctx);
                for bg in [p.bg, p.panel, p.field, p.purple, p.pattern[0], p.pattern[1]] {
                    let (text, bg) = (luminance(p.text), luminance(bg));
                    assert!(
                        (text.max(bg) + 0.05) / (text.min(bg) + 0.05) >= 4.5,
                        "{id} text contrast"
                    );
                }
            }
        }
        assert_eq!(choices(&ctx).len(), 4);
        let restarted = egui::Context::default();
        assert!(load(&restarted, &packs).is_empty());
        assert_eq!(choices(&restarted), choices(&ctx));
        let isolated = egui::Context::default();
        assert!(choices(&isolated).is_empty());
        let missing = Theme::Pack("minecraft/overworld".into());
        missing.apply(&isolated);
        assert_eq!(Theme::current(&isolated), missing);
        assert_eq!(
            missing.resolved_palette(&isolated).bg,
            Theme::Midra.palette().bg
        );
        assert!(missing.display_name(&isolated).contains("unavailable"));
        Theme::HighContrast.select(&ctx);
        Theme::HighContrast.select(&ctx);
        assert_eq!(Theme::current(&ctx), missing);
    }
    #[test]
    fn complete_collection_imports_every_theme_and_survives_restart() {
        let bytes = include_bytes!("../../../docs/theme-packs/extra-themes.zip");
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("extra-themes.zip");
        let packs = dir.path().join("installed");
        std::fs::write(&source, bytes).unwrap();
        let ctx = egui::Context::default();
        install(&ctx, &packs, &source).unwrap();
        assert_eq!(installed(&ctx).len(), 1);
        assert_eq!(choices(&ctx).len(), 16);
        assert_eq!(
            std::fs::read(packs.join("extra-themes.zip")).unwrap(),
            bytes
        );
        let restarted = egui::Context::default();
        assert!(load(&restarted, &packs).is_empty());
        assert_eq!(choices(&ctx), choices(&restarted));
        let luminance = |c: egui::Color32| {
            let rgb = [c.r(), c.g(), c.b()].map(|v| {
                let v = f64::from(v) / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            });
            rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722
        };
        for theme in parse(bytes).unwrap().manifest.themes {
            let choice = Theme::Pack(format!("extra-themes/{}", theme.id));
            choice.select(&restarted);
            assert_eq!(Theme::current(&restarted), choice);
            assert_eq!(choice.display_name(&restarted), theme.name);
            let p = choice.resolved_palette(&restarted);
            for bg in [p.bg, p.panel, p.field, p.purple, p.pattern[0], p.pattern[1]] {
                let (fg, bg) = (luminance(p.text), luminance(bg));
                assert!(
                    (fg.max(bg) + 0.05) / (fg.min(bg) + 0.05) >= 4.5,
                    "{} contrast",
                    theme.name
                );
            }
        }
    }
    #[test]
    fn updates_require_review_and_detect_disk_changes_before_commit_or_remove() {
        let dir = tempfile::tempdir().unwrap();
        let packs = dir.path().join("packs");
        let source = dir.path().join("source.zip");
        let ctx = egui::Context::default();
        std::fs::write(&source, EXAMPLES[0].1).unwrap();
        install(&ctx, &packs, &source).unwrap();
        let original = installed(&ctx)[0].clone();
        let pending = prepare(&ctx, &packs, &source).unwrap();
        assert!(pending.needs_confirmation());
        assert!(commit(&ctx, pending, false).is_err());
        let updated = modified(|m| {
            m["revision"] = 2.into();
            m["themes"][0]["colors"]["bg"] = "#112233".into();
        });
        std::fs::write(&source, &updated).unwrap();
        let pending = prepare(&ctx, &packs, &source).unwrap();
        assert!(!pending.needs_confirmation());
        commit(&ctx, pending, false).unwrap();
        assert_eq!(installed(&ctx)[0].revision, 2);
        assert!(remove(&ctx, &original).is_err());
        std::fs::write(&source, EXAMPLES[0].1).unwrap();
        let pending = prepare(&ctx, &packs, &source).unwrap();
        assert!(pending.needs_confirmation());
        let destination = packs.join("tokyo-night.zip");
        std::fs::write(&destination, b"external change").unwrap();
        assert!(commit(&ctx, pending, true).is_err());
        assert_eq!(installed(&ctx)[0].revision, 2);
        assert!(remove(&ctx, &installed(&ctx)[0]).is_err());
        std::fs::write(&destination, &updated).unwrap();
        commit(&ctx, prepare(&ctx, &packs, &source).unwrap(), true).unwrap();
        assert_eq!(installed(&ctx)[0].revision, 1);
        remove(&ctx, &installed(&ctx)[0]).unwrap();
        assert!(installed(&ctx).is_empty());
        assert!(source.exists());
        assert!(!destination.exists());
    }
    #[test]
    fn invalid_archives_never_register_or_replace_installed_data() {
        let mut cases = vec![
            b"not zip".to_vec(),
            vec![0; MAX_ARCHIVE as usize + 1],
            zip(&[]),
            zip(&[("../manifest.json", EXAMPLES[0].2.as_bytes())]),
            zip(&[
                ("manifest.json", EXAMPLES[0].2.as_bytes()),
                ("extra.json", b"{}"),
            ]),
            zip(&[("manifest.json", &vec![b' '; MAX_EXPANDED as usize + 1])]),
        ];
        for (pointer, value) in [
            ("/version", serde_json::json!(2)),
            ("/revision", serde_json::json!(0)),
            ("/id", serde_json::json!("../escape")),
            ("/themes/0/id", serde_json::json!("bad/id")),
            ("/themes/0/name", serde_json::json!("\n")),
            ("/themes/0/colors/text", serde_json::json!("#ffffff00")),
            ("/themes/0/colors/bg", serde_json::json!("#gggggg")),
            ("/themes/0/light", serde_json::json!("dark")),
            ("/themes", serde_json::json!([])),
        ] {
            cases.push(modified(|m| *m.pointer_mut(pointer).unwrap() = value));
        }
        cases.push(modified(|m| {
            m["unknown"] = true.into();
        }));
        cases.push(modified(|m| {
            let theme = m["themes"][0].clone();
            m["themes"].as_array_mut().unwrap().push(theme);
        }));
        cases.push(modified(|m| {
            m["themes"][0]["colors"]
                .as_object_mut()
                .unwrap()
                .remove("line");
        }));
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.zip");
        let packs = dir.path().join("packs");
        let ctx = egui::Context::default();
        std::fs::write(&source, EXAMPLES[0].1).unwrap();
        install(&ctx, &packs, &source).unwrap();
        for bytes in cases {
            std::fs::write(&source, bytes).unwrap();
            assert!(prepare(&ctx, &packs, &source).is_err());
            assert_eq!(installed(&ctx).len(), 1);
            assert_eq!(
                std::fs::read(packs.join("tokyo-night.zip")).unwrap(),
                EXAMPLES[0].1
            );
        }
        std::fs::write(packs.join("bad.zip"), b"bad").unwrap();
        std::fs::write(packs.join("duplicate.zip"), EXAMPLES[0].1).unwrap();
        assert_eq!(load(&ctx, &packs).len(), 2);
        assert_eq!(installed(&ctx).len(), 1);
        for i in 0..32 {
            std::fs::write(packs.join(format!("invalid-{i}.zip")), b"bad").unwrap();
        }
        assert!(load(&ctx, &packs).iter().any(|e| e.contains("maximum 32")));
    }
}
