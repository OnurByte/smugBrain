//! Lifecycle hooks (Claude Code): prompt, pre-edit, stop, session-end, pre-compact. Silent unless useful, never fatal.
//! Every hook keeps its state per session id, so two agents on one vault cannot overwrite each other.
use crate::{learn, mem, session, store::State, util::*};
use serde_json::{json, Value};
use std::{fs, io::Read, path::Path};

fn flatten(v: &Value, out: &mut String) {
    match v {
        Value::String(s) => {
            out.push_str(s);
            out.push('\n');
        }
        Value::Array(a) => a.iter().for_each(|x| flatten(x, out)),
        Value::Object(o) => o.values().for_each(|x| flatten(x, out)),
        _ => {}
    }
}

fn norm(p: &str) -> String {
    p.replace('\\', "/").trim_start_matches("./").to_string()
}

/// Active decisions and failures that name this file (frontmatter `files:` or the file name in their text).
pub fn guard_for_file(notes: &[mem::Note], file: &str) -> Vec<String> {
    let f = norm(file);
    let base = f.rsplit('/').next().unwrap_or("").to_string();
    let mut out = vec![];
    for n in notes {
        let ty = n.fm.get("type").map(String::as_str).unwrap_or("");
        if !matches!(ty, "decision" | "failure") || n.fm.get("status").map(String::as_str) == Some("superseded") {
            continue;
        }
        let listed = list(n.fm.get("files").map(String::as_str).unwrap_or("")).iter().any(|e| {
            let e = norm(e);
            !e.is_empty() && (f == e || f.ends_with(&format!("/{e}")) || e.ends_with(&format!("/{f}")))
        });
        let mentioned = base.chars().count() >= 5 && (n.body.contains(&base) || n.fm.get("title").is_some_and(|t| t.contains(&base)));
        if listed || mentioned {
            let g = |k: &str| n.fm.get(k).map(String::as_str).unwrap_or("");
            let id = g("id");
            let (status, why) = mem::check_status(notes, id);
            out.push(if ty == "failure" {
                format!("- FAILED BEFORE [{id}] {}: {} (verified: {}{}{}) [{status}]", g("method"), g("reason"), if g("verified").is_empty() { "no" } else { g("verified") }, if g("commit").is_empty() { String::new() } else { format!(", commit {}", g("commit")) }, if g("test_result").is_empty() { String::new() } else { format!(", test: {}", g("test_result")) })
            } else {
                format!("- DECISION [{id}] {} [{status}: {why}]", g("title"))
            });
        }
    }
    out
}

pub fn hook(root: &Path, a: &[String]) -> R {
    let mut s = String::new();
    let _ = std::io::stdin().read_to_string(&mut s);
    let input: Value = serde_json::from_str(&s).unwrap_or_default();
    let sdir = session::dir(root, input["session_id"].as_str());
    match a.first().map(String::as_str) {
        Some("prompt") => {
            let prompt = input["prompt"].as_str().unwrap_or("").to_string();
            if prompt.trim().is_empty() {
                return Ok(0);
            }
            for w in mem::warn_texts(&mem::load_notes(root), &prompt) {
                println!("{w}");
            }
            if let Ok(st) = State::open(root) {
                if let Ok(h) = mem::search_valid(root, &st, &prompt, 3, false) {
                    let paths: Vec<String> = h.iter().map(|x| x.path.clone()).collect();
                    session::set_surfaced(&sdir, &paths);
                    paths.iter().for_each(|p| session::receipt(&sdir, "surfaced", p));
                    if !h.is_empty() {
                        println!("smug: note candidates (open before relying on them):");
                        h.iter().for_each(|x| println!("- {}", x.path));
                    }
                }
            }
            Ok(0)
        }
        Some("pre-edit") => {
            let tool = &input["tool_input"];
            let Some(file) = tool["file_path"].as_str().or(tool["path"].as_str()) else { return Ok(0) };
            let cwd = std::env::current_dir().ok();
            let rel = cwd.as_ref().and_then(|c| Path::new(file).strip_prefix(c).ok().map(|p| p.to_string_lossy().to_string())).unwrap_or_else(|| file.to_string());
            let found = guard_for_file(&mem::load_notes(root), &rel);
            if !found.is_empty() {
                session::receipt(&sdir, "edit-guard", &rel);
                let msg = format!("smug memory for {rel} (check before editing; the code may have changed):\n{}", found.join("\n"));
                println!("{}", json!({"hookSpecificOutput": {"hookEventName": "PreToolUse", "additionalContext": msg}}));
            }
            Ok(0)
        }
        Some("stop") => {
            let surfaced = session::surfaced(&sdir);
            let Some(tp) = input["transcript_path"].as_str() else { return Ok(0) };
            let Ok(tr) = fs::read_to_string(tp) else { return Ok(0) };
            // only what the assistant wrote or did; the hook's own injected candidate list is not evidence
            let mut said = String::new();
            for l in tr.lines().rev().take(400) {
                if let Ok(v) = serde_json::from_str::<Value>(l) {
                    if v["type"] == "assistant" || v["message"]["role"] == "assistant" {
                        flatten(&v["message"], &mut said);
                    }
                }
            }
            if let Ok(st) = State::open(root) {
                for p in surfaced.iter().filter(|p| !p.is_empty()) {
                    let base = Path::new(p).file_name().map(|b| b.to_string_lossy().to_string()).unwrap_or_default();
                    if !base.is_empty() && said.contains(&base) {
                        let _ = st.append("feedback", p, "used", None);
                        let _ = st.add_feedback(p, 1);
                        session::receipt(&sdir, "used", p);
                    }
                }
            }
            session::clear_surfaced(&sdir);
            Ok(0)
        }
        Some("session-end") | Some("pre-compact") => {
            let Some(tp) = input["transcript_path"].as_str() else { return Ok(0) };
            let Ok(tr) = fs::read_to_string(tp) else { return Ok(0) };
            let text = learn::transcript_text(&tr);
            if text.trim().is_empty() {
                return Ok(0);
            }
            let key = sdir.file_name().map(|k| k.to_string_lossy().to_string()).unwrap_or_else(|| "default".into());
            if let Ok(st) = State::open(root) {
                if let Ok(rel) = learn::capture_text(&st, &key, &text) {
                    session::receipt(&sdir, "captured", &rel);
                }
            }
            Ok(0)
        }
        _ => Err("usage: smug hook prompt|pre-edit|stop|session-end|pre-compact".into()),
    }
}

pub fn receipts(root: &Path) -> R {
    let all = session::summary(root);
    if all.is_empty() {
        println!("no sessions recorded yet");
    }
    for (k, m) in all {
        println!("session {k}: {}", m.iter().map(|(a, b)| format!("{a} {b}")).collect::<Vec<_>>().join(", "));
    }
    Ok(0)
}
