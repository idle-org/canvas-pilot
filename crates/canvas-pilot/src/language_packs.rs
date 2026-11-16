//! Versioned, bounded runtime catalogs. Contract: docs/interfaces/language-packs.md.
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::Path,
    sync::{LazyLock, RwLock},
};

const MAX_ARCHIVE: u64 = 2 * 1024 * 1024;
const MAX_EXPANDED: u64 = 4 * 1024 * 1024;
const MAX_PACKS: usize = 32;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    id: String,
    languages: Vec<Description>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Description {
    pub locale: String,
    pub name: String,
    pub native_name: String,
}
#[derive(Clone)]
struct Pack {
    manifest: Manifest,
    catalogs: BTreeMap<String, BTreeMap<String, String>>,
}

// The backend stores catalogs, never the selected locale. Each context selects explicitly.
static CATALOGS: LazyLock<RwLock<BTreeMap<String, BTreeMap<String, String>>>> =
    LazyLock::new(Default::default);
pub struct Backend;
impl rust_i18n::Backend for Backend {
    fn available_locales(&self) -> Vec<Cow<'_, str>> {
        CATALOGS
            .read()
            .unwrap()
            .keys()
            .cloned()
            .map(Cow::Owned)
            .collect()
    }
    fn translate(&self, locale: &str, key: &str) -> Option<Cow<'_, str>> {
        CATALOGS
            .read()
            .unwrap()
            .get(locale)?
            .get(key)
            .cloned()
            .map(Cow::Owned)
    }
    fn messages_for_locale(&self, locale: &str) -> Option<Vec<(Cow<'_, str>, Cow<'_, str>)>> {
        Some(
            CATALOGS
                .read()
                .unwrap()
                .get(locale)?
                .iter()
                .map(|(k, v)| (Cow::Owned(k.clone()), Cow::Owned(v.clone())))
                .collect(),
        )
    }
}
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 63
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
}
pub fn available(ctx: &egui::Context) -> Vec<Description> {
    ctx.data(|data| data.get_temp::<Vec<Description>>(egui::Id::new("language-packs")))
        .unwrap_or_default()
}
fn register(ctx: &egui::Context, pack: &Pack) {
    let mut languages = available(ctx);
    let mut backend = CATALOGS.write().unwrap();
    for language in &pack.manifest.languages {
        backend.insert(
            format!("pack:{}", language.locale),
            pack.catalogs[&language.locale].clone(),
        );
        languages.push(language.clone());
    }
    languages.sort_by(|a, b| a.locale.cmp(&b.locale));
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("language-packs"), languages));
}
fn placeholders(text: &str) -> Result<BTreeSet<&str>, String> {
    text.split("%{")
        .skip(1)
        .map(|part| {
            part.split_once('}')
                .map(|(key, _)| key)
                .filter(|key| !key.is_empty())
                .ok_or_else(|| "Malformed placeholder".into())
        })
        .collect()
}
fn parse(bytes: &[u8]) -> Result<Pack, String> {
    if bytes.len() as u64 > MAX_ARCHIVE {
        return Err("Language pack exceeds 2 MiB".into());
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if zip.is_empty() || zip.len() > 17 {
        return Err("Expected manifest and at most 16 catalogs".into());
    }
    let mut entries = BTreeMap::new();
    let mut total = 0;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
        let name = entry.name().to_owned();
        if name.contains(['/', '\\'])
            || !name.ends_with(".json")
            || entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("Pack entries must be regular JSON files at the ZIP root".into());
        }
        let mut data = Vec::new();
        (&mut entry)
            .take(MAX_EXPANDED - total + 1)
            .read_to_end(&mut data)
            .map_err(|e| e.to_string())?;
        total += data.len() as u64;
        if total > MAX_EXPANDED {
            return Err("Expanded language pack exceeds 4 MiB".into());
        }
        if entries.insert(name, data).is_some() {
            return Err("Duplicate ZIP entry".into());
        }
    }
    let manifest: Manifest = serde_json::from_slice(
        &entries
            .remove("manifest.json")
            .ok_or("Missing manifest.json")?,
    )
    .map_err(|e| e.to_string())?;
    if manifest.version != 1
        || !valid_id(&manifest.id)
        || manifest.languages.is_empty()
        || manifest.languages.len() > 16
    {
        return Err("Invalid language-pack manifest or unsupported version".into());
    }
    let english: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../locales/en.json")).map_err(|e| e.to_string())?;
    let mut catalogs = BTreeMap::new();
    for language in &manifest.languages {
        if !valid_id(&language.locale)
            || language.locale == "system"
            || [
                "en", "fr", "ja", "zh-cn", "es", "grc", "eo", "pt-pt", "ne", "th", "hi", "tok",
            ]
            .contains(&language.locale.as_str())
        {
            return Err("Invalid or built-in locale identifier".into());
        }
        for name in [&language.name, &language.native_name] {
            if name.trim().is_empty() || name.len() > 160 || name.chars().any(char::is_control) {
                return Err("Invalid language display name".into());
            }
        }
        let messages: BTreeMap<String, String> = serde_json::from_slice(
            &entries
                .remove(&format!("{}.json", language.locale))
                .ok_or("Missing or repeated language catalog")?,
        )
        .map_err(|e| e.to_string())?;
        if messages.is_empty() {
            return Err("Empty language catalog".into());
        }
        for (key, value) in &messages {
            let source = english
                .get(key)
                .ok_or_else(|| format!("Unknown translation key: {key}"))?;
            if value.trim().is_empty()
                || value.len() > 16384
                || value
                    .chars()
                    .any(|c| c.is_control() && c != '\n' && c != '\t')
                || placeholders(source)? != placeholders(value)?
            {
                return Err(format!("Invalid text or mismatched placeholders: {key}"));
            }
        }
        catalogs.insert(language.locale.clone(), messages);
    }
    if !entries.is_empty() {
        return Err("Unexpected files in language pack".into());
    }
    Ok(Pack { manifest, catalogs })
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
fn check_collisions(ctx: &egui::Context, pack: &Pack) -> Result<(), String> {
    let existing = available(ctx);
    if pack
        .manifest
        .languages
        .iter()
        .any(|new| existing.iter().any(|old| old.locale == new.locale))
    {
        return Err("A language in this pack is already installed; remove its old ZIP and restart before replacing it".into());
    }
    Ok(())
}
/// Load once at application startup; invalid files do not prevent startup.
pub fn load(ctx: &egui::Context, directory: &Path) -> Vec<String> {
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("language-packs"), Vec::<Description>::new())
    });
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
            errors.push("Too many language packs (maximum 32)".into());
            break;
        }
    }
    paths.sort();
    for path in paths.into_iter().take(MAX_PACKS) {
        let result = read(&path)
            .and_then(|bytes| parse(&bytes))
            .and_then(|pack| {
                check_collisions(ctx, &pack)?;
                register(ctx, &pack);
                Ok(())
            });
        if let Err(error) = result {
            errors.push(format!("{}: {error}", path.display()));
        }
    }
    errors
}
/// Validate the entire pack, persist without replacing existing files, then register.
pub fn install(ctx: &egui::Context, directory: &Path, source: &Path) -> Result<(), String> {
    let bytes = read(source)?;
    let pack = parse(&bytes)?;
    check_collisions(ctx, &pack)?;
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    if std::fs::read_dir(directory)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "zip"))
        .count()
        >= MAX_PACKS
    {
        return Err("Too many language packs (maximum 32)".into());
    }
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist_noclobber(directory.join(format!("{}.zip", pack.manifest.id)))
        .map_err(|e| e.to_string())?;
    register(ctx, &pack);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::{Language, tr};
    const EXAMPLE: &[u8] = include_bytes!("../../../docs/language-packs/language-pack.zip");
    fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, data) in files {
            writer
                .start_file(
                    *name,
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    fn modified(manifest: serde_json::Value, messages: serde_json::Value) -> Vec<u8> {
        archive(&[
            ("manifest.json", &serde_json::to_vec(&manifest).unwrap()),
            ("uk.json", &serde_json::to_vec(&messages).unwrap()),
        ])
    }
    fn manifest() -> serde_json::Value {
        serde_json::json!({"version":1,"id":"test","languages":[{"locale":"uk","name":"Ukrainian","native_name":"Українська"}]})
    }
    #[test]
    fn example_matches_sources_installs_restarts_and_falls_back() {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(EXAMPLE)).unwrap();
        for (name, text) in [
            (
                "manifest.json",
                include_str!("../../../docs/language-packs/example/manifest.json"),
            ),
            (
                "uk.json",
                include_str!("../../../docs/language-packs/example/uk.json"),
            ),
            (
                "bho.json",
                include_str!("../../../docs/language-packs/example/bho.json"),
            ),
        ] {
            let mut actual = String::new();
            zip.by_name(name)
                .unwrap()
                .read_to_string(&mut actual)
                .unwrap();
            assert_eq!(actual, text);
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.zip");
        std::fs::write(&source, EXAMPLE).unwrap();
        let destination = dir.path().join("packs");
        let ctx = egui::Context::default();
        assert!(load(&ctx, &destination).is_empty());
        install(&ctx, &destination, &source).unwrap();
        assert_eq!(available(&ctx).len(), 2);
        Language::Pack("uk".into()).apply(&ctx);
        assert_eq!(tr(&ctx, "ui.settings"), "Налаштування");
        assert_eq!(
            rust_i18n::t!(
                "theme.current",
                locale = &crate::localization::locale(&ctx),
                name = "Test"
            ),
            "Поточна тема: Test"
        );
        assert_eq!(tr(&ctx, "language.name.en"), "English");
        let original = std::fs::read(destination.join("example-uk-bho.zip")).unwrap();
        assert!(install(&ctx, &destination, &source).is_err());
        assert_eq!(
            original,
            std::fs::read(destination.join("example-uk-bho.zip")).unwrap()
        );
        let restarted = egui::Context::default();
        assert!(load(&restarted, &destination).is_empty());
        Language::Pack("bho".into()).apply(&restarted);
        assert_eq!(tr(&restarted, "ui.close"), "बंद करीं");
        assert_eq!(tr(&ctx, "ui.close"), "Закрити");
        std::fs::remove_file(destination.join("example-uk-bho.zip")).unwrap();
        assert!(load(&restarted, &destination).is_empty());
        Language::Pack("bho".into()).apply(&restarted);
        assert_eq!(tr(&restarted, "ui.close"), "Close");
    }
    #[test]
    fn invalid_packs_never_install_and_startup_skips_bad_files() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.zip");
        let destination = dir.path().join("packs");
        let ctx = egui::Context::default();
        let mut cases = vec![
            b"not a ZIP".to_vec(),
            vec![0; MAX_ARCHIVE as usize + 1],
            archive(&[("../escape.json", b"{}")]),
            archive(&[("manifest.json", b"broken")]),
            archive(&[("other.json", b"{}")]),
            archive(&[("manifest.json", &vec![b'x'; MAX_EXPANDED as usize + 1])]),
        ];
        for messages in [
            serde_json::json!({}),
            serde_json::json!({"unknown.key":"bad"}),
            serde_json::json!({"ui.close":""}),
            serde_json::json!({"ui.close":4}),
            serde_json::json!({"ui.close":"%{extra}"}),
            serde_json::json!({"theme.current":"missing placeholder"}),
            serde_json::json!({"theme.current":"%{name"}),
            serde_json::json!({"ui.close":"bad\u{0000}"}),
        ] {
            cases.push(modified(manifest(), messages));
        }
        for (field, value) in [
            ("version", serde_json::json!(2)),
            ("id", serde_json::json!("../escape")),
            ("languages", serde_json::json!([])),
            ("extra", serde_json::json!(true)),
        ] {
            let mut m = manifest();
            m[field] = value;
            cases.push(modified(m, serde_json::json!({"ui.close":"Закрити"})));
        }
        for locale in ["en", "system", "uk_UA", "../uk", ""] {
            let mut m = manifest();
            m["languages"][0]["locale"] = locale.into();
            cases.push(modified(m, serde_json::json!({"ui.close":"Закрити"})));
        }
        let mut m = manifest();
        m["languages"][0]["native_name"] = "\n".into();
        cases.push(modified(m, serde_json::json!({"ui.close":"Закрити"})));
        cases.push(archive(&[
            ("manifest.json", &serde_json::to_vec(&manifest()).unwrap()),
            ("uk.json", b"{\"ui.close\":\"Close\"}"),
            ("unexpected.json", b"{}"),
        ]));
        for bytes in cases {
            std::fs::write(&source, bytes).unwrap();
            assert!(install(&ctx, &destination, &source).is_err());
            assert!(available(&ctx).is_empty());
            assert!(!destination.exists());
        }
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(destination.join("broken.zip"), b"bad").unwrap();
        std::fs::write(destination.join("good.zip"), EXAMPLE).unwrap();
        assert_eq!(load(&ctx, &destination).len(), 1);
        assert_eq!(available(&ctx).len(), 2);
        // A pre-existing target is never overwritten even when it was invalid at startup.
        std::fs::write(destination.join("example-uk-bho.zip"), b"preserve").unwrap();
        let fresh = egui::Context::default();
        std::fs::write(&source, EXAMPLE).unwrap();
        assert!(install(&fresh, &destination, &source).is_err());
        assert_eq!(
            std::fs::read(destination.join("example-uk-bho.zip")).unwrap(),
            b"preserve"
        );
    }
}
