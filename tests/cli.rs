use std::{fs, io::Write, path::{Path, PathBuf}, process::{Command, Stdio}};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("smug-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn smug(vault: &Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_smug"))
        .args(args).env("SMUG_VAULT", vault)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    c.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    let o = c.wait_with_output().unwrap();
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into())
}

fn vault(name: &str) -> PathBuf {
    let d = tmp(name);
    let v = d.join("v");
    assert_eq!(smug(&d, &["init", v.to_str().unwrap()], "").0, 0);
    v
}

#[test]
fn expect_is_required() {
    let v = vault("req");
    assert_eq!(smug(&v, &["write", "a.md"], "x").0, 1);
    assert_eq!(smug(&v, &["write", "a.md", "--force"], "x").0, 0);
}

#[test]
fn cas_conflict_and_hash() {
    let v = vault("cas");
    let (c, h, _) = smug(&v, &["write", "a.md", "--expect", "new"], "one");
    assert_eq!(c, 0);
    assert_eq!(smug(&v, &["write", "a.md", "--expect", "new"], "two").0, 2);
    assert_eq!(smug(&v, &["write", "a.md", "--expect", "deadbeef"], "two").0, 2);
    assert_eq!(smug(&v, &["write", "a.md", "--expect", h.trim()], "two").0, 0);
    assert_eq!(fs::read_to_string(v.join("a.md")).unwrap(), "two");
}

#[test]
fn eight_writers_one_wins() {
    let v = vault("conc");
    let (_, h, _) = smug(&v, &["write", "a.md", "--expect", "new"], "base");
    let hs: Vec<_> = (0..8).map(|i| {
        let (v, h) = (v.clone(), h.trim().to_string());
        std::thread::spawn(move || smug(&v, &["write", "a.md", "--expect", &h], &format!("w{i}")).0)
    }).collect();
    let codes: Vec<i32> = hs.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(codes.iter().filter(|&&c| c == 0).count(), 1, "{codes:?}");
    assert_eq!(codes.iter().filter(|&&c| c == 2).count(), 7, "{codes:?}");
}

#[test]
fn escapes_and_secrets_refused() {
    let v = vault("esc");
    assert_eq!(smug(&v, &["write", "../x.md", "--force"], "x").0, 1);
    assert_eq!(smug(&v, &["write", ".secrets/x.md", "--force"], "x").0, 1);
    assert_eq!(smug(&v, &["write", "a.md", "--force"], "password: hunter2hunter2").0, 1);
}

#[cfg(unix)]
#[test]
fn parent_symlink_refused() {
    let d = tmp("sym");
    let v = d.join("v");
    smug(&d, &["init", v.to_str().unwrap()], "");
    let outside = d.join("outside");
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, v.join("300-projects/link")).unwrap();
    assert_eq!(smug(&v, &["write", "300-projects/link/pwn.md", "--expect", "new"], "x").0, 1);
    assert!(!outside.join("pwn.md").exists());
}

#[test]
fn stale_lock_file_does_not_block() {
    let v = vault("lock");
    fs::create_dir_all(v.join(".smug")).unwrap();
    fs::write(v.join(".smug/lock"), "left over from a crash").unwrap();
    assert_eq!(smug(&v, &["write", "a.md", "--expect", "new"], "x").0, 0);
}

#[test]
fn crash_recovery() {
    let v = vault("crash");
    smug(&v, &["write", "a.md", "--expect", "new"], "x");
    let c = rusqlite_free_insert(&v);
    assert!(c);
    let (code, out, _) = smug(&v, &["doctor"], "");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("1 unfinished"), "{out}");
}

// Insert a dangling intent through the sqlite3 CLI-free route: reuse smug's own DB via a tiny SQL file is not possible,
// so we simulate a crash with the same schema using the `sqlite3` binary when present; skip otherwise.
fn rusqlite_free_insert(v: &Path) -> bool {
    match Command::new("sqlite3").arg(v.join(".smug/state.db")).arg("INSERT INTO events(ts,kind,path,hash) VALUES(0,'intent','x.md','deadbeef')").status() {
        Ok(s) => s.success(),
        Err(_) => true, // no sqlite3 binary: nothing to simulate; doctor test below still runs
    }
}

#[test]
fn persistent_index_updates() {
    let v = vault("idx");
    fs::write(v.join("500-knowledge/a.md"), "alpha rust note").unwrap();
    let (_, out, err) = smug(&v, &["search", "alpha"], "");
    assert!(out.contains("a.md") && err.contains("reindexed"), "{out}{err}");
    let (_, _, err2) = smug(&v, &["search", "alpha"], "");
    assert!(err2.contains("(0 reindexed)"), "{err2}");
    fs::write(v.join("500-knowledge/a.md"), "beta only now, longer").unwrap();
    assert!(smug(&v, &["search", "alpha"], "").1.is_empty());
    assert!(smug(&v, &["search", "beta"], "").1.contains("a.md"));
    fs::remove_file(v.join("500-knowledge/a.md")).unwrap();
    assert!(smug(&v, &["search", "beta"], "").1.is_empty());
}

#[test]
fn secret_notes_not_indexed_and_archive_skipped() {
    let v = vault("sec");
    fs::write(v.join("500-knowledge/s.md"), "token: abcdefgh12345678\nzebra").unwrap();
    fs::write(v.join("900-archive/old.md"), "zebra").unwrap();
    assert!(smug(&v, &["search", "zebra"], "").1.is_empty());
}

#[test]
fn backup_blocks_secrets() {
    let v = vault("bk");
    let git = |a: &[&str]| Command::new("git").arg("-C").arg(&v).args(a).output().unwrap();
    git(&["init", "-q"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    fs::write(v.join("500-knowledge/leak.md"), "password: hunter2hunter2\n").unwrap();
    let (c, out, _) = smug(&v, &["backup"], "");
    assert_eq!(c, 1);
    assert!(out.contains("BLOCKED"), "{out}");
    assert!(!git(&["log", "-1"]).status.success(), "nothing may be committed");
    assert!(git(&["diff", "--cached", "--name-only"]).stdout.is_empty(), "nothing may stay staged");
}
