//! Bounded local diagnostics for transfer approximations; never a condition for sending.
use crate::device_send::Plan;
use serde_json::json;
use std::{io::Write, path::Path, time::SystemTime};

const LIMIT: u64 = 1024 * 1024;

pub(crate) fn record(plan: &Plan, success: bool) {
    if plan.discrepancies.is_empty() {
        return;
    }
    let Some(dirs) = directories::ProjectDirs::from("org", "beffroi", "midra-layout") else {
        return;
    };
    let entry = json!({
        "version": 1,
        "unix_seconds": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs(),
        "application_version": env!("CARGO_PKG_VERSION"),
        "profile": format!("{:?}", plan.profile),
        "screen": plan.screen, "bank": plan.bank, "target": format!("{:?}", plan.target),
        "verified": success, "discrepancies": plan.discrepancies,
    });
    if let Err(error) = append(&dirs.config_dir().join("logs"), &entry) {
        eprintln!("Cannot save transfer discrepancy log: {error}");
    }
}
fn append(directory: &Path, entry: &serde_json::Value) -> std::io::Result<()> {
    std::fs::create_dir_all(directory)?;
    let path = directory.join("device-transfers.jsonl");
    let mut bytes = serde_json::to_vec(entry)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > LIMIT {
        return Err(std::io::Error::other("Transfer log entry too large"));
    }
    if std::fs::metadata(&path).is_ok_and(|m| m.len() + bytes.len() as u64 > LIMIT) {
        let previous = directory.join("device-transfers.previous.jsonl");
        match std::fs::remove_file(&previous) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        std::fs::rename(&path, previous)?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(&bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn documented_log_example_has_supported_units_and_version() {
        let entry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/interfaces/device-transfer-log-v1.example.jsonl"
        ))
        .unwrap();
        assert_eq!(entry["version"], 1);
        let discrepancy = &entry["discrepancies"][0];
        assert_eq!(discrepancy["field"], "mask_bounds");
        assert_eq!(discrepancy["actual"].as_array().unwrap().len(), 4);
        assert!(
            (discrepancy["actual"][0].as_f64().unwrap()
                - discrepancy["requested"][0].as_f64().unwrap())
            .abs()
                > 1.
        );
    }
    #[test]
    fn logs_are_json_lines_and_rotate_to_one_previous_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("device-transfers.jsonl");
        append(dir.path(), &json!({"version":1,"requested": [10.25,20.0]})).unwrap();
        let line = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&line).unwrap()["version"],
            1
        );
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(LIMIT)
            .unwrap();
        append(dir.path(), &json!({"version":1})).unwrap();
        assert_eq!(
            std::fs::metadata(dir.path().join("device-transfers.previous.jsonl"))
                .unwrap()
                .len(),
            LIMIT
        );
        assert!(std::fs::metadata(&path).unwrap().len() < LIMIT);
        assert!(append(dir.path(), &json!("x".repeat(LIMIT as usize))).is_err());
    }
}
