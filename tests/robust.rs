mod common;
use common::*;
use std::{fs, path::Path};

macro_rules! t {
    ($n:ident, $b:expr) => {
        #[test]
        fn $n() {
            $b
        }
    };
}

fn patched_vault(name: &str) -> std::path::PathBuf {
    let v = vault(name);
    smug(&v, &["decide", "a", "--source", "s", "--topic", "t", "--date", "2026-01-01"], "alpha text");
    fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"alpha\"}\n").unwrap();
    assert_eq!(smug(&v, &["patch", "a", "--new-id", "b", "--source", "s2", "--date", "2026-06-01"], "beta text").0, 0);
    v
}

fn set_patch_status(v: &Path, status: &str) {
    let p = v.join("patches/001.json");
    let t = fs::read_to_string(&p).unwrap();
    fs::write(&p, t.replace("\"proposed\"", &format!("\"{status}\"")).replace("\"applied\"", &format!("\"{status}\""))).unwrap();
}

// ---------- crash-safe apply ----------
t!(apply_journals_applying_before_touching_notes, {
    let v = patched_vault("rj");
    assert_eq!(smug(&v, &["apply", "1"], "").0, 0);
    let j = fs::read_to_string(v.join("history/events.jsonl")).unwrap();
    let pos = |needle: &str| j.find(needle).unwrap_or_else(|| panic!("{needle}"));
    assert!(pos("patches/001.json") < pos("decisions/b.md"), "patch status must be journalled first");
});
t!(doctor_completes_an_apply_that_died_before_any_note_changed, {
    let v = patched_vault("rd0");
    set_patch_status(&v, "applying");
    let (c, o, _) = smug(&v, &["doctor"], "");
    assert_eq!(c, 0, "{o}");
    assert!(o.contains("1 interrupted patch apply"), "{o}");
    assert!(fs::read_to_string(v.join("500-knowledge/decisions/a.md")).unwrap().contains("status: superseded"));
    assert!(v.join("500-knowledge/decisions/b.md").exists());
    assert!(fs::read_to_string(v.join("patches/001.json")).unwrap().contains("\"applied\""));
});
t!(doctor_completes_an_apply_that_died_halfway, {
    let v = patched_vault("rd1");
    assert_eq!(smug(&v, &["apply", "1"], "").0, 0);
    // rewind: new note written, old one not yet marked, patch still "applying"
    let old = v.join("500-knowledge/decisions/a.md");
    let t = fs::read_to_string(&old).unwrap().replace("status: superseded", "status: active").lines().filter(|l| !l.starts_with("valid_to:") && !l.starts_with("superseded_by:")).collect::<Vec<_>>().join("\n");
    fs::write(&old, t).unwrap();
    set_patch_status(&v, "applying");
    assert_eq!(smug(&v, &["doctor"], "").0, 0);
    let fixed = fs::read_to_string(&old).unwrap();
    assert!(fixed.contains("status: superseded") && fixed.contains("superseded_by: b"), "{fixed}");
    assert_eq!(fs::read_dir(v.join("500-knowledge/decisions")).unwrap().count(), 2, "no duplicate decision may appear");
});
t!(apply_command_resumes_without_needing_the_eval_gate_again, {
    let v = patched_vault("rd2");
    fs::remove_file(v.join("evals/cases.jsonl")).unwrap();
    set_patch_status(&v, "applying");
    assert_eq!(smug(&v, &["apply", "1"], "").0, 0);
    assert!(v.join("500-knowledge/decisions/b.md").exists());
});
t!(doctor_second_run_has_nothing_to_resume, {
    let v = patched_vault("rd3");
    set_patch_status(&v, "applying");
    smug(&v, &["doctor"], "");
    assert!(smug(&v, &["doctor"], "").1.contains("0 interrupted"));
});
t!(apply_refuses_a_different_note_already_using_the_new_id, {
    let v = patched_vault("rd4");
    fs::write(v.join("500-knowledge/decisions/b.md"), "---\ntype: decision\nid: b\nsource: x\n---\nsomeone else").unwrap();
    assert_eq!(smug(&v, &["apply", "1"], "").0, 1);
    assert!(fs::read_to_string(v.join("500-knowledge/decisions/a.md")).unwrap().contains("status: active"));
});

// ---------- the search index heals itself ----------
fn vault_with_note(name: &str) -> std::path::PathBuf {
    let v = vault(name);
    fs::write(v.join("500-knowledge/a.md"), "healing keyword").unwrap();
    fs::write(v.join("500-knowledge/b.md"), "another healing note").unwrap();
    assert!(smug(&v, &["search", "healing"], "").1.contains("a.md"));
    v
}
t!(index_dir_deleted_is_rebuilt, {
    let v = vault_with_note("h1");
    fs::remove_dir_all(v.join(".smug/tantivy")).unwrap();
    assert!(smug(&v, &["search", "healing"], "").1.contains("a.md"));
});
t!(index_meta_corrupted_is_rebuilt, {
    let v = vault_with_note("h2");
    fs::write(v.join(".smug/tantivy/meta.json"), "{ this is not json").unwrap();
    assert!(smug(&v, &["search", "healing"], "").1.contains("a.md"));
});
t!(index_segment_file_lost_is_rebuilt, {
    let v = vault_with_note("h3");
    let mut files: Vec<_> = fs::read_dir(v.join(".smug/tantivy")).unwrap().flatten().filter(|e| e.file_name() != "meta.json" && e.path().is_file() && !e.file_name().to_string_lossy().starts_with('.')).collect();
    files.sort_by_key(|e| std::cmp::Reverse(e.metadata().unwrap().len()));
    fs::remove_file(files[0].path()).unwrap();
    let o = smug(&v, &["search", "healing"], "").1;
    assert!(o.contains("a.md") && o.contains("b.md"), "{o}");
});
t!(index_segment_file_truncated_is_rebuilt, {
    let v = vault_with_note("h4");
    for e in fs::read_dir(v.join(".smug/tantivy")).unwrap().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if e.path().is_file() && n != "meta.json" && !n.starts_with('.') {
            fs::write(e.path(), b"").unwrap();
        }
    }
    assert!(smug(&v, &["search", "healing"], "").1.contains("a.md"));
});
t!(state_lost_and_a_note_deleted_leaves_no_ghost_results, {
    let v = vault_with_note("h5");
    fs::remove_file(v.join(".smug/state.redb")).unwrap();
    fs::remove_file(v.join("500-knowledge/a.md")).unwrap();
    let o = smug(&v, &["search", "healing"], "").1;
    assert!(o.contains("b.md") && !o.contains("a.md"), "{o}");
});
t!(index_is_consistent_after_every_edit_cycle, {
    let v = vault_with_note("h6");
    for i in 0..5 {
        fs::write(v.join(format!("500-knowledge/n{i}.md")), format!("cycle {i} healing")).unwrap();
        smug(&v, &["search", "healing"], "");
    }
    fs::remove_file(v.join("500-knowledge/n2.md")).unwrap();
    let o = smug(&v, &["search", "cycle"], "").1;
    assert!(!o.contains("n2.md") && o.contains("n3.md"), "{o}");
});
t!(doctor_reports_index_consistency, {
    let v = vault_with_note("h7");
    fs::remove_dir_all(v.join(".smug/tantivy")).unwrap();
    let (c, o, _) = smug(&v, &["doctor"], "");
    assert!(c == 0 && o.contains("search index consistent"), "{o}");
});

// ---------- model-in-the-loop eval ----------
#[cfg(unix)]
mod model {
    use super::*;
    fn mvault(name: &str) -> std::path::PathBuf {
        let v = vault(name);
        smug(&v, &["decide", "postgres", "--source", "adr-7", "--title", "database"], "we use postgres for storage");
        v
    }
    t!(model_answer_checked_and_passing, {
        let v = mvault("m1");
        fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"which database do we use\",\"expect\":[\"500-knowledge/decisions/postgres.md\"],\"answer_contains\":[\"postgres\"]}\n").unwrap();
        // `cat` plays a model that parrots its context: the answer contains whatever retrieval put in front of it
        let (c, o, _) = smug(&v, &["eval", "--model", "cat"], "");
        assert_eq!(c, 0, "{o}");
    });
    t!(model_answer_missing_fact_fails, {
        let v = mvault("m2");
        fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"which database do we use\",\"answer_contains\":[\"mysql\"]}\n").unwrap();
        let (c, o, _) = smug(&v, &["eval", "--model", "cat"], "");
        assert!(c == 1 && o.contains("lacked"), "{o}");
    });
    t!(model_answer_forbidden_fact_fails, {
        let v = mvault("m3");
        fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"which database do we use\",\"answer_absent\":[\"postgres\"]}\n").unwrap();
        assert_eq!(smug(&v, &["eval", "--model", "cat"], "").0, 1);
    });
    t!(failing_model_command_is_an_error, {
        let v = mvault("m4");
        fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"database\",\"answer_contains\":[\"x\"]}\n").unwrap();
        assert_eq!(smug(&v, &["eval", "--model", "false"], "").0, 1);
    });
    t!(answer_checks_are_skipped_without_model, {
        let v = mvault("m5");
        fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"database\",\"answer_contains\":[\"never-appears\"]}\n").unwrap();
        assert_eq!(smug(&v, &["eval"], "").0, 0);
    });
    t!(model_eval_catches_a_regression_a_patch_introduces, {
        let v = mvault("m6");
        fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"which database do we use\",\"answer_contains\":[\"we use postgres\"]}\n").unwrap();
        smug(&v, &["patch", "postgres", "--new-id", "mysql", "--source", "adr-8"], "we use mysql for storage");
        let (c, o, _) = smug(&v, &["eval", "1", "--model", "cat"], "");
        assert!(c == 1 && o.contains("REGRESSED"), "{o}");
        assert_eq!(smug(&v, &["apply", "1", "--model", "cat"], "").0, 1);
        assert!(!v.join("500-knowledge/decisions/mysql.md").exists());
    });
}

// ---------- automatic outcome feedback (Stop hook) ----------
fn transcript(v: &Path, lines: &[&str]) -> String {
    let p = v.join("transcript.jsonl");
    fs::write(&p, lines.join("\n")).unwrap();
    format!("{{\"transcript_path\":{}}}", serde_json::to_string(p.to_str().unwrap()).unwrap())
}
fn surfaced_vault(name: &str) -> std::path::PathBuf {
    let v = vault(name);
    fs::write(v.join("500-knowledge/tantivy.md"), "tantivy notes").unwrap();
    smug(&v, &["hook", "prompt"], r#"{"prompt":"how does tantivy work"}"#);
    v
}
t!(stop_hook_credits_a_note_the_assistant_read, {
    let v = surfaced_vault("s1");
    let input = transcript(&v, &[r#"{"type":"user","message":{"role":"user","content":"how does tantivy work"}}"#, r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Read","input":{"file_path":"/x/500-knowledge/tantivy.md"}}]}}"#]);
    assert_eq!(smug(&v, &["hook", "stop"], &input).0, 0);
    assert!(smug(&v, &["log", "5"], "").1.contains("feedback"));
    assert!(smug(&v, &["feedback", "500-knowledge/tantivy.md", "used"], "").1.contains("utility 2"));
});
t!(stop_hook_credits_a_note_named_in_the_answer, {
    let v = surfaced_vault("s2");
    let input = transcript(&v, &[r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"According to tantivy.md the index is incremental."}]}}"#]);
    smug(&v, &["hook", "stop"], &input);
    assert!(smug(&v, &["log", "5"], "").1.contains("feedback"));
});
t!(stop_hook_does_not_count_the_users_or_the_hooks_own_mention, {
    let v = surfaced_vault("s3");
    let input = transcript(&v, &[r#"{"type":"user","message":{"role":"user","content":"smug: note candidates\n- 500-knowledge/tantivy.md"}}"#, r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"I answered from memory."}]}}"#]);
    smug(&v, &["hook", "stop"], &input);
    assert!(!smug(&v, &["log", "5"], "").1.contains("feedback"));
});
t!(stop_hook_without_transcript_is_silent_and_ok, {
    let v = surfaced_vault("s4");
    let (c, o, e) = smug(&v, &["hook", "stop"], "{}");
    assert!(c == 0 && o.is_empty() && e.is_empty());
});
t!(stop_hook_clears_the_surfaced_list_so_it_credits_once, {
    let v = surfaced_vault("s5");
    let input = transcript(&v, &[r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"tantivy.md"}]}}"#]);
    smug(&v, &["hook", "stop"], &input);
    smug(&v, &["hook", "stop"], &input);
    assert_eq!(smug(&v, &["log", "50"], "").1.matches("feedback").count(), 1);
});
t!(stop_hook_survives_garbage_input, assert_eq!(smug(&vault("s6"), &["hook", "stop"], "not json at all").0, 0));
t!(hook_unknown_subcommand_errors, assert_eq!(smug(&vault("s7"), &["hook", "nope"], "").0, 1));

// --- round 1 regressions
#[test]
fn bare_query_operators_are_words_not_syntax_errors() {
    let v = common::vault("r1-ops");
    std::fs::write(v.join("500-knowledge/a.md"), "this and that or not").unwrap();
    for q in ["AND", "OR", "NOT", "foo NOT", "AND foo"] {
        assert_eq!(common::smug(&v, &["search", q], "").0, 0, "query {q}");
    }
    assert!(!v.join(".smug/state.redb.corrupt").exists());
}

#[test]
fn crlf_frontmatter_is_read() {
    let v = common::vault("r1-crlf");
    common::smug(&v, &["decide", "db", "--source", "adr"], "why");
    let p = v.join("500-knowledge/decisions/db.md");
    let t = std::fs::read_to_string(&p).unwrap().replace('\n', "\r\n");
    std::fs::write(&p, t).unwrap();
    assert!(common::smug(&v, &["check", "db"], "").1.starts_with("CURRENT"));
}

#[test]
fn heading_link_counts_as_dependency() {
    let v = common::vault("r1-impact");
    common::smug(&v, &["decide", "db", "--source", "adr"], "why");
    std::fs::write(v.join("500-knowledge/dep.md"), "see [[db#why]]").unwrap();
    assert!(common::smug(&v, &["impact", "db"], "").1.contains("dep.md"));
}

#[test]
fn bad_dates_are_refused() {
    let v = common::vault("r1-date");
    assert_ne!(common::smug(&v, &["decide", "d", "--source", "s", "--date", "tomorrow"], "w").0, 0);
    assert_ne!(common::smug(&v, &["decide", "d", "--source", "s", "--known-at", "2026-1-1"], "w").0, 0);
    assert_eq!(common::smug(&v, &["decide", "d", "--source", "s", "--date", "2026-11-12"], "w").0, 0);
}

#[test]
fn patch_numbers_survive_a_deleted_earlier_patch() {
    let v = common::vault("r1-patch");
    common::smug(&v, &["decide", "db", "--source", "adr"], "why");
    for i in 1..=3 {
        assert_eq!(common::smug(&v, &["patch", "db", "--new-id", &format!("n{i}"), "--source", "s"], "x").0, 0);
    }
    std::fs::remove_file(v.join("patches/001.json")).unwrap();
    let (c, o, _) = common::smug(&v, &["patch", "db", "--new-id", "m", "--source", "s"], "y");
    assert_eq!(c, 0, "{o}");
    assert!(v.join("patches/004.json").exists());
}

#[test]
fn nul_byte_cannot_hide_a_secret_from_the_write_guard() {
    let v = common::vault("r2-nul");
    let (c, _, e) = common::smug(&v, &["write", "x.md", "--expect", "new"], "\0password: hunter2hunter2\n");
    assert_ne!(c, 0, "{e}");
    assert!(!v.join("x.md").exists());
}

#[test]
fn a_second_capture_keeps_the_first() {
    let v = common::vault("r3-cap");
    common::smug(&v, &["capture", "--label", "s"], "early part of the session");
    common::smug(&v, &["capture", "--label", "s"], "late part of the session");
    common::smug(&v, &["capture", "--label", "s"], "late part of the session"); // same text again: no duplicate
    let f = std::fs::read_dir(v.join("000-inbox/captures")).unwrap().flatten().next().unwrap().path();
    let t = std::fs::read_to_string(f).unwrap();
    assert!(t.contains("early part") && t.contains("late part"));
    assert_eq!(t.matches("late part").count(), 1);
}
