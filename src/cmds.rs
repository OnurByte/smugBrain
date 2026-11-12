use crate::{index, mem, store::State, util::*, write::write_note};
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
    let mut out = String::new();
    for f in ["Core", "Rules", "Last-Session", "Threads"] {
        if let Ok(t) = fs::read_to_string(root.join(format!("850-companion/{f}.md"))) {
            out.push_str(&format!("## {f}\n{t}\n"));
        }
    }
    let notes = mem::load_notes(root);
    let act: Vec<String> = notes.iter().filter(|n| n.fm.get("type").map(String::as_str) == Some("decision") && n.fm.get("status").map(String::as_str).unwrap_or("active") == "active").map(|n| format!("- {}: {} ({})", n.key(), n.fm.get("title").cloned().unwrap_or_default(), n.rel)).take(25).collect();
    if !act.is_empty() {
        out.push_str(&format!("## Active decisions (check with `smug check <id>` before acting)\n{}\n", act.join("\n")));
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
    // crash recovery: an intent with no commit is either finished (file matches) or aborted
    let dangling = st.dangling()?;
    for e in &dangling {
        let done = fs::read(root.join(&e.path)).map(|d| sha(&d) == e.hash).unwrap_or(false);
        st.append(if done { "recovered" } else { "abort" }, &e.path, &e.hash, Some(e.id))?;
    }
    check(true, &format!("journal ok, {} unfinished write(s) reconciled", dangling.len()));
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
            let json = r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"smug context"}]}],"UserPromptSubmit":[{"hooks":[{"type":"command","command":"smug hook prompt"}]}]}}"#;
            if p.exists() {
                println!("{} exists. Merge this by hand:\n{json}", p.display());
            } else {
                fs::create_dir_all(p.parent().unwrap())?;
                fs::write(&p, json)?;
                println!("wrote {}: sessions start with your memory, and every prompt gets note candidates and failure warnings.", p.display());
            }
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

/// `smug hook prompt`: reads {"prompt": "..."} on stdin (Claude Code UserPromptSubmit). Silent unless useful. Never fails.
pub fn hook(root: &Path, a: &[String]) -> R {
    if a.first().map(String::as_str) != Some("prompt") {
        return Err("usage: smug hook prompt".into());
    }
    let mut s = String::new();
    let _ = std::io::stdin().read_to_string(&mut s);
    let prompt = serde_json::from_str::<serde_json::Value>(&s).ok().and_then(|v| v["prompt"].as_str().map(String::from)).unwrap_or_default();
    if prompt.trim().is_empty() {
        return Ok(0);
    }
    for w in mem::warn_texts(&mem::load_notes(root), &prompt) {
        println!("{w}");
    }
    if let Ok(st) = State::open(root) {
        if let Ok(h) = mem::search_valid(root, &st, &prompt, 3, false) {
            if !h.is_empty() {
                println!("smug: note candidates (open before relying on them):");
                h.iter().for_each(|x| println!("- {}", x.path));
            }
        }
    }
    Ok(0)
}
