//! Memory model: decisions with sources and validity, failures, impact, patches, behavioural eval, guard, time machine.
use crate::{index, store::State, util::*, write::write_note};
use serde_json::{json, Value};
use std::{collections::HashSet, fs, io::Read, path::Path, time::Instant};

pub struct Note {
    pub rel: String,
    pub fm: Fm,
    pub body: String,
}
impl Note {
    fn get(&self, k: &str) -> &str {
        self.fm.get(k).map(String::as_str).unwrap_or("")
    }
    pub fn key(&self) -> String {
        let id = self.get("id");
        if id.is_empty() { Path::new(&self.rel).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default() } else { id.to_string() }
    }
    fn status(&self) -> &str {
        let s = self.get("status");
        if s.is_empty() { "active" } else { s }
    }
}

pub fn load_notes(root: &Path) -> Vec<Note> {
    md_files(root)
        .into_iter()
        .filter_map(|f| {
            let t = fs::read_to_string(&f).ok()?;
            let (fm, body) = split_fm(&t);
            Some(Note { rel: rel(root, &f), fm, body: body.to_string() })
        })
        .collect()
}

pub fn opt(a: &[String], flag: &str) -> Option<String> {
    a.iter().position(|x| x == flag).and_then(|i| a.get(i + 1)).cloned()
}

fn stdin_text() -> String {
    let mut s = String::new();
    let _ = std::io::stdin().read_to_string(&mut s);
    s
}

fn one_line(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
}

fn valid_id(id: &str) -> Res<()> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(format!("bad id '{id}': use letters, digits, - and _").into());
    }
    Ok(())
}

fn fm_note(ty: &str, id: &str, extra: &[(&str, String)], body: &str) -> String {
    let mut s = format!("---\ntype: {ty}\nid: {id}\n");
    for (k, v) in extra {
        s += &format!("{k}: {}\n", one_line(v));
    }
    s += &format!("---\n\n{}\n", body.trim());
    s
}

fn created(st: &State, rel: &str, text: &str) -> R {
    match write_note(st, Path::new(rel), text.as_bytes(), Some("new"))? {
        Ok(h) => {
            println!("{rel}\n{h}");
            Ok(0)
        }
        Err(m) => {
            eprintln!("smug: {m}");
            Ok(2)
        }
    }
}

// ---------- decisions and failures ----------
pub fn decide(root: &Path, a: &[String]) -> R {
    let id = a.first().ok_or("usage: smug decide <id> --source <s> [--topic t] [--depends a,b] [--title t] [--date YYYY-MM-DD] [--known-at YYYY-MM-DD] < why")?;
    valid_id(id)?;
    let source = opt(a, "--source").ok_or("--source is required: a decision without a source is UNSUPPORTED")?;
    let d = opt(a, "--date").unwrap_or_else(today);
    let extra = [
        ("title", opt(a, "--title").unwrap_or_else(|| id.clone())),
        ("status", "active".into()),
        ("topic", opt(a, "--topic").unwrap_or_default()),
        ("source", source),
        ("valid_from", d.clone()),
        ("known_at", opt(a, "--known-at").unwrap_or_else(today)),
        ("depends_on", format!("[{}]", opt(a, "--depends").unwrap_or_default())),
    ];
    let st = State::open(root)?;
    created(&st, &format!("500-knowledge/decisions/{id}.md"), &fm_note("decision", id, &extra, &stdin_text()))
}

pub fn fail(root: &Path, a: &[String]) -> R {
    let id = a.first().ok_or("usage: smug fail <id> --method <m> --reason <r> [--version v] [--project p] [--source s]")?;
    valid_id(id)?;
    let extra = [
        ("title", opt(a, "--method").ok_or("--method required")?),
        ("method", opt(a, "--method").unwrap_or_default()),
        ("reason", opt(a, "--reason").ok_or("--reason required")?),
        ("version", opt(a, "--version").unwrap_or_default()),
        ("project", opt(a, "--project").unwrap_or_default()),
        ("source", opt(a, "--source").unwrap_or_default()),
        ("status", "active".into()),
        ("valid_from", today()),
        ("known_at", today()),
    ];
    let st = State::open(root)?;
    created(&st, &format!("500-knowledge/failures/{id}.md"), &fm_note("failure", id, &extra, &stdin_text()))
}

// ---------- guard ----------
pub fn check_status(notes: &[Note], id: &str) -> (&'static str, String) {
    let Some(n) = notes.iter().find(|n| n.get("id") == id) else { return ("UNKNOWN", "no note with that id".into()) };
    if n.status() == "superseded" {
        return ("STALE", format!("superseded by {} on {}", n.get("superseded_by"), n.get("valid_to")));
    }
    let topic = n.get("topic");
    if !topic.is_empty() {
        if let Some(o) = notes.iter().find(|o| o.get("id") != id && o.get("type") == n.get("type") && o.get("topic") == topic && o.status() == "active") {
            return ("CONFLICTED", format!("{} is also active for topic '{topic}'", o.get("id")));
        }
    }
    if n.get("source").is_empty() {
        return ("UNSUPPORTED", "no source".into());
    }
    ("CURRENT", format!("source: {}", n.get("source")))
}

pub fn check(root: &Path, a: &[String]) -> R {
    let id = a.first().ok_or("usage: smug check <id>")?;
    let (s, why) = check_status(&load_notes(root), id);
    println!("{s}  {id}  {why}");
    Ok(if s == "CURRENT" { 0 } else { 3 })
}

pub fn warn_texts(notes: &[Note], text: &str) -> Vec<String> {
    let toks = |s: &str| -> HashSet<String> { s.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| w.chars().count() > 3).map(String::from).collect() };
    let t = toks(text);
    notes
        .iter()
        .filter(|n| n.get("type") == "failure" && n.status() == "active")
        .filter_map(|n| {
            let m = toks(n.get("method"));
            let need = m.len().clamp(1, 2);
            (m.intersection(&t).count() >= need).then(|| {
                format!("WARN {}: '{}' failed before because {} (version {}, source {})", n.get("id"), n.get("method"), n.get("reason"), n.get("version"), n.get("source"))
            })
        })
        .collect()
}

pub fn warn(root: &Path, a: &[String]) -> R {
    let w = warn_texts(&load_notes(root), &a.join(" "));
    w.iter().for_each(|l| println!("{l}"));
    Ok(0)
}

// ---------- impact ----------
pub fn impact_of(notes: &[Note], id: &str) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = vec![];
    let mut seen: HashSet<String> = HashSet::from([id.to_string()]);
    let mut frontier = vec![id.to_string()];
    let mut depth = 0;
    while !frontier.is_empty() {
        depth += 1;
        let mut next = vec![];
        for cur in &frontier {
            for n in notes {
                let k = n.key();
                if seen.contains(&k) {
                    continue;
                }
                let dep = list(n.get("depends_on")).iter().any(|d| d == cur) || n.body.contains(&format!("[[{cur}]]")) || n.body.contains(&format!("[[{cur}|"));
                if dep {
                    seen.insert(k.clone());
                    out.push((n.rel.clone(), depth));
                    next.push(k);
                }
            }
        }
        frontier = next;
    }
    out
}

pub fn impact(root: &Path, a: &[String]) -> R {
    let id = a.first().ok_or("usage: smug impact <id>")?;
    let r = impact_of(&load_notes(root), id);
    if r.is_empty() {
        println!("nothing depends on {id}");
    }
    for (p, d) in r {
        println!("{}{p}", "  ".repeat(d - 1));
    }
    Ok(0)
}

// ---------- time machine ----------
pub fn at(root: &Path, a: &[String]) -> R {
    let date = a.first().ok_or("usage: smug at <YYYY-MM-DD> [words]")?;
    let words: Vec<String> = a[1..].iter().map(|w| w.to_lowercase()).collect();
    for n in load_notes(root).iter().filter(|n| matches!(n.get("type"), "decision" | "claim" | "failure")) {
        let from = n.get("valid_from");
        let to = n.get("valid_to");
        let known = if n.get("known_at").is_empty() { from } else { n.get("known_at") };
        let valid = from <= date.as_str() && (to.is_empty() || date.as_str() < to);
        let hay = format!("{} {}", n.get("title"), n.body).to_lowercase();
        if valid && known <= date.as_str() && (words.is_empty() || words.iter().any(|w| hay.contains(w))) {
            println!("{} [{}] {}  ({})", n.get("id"), n.get("type"), n.get("title"), n.rel);
        }
    }
    Ok(0)
}

// ---------- retrieval with validity filter ----------
fn is_superseded(root: &Path, r: &str) -> bool {
    fs::read_to_string(root.join(r)).map(|t| split_fm(&t).0.get("status").map(String::as_str) == Some("superseded")).unwrap_or(false)
}

/// Retrieval, then a hard filter: superseded notes never come back (unless `history`), feedback nudges the order.
pub fn search_valid(root: &Path, st: &State, q: &str, k: usize, history: bool) -> Res<Vec<index::Hit>> {
    let fb = st.feedback()?;
    let mut h = index::retrieve(root, st, q, k * 4 + 8)?;
    if !history {
        h.retain(|x| !is_superseded(root, &x.path));
    }
    for x in &mut h {
        x.score *= 1.0 + 0.05 * fb.get(&x.path).copied().unwrap_or(0).clamp(-5, 10) as f32;
    }
    h.sort_by(|a, b| b.score.total_cmp(&a.score));
    h.truncate(k);
    Ok(h)
}

pub fn feedback(root: &Path, a: &[String]) -> R {
    let p = a.first().ok_or("usage: smug feedback <note.md> used|useful|harmful")?;
    safe_rel(p)?;
    let (kind, delta) = match a.get(1).map(String::as_str) {
        Some("used") => ("used", 1),
        Some("useful") => ("useful", 2),
        Some("harmful") => ("harmful", -3),
        _ => return Err("kind must be used, useful or harmful".into()),
    };
    let st = State::open(root)?;
    st.append("feedback", p, kind, None)?;
    println!("{p}: utility {}", st.add_feedback(p, delta)?);
    Ok(0)
}

// ---------- patches ----------
fn patch_rel(n: &str) -> Res<String> {
    Ok(format!("patches/{:03}.json", n.parse::<u32>().map_err(|_| "patch number expected")?))
}

pub fn patch(root: &Path, a: &[String]) -> R {
    let old = a.first().ok_or("usage: smug patch <old-id> --new-id <id> --source <s> [--topic t] [--title t] < new decision text")?;
    let new_id = opt(a, "--new-id").ok_or("--new-id required")?;
    valid_id(&new_id)?;
    let source = opt(a, "--source").ok_or("--source required")?;
    let st = State::open(root)?;
    let notes = load_notes(root);
    let on = notes.iter().find(|n| n.get("id") == old && n.get("type") == "decision").ok_or("no decision with that id")?;
    if on.status() != "active" {
        return Err("that decision is already superseded".into());
    }
    let impacted = impact_of(&notes, old);
    let n = fs::read_dir(root.join("patches")).map(|d| d.flatten().count()).unwrap_or(0) + 1;
    let p = json!({"n": n, "old": old, "new_id": new_id, "source": source, "topic": opt(a, "--topic").unwrap_or_else(|| on.get("topic").to_string()),
        "title": opt(a, "--title").unwrap_or_else(|| new_id.clone()), "valid_from": opt(a, "--date").unwrap_or_else(today), "body": stdin_text().trim(),
        "impacted": impacted.iter().map(|(p, d)| json!({"path": p, "depth": d})).collect::<Vec<_>>(), "status": "proposed"});
    let rel = patch_rel(&n.to_string())?;
    match write_note(&st, Path::new(&rel), serde_json::to_string_pretty(&p)?.as_bytes(), Some("new"))? {
        Ok(_) => {
            println!("Memory Patch #{n}: {old} -> {new_id}  (proposed, nothing applied)");
            println!("may need review ({}):", impacted.len());
            impacted.iter().for_each(|(p, d)| println!("  {}{p}", "  ".repeat(d - 1)));
            println!("next: smug eval {n}   then: smug apply {n}");
            Ok(0)
        }
        Err(m) => {
            eprintln!("smug: {m}");
            Ok(2)
        }
    }
}

fn apply_to(st: &State, p: &Value) -> Res<()> {
    // Idempotent: every step checks whether it already happened, so an interrupted apply can simply be run again.
    let s = |k: &str| p[k].as_str().unwrap_or("").to_string();
    let (old, new_id, date) = (s("old"), s("new_id"), s("valid_from"));
    let notes = load_notes(&st.root);
    let on = notes.iter().find(|n| n.get("id") == old).ok_or("old decision vanished")?;
    let new_rel = format!("500-knowledge/decisions/{new_id}.md");
    match notes.iter().find(|n| n.rel == new_rel) {
        Some(n) if n.get("supersedes") == old => {} // step 1 already done
        Some(_) => return Err("a different note already uses that id".into()),
        None => {
            let deps = on.get("depends_on").to_string();
            let extra = [
                ("title", s("title")),
                ("status", "active".into()),
                ("topic", s("topic")),
                ("source", s("source")),
                ("valid_from", date.clone()),
                ("known_at", today()),
                ("supersedes", old.clone()),
                ("depends_on", if deps.is_empty() { "[]".into() } else { deps }),
            ];
            write_note(st, Path::new(&new_rel), fm_note("decision", &new_id, &extra, &s("body")).as_bytes(), Some("new"))??;
        }
    }
    if !(on.status() == "superseded" && on.get("superseded_by") == new_id) {
        let text = fs::read_to_string(st.root.join(&on.rel))?;
        let t2 = set_fm(&set_fm(&set_fm(&text, "status", "superseded"), "valid_to", &date), "superseded_by", &new_id);
        write_note(st, Path::new(&on.rel), t2.as_bytes(), Some(&sha(text.as_bytes())))??;
    }
    Ok(())
}

fn set_patch_status(st: &State, rel: &str, text: &str, p: &mut Value, status: &str) -> Res<String> {
    p["status"] = json!(status);
    if status == "applied" {
        p["applied_at"] = json!(today());
    }
    let new = serde_json::to_string_pretty(p)?;
    write_note(st, Path::new(rel), new.as_bytes(), Some(&sha(text.as_bytes())))??;
    Ok(new)
}

/// Finishes any patch whose apply was interrupted (status "applying"). Called by `doctor`.
pub fn resume_applying(st: &State) -> Res<usize> {
    let Ok(rd) = fs::read_dir(st.root.join("patches")) else { return Ok(0) };
    let mut done = 0;
    for e in rd.flatten() {
        let rel = format!("patches/{}", e.file_name().to_string_lossy());
        let Ok(text) = fs::read_to_string(st.root.join(&rel)) else { continue };
        let Ok(mut p) = serde_json::from_str::<Value>(&text) else { continue };
        if p["status"] == "applying" {
            apply_to(st, &p)?;
            set_patch_status(st, &rel, &text, &mut p, "applied")?;
            done += 1;
        }
    }
    Ok(done)
}

// ---------- behavioural eval ----------
fn cases(root: &Path) -> Res<Vec<Value>> {
    let t = fs::read_to_string(root.join("evals/cases.jsonl")).map_err(|_| "no eval cases: add evals/cases.jsonl (one JSON per line: {\"q\":..., \"expect\":[...], \"absent\":[...], \"k\":5})")?;
    let mut v = vec![];
    for (i, l) in t.lines().enumerate().filter(|(_, l)| !l.trim().is_empty()) {
        v.push(serde_json::from_str(l).map_err(|e| format!("evals/cases.jsonl line {}: {e}", i + 1))?);
    }
    if v.is_empty() {
        return Err("evals/cases.jsonl has no cases: an empty eval proves nothing, so smug will not treat it as a pass".into());
    }
    Ok(v)
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

/// Runs `cmd` through the shell with `prompt` on stdin and returns its stdout. This is how a real model joins the eval:
/// any command that reads a prompt and prints an answer (for example `claude -p`).
fn ask_model(cmd: &str, prompt: &str) -> Res<String> {
    use std::{io::Write, process::{Command, Stdio}};
    let mut c = if cfg!(windows) { Command::new("cmd") } else { Command::new("sh") };
    c.args([if cfg!(windows) { "/C" } else { "-c" }, cmd]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());
    let mut ch = c.spawn()?;
    let _ = ch.stdin.take().unwrap().write_all(prompt.as_bytes());
    let o = ch.wait_with_output()?;
    if !o.status.success() {
        return Err(format!("model command failed with {}", o.status).into());
    }
    Ok(String::from_utf8_lossy(&o.stdout).to_string())
}

fn run_cases(root: &Path, st: &State, cs: &[Value], model: Option<&str>) -> Res<Vec<(bool, String)>> {
    cs.iter()
        .map(|c| {
            let k = c["k"].as_u64().unwrap_or(5) as usize;
            let q = c["q"].as_str().unwrap_or("");
            let got: Vec<String> = search_valid(root, st, q, k, false)?.into_iter().map(|h| h.path).collect();
            let missing: Vec<String> = strs(&c["expect"]).into_iter().filter(|e| !got.contains(e)).collect();
            let leaked: Vec<String> = strs(&c["absent"]).into_iter().filter(|e| got.contains(e)).collect();
            let mut detail = if missing.is_empty() && leaked.is_empty() { String::new() } else { format!("missing {missing:?} leaked {leaked:?}") };
            let (want, forbid) = (strs(&c["answer_contains"]), strs(&c["answer_absent"]));
            if let (Some(m), false) = (model, want.is_empty() && forbid.is_empty()) {
                let ctx: Vec<String> = got.iter().map(|p| format!("### {p}\n{}", fs::read_to_string(root.join(p)).unwrap_or_default().chars().take(3000).collect::<String>())).collect();
                let prompt = format!("Answer using only the notes below. If they do not contain the answer, say so.\n\n{}\n\nQuestion: {q}\n", ctx.join("\n\n"));
                let ans = ask_model(m, &prompt)?.to_lowercase();
                let miss: Vec<&String> = want.iter().filter(|w| !ans.contains(&w.to_lowercase())).collect();
                let bad: Vec<&String> = forbid.iter().filter(|w| ans.contains(&w.to_lowercase())).collect();
                if !miss.is_empty() || !bad.is_empty() {
                    detail = format!("{detail} answer lacked {miss:?} and contained {bad:?}").trim().to_string();
                }
            }
            Ok((detail.is_empty(), if detail.is_empty() { "ok".into() } else { detail }))
        })
        .collect()
}

fn copy_md(src: &Path, dst: &Path) -> Res<()> {
    for f in md_files(src) {
        let to = dst.join(rel(src, &f));
        fs::create_dir_all(to.parent().unwrap())?;
        fs::copy(&f, to)?;
    }
    Ok(())
}

pub fn eval(root: &Path, a: &[String]) -> R {
    let cs = cases(root)?;
    let model = opt(a, "--model");
    if model.is_some() && !cs.iter().any(|c| c.get("answer_contains").is_some() || c.get("answer_absent").is_some()) {
        eprintln!("smug: --model given but no case has answer_contains or answer_absent; only retrieval is checked");
    }
    let st = State::open(root)?;
    let before = run_cases(root, &st, &cs, model.as_deref())?;
    let after = if let Some(n) = a.first().filter(|x| !x.starts_with("--")) {
        let p: Value = serde_json::from_str(&fs::read_to_string(root.join(patch_rel(n)?))?)?;
        let sim = root.join(".smug/sim");
        let _ = fs::remove_dir_all(&sim);
        copy_md(root, &sim)?;
        let r = (|| -> Res<Vec<(bool, String)>> {
            let sst = State::open(&sim)?;
            apply_to(&sst, &p)?;
            run_cases(&sim, &sst, &cs, model.as_deref())
        })();
        let _ = fs::remove_dir_all(&sim);
        Some(r?)
    } else {
        None
    };
    let mut regress = 0;
    for (i, c) in cs.iter().enumerate() {
        let q = c["q"].as_str().unwrap_or("");
        match &after {
            Some(af) => {
                let bad = before[i].0 && !af[i].0;
                regress += bad as usize;
                println!("{:<9} before {:<4} after {:<4} {q}{}", if bad { "REGRESSED" } else if !before[i].0 && af[i].0 { "FIXED" } else { "same" }, if before[i].0 { "pass" } else { "fail" }, if af[i].0 { "pass" } else { "fail" }, if af[i].0 { String::new() } else { format!("  [{}]", af[i].1) });
            }
            None => println!("{:<4} {q}{}", if before[i].0 { "pass" } else { "FAIL" }, if before[i].0 { String::new() } else { format!("  [{}]", before[i].1) }),
        }
    }
    let fails = before.iter().filter(|x| !x.0).count();
    println!("{} case(s), {} regression(s){}", cs.len(), regress, if after.is_none() { format!(", {fails} failing now") } else { String::new() });
    Ok(if regress > 0 || (after.is_none() && fails > 0) { 1 } else { 0 })
}

pub fn apply(root: &Path, a: &[String]) -> R {
    let n = a.first().ok_or("usage: smug apply <patch-number> [--force] [--model <cmd>]")?;
    let force = a.iter().any(|x| x == "--force");
    let rel = patch_rel(n)?;
    let text = fs::read_to_string(root.join(&rel)).map_err(|_| "no such patch")?;
    let mut p: Value = serde_json::from_str(&text)?;
    let resuming = p["status"] == "applying"; // the eval gate already passed before the interruption
    if !resuming {
        let mut ea = vec![n.clone()];
        ea.extend(opt(a, "--model").map(|m| vec!["--model".to_string(), m]).unwrap_or_default());
        let code = if force && cases(root).is_err() { 0 } else { eval(root, &ea)? };
        if code != 0 && !force {
            eprintln!("smug: eval failed or regressed, refusing to apply (use --force to override)");
            return Ok(1);
        }
    }
    let st = State::open(root)?;
    let mut text = fs::read_to_string(root.join(&rel))?;
    p = serde_json::from_str(&text)?;
    match p["status"].as_str() {
        Some("proposed") => text = set_patch_status(&st, &rel, &text, &mut p, "applying")?, // journalled before any note changes
        Some("applying") => {}
        _ => return Err("patch is not in 'proposed' state".into()),
    }
    apply_to(&st, &p)?;
    set_patch_status(&st, &rel, &text, &mut p, "applied")?;
    println!("applied patch {n}: {} superseded by {}", p["old"].as_str().unwrap_or(""), p["new_id"].as_str().unwrap_or(""));
    Ok(0)
}

// ---------- bench ----------
pub fn bench(root: &Path, _: &[String]) -> R {
    let cs = cases(root)?;
    let st = State::open(root)?;
    index::refresh(root, &st)?;
    let (mut recall, mut mrr, mut lat) = (0.0, 0.0, vec![]);
    for c in &cs {
        let k = c["k"].as_u64().unwrap_or(5) as usize;
        let exp = strs(&c["expect"]);
        let mut got = vec![];
        for _ in 0..20 {
            let t = Instant::now();
            got = search_valid(root, &st, c["q"].as_str().unwrap_or(""), k, false)?;
            lat.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        let paths: Vec<&str> = got.iter().map(|h| h.path.as_str()).collect();
        if !exp.is_empty() {
            recall += exp.iter().filter(|e| paths.contains(&e.as_str())).count() as f64 / exp.len() as f64;
            mrr += exp.iter().filter_map(|e| paths.iter().position(|p| p == e)).min().map(|i| 1.0 / (i as f64 + 1.0)).unwrap_or(0.0);
        }
    }
    lat.sort_by(|a, b| a.total_cmp(b));
    let pct = |p: f64| lat.get(((lat.len() as f64 * p) as usize).min(lat.len().saturating_sub(1))).copied().unwrap_or(0.0);
    let n = cs.len().max(1) as f64;
    println!("cases {}  recall@k {:.3}  MRR {:.3}  p50 {:.1} ms  p95 {:.1} ms", cs.len(), recall / n, mrr / n, pct(0.5), pct(0.95));
    Ok(0)
}
