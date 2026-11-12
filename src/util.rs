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

/// A smug vault has a companion folder (made by `smug init`) or a journal. Anything else is somebody's project:
/// smug must not scatter .smug/ and history/ into it.
pub fn is_vault(root: &Path) -> bool {
    root.join("850-companion").is_dir() || root.join("history/events.jsonl").is_file()
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
        ("credential-assignment", r#"(?m)^\s*(?:export\s+|set\s+|\$env:)?[A-Z][A-Z0-9_]*_(?:API_?KEY|SECRET|PASSWORD|PASSWD|TOKEN)\s*=\s*["']?[^\s"'${}<]{6,}[^\s"'${}<.]{2}["']?\s*$"#), // value may not end in "..": that is a docs placeholder
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

/// A NEW note must be nameable on Windows, macOS and Linux alike, or the vault stops syncing between machines.
pub fn portable(rel: &Path) -> Result<(), String> {
    for c in rel.components().filter_map(|c| c.as_os_str().to_str()) {
        let stem = c.split('.').next().unwrap_or("").to_ascii_uppercase();
        let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL") || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && stem.ends_with(|d: char| d.is_ascii_digit() && d != '0'));
        if device || c.len() > 255 || c.ends_with(['.', ' ']) && c != "." || c.chars().any(|ch| ch.is_control() || "<>:\"|?*".contains(ch)) {
            return Err(format!("refusing new note '{}': '{c}' is not a portable file name (Windows device name, one of <>:\"|?*, trailing dot or space, or too long)", rel.display()));
        }
    }
    Ok(())
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
/// shortcut: Haiku's std has no flock, so there we fall back to a lock file that is stolen after 30 s; upgrade if Haiku gains flock support.
pub struct Lock {
    _f: Option<fs::File>,
    file: Option<PathBuf>,
}
impl Lock {
    pub fn take(root: &Path, wait_ms: u64) -> Result<Lock, String> {
        fs::create_dir_all(root.join(".smug")).map_err(|e| e.to_string())?;
        let f = fs::OpenOptions::new().create(true).write(true).truncate(false).open(root.join(".smug/lock")).map_err(|e| e.to_string())?;
        for _ in 0..=(wait_ms / 25) {
            match f.try_lock() {
                Ok(()) => return Ok(Lock { _f: Some(f), file: None }),
                Err(fs::TryLockError::WouldBlock) => sleep(Duration::from_millis(25)),
                Err(fs::TryLockError::Error(e)) if e.kind() == std::io::ErrorKind::Unsupported => return Self::take_by_file(root, wait_ms, 30),
                Err(fs::TryLockError::Error(e)) => return Err(e.to_string()),
            }
        }
        Err("vault is busy (another smug is running)".into())
    }

    pub fn take_by_file(root: &Path, wait_ms: u64, stale_secs: u64) -> Result<Lock, String> {
        let p = root.join(".smug/lock.file");
        for _ in 0..=(wait_ms / 25) {
            match fs::OpenOptions::new().write(true).create_new(true).open(&p) {
                Ok(_) => return Ok(Lock { _f: None, file: Some(p) }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let age = fs::metadata(&p).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).map(|d| d.as_secs()).unwrap_or(0);
                    if age >= stale_secs {
                        let _ = fs::remove_file(&p);
                    } else {
                        sleep(Duration::from_millis(25));
                    }
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("vault is busy (another smug is running)".into())
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        if let Some(p) = &self.file {
            let _ = fs::remove_file(p);
        }
    }
}

// ---------- tiny frontmatter ----------
pub type Fm = std::collections::HashMap<String, String>;

/// Splits `---\nkey: value\n---\nbody`. No frontmatter -> empty map and the whole text.
pub fn split_fm(text: &str) -> (Fm, &str) {
    let mut m = Fm::new();
    let Some(rest) = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n")) else { return (m, text) }; // CRLF: git autocrlf on Windows
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
    let Some(rest) = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n")) else { return format!("---\n{line}\n---\n{text}") };
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
    macro_rules! t {
        ($n:ident, $b:expr) => {
            #[test]
            fn $n() {
                $b
            }
        };
    }
    fn found(cat: &str, text: &str) -> bool {
        hits(&patterns(), text.as_bytes()).contains(&cat)
    }
    fn clean(text: &str) -> bool {
        hits(&patterns(), text.as_bytes()).is_empty()
    }

    // --- secret guard: things that must be caught
    t!(secret_rsa_key, assert!(found("private-key", "-----BEGIN RSA PRIVATE KEY-----")));
    t!(secret_ec_key, assert!(found("private-key", "-----BEGIN EC PRIVATE KEY-----")));
    t!(secret_dsa_key, assert!(found("private-key", "-----BEGIN DSA PRIVATE KEY-----")));
    t!(secret_pgp_key, assert!(found("private-key", "-----BEGIN PGP PRIVATE KEY-----")));
    t!(secret_openssh_key, assert!(found("private-key", "x\n-----BEGIN OPENSSH PRIVATE KEY-----\ny")));
    t!(secret_ghp, assert!(found("github-token", "ghp_abcdefghijklmnopqrstuvwxyz0123456789")));
    t!(secret_github_pat, assert!(found("github-token", "use github_pat_11ABCDEFG0123456789abcdef here")));
    t!(secret_sk, assert!(found("api-key", "key sk-abcdefghijklmnopqrstuvwx")));
    t!(secret_slack, assert!(found("api-key", "xoxb-123456789012-abcdefghijkl")));
    t!(secret_api_key_colon, assert!(found("credential-assignment", "api_key: abcdefgh12345")));
    t!(secret_api_dash_key, assert!(found("credential-assignment", "API-KEY=abcdefgh12345")));
    t!(secret_secret_eq, assert!(found("credential-assignment", "SECRET=abcdefghij1")));
    t!(secret_password, assert!(found("credential-assignment", "password: hunter2hunter2")));
    t!(secret_passwd, assert!(found("credential-assignment", "passwd: abcdefgh")));
    t!(secret_token_spaces, assert!(found("credential-assignment", "  token = abcdefgh1234  ")));
    t!(secret_phrase_12, assert!(found("recovery-phrase", "one two three four five six seven eight nine ten eleven twelve")));
    t!(secret_phrase_24, assert!(found("recovery-phrase", &["word"; 24].join(" "))));
    t!(secret_phrase_tabs, assert!(found("recovery-phrase", &["abc"; 15].join("\t"))));
    t!(secret_in_middle_of_note, assert!(found("credential-assignment", "# Notes\nhello\npassword: hunter2hunter2\nbye")));

    // --- secret guard: things that must pass
    t!(clean_plain_note, assert!(clean("just a normal note about passwords")));
    t!(clean_env_placeholder, assert!(clean("password: ${PASSWORD}")));
    t!(clean_angle_placeholder, assert!(clean("password: <your-password>")));
    t!(clean_short_token, assert!(clean("token: short")));
    t!(clean_prose_about_keys, assert!(clean("the api key is stored in the vault elsewhere")));
    t!(clean_short_ghp, assert!(clean("ghp_short")));
    t!(clean_short_sk, assert!(clean("sk-short")));
    t!(clean_public_key, assert!(clean("-----BEGIN PUBLIC KEY-----")));
    t!(clean_eleven_words, assert!(clean(&["word"; 11].join(" "))));
    t!(clean_twentyfive_words, assert!(clean(&["word"; 25].join(" "))));
    t!(clean_capitalised_sentence, assert!(clean("One two three four five six seven eight nine ten eleven twelve")));
    t!(clean_punctuated_sentence, assert!(clean("one, two, three, four, five, six, seven, eight, nine, ten, eleven, twelve")));
    t!(clean_binary_with_nul, assert!(hits(&patterns(), b"\0password: hunter2hunter2").is_empty()));
    t!(clean_empty, assert!(clean("")));
    t!(clean_markdown_heading, assert!(clean("# Secrets handling\n\nWe never store them.")));

    // --- path safety
    t!(secret_env_prefixed, assert!(found("credential-assignment", "OPENAI_API_KEY=abcdefgh12345678")));
    t!(secret_env_export, assert!(found("credential-assignment", "export DB_PASSWORD='hunter2hunter2'")));
    t!(secret_env_powershell, assert!(found("credential-assignment", "$env:GITHUB_TOKEN = \"abcdefgh12345678\"")));
    t!(secret_env_set, assert!(found("credential-assignment", "set STRIPE_SECRET=abcdefgh12345678")));
    t!(clean_env_config_name, assert!(clean("PUBLIC_API_KEY_NAME=my-header-name")));
    t!(clean_env_prefixed_placeholder, assert!(clean("OPENAI_API_KEY=${OPENAI_API_KEY}")));
    t!(clean_env_ellipsis_placeholder, assert!(clean("OPENROUTER_API_KEY=sk-or-...") && clean("```env\nTELEGRAM_BOT_TOKEN=12345...\n```")));
    t!(clean_env_lowercase_setting, assert!(clean("max_token=1234567890")));
    t!(portable_ok, assert!(portable(Path::new("500-knowledge/ışık/-dash.v2.md")).is_ok() && portable(Path::new("./a.md")).is_ok() && portable(Path::new("console.md")).is_ok()));
    t!(portable_bad_device, assert!(portable(Path::new("CON.md")).is_err() && portable(Path::new("a/lpt1.txt")).is_err() && portable(Path::new("com0.md")).is_ok()));
    t!(portable_bad_chars, assert!(portable(Path::new("what?.md")).is_err() && portable(Path::new("a:b.md")).is_err() && portable(Path::new("x/tab\tname.md")).is_err()));
    t!(portable_bad_trailing, assert!(portable(Path::new("note.")).is_err() && portable(Path::new("dir /a.md")).is_err()));
    t!(path_ok_simple, assert!(safe_rel("a.md").is_ok()));
    t!(path_ok_nested, assert!(safe_rel("a/b/c.md").is_ok()));
    t!(path_ok_vault_dir, assert!(safe_rel("500-knowledge/x.md").is_ok()));
    t!(path_ok_space, assert!(safe_rel("with space.md").is_ok()));
    t!(path_ok_dots, assert!(safe_rel("dots.in.name.md").is_ok()));
    t!(path_ok_unicode, assert!(safe_rel("ışık/şemsiye.md").is_ok()));
    t!(path_ok_leading_dash, assert!(safe_rel("-dash.md").is_ok()));
    t!(path_ok_curdir, assert!(safe_rel("./a.md").is_ok()));
    t!(path_bad_parent, assert!(safe_rel("../x").is_err()));
    t!(path_bad_deep_parent, assert!(safe_rel("a/../../x").is_err()));
    t!(path_bad_only_parents, assert!(safe_rel("../../..").is_err()));
    t!(path_bad_absolute, assert!(safe_rel("/etc/passwd").is_err()));
    t!(path_bad_empty, assert!(safe_rel("").is_err()));
    t!(path_bad_secrets_dir, assert!(safe_rel(".secrets/a.md").is_err()));
    t!(path_bad_nested_secrets, assert!(safe_rel("a/.secrets/x").is_err()));
    t!(path_bad_smug_dir, assert!(safe_rel(".smug/state.redb").is_err()));
    t!(path_bad_git_dir, assert!(safe_rel(".git/config").is_err()));
    t!(path_bad_nested_git, assert!(safe_rel("x/.git/hooks/pre-commit").is_err()));

    // --- dates (checked against an independent implementation)
    t!(ymd_0, assert_eq!(ymd(0), "1970-01-01"));
    t!(ymd_86399, assert_eq!(ymd(86399), "1970-01-01"));
    t!(ymd_86400, assert_eq!(ymd(86400), "1970-01-02"));
    t!(ymd_951782400, assert_eq!(ymd(951782400), "2000-02-29"));
    t!(ymd_951868800, assert_eq!(ymd(951868800), "2000-03-01"));
    t!(ymd_1078012800, assert_eq!(ymd(1078012800), "2004-02-29"));
    t!(ymd_1709164800, assert_eq!(ymd(1709164800), "2024-02-29"));
    t!(ymd_1709251200, assert_eq!(ymd(1709251200), "2024-03-01"));
    t!(ymd_4102444800, assert_eq!(ymd(4102444800), "2100-01-01"));
    t!(ymd_1794455602, assert_eq!(ymd(1794455602), "2026-11-12"));
    t!(ymd_1735689600, assert_eq!(ymd(1735689600), "2025-01-01"));
    t!(ymd_1767225599, assert_eq!(ymd(1767225599), "2025-12-31"));
    t!(ymd_1767225600, assert_eq!(ymd(1767225600), "2026-01-01"));
    t!(ymd_978307200, assert_eq!(ymd(978307200), "2001-01-01"));
    t!(ymd_1234567890, assert_eq!(ymd(1234567890), "2009-02-13"));

    // --- frontmatter
    t!(fm_none, assert_eq!(split_fm("hello").1, "hello"));
    t!(fm_none_map_empty, assert!(split_fm("hello").0.is_empty()));
    t!(fm_unterminated, assert_eq!(split_fm("---\nid: a\nno end").1, "---\nid: a\nno end"));
    t!(fm_basic, {
        let (m, b) = split_fm("---\nid: a\n---\n\nbody");
        assert_eq!((m["id"].as_str(), b), ("a", "body"));
    });
    t!(fm_value_with_colon, assert_eq!(split_fm("---\nsource: https://x.y/z\n---\nb").0["source"], "https://x.y/z"));
    t!(fm_quoted_value, assert_eq!(split_fm("---\ntitle: \"Hello\"\n---\nb").0["title"], "Hello"));
    t!(fm_multiple_keys, assert_eq!(split_fm("---\na: 1\nb: 2\nc: 3\n---\nx").0.len(), 3));
    t!(fm_body_keeps_dashes, assert_eq!(split_fm("---\na: 1\n---\nx\n---\ny").1, "x\n---\ny"));
    t!(list_empty, assert!(list("[]").is_empty()));
    t!(list_one, assert_eq!(list("[a]"), ["a"]));
    t!(list_two, assert_eq!(list("[a, b]"), ["a", "b"]));
    t!(list_bare, assert_eq!(list("a,b"), ["a", "b"]));
    t!(list_quoted, assert_eq!(list("[\"a\", \"b\"]"), ["a", "b"]));
    t!(list_spaced, assert_eq!(list(" [ a , b ] "), ["a", "b"]));
    t!(set_fm_replace, assert!(set_fm("---\nstatus: active\n---\nx", "status", "superseded").contains("status: superseded")));
    t!(set_fm_replace_removes_old, assert!(!set_fm("---\nstatus: active\n---\nx", "status", "done").contains("active")));
    t!(set_fm_add, assert!(set_fm("---\nid: a\n---\nx", "valid_to", "2026-01-01").contains("valid_to: 2026-01-01")));
    t!(set_fm_keeps_body, assert!(set_fm("---\nid: a\n---\n\nhello", "k", "v").ends_with("hello")));
    t!(set_fm_creates_block, assert!(set_fm("no fm", "k", "v").starts_with("---\nk: v\n---\n")));
    t!(set_fm_unterminated_untouched, assert_eq!(set_fm("---\nid: a", "k", "v"), "---\nid: a"));
    t!(set_fm_roundtrip, {
        let t = set_fm(&set_fm("---\nid: a\n---\nb", "x", "1"), "y", "2");
        let (m, b) = split_fm(&t);
        assert_eq!((m["id"].as_str(), m["x"].as_str(), m["y"].as_str(), b), ("a", "1", "2", "b"));
    });

    // --- hashing
    t!(sha_empty, assert_eq!(sha(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));
    t!(sha_abc, assert_eq!(sha(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"));

    #[test]
    fn file_lock_fallback() {
        let d = std::env::temp_dir().join(format!("smug-lock-{}", std::process::id()));
        fs::create_dir_all(d.join(".smug")).unwrap();
        let a = Lock::take_by_file(&d, 100, 30).unwrap();
        assert!(Lock::take_by_file(&d, 100, 30).is_err(), "held lock must block");
        drop(a);
        let b = Lock::take_by_file(&d, 100, 30).unwrap();
        // a crashed holder: file left behind and old enough -> stolen
        std::mem::forget(b);
        let f = fs::OpenOptions::new().write(true).open(d.join(".smug/lock.file")).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(120)).unwrap();
        assert!(Lock::take_by_file(&d, 100, 30).is_ok());
        let _ = fs::remove_dir_all(&d);
    }
}
