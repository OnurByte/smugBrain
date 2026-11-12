//! Learning loop: capture a session, propose candidate memories, review, promote. Nothing an agent says becomes a
//! decision on its own: candidates are inert until a human runs `promote`, which needs a source and refuses conflicts.
use crate::{
    mem::{fm_note, load_notes, opt, stdin_text, valid_id, Note},
    model,
    store::State,
    util::*,
    write::write_note,
};
use serde_json::Value;
use std::{fs, path::Path};

pub const CAND_DIR: &str = "500-knowledge/candidates";

fn one(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
}

/// Writes an inert candidate. `fields` are extra frontmatter (kind-specific).
pub fn write_candidate(st: &State, kind: &str, id: &str, fields: &[(&str, String)], body: &str) -> Res<Result<String, String>> {
    valid_id(id)?;
    if !matches!(kind, "decision" | "failure") {
        return Err("kind must be decision or failure".into());
    }
    let mut extra: Vec<(&str, String)> = vec![("kind", kind.into()), ("status", "candidate".into()), ("proposed_at", today())];
    extra.extend(fields.iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| (*k, v.clone())));
    write_note(st, Path::new(&format!("{CAND_DIR}/{id}.md")), fm_note("candidate", id, &extra, body).as_bytes(), Some("new"))
}

fn candidates(notes: &[Note]) -> Vec<&Note> {
    notes.iter().filter(|n| n.fm.get("type").map(String::as_str) == Some("candidate")).collect()
}

fn g<'a>(n: &'a Note, k: &str) -> &'a str {
    n.fm.get(k).map(String::as_str).unwrap_or("")
}

pub fn propose(root: &Path, a: &[String]) -> R {
    let id = a.first().ok_or("usage: smug propose <id> --kind decision|failure [--source s] [--topic t] [--method m] [--reason r] [--files a,b] < text")?;
    let kind = opt(a, "--kind").ok_or("--kind decision|failure required")?;
    let st = State::open(root)?;
    let fields = [
        ("topic", opt(a, "--topic").unwrap_or_default()),
        ("source", opt(a, "--source").unwrap_or_default()),
        ("method", opt(a, "--method").unwrap_or_default()),
        ("reason", opt(a, "--reason").unwrap_or_default()),
        ("files", opt(a, "--files").map(|f| format!("[{f}]")).unwrap_or_default()),
    ];
    match write_candidate(&st, &kind, id, &fields, &stdin_text())? {
        Ok(_) => {
            println!("{CAND_DIR}/{id}.md  (candidate: inert until `smug promote {id}`)");
            Ok(0)
        }
        Err(m) => {
            eprintln!("smug: {m}");
            Ok(2)
        }
    }
}

pub fn review(root: &Path) -> R {
    let notes = load_notes(root);
    let cs = candidates(&notes);
    let open: Vec<&&Note> = cs.iter().filter(|n| g(n, "status") == "candidate").collect();
    if open.is_empty() {
        println!("no candidates waiting for review");
    }
    for n in open {
        let id = g(n, "id");
        let mut flags = vec![];
        if g(n, "kind") == "decision" && g(n, "source").is_empty() {
            flags.push("NO SOURCE".to_string());
        }
        let topic = g(n, "topic");
        if !topic.is_empty() {
            if let Some(o) = notes.iter().find(|o| o.fm.get("type").map(String::as_str) == Some("decision") && g(o, "topic") == topic && g(o, "status") != "superseded") {
                flags.push(format!("CONFLICTS with {}", g(o, "id")));
            }
        }
        println!("{id}  [{}]  {}{}", g(n, "kind"), n.body.lines().next().unwrap_or(""), if flags.is_empty() { String::new() } else { format!("   !! {}", flags.join(", ")) });
    }
    Ok(0)
}

fn set_status(st: &State, rel: &str, status: &str, extra: &[(&str, String)]) -> Res<()> {
    let text = fs::read_to_string(st.root.join(rel))?;
    let mut t = set_fm(&text, "status", status);
    for (k, v) in extra {
        t = set_fm(&t, k, &one(v));
    }
    write_note(st, Path::new(rel), t.as_bytes(), Some(&sha(text.as_bytes())))??;
    Ok(())
}

/// Human step. Not exposed over MCP.
pub fn promote(root: &Path, a: &[String]) -> R {
    let id = a.first().ok_or("usage: smug promote <id> [--source s] [--allow-conflict]")?;
    valid_id(id)?;
    let st = State::open(root)?;
    let notes = load_notes(root);
    let n = candidates(&notes).into_iter().find(|n| g(n, "id") == id && g(n, "status") == "candidate").ok_or("no open candidate with that id")?;
    let kind = g(n, "kind").to_string();
    let source = opt(a, "--source").unwrap_or_else(|| g(n, "source").to_string());
    let topic = g(n, "topic").to_string();
    let (rel, text) = if kind == "decision" {
        if source.is_empty() {
            return Err("refusing: a decision without a source is UNSUPPORTED. Pass --source.".into());
        }
        if let Some(o) = notes.iter().find(|o| o.fm.get("type").map(String::as_str) == Some("decision") && !topic.is_empty() && g(o, "topic") == topic && g(o, "status") != "superseded") {
            if !a.iter().any(|x| x == "--allow-conflict") {
                return Err(format!("refusing: '{}' is already the active decision for topic '{topic}'. Replace it with `smug patch {} --new-id {id} --source ...`.", g(o, "id"), g(o, "id")).into());
            }
        }
        let extra = [
            ("title", id.to_string()),
            ("status", "active".into()),
            ("topic", topic),
            ("source", source),
            ("valid_from", today()),
            ("known_at", today()),
            ("depends_on", "[]".into()),
            ("files", g(n, "files").to_string()),
            ("promoted_from", format!("{CAND_DIR}/{id}.md")),
        ];
        (format!("500-knowledge/decisions/{id}.md"), fm_note("decision", id, &extra, &n.body))
    } else {
        let extra = [
            ("title", g(n, "method").to_string()),
            ("method", g(n, "method").to_string()),
            ("reason", g(n, "reason").to_string()),
            ("source", source),
            ("files", g(n, "files").to_string()),
            ("verified", "no".into()),
            ("status", "active".into()),
            ("valid_from", today()),
            ("known_at", today()),
            ("promoted_from", format!("{CAND_DIR}/{id}.md")),
        ];
        (format!("500-knowledge/failures/{id}.md"), fm_note("failure", id, &extra, &n.body))
    };
    match write_note(&st, Path::new(&rel), text.as_bytes(), Some("new"))? {
        Ok(_) => {}
        Err(m) => return Err(format!("{m} (a {kind} with that id already exists)").into()),
    }
    set_status(&st, &n.rel, "promoted", &[("promoted_to", rel.clone())])?;
    st.append("promote", &rel, "", None)?;
    println!("promoted {id} -> {rel}");
    Ok(0)
}

pub fn reject(root: &Path, a: &[String]) -> R {
    let id = a.first().ok_or("usage: smug reject <id> [--reason r]")?;
    valid_id(id)?;
    let st = State::open(root)?;
    let notes = load_notes(root);
    let n = candidates(&notes).into_iter().find(|n| g(n, "id") == id && g(n, "status") == "candidate").ok_or("no open candidate with that id")?;
    set_status(&st, &n.rel, "rejected", &[("rejected_reason", opt(a, "--reason").unwrap_or_default())])?;
    println!("rejected {id}");
    Ok(0)
}

// ---------- capture ----------
fn redact(text: &str) -> String {
    let pats = patterns();
    text.lines().map(|l| if hits(&pats, l.as_bytes()).is_empty() { l.to_string() } else { "[redacted: secret-shaped line]".to_string() }).collect::<Vec<_>>().join("\n")
}

fn content_text(c: &Value) -> String {
    match c {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().filter_map(|b| if b["type"] == "text" { b["text"].as_str().map(String::from) } else { None }).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

/// Claude Code transcript JSONL -> "user: ..." / "assistant: ..." text, last 20k chars, secret-shaped lines redacted.
pub fn transcript_text(tr: &str) -> String {
    let mut out = String::new();
    for l in tr.lines() {
        let Ok(v) = serde_json::from_str::<Value>(l) else { continue };
        let role = v["message"]["role"].as_str().or(v["type"].as_str()).unwrap_or("");
        if role == "user" || role == "assistant" {
            let t = content_text(&v["message"]["content"]);
            if !t.trim().is_empty() {
                out.push_str(&format!("{role}: {}\n\n", t.trim()));
            }
        }
    }
    let skip = out.chars().count().saturating_sub(20000);
    redact(&out.chars().skip(skip).collect::<String>())
}

pub fn capture_text(st: &State, label: &str, text: &str) -> Res<String> {
    let rel = format!("000-inbox/captures/{}-{}.md", today(), label);
    let path = st.root.join(&rel);
    let cur = fs::read(&path).ok().map(|d| sha(&d));
    let body = format!("---\ntype: capture\nstatus: raw\ncaptured_at: {}\n---\n\n{}\n", today(), redact(text));
    write_note(st, Path::new(&rel), body.as_bytes(), Some(cur.as_deref().unwrap_or("new")))??;
    Ok(rel)
}

pub fn capture(root: &Path, a: &[String]) -> R {
    let label = opt(a, "--label").unwrap_or_else(|| "manual".into());
    valid_id(&label)?;
    let st = State::open(root)?;
    let text = match opt(a, "--transcript") {
        Some(p) => transcript_text(&fs::read_to_string(p)?),
        None => stdin_text(),
    };
    if text.trim().is_empty() {
        return Err("nothing to capture".into());
    }
    println!("{}", capture_text(&st, &label, &text)?);
    Ok(0)
}

// ---------- learn ----------
const LEARN_PROMPT: &str = "You extract durable project knowledge from a coding-session transcript.\n\
Output ONLY JSON lines, one object per line, nothing else:\n\
{\"kind\":\"decision\"|\"failure\",\"id\":\"kebab-case-id\",\"text\":\"one or two sentences\",\"source\":\"where this is shown\",\"topic\":\"short topic\",\"method\":\"what was tried (failure only)\",\"reason\":\"why it failed (failure only)\",\"files\":[\"paths\"]}\n\
Include only what the transcript clearly shows. If there is nothing durable, output nothing.\n\nTRANSCRIPT:\n";

pub fn learn(root: &Path, a: &[String]) -> R {
    let file = a.iter().find(|x| !x.starts_with("--") && Some((*x).clone()) != opt(a, "--model") && Some((*x).clone()) != opt(a, "--timeout")).ok_or("usage: smug learn <capture.md> --model <cmd> --propose")?;
    let model_cmd = opt(a, "--model").ok_or("--model <command> is required: smug does not bundle a model")?;
    if !a.iter().any(|x| x == "--propose") {
        return Err("learn only proposes (--propose): candidates stay inert until you promote them".into());
    }
    if let Some(t) = opt(a, "--timeout") {
        std::env::set_var("SMUG_MODEL_TIMEOUT", t);
    }
    let rel = safe_rel(file)?;
    let text = fs::read_to_string(root.join(&rel))?;
    let st = State::open(root)?;
    let out = model::ask(&model_cmd, &format!("{LEARN_PROMPT}{text}"))?;
    let (mut made, mut skipped) = (0, 0);
    for l in out.lines().map(str::trim).filter(|l| l.starts_with('{')) {
        let Ok(v) = serde_json::from_str::<Value>(l) else { skipped += 1; continue };
        let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
        let files = v["files"].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(",")).unwrap_or_default();
        let fields = [
            ("topic", s("topic")),
            ("source", if s("source").is_empty() { format!("capture:{}", rel.display()) } else { s("source") }),
            ("method", s("method")),
            ("reason", s("reason")),
            ("files", if files.is_empty() { String::new() } else { format!("[{files}]") }),
        ];
        match write_candidate(&st, &s("kind"), &s("id"), &fields, &s("text")) {
            Ok(Ok(_)) => made += 1,
            _ => skipped += 1, // bad id, bad kind, duplicate, or secret-shaped: never fatal
        }
    }
    println!("{made} candidate(s) proposed, {skipped} skipped. Review with `smug review`.");
    Ok(0)
}
