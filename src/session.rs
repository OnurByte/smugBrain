//! Per-session state under .smug/sessions/<hash>/ so concurrent agent sessions never overwrite each other.
use crate::util::*;
use serde_json::json;
use std::{fs, io::Write, path::{Path, PathBuf}};

pub fn dir(root: &Path, session_id: Option<&str>) -> PathBuf {
    let key = match session_id {
        Some(s) if !s.is_empty() => sha(s.as_bytes())[..16].to_string(),
        _ => "default".into(),
    };
    root.join(".smug/sessions").join(key)
}

pub fn set_surfaced(d: &Path, paths: &[String]) {
    let _ = fs::create_dir_all(d);
    let _ = fs::write(d.join("surfaced.json"), serde_json::to_string(paths).unwrap_or_default());
}

pub fn surfaced(d: &Path) -> Vec<String> {
    fs::read_to_string(d.join("surfaced.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn clear_surfaced(d: &Path) {
    let _ = fs::remove_file(d.join("surfaced.json"));
}

/// Appends one receipt: what happened in this session (surfaced, used, edit-guard, captured).
pub fn receipt(d: &Path, kind: &str, path: &str) {
    let _ = fs::create_dir_all(d);
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(d.join("receipts.jsonl")) {
        let _ = writeln!(f, "{}", json!({"ts": now(), "kind": kind, "path": path}));
    }
}

/// (session key, receipts per kind)
pub fn summary(root: &Path) -> Vec<(String, std::collections::BTreeMap<String, usize>)> {
    let mut out = vec![];
    let Ok(rd) = fs::read_dir(root.join(".smug/sessions")) else { return out };
    for e in rd.flatten() {
        let mut m = std::collections::BTreeMap::new();
        for l in fs::read_to_string(e.path().join("receipts.jsonl")).unwrap_or_default().lines() {
            if let Some(k) = serde_json::from_str::<serde_json::Value>(l).ok().and_then(|v| v["kind"].as_str().map(String::from)) {
                *m.entry(k).or_insert(0) += 1;
            }
        }
        out.push((e.file_name().to_string_lossy().to_string(), m));
    }
    out.sort();
    out
}
