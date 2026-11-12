//! smug: plain-Markdown memory for agents. Markdown is truth; .smug/ is disposable state.
use regex::bytes::Regex;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{exit, Command},
    thread::sleep,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

type R = Result<i32, Box<dyn std::error::Error>>;

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
];

fn vault() -> PathBuf {
    PathBuf::from(env::var("SMUG_VAULT").unwrap_or_else(|_| ".".into()))
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn sha(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

// ---------- secret guard ----------
fn patterns() -> Vec<(&'static str, Regex)> {
    [
        ("private-key", r"-----BEGIN (?:OPENSSH|RSA|EC|DSA|PGP) PRIVATE KEY-----"),
        ("github-token", r"\b(?:ghp|github_pat)_[A-Za-z0-9_]{20,}\b"),
        ("api-key", r"\b(?:sk-[A-Za-z0-9_-]{20,}|xox[baprs]-[A-Za-z0-9-]{20,})\b"),
        ("credential-assignment", r"(?im)^\s*(?:api[_-]?key|secret|password|passwd|token)\s*[:=]\s*[^\s${}<]{8,}\s*$"),
        ("recovery-phrase", r"(?m)^[a-z]{2,}(?:[ \t]+[a-z]{2,}){11,23}[ \t]*$"),
    ]
    .into_iter()
    .map(|(n, p)| (n, Regex::new(p).expect("static regex")))
    .collect()
}

fn hits(pats: &[(&'static str, Regex)], data: &[u8]) -> Vec<&'static str> {
    if data.iter().take(8192).any(|&b| b == 0) {
        return vec![];
    }
    pats.iter().filter(|(_, r)| r.is_match(data)).map(|(n, _)| *n).collect()
}

// ---------- files ----------
fn walk(dir: &Path, skip: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        let p = e.path();
        if ft.is_dir() {
            if !skip.contains(&e.file_name().to_string_lossy().as_ref()) {
                walk(&p, skip, out);
            }
        } else {
            out.push(p);
        }
    }
}

/// Files git would commit (respects .gitignore); falls back to a walk outside a repo.
fn candidates(root: &Path) -> Vec<PathBuf> {
    if let Ok(o) = Command::new("git").arg("-C").arg(root).args(["ls-files", "-z", "-co", "--exclude-standard"]).output() {
        if o.status.success() {
            return o.stdout.split(|&b| b == 0).filter(|s| !s.is_empty()).map(|s| root.join(String::from_utf8_lossy(s).as_ref())).collect();
        }
    }
    let mut v = vec![];
    walk(root, &[".git", ".smug", ".secrets"], &mut v);
    v
}

fn safe_rel(p: &str) -> Result<PathBuf, String> {
    let pb = PathBuf::from(p);
    let bad = pb.is_absolute() || pb.components().any(|c| matches!(c, Component::ParentDir | Component::Prefix(_) | Component::RootDir));
    let reserved = pb.components().any(|c| matches!(c.as_os_str().to_str(), Some(".secrets" | ".smug" | ".git")));
    if bad || reserved || p.is_empty() {
        return Err(format!("refusing path '{p}' (escapes vault or reserved)"));
    }
    Ok(pb)
}

// ---------- state ----------
fn db(root: &Path) -> Result<Connection, Box<dyn std::error::Error>> {
    fs::create_dir_all(root.join(".smug"))?;
    let c = Connection::open(root.join(".smug/state.db"))?;
    c.execute_batch("CREATE TABLE IF NOT EXISTS events(id INTEGER PRIMARY KEY, ts INTEGER, kind TEXT, path TEXT, hash TEXT, ref INTEGER);")?;
    Ok(c)
}

fn log_event(c: &Connection, kind: &str, path: &str, hash: &str, r: Option<i64>) -> Result<i64, rusqlite::Error> {
    c.execute("INSERT INTO events(ts,kind,path,hash,ref) VALUES(?,?,?,?,?)", params![now(), kind, path, hash, r])?;
    Ok(c.last_insert_rowid())
}

struct Lock(PathBuf);
impl Lock {
    fn take(root: &Path) -> Result<Lock, String> {
        fs::create_dir_all(root.join(".smug")).map_err(|e| e.to_string())?;
        let p = root.join(".smug/lock");
        for _ in 0..100 {
            if fs::OpenOptions::new().write(true).create_new(true).open(&p).is_ok() {
                return Ok(Lock(p));
            }
            sleep(Duration::from_millis(50));
        }
        Err("vault is locked (stale .smug/lock? delete it if no smug is running)".into())
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

// ---------- commands ----------
fn init(a: &[String]) -> R {
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

fn scan(_: &[String]) -> R {
    let root = vault();
    let pats = patterns();
    let (mut n, mut bad) = (0, 0);
    for f in candidates(&root) {
        let Ok(d) = fs::read(&f) else { continue };
        n += 1;
        for h in hits(&pats, &d) {
            bad += 1;
            println!("BLOCKED {h}: {}", f.strip_prefix(&root).unwrap_or(&f).display());
        }
    }
    println!("scanned {n} files, {bad} secret-shaped hits");
    Ok(if bad > 0 { 1 } else { 0 })
}

fn backup(_: &[String]) -> R {
    if scan(&[])? != 0 {
        eprintln!("smug: backup stopped. Move the files above into .secrets/ and retry.");
        return Ok(1);
    }
    let root = vault();
    let git = |args: &[&str]| Command::new("git").arg("-C").arg(&root).args(args).status();
    if !git(&["add", "-A"])?.success() {
        return Ok(1);
    }
    let msg = format!("Vault backup: {}", now());
    git(&["commit", "-q", "-m", &msg])?; // nothing to commit is fine
    Ok(if git(&["push", "-q"])?.success() { 0 } else { 1 })
}

fn write(a: &[String]) -> R {
    let rel = safe_rel(a.first().ok_or("usage: smug write <note.md> [--expect <sha256|new>]")?)?;
    let expect = a.iter().position(|x| x == "--expect").and_then(|i| a.get(i + 1));
    let mut data = vec![];
    std::io::stdin().read_to_end(&mut data)?;
    if !hits(&patterns(), &data).is_empty() {
        return Err("content looks like a secret; put it in .secrets/ by hand".into());
    }
    let root = vault();
    let _lock = Lock::take(&root)?;
    let path = root.join(&rel);
    if fs::symlink_metadata(&path).map(|m| m.file_type().is_symlink()).unwrap_or(false) {
        return Err("target is a symlink".into());
    }
    let cur = fs::read(&path).ok().map(|d| sha(&d));
    if let Some(e) = expect {
        let ok = match (&cur, e.as_str()) {
            (None, "new") => true,
            (Some(h), x) => h == x,
            _ => false,
        };
        if !ok {
            eprintln!("smug: CONFLICT {} changed (now {})", rel.display(), cur.as_deref().unwrap_or("missing"));
            return Ok(2);
        }
    }
    let new = sha(&data);
    let c = db(&root)?;
    let id = log_event(&c, "intent", &rel.to_string_lossy(), &new, None)?;
    fs::create_dir_all(path.parent().unwrap())?;
    let tmp = path.with_extension(format!("smug-tmp-{}", std::process::id()));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(&data)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, &path)?;
    log_event(&c, "commit", &rel.to_string_lossy(), &new, Some(id))?;
    println!("{new}");
    Ok(0)
}

fn history(a: &[String]) -> R {
    let c = db(&vault())?;
    let n: i64 = a.first().and_then(|x| x.parse().ok()).unwrap_or(20);
    let mut st = c.prepare("SELECT id,ts,kind,path,substr(hash,1,12) FROM events ORDER BY id DESC LIMIT ?")?;
    for r in st.query_map([n], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?)))? {
        let (id, ts, k, p, h) = r?;
        println!("{id:>5} {ts} {k:<7} {h} {p}");
    }
    Ok(0)
}

fn search(a: &[String]) -> R {
    if a.is_empty() {
        return Err("usage: smug search <words>".into());
    }
    let t0 = Instant::now();
    let root = vault();
    let pats = patterns();
    let c = Connection::open_in_memory()?;
    c.execute_batch("CREATE VIRTUAL TABLE n USING fts5(path, body, tokenize='unicode61 remove_diacritics 2');")?;
    let mut files = vec![];
    walk(&root, &[".git", ".smug", ".secrets", ".obsidian", "900-archive"], &mut files);
    {
        let tx = c.unchecked_transaction()?;
        for f in files.iter().filter(|f| f.extension().is_some_and(|e| e == "md")) {
            let Ok(d) = fs::read(f) else { continue };
            if !hits(&pats, &d).is_empty() {
                continue; // secret-shaped notes are never indexed
            }
            tx.execute("INSERT INTO n(path,body) VALUES(?,?)", params![f.strip_prefix(&root).unwrap_or(f).to_string_lossy(), String::from_utf8_lossy(&d)])?;
        }
        tx.commit()?;
    }
    let q = a.iter().map(|w| format!("\"{}\"", w.replace('"', "\"\""))).collect::<Vec<_>>().join(" OR ");
    let mut st = c.prepare("SELECT path, snippet(n,1,'[',']','…',10) FROM n WHERE n MATCH ? ORDER BY bm25(n) LIMIT 5")?;
    for r in st.query_map([q], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (p, s) = r?;
        println!("{p}\n    {}", s.replace('\n', " "));
    }
    eprintln!("{} ms", t0.elapsed().as_millis());
    Ok(0)
}

fn context(_: &[String]) -> R {
    let root = vault();
    let mut out = String::new();
    for f in ["Core", "Rules", "Last-Session", "Threads"] {
        if let Ok(t) = fs::read_to_string(root.join(format!("850-companion/{f}.md"))) {
            out.push_str(&format!("## {f}\n{t}\n"));
        }
    }
    let cut = out.char_indices().nth(9500).map(|(i, _)| i).unwrap_or(out.len());
    print!("{}", &out[..cut]);
    Ok(0)
}

fn doctor(_: &[String]) -> R {
    let root = vault();
    let mut bad = 0;
    let mut check = |ok: bool, what: &str| {
        println!("{} {what}", if ok { "ok  " } else { "FAIL" });
        if !ok {
            bad += 1;
        }
    };
    check(root.join("CLAUDE.md").exists(), "CLAUDE.md present");
    check(root.join("850-companion/Core.md").exists(), "850-companion/Core.md present");
    check(fs::read_to_string(root.join(".gitignore")).map(|g| g.contains(".secrets")).unwrap_or(false), ".gitignore covers .secrets");
    let c = db(&root)?;
    // crash recovery: an intent with no commit is either finished (file matches) or aborted
    let mut dangling = vec![];
    {
        let mut st = c.prepare("SELECT id,path,hash FROM events e WHERE kind='intent' AND NOT EXISTS (SELECT 1 FROM events x WHERE x.ref=e.id)")?;
        for r in st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))? {
            dangling.push(r?);
        }
    }
    for (id, p, h) in &dangling {
        let done = fs::read(root.join(p)).map(|d| &sha(&d) == h).unwrap_or(false);
        log_event(&c, if done { "recovered" } else { "abort" }, p, h, Some(*id))?;
    }
    check(true, &format!("event log ok, {} unfinished write(s) reconciled", dangling.len()));
    let pats = patterns();
    let leaks = candidates(&root).iter().filter(|f| fs::read(f).map(|d| !hits(&pats, &d).is_empty()).unwrap_or(false)).count();
    check(leaks == 0, &format!("{leaks} secret-shaped file(s) would be committed"));
    Ok(if bad > 0 { 1 } else { 0 })
}

fn main() {
    let a: Vec<String> = env::args().skip(1).collect();
    let rest = a.get(1..).unwrap_or(&[]);
    let r = match a.first().map(String::as_str) {
        Some("init") => init(rest),
        Some("scan") => scan(rest),
        Some("backup") => backup(rest),
        Some("write") => write(rest),
        Some("log") => history(rest),
        Some("search") => search(rest),
        Some("context") => context(rest),
        Some("doctor") => doctor(rest),
        _ => {
            println!("smug: your brain has been replaced.\n  init [dir]   new vault\n  search <q>   find notes (words)\n  context      print what an agent should read first\n  write <f> [--expect <sha256|new>]   atomic, hash-checked edit from stdin\n  log [n]      write history\n  scan         secret guard\n  backup       scan, commit, push\n  doctor       health check + crash recovery");
            Ok(0)
        }
    };
    match r {
        Ok(c) => exit(c),
        Err(e) => {
            eprintln!("smug: {e}");
            exit(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths() {
        assert!(safe_rel("a/b.md").is_ok());
        for p in ["../x", "/etc/passwd", ".secrets/a.md", "a/../../b", ".git/config", ""] {
            assert!(safe_rel(p).is_err(), "{p}");
        }
    }
    #[test]
    fn secrets() {
        let p = patterns();
        assert!(hits(&p, b"-----BEGIN OPENSSH PRIVATE KEY-----").contains(&"private-key"));
        assert!(hits(&p, b"password: hunter2hunter2").contains(&"credential-assignment"));
        assert!(hits(&p, b"one two three four five six seven eight nine ten eleven twelve").contains(&"recovery-phrase"));
        assert!(hits(&p, b"just a normal note about passwords").is_empty());
    }
}
