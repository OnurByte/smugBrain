mod common;
use common::*;
use std::{fs, io::Write, process::Command};

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
fn crash_recovery_and_journal_replay() {
    let v = vault("crash");
    smug(&v, &["write", "a.md", "--expect", "new"], "x");
    // a write that died after journalling its intent, then the whole .smug/ state is lost
    let mut j = fs::OpenOptions::new().append(true).open(v.join("history/events.jsonl")).unwrap();
    writeln!(j, r#"{{"id":99,"ts":0,"kind":"intent","path":"x.md","hash":"deadbeef","ref":null}}"#).unwrap();
    fs::remove_dir_all(v.join(".smug")).unwrap();
    let (code, out, _) = smug(&v, &["doctor"], "");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("1 unfinished"), "{out}");
    let (_, out2, _) = smug(&v, &["doctor"], "");
    assert!(out2.contains("0 unfinished"), "{out2}");
    assert!(smug(&v, &["log", "50"], "").1.contains("abort"));
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

#[test]
fn decisions_check_and_time_machine() {
    let v = vault("dec");
    assert_eq!(smug(&v, &["decide", "sqlite", "--topic", "db", "--date", "2026-01-01"], "use sqlite").0, 1, "source is required");
    assert_eq!(smug(&v, &["decide", "sqlite", "--topic", "db", "--source", "adr-1", "--date", "2026-01-01", "--known-at", "2026-01-02"], "use sqlite everywhere").0, 0);
    assert_eq!(smug(&v, &["decide", "sqlite", "--source", "x"], "dup").0, 2);
    assert_eq!(smug(&v, &["decide", "../evil", "--source", "x"], "x").0, 1);
    assert_eq!(smug(&v, &["check", "sqlite"], "").0, 0);
    assert_eq!(smug(&v, &["decide", "mysql", "--topic", "db", "--source", "adr-2"], "use mysql").0, 0);
    let (c, out, _) = smug(&v, &["check", "sqlite"], "");
    assert_eq!(c, 3);
    assert!(out.contains("CONFLICTED"), "{out}");
    assert!(smug(&v, &["at", "2026-02-01", "sqlite"], "").1.contains("sqlite"));
    assert!(smug(&v, &["at", "2025-12-31", "sqlite"], "").1.is_empty(), "not valid yet");
    assert!(smug(&v, &["at", "2026-01-01", "sqlite"], "").1.is_empty(), "valid but nobody knew yet");
}

#[test]
fn patch_eval_apply_and_stale_never_returned() {
    let v = vault("patch");
    smug(&v, &["decide", "sqlite", "--topic", "db", "--source", "adr-1", "--date", "2026-01-01"], "we use sqlite for storage");
    fs::write(v.join("300-projects/backup.md"), "---\ndepends_on: [sqlite]\n---\nbackup plan for sqlite file copy").unwrap();
    fs::write(v.join("300-projects/deploy.md"), "see [[backup]] before deploy").unwrap();
    let imp = smug(&v, &["impact", "sqlite"], "").1;
    assert!(imp.contains("300-projects/backup.md") && imp.contains("  300-projects/deploy.md"), "{imp}");
    fs::write(v.join("evals/cases.jsonl"), concat!(
        r#"{"q":"which storage do we use","expect":["500-knowledge/decisions/postgres.md"],"absent":["500-knowledge/decisions/sqlite.md"]}"#, "\n",
        r#"{"q":"backup plan","expect":["300-projects/backup.md"]}"#, "\n")).unwrap();
    let (c, out, _) = smug(&v, &["patch", "sqlite", "--new-id", "postgres", "--source", "adr-7", "--date", "2026-11-01", "--title", "storage"], "we use postgres for storage");
    assert_eq!(c, 0, "{out}");
    assert!(out.contains("300-projects/backup.md"), "{out}");
    assert!(!v.join("500-knowledge/decisions/postgres.md").exists(), "a patch must not apply anything");
    let (c, out, _) = smug(&v, &["eval", "1"], "");
    assert_eq!(c, 0, "{out}");
    assert!(out.contains("FIXED"), "{out}");
    assert_eq!(smug(&v, &["eval"], "").0, 1, "before the patch the first case fails");
    assert_eq!(smug(&v, &["apply", "1"], "").0, 0);
    let (_, out, _) = smug(&v, &["search", "storage"], "");
    assert!(out.contains("postgres.md") && !out.contains("sqlite.md"), "{out}");
    assert!(smug(&v, &["search", "--history", "sqlite"], "").1.contains("sqlite.md"));
    let (c, out, _) = smug(&v, &["check", "sqlite"], "");
    assert!(c == 3 && out.contains("STALE"), "{out}");
    assert_eq!(smug(&v, &["apply", "1"], "").0, 1, "already applied");
}

#[test]
fn eval_blocks_a_regressing_patch() {
    let v = vault("reg");
    smug(&v, &["decide", "a", "--source", "s", "--date", "2026-01-01"], "alpha topic text");
    fs::write(v.join("evals/cases.jsonl"), concat!(r#"{"q":"alpha topic","expect":["500-knowledge/decisions/a.md"]}"#, "\n")).unwrap();
    smug(&v, &["patch", "a", "--new-id", "b", "--source", "s2"], "totally different words");
    let (c, out, _) = smug(&v, &["eval", "1"], "");
    assert_eq!(c, 1, "{out}");
    assert!(out.contains("REGRESSED"), "{out}");
    assert_eq!(smug(&v, &["apply", "1"], "").0, 1);
    assert!(v.join("500-knowledge/decisions/a.md").exists() && !v.join("500-knowledge/decisions/b.md").exists());
}

#[test]
fn negative_memory_warns() {
    let v = vault("neg");
    assert_eq!(smug(&v, &["fail", "retry-loop", "--method", "retry forever on timeout", "--reason", "hammered the API", "--version", "1.2"], "").0, 0);
    let (_, out, _) = smug(&v, &["warn", "lets retry forever on the timeout"], "");
    assert!(out.contains("WARN retry-loop") && out.contains("hammered the API"), "{out}");
    assert!(smug(&v, &["warn", "write documentation"], "").1.is_empty());
}

#[test]
fn prompt_hook_and_feedback() {
    let v = vault("hook");
    fs::write(v.join("500-knowledge/tantivy.md"), "tantivy index notes").unwrap();
    let (c, out, _) = smug(&v, &["hook", "prompt"], r#"{"prompt":"how does tantivy work"}"#);
    assert_eq!(c, 0);
    assert!(out.contains("500-knowledge/tantivy.md"), "{out}");
    assert_eq!(smug(&v, &["hook", "prompt"], "not json").0, 0);
    assert!(smug(&v, &["feedback", "500-knowledge/tantivy.md", "useful"], "").1.contains("utility 2"));
}

#[test]
fn turkish_folding_and_bench() {
    let v = vault("tr");
    fs::write(v.join("500-knowledge/tr.md"), "Karar: ışık şemsiye çocuk").unwrap();
    assert!(smug(&v, &["search", "isik", "semsiye", "cocuk"], "").1.contains("tr.md"));
    fs::write(v.join("evals/cases.jsonl"), concat!(r#"{"q":"semsiye","expect":["500-knowledge/tr.md"]}"#, "\n")).unwrap();
    let (c, out, _) = smug(&v, &["bench"], "");
    assert_eq!(c, 0);
    assert!(out.contains("recall@k 1.000") && out.contains("MRR 1.000"), "{out}");
}
