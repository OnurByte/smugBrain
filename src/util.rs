//! Paths, hashing, secret patterns, OS lock, small helpers.
use regex::bytes::Regex;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Component, Path, PathBuf},
    process::Command,
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub type Res<T> = Result<T, Box<dyn std::error::Error>>;
pub type R = Res<i32>;

pub fn vault() -> PathBuf {
    PathBuf::from(env::var("SMUG_VAULT").unwrap_or_else(|_| ".".into()))
}

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Unix seconds -> "YYYY-MM-DD" (UTC), Hinnant's civil_from_days.
pub fn ymd(secs: i64) -> String {
    let z = secs.div_euclid(86400) + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn today() -> String {
    ymd(now())
}

pub fn sha(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

// ---------- secret guard ----------
pub type Pats = Vec<(&'static str, Regex)>;

pub fn patterns() -> Pats {
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

pub fn hits(pats: &Pats, data: &[u8]) -> Vec<&'static str> {
    if data.iter().take(8192).any(|&b| b == 0) {
        return vec![];
    }
    pats.iter().filter(|(_, r)| r.is_match(data)).map(|(n, _)| *n).collect()
}

// ---------- files ----------
pub const SKIP_INDEX: &[&str] = &[".git", ".smug", ".secrets", ".obsidian", "900-archive"];

pub fn walk(dir: &Path, skip: &[&str], out: &mut Vec<PathBuf>) {
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

pub fn md_files(root: &Path) -> Vec<PathBuf> {
    let mut v = vec![];
    walk(root, SKIP_INDEX, &mut v);
    v.retain(|f| f.extension().is_some_and(|e| e == "md"));
    v.sort();
    v
}

pub fn rel(root: &Path, f: &Path) -> String {
    f.strip_prefix(root).unwrap_or(f).to_string_lossy().replace('\\', "/")
}

/// Files git would commit (respects .gitignore); falls back to a walk outside a repo.
pub fn candidates(root: &Path) -> Vec<PathBuf> {
    if let Ok(o) = Command::new("git").arg("-C").arg(root).args(["ls-files", "-z", "-co", "--exclude-standard"]).output() {
        if o.status.success() {
            return o.stdout.split(|&b| b == 0).filter(|s| !s.is_empty()).map(|s| root.join(String::from_utf8_lossy(s).as_ref())).collect();
        }
    }
    let mut v = vec![];
    walk(root, &[".git", ".smug", ".secrets"], &mut v);
    v
}

pub fn safe_rel(p: &str) -> Result<PathBuf, String> {
    let pb = PathBuf::from(p);
    let bad = pb.is_absolute() || pb.components().any(|c| matches!(c, Component::ParentDir | Component::Prefix(_) | Component::RootDir));
    let reserved = pb.components().any(|c| matches!(c.as_os_str().to_str(), Some(".secrets" | ".smug" | ".git")));
    if bad || reserved || p.is_empty() {
        return Err(format!("refusing path '{p}' (escapes vault or reserved)"));
    }
    Ok(pb)
}

/// Every existing component of `rel` under `root` must be a real file or directory.
pub fn no_symlinks(root: &Path, rel: &Path) -> Result<(), String> {
    let mut p = root.to_path_buf();
    for c in rel.components() {
        p.push(c);
        if fs::symlink_metadata(&p).map(|m| m.file_type().is_symlink()).unwrap_or(false) {
            return Err(format!("symlink in path: {}", p.display()));
        }
    }
    Ok(())
}

// ---------- OS lock ----------
/// Advisory lock released by the kernel if the process dies: it can never go stale.
pub struct Lock(#[allow(dead_code)] fs::File);
impl Lock {
    pub fn take(root: &Path, wait_ms: u64) -> Result<Lock, String> {
        fs::create_dir_all(root.join(".smug")).map_err(|e| e.to_string())?;
        let f = fs::OpenOptions::new().create(true).write(true).truncate(false).open(root.join(".smug/lock")).map_err(|e| e.to_string())?;
        for _ in 0..=(wait_ms / 25) {
            match f.try_lock() {
                Ok(()) => return Ok(Lock(f)),
                Err(fs::TryLockError::WouldBlock) => sleep(Duration::from_millis(25)),
                Err(fs::TryLockError::Error(e)) => return Err(e.to_string()),
            }
        }
        Err("vault is busy (another smug is running)".into())
    }
}

// ---------- tiny frontmatter ----------
pub type Fm = std::collections::HashMap<String, String>;

/// Splits `---\nkey: value\n---\nbody`. No frontmatter -> empty map and the whole text.
pub fn split_fm(text: &str) -> (Fm, &str) {
    let mut m = Fm::new();
    let Some(rest) = text.strip_prefix("---\n") else { return (m, text) };
    let Some(end) = rest.find("\n---") else { return (m, text) };
    for l in rest[..end].lines() {
        if let Some((k, v)) = l.split_once(':') {
            m.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
        }
    }
    let body = rest[end + 4..].trim_start_matches(['\n', '\r']);
    (m, body)
}

pub fn list(v: &str) -> Vec<String> {
    v.trim().trim_start_matches('[').trim_end_matches(']').split(',').map(|s| s.trim().trim_matches('"').to_string()).filter(|s| !s.is_empty()).collect()
}

/// Sets `key: val` in the frontmatter (adds a frontmatter block if missing).
pub fn set_fm(text: &str, key: &str, val: &str) -> String {
    let line = format!("{key}: {val}");
    let Some(rest) = text.strip_prefix("---\n") else { return format!("---\n{line}\n---\n{text}") };
    let Some(end) = rest.find("\n---") else { return text.to_string() };
    let (head, tail) = rest.split_at(end);
    let mut done = false;
    let mut out: Vec<String> = head
        .lines()
        .map(|l| {
            if l.split_once(':').is_some_and(|(k, _)| k.trim() == key) {
                done = true;
                line.clone()
            } else {
                l.to_string()
            }
        })
        .collect();
    if !done {
        out.push(line);
    }
    format!("---\n{}{tail}", out.join("\n"))
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
    #[test]
    fn dates_and_frontmatter() {
        assert_eq!(ymd(0), "1970-01-01");
        assert_eq!(ymd(1794455602), "2026-11-12");
        let (m, body) = split_fm("---\nid: a\ndepends_on: [x, y]\n---\n\nhello");
        assert_eq!((m["id"].as_str(), body), ("a", "hello"));
        assert_eq!(list(&m["depends_on"]), ["x", "y"]);
        let t = set_fm("---\nid: a\nstatus: active\n---\n\nhello", "status", "superseded");
        assert!(t.contains("status: superseded") && !t.contains("active") && t.ends_with("hello"));
        assert!(set_fm("no fm", "k", "v").starts_with("---\nk: v\n---\n"));
    }
}
