use crate::{index, mem, store::State, util::*, write::write_note};
use serde_json::{json, Value};
use std::{fs, io::Read, path::{Path, PathBuf}, process::Command, time::Instant};

const TEMPLATE: &[(&str, &str)] = &[
    ("CLAUDE.md", include_str!("../template/CLAUDE.md")),
    ("AGENTS.md", include_str!("../template/AGENTS.md")),
    (".gitignore", include_str!("../template/.gitignore")),
    (".secrets/README.md", include_str!("../template/.secrets/README.md")),
    ("850-companion/Core.md", include_str!("../template/850-companion/Core.md")),
    ("850-companion/Last-Session.md", include_str!("../template/850-companion/Last-Session.md")),
    ("850-companion/Threads.md", include_str!("../template/850-companion/Threads.md")),
    ("850-companion/Rules.md", include_str!("../template/850-companion/Rules.md")),
    ("000-inbox/README.md", include_str!("../template/000-inbox/README.md")),
    ("100-command-center/README.md", include_str!("../template/100-command-center/README.md")),
    ("300-projects/README.md", include_str!("../template/300-projects/README.md")),
    ("500-knowledge/README.md", include_str!("../template/500-knowledge/README.md")),
    ("600-arsenal/README.md", include_str!("../template/600-arsenal/README.md")),
    ("900-archive/README.md", include_str!("../template/900-archive/README.md")),
    ("daily/README.md", include_str!("../template/daily/README.md")),
    ("templates/README.md", include_str!("../template/templates/README.md")),
    ("evals/README.md", include_str!("../template/evals/README.md")),
];

pub fn init(a: &[String]) -> R {
    let dest = PathBuf::from(a.first().map(String::as_str).unwrap_or("smugbrain"));
    if dest.exists() && fs::read_dir(&dest)?.next().is_some() {
        return Err(format!("'{}' exists and is not empty, refusing to overwrite", dest.display()).into());
    }
    for (rel, body) in TEMPLATE {
        let p = dest.join(rel);
        fs::create_dir_all(p.parent().unwrap())?;
        fs::write(p, body)?;
    }
    println!("smug: your brain has been replaced. Open '{}' in Obsidian.", dest.display());
    Ok(0)
}

pub fn scan(root: &Path) -> R {
    let pats = patterns();
    let (mut n, mut bad) = (0, 0);
    for f in candidates(root) {
        let Ok(d) = fs::read(&f) else { continue };
        n += 1;
        for h in hits(&pats, &d) {
            bad += 1;
            println!("BLOCKED {h}: {}", rel(root, &f));
        }
    }
    println!("scanned {n} files, {bad} secret-shaped hits");
    Ok(if bad > 0 { 1 } else { 0 })
}

pub fn backup(root: &Path) -> R {
    let git = |args: &[&str]| Command::new("git").arg("-C").arg(root).args(args).status();
    if !git(&["add", "-A"])?.success() {
        return Ok(1);
    }
    // Scan exactly what is staged (the blobs), not the working tree: nothing can change between scan and commit.
    let o = Command::new("git").arg("-C").arg(root).args(["diff", "--cached", "--name-only", "-z"]).output()?;
    let pats = patterns();
    let mut bad = 0;
    for name in o.stdout.split(|&b| b == 0).filter(|s| !s.is_empty()) {
        let name = String::from_utf8_lossy(name).to_string();
        let blob = Command::new("git").arg("-C").arg(root).args(["show", &format!(":{name}")]).output()?;
        for h in hits(&pats, &blob.stdout) {
            bad += 1;
            println!("BLOCKED {h}: {name}");
        }
    }
    if bad > 0 {
        git(&["reset", "-q"])?;
        eprintln!("smug: backup stopped, nothing committed. Move those files into .secrets/ and retry.");
        return Ok(1);
    }
    git(&["commit", "-q", "-m", &format!("Vault backup: {}", today())])?; // nothing to commit is fine
    Ok(if git(&["push", "-q"])?.success() { 0 } else { 1 })
}

pub fn write(root: &Path, a: &[String]) -> R {
    let rel = safe_rel(a.first().ok_or("usage: smug write <note.md> --expect <sha256|new>")?)?;
    if !root.join(&rel).exists() {
        portable(&rel)?;
    }
    let expect = mem::opt(a, "--expect");
    if expect.is_none() && !a.iter().any(|x| x == "--force") {
        return Err("--expect <sha256|new> is required (or --force to overwrite blindly)".into());
    }
    let mut data = vec![];
    std::io::stdin().read_to_end(&mut data)?;
    let st = State::open(root)?;
    match write_note(&st, &rel, &data, expect.as_deref())? {
        Ok(h) => {
            println!("{h}");
            Ok(0)
        }
        Err(m) => {
            eprintln!("smug: {m}");
            Ok(2)
        }
    }
}

pub fn history(root: &Path, a: &[String]) -> R {
    let n: usize = a.first().and_then(|x| x.parse().ok()).unwrap_or(20);
    let ev = State::open(root)?.events()?;
    for e in ev.iter().rev().take(n) {
        println!("{:>5} {} {:<8} {} {}", e.id, e.ts, e.kind, &e.hash[..e.hash.len().min(12)], e.path);
    }
    Ok(0)
}

pub fn search(root: &Path, a: &[String]) -> R {
    let deep = a.iter().any(|x| x == "--deep");
    let history = a.iter().any(|x| x == "--history");
    let words: Vec<String> = a.iter().filter(|x| !x.starts_with("--")).cloned().collect();
    if words.is_empty() {
        return Err("usage: smug search [--deep] [--history] <words>".into());
    }
    if deep {
        // semantic search via QMD when it is installed; smug stays a single binary without it
        return match Command::new("qmd").arg("query").args(&words).status() {
            Ok(s) => Ok(s.code().unwrap_or(1)),
            Err(_) => Err("--deep needs QMD (https://github.com/tobi/qmd) on PATH; plain `smug search` works without it".into()),
        };
    }
    let t0 = Instant::now();
    let st = State::open(root)?;
    let changed = index::refresh(root, &st)?;
    for h in mem::search_valid(root, &st, &words.join(" "), 5, history)? {
        println!("{}\n    {}", h.path, h.snippet);
    }
    eprintln!("{} ms ({changed} reindexed)", t0.elapsed().as_millis());
    Ok(0)
}

pub fn reindex(root: &Path) -> R {
    let n = index::refresh(root, &State::open(root)?)?;
    println!("index up to date ({n} changed)");
    Ok(0)
}

pub fn context(root: &Path) -> R {
    if !is_vault(root) {
        return Ok(0);
    }
    let read = |names: &[&str]| names.iter().find_map(|f| fs::read_to_string(root.join(format!("850-companion/{f}.md"))).ok());
    let last = read(&["Last-Session"]).unwrap_or_default();
    // The newest handoff card is the first "## " section; older sessions go to the back of the queue.
    let first = if last.starts_with("## ") { 0 } else { last.find("\n## ").map(|i| i + 1).unwrap_or(0) };
    let end = last[first..].match_indices("\n## ").next().map(|(i, _)| first + i + 1).unwrap_or(last.len());
    let notes = mem::load_notes(root);
    let act: Vec<String> = notes.iter().filter(|n| n.fm.get("type").map(String::as_str) == Some("decision") && n.fm.get("status").map(String::as_str).unwrap_or("active") == "active").map(|n| format!("- {}: {} ({})", n.key(), n.fm.get("title").cloned().unwrap_or_default(), n.rel)).take(25).collect();
    // Order is priority: the budget cuts the tail, so rules, identity, the newest card and open threads arrive first.
    let parts = [
        ("Rules", read(&["Rules", "Kurallar"])),
        ("Core", read(&["Core"])),
        ("Last-Session", Some(last[..end].to_string())),
        ("Threads", read(&["Threads"])),
        ("Active decisions (check with `smug check <id>` before acting)", Some(act.join("\n"))),
        ("Earlier sessions", Some(last[end..].to_string())),
    ];
    let mut out = String::new();
    for (h, t) in parts {
        if let Some(t) = t.filter(|t| !t.trim().is_empty()) {
            out.push_str(&format!("## {h}\n{t}\n"));
        }
    }
    let cut = out.char_indices().nth(9500).map(|(i, _)| i).unwrap_or(out.len());
    print!("{}", &out[..cut]);
    Ok(0)
}

pub fn doctor(root: &Path) -> R {
    let mut bad = 0;
    let mut check = |ok: bool, what: &str| {
        println!("{} {what}", if ok { "ok  " } else { "FAIL" });
        if !ok {
            bad += 1;
        }
    };
    check(root.join("CLAUDE.md").exists(), "CLAUDE.md present");
    check(root.join("850-companion/Core.md").exists(), "850-companion/Core.md present");
    check(fs::read_to_string(root.join(".gitignore")).map(|g| g.contains(".secrets") && g.contains(".smug")).unwrap_or(false), ".gitignore covers .secrets and .smug");
    let st = State::open(root)?;
    let mut leftovers = vec![];
    walk(root, &[".git", ".smug", ".secrets", ".obsidian"], &mut leftovers);
    leftovers.retain(|f| f.file_name().is_some_and(|n| n.to_string_lossy().contains(".smug-tmp-")));
    leftovers.iter().for_each(|f| { let _ = fs::remove_file(f); }); // we hold the vault lock, so no writer owns these
    check(true, &format!("{} leftover temp file(s) from interrupted writes removed", leftovers.len()));
    // crash recovery: an intent with no commit is either finished (file matches) or aborted
    let dangling = st.dangling()?;
    for e in &dangling {
        let done = fs::read(root.join(&e.path)).map(|d| sha(&d) == e.hash).unwrap_or(false);
        st.append(if done { "recovered" } else { "abort" }, &e.path, &e.hash, Some(e.id))?;
    }
    check(true, &format!("journal ok, {} unfinished write(s) reconciled", dangling.len()));
    let resumed = mem::resume_applying(&st)?;
    check(true, &format!("{resumed} interrupted patch apply(s) completed"));
    let idx_ok = index::refresh(root, &st).is_ok();
    check(idx_ok, "search index consistent (rebuilt from Markdown if it was not)");
    let pats = patterns();
    let leaks = candidates(root).iter().filter(|f| fs::read(f).map(|d| !hits(&pats, &d).is_empty()).unwrap_or(false)).count();
    check(leaks == 0, &format!("{leaks} secret-shaped file(s) would be committed"));
    let notes = mem::load_notes(root);
    let stale: Vec<String> = notes.iter().filter(|n| n.fm.get("type").map(String::as_str) == Some("decision")).filter_map(|n| {
        let id = n.fm.get("id")?;
        let (s, _) = mem::check_status(&notes, id);
        matches!(s, "CONFLICTED" | "UNSUPPORTED").then(|| format!("{id}={s}"))
    }).collect();
    check(stale.is_empty(), &format!("decisions healthy{}", if stale.is_empty() { String::new() } else { format!(" (problems: {})", stale.join(", ")) }));
    Ok(if bad > 0 { 1 } else { 0 })
}

pub fn setup(root: &Path, a: &[String]) -> R {
    match a.first().map(String::as_str) {
        Some("claude") => {
            let p = root.join(".claude/settings.json");
            let mut cfg: Value = match fs::read_to_string(&p) {
                Ok(t) if !t.trim().is_empty() => serde_json::from_str(&t).map_err(|e| format!("{} is not valid JSON ({e}); left untouched", p.display()))?,
                _ => json!({}),
            };
            let hooks = cfg.as_object_mut().ok_or("settings.json is not an object")?.entry("hooks").or_insert_with(|| json!({}));
            let hooks = hooks.as_object_mut().ok_or("hooks is not an object")?;
            let exe = std::env::current_exe().map(|e| e.to_string_lossy().to_string()).unwrap_or_else(|_| "smug".into());
            let vault = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf()).to_string_lossy().trim_start_matches("\\\\?\\").to_string();
            let base = format!("\"{exe}\" --vault \"{vault}\"");
            let mut added = vec![];
            for (event, sub, matcher) in [("SessionStart", "context", None), ("UserPromptSubmit", "hook prompt", None), ("PreToolUse", "hook pre-edit", Some("Edit|Write|MultiEdit")), ("Stop", "hook stop", None), ("SessionEnd", "hook session-end", None), ("PreCompact", "hook pre-compact", None)] {
                let cmd = format!("{base} {sub}");
                let arr = hooks.entry(event).or_insert_with(|| json!([])).as_array_mut().ok_or("a hook event is not an array")?;
                if arr.iter().any(|e| e.to_string().contains(&serde_json::to_string(&cmd).unwrap_or_default()[1..serde_json::to_string(&cmd).unwrap_or_default().len() - 1])) {
                    continue; // exactly ours already: nothing to do
                }
                // an older smug entry for the same hook (no --vault) is replaced; the user's own hooks are never touched
                arr.retain(|e| { let s = e.to_string(); !(s.contains("smug") && s.contains(&format!(" {sub}\""))) });
                let mut entry = json!({"hooks": [{"type": "command", "command": cmd}]});
                if let Some(m) = matcher {
                    entry["matcher"] = json!(m);
                }
                arr.push(entry);
                added.push(event);
            }
            fs::create_dir_all(p.parent().unwrap())?;
            fs::write(&p, serde_json::to_string_pretty(&cfg)? + "\n")?;
            println!("{}: {} (your other settings and hooks were kept)", p.display(), if added.is_empty() { "already set up".to_string() } else { format!("added {}", added.join(", ")) });
            Ok(0)
        }
        Some("codex") => {
            let p = root.join("AGENTS.md");
            let add = "\n## smug\nAt session start run `smug context`. Before acting on a decision run `smug check <id>`. Edit notes only with `smug write <file> --expect <sha256|new>`.\n";
            let cur = fs::read_to_string(&p).unwrap_or_default();
            if !cur.contains("## smug") {
                fs::write(&p, cur + add)?;
            }
            println!("Codex reads {}: smug instructions are in place.", p.display());
            Ok(0)
        }
        Some("hermes") => {
            println!("Run `smug context` at session start and `smug hook prompt` (prompt JSON on stdin) per message, and feed the output to Hermes as context.");
            Ok(0)
        }
        _ => Err("usage: smug setup claude|codex|hermes".into()),
    }
}
