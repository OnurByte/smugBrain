mod common;
use common::*;
use std::{fs, process::Command};

macro_rules! t {
    ($n:ident, $b:expr) => {
        #[test]
        fn $n() {
            $b
        }
    };
}

fn git(v: &std::path::Path, a: &[&str]) -> std::process::Output {
    Command::new("git").arg("-C").arg(v).args(a).output().unwrap()
}

// ---------- init creates every template file ----------
macro_rules! init_has {
    ($($n:ident => $p:expr),* $(,)?) => {$(
        t!($n, { let v = vault(stringify!($n)); assert!(v.join($p).is_file(), "missing {}", $p); });
    )*};
}
init_has! {
    init_claude_md => "CLAUDE.md",
    init_agents_md => "AGENTS.md",
    init_gitignore => ".gitignore",
    init_secrets_readme => ".secrets/README.md",
    init_core => "850-companion/Core.md",
    init_last_session => "850-companion/Last-Session.md",
    init_threads => "850-companion/Threads.md",
    init_rules => "850-companion/Rules.md",
    init_inbox => "000-inbox/README.md",
    init_command_center => "100-command-center/README.md",
    init_projects => "300-projects/README.md",
    init_knowledge => "500-knowledge/README.md",
    init_arsenal => "600-arsenal/README.md",
    init_archive => "900-archive/README.md",
    init_daily => "daily/README.md",
    init_templates => "templates/README.md",
    init_evals => "evals/README.md",
}

t!(init_refuses_non_empty_dir, {
    let d = tmp("initne");
    fs::write(d.join("keep.txt"), "x").unwrap();
    assert_eq!(smug(&d, &["init", d.to_str().unwrap()], "").0, 1);
    assert_eq!(fs::read_to_string(d.join("keep.txt")).unwrap(), "x");
});
t!(init_into_existing_empty_dir_ok, {
    let d = tmp("initempty");
    assert_eq!(smug(&d, &["init", d.to_str().unwrap()], "").0, 0);
});
t!(init_gitignore_hides_state_and_secrets, {
    let g = fs::read_to_string(vault("ign").join(".gitignore")).unwrap();
    assert!(g.contains(".smug/") && g.contains(".secrets/*"));
});
t!(init_template_tells_agents_about_smug, {
    let c = fs::read_to_string(vault("tpl").join("CLAUDE.md")).unwrap();
    for w in ["smug search", "smug write", "smug check", "smug patch", "smug warn"] {
        assert!(c.contains(w), "{w}");
    }
});

// ---------- cli basics ----------
t!(version_prints, assert!(smug(&tmp("ver"), &["--version"], "").1.starts_with("smug ")));
t!(version_word, assert!(smug(&tmp("ver2"), &["version"], "").1.starts_with("smug ")));
t!(no_args_prints_usage, {
    let (c, o, _) = smug(&tmp("usage"), &[], "");
    assert_eq!(c, 0);
    assert!(o.contains("decide") && o.contains("patch") && o.contains("backup"));
});
t!(unknown_command_prints_usage, assert_eq!(smug(&tmp("unk"), &["frobnicate"], "").0, 0));

// ---------- write ----------
t!(write_creates_parent_dirs, {
    let v = vault("wdirs");
    assert_eq!(smug(&v, &["write", "a/b/c/d.md", "--expect", "new"], "x").0, 0);
    assert_eq!(fs::read_to_string(v.join("a/b/c/d.md")).unwrap(), "x");
});
t!(write_prints_sha256_of_content, {
    let v = vault("wsha");
    let (_, h, _) = smug(&v, &["write", "a.md", "--expect", "new"], "abc");
    assert_eq!(h.trim(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
});
t!(write_refuses_git_dir, assert_eq!(smug(&vault("wgit"), &["write", ".git/x", "--force"], "x").0, 1));
t!(write_refuses_smug_dir, assert_eq!(smug(&vault("wsmug"), &["write", ".smug/x", "--force"], "x").0, 1));
t!(write_refuses_absolute, assert_eq!(smug(&vault("wabs"), &["write", "/tmp/smug-x", "--force"], "x").0, 1));
t!(write_new_on_existing_conflicts, {
    let v = vault("wnew");
    smug(&v, &["write", "a.md", "--expect", "new"], "1");
    assert_eq!(smug(&v, &["write", "a.md", "--expect", "new"], "2").0, 2);
    assert_eq!(fs::read_to_string(v.join("a.md")).unwrap(), "1");
});
t!(write_hash_on_missing_file_conflicts, assert_eq!(smug(&vault("wmiss"), &["write", "a.md", "--expect", "abc123"], "x").0, 2));
t!(write_leaves_no_temp_files, {
    let v = vault("wtmp");
    smug(&v, &["write", "a.md", "--expect", "new"], "x");
    let n = fs::read_dir(&v).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains("smug-tmp")).count();
    assert_eq!(n, 0);
});
t!(write_conflict_leaves_file_untouched, {
    let v = vault("wuntouched");
    smug(&v, &["write", "a.md", "--expect", "new"], "orig");
    smug(&v, &["write", "a.md", "--expect", "wronghash"], "changed");
    assert_eq!(fs::read_to_string(v.join("a.md")).unwrap(), "orig");
});
t!(write_force_overwrites, {
    let v = vault("wforce");
    smug(&v, &["write", "a.md", "--expect", "new"], "1");
    assert_eq!(smug(&v, &["write", "a.md", "--force"], "2").0, 0);
    assert_eq!(fs::read_to_string(v.join("a.md")).unwrap(), "2");
});
t!(write_without_filename_errors, assert_eq!(smug(&vault("wnoarg"), &["write"], "").0, 1));
t!(write_refuses_private_key, assert_eq!(smug(&vault("wpk"), &["write", "a.md", "--force"], "-----BEGIN RSA PRIVATE KEY-----").0, 1));
t!(write_refuses_recovery_phrase, assert_eq!(smug(&vault("wph"), &["write", "a.md", "--force"], "one two three four five six seven eight nine ten eleven twelve\n").0, 1));
t!(write_secret_leaves_no_file, {
    let v = vault("wnofile");
    smug(&v, &["write", "a.md", "--force"], "password: hunter2hunter2");
    assert!(!v.join("a.md").exists());
});

// ---------- journal / log ----------
t!(log_shows_intent_and_commit, {
    let v = vault("log");
    smug(&v, &["write", "a.md", "--expect", "new"], "x");
    let o = smug(&v, &["log"], "").1;
    assert!(o.contains("intent") && o.contains("commit") && o.contains("a.md"), "{o}");
});
t!(log_is_in_the_vault_for_git, {
    let v = vault("jrn");
    smug(&v, &["write", "a.md", "--expect", "new"], "x");
    assert_eq!(fs::read_to_string(v.join("history/events.jsonl")).unwrap().lines().count(), 2);
});
t!(log_limit_respected, {
    let v = vault("loglim");
    for i in 0..5 {
        smug(&v, &["write", &format!("n{i}.md"), "--expect", "new"], "x");
    }
    assert_eq!(smug(&v, &["log", "3"], "").1.lines().count(), 3);
});
t!(journal_ids_increase, {
    let v = vault("ids");
    for i in 0..3 {
        smug(&v, &["write", &format!("n{i}.md"), "--expect", "new"], "x");
    }
    let ids: Vec<u64> = fs::read_to_string(v.join("history/events.jsonl")).unwrap().lines().map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["id"].as_u64().unwrap()).collect();
    assert_eq!(ids, [1, 2, 3, 4, 5, 6]);
});

// ---------- scan / backup ----------
macro_rules! scan_flags {
    ($($n:ident => ($text:expr, $cat:expr)),* $(,)?) => {$(
        t!($n, {
            let v = vault(stringify!($n));
            fs::write(v.join("500-knowledge/leak.md"), $text).unwrap();
            let (c, o, _) = smug(&v, &["scan"], "");
            assert_eq!(c, 1, "{o}");
            assert!(o.contains($cat) && o.contains("leak.md"), "{o}");
        });
    )*};
}
scan_flags! {
    scan_flags_private_key => ("-----BEGIN OPENSSH PRIVATE KEY-----\n", "private-key"),
    scan_flags_github_token => ("ghp_abcdefghijklmnopqrstuvwxyz0123456789\n", "github-token"),
    scan_flags_api_key => ("sk-abcdefghijklmnopqrstuvwx\n", "api-key"),
    scan_flags_password => ("password: hunter2hunter2\n", "credential-assignment"),
    scan_flags_phrase => ("one two three four five six seven eight nine ten eleven twelve\n", "recovery-phrase"),
}
t!(scan_clean_vault_passes, {
    let (c, o, _) = smug(&vault("scanok"), &["scan"], "");
    assert_eq!(c, 0, "{o}");
    assert!(o.contains("0 secret-shaped"));
});
t!(scan_never_prints_the_secret, {
    let v = vault("scanquiet");
    fs::write(v.join("a.md"), "password: hunter2hunter2\n").unwrap();
    assert!(!smug(&v, &["scan"], "").1.contains("hunter2"));
});
t!(scan_ignores_secrets_dir, {
    let v = vault("scansec");
    fs::write(v.join(".secrets/mine.md"), "password: hunter2hunter2\n").unwrap();
    git(&v, &["init", "-q"]);
    assert_eq!(smug(&v, &["scan"], "").0, 0);
});
t!(backup_commits_clean_vault, {
    let v = vault("bkok");
    git(&v, &["init", "-q"]);
    git(&v, &["config", "user.email", "t@t"]);
    git(&v, &["config", "user.name", "t"]);
    smug(&v, &["backup"], ""); // push fails without a remote; the commit is what we check
    assert!(git(&v, &["log", "-1", "--format=%s"]).stdout.starts_with(b"Vault backup"));
});
t!(backup_never_commits_secrets_dir, {
    let v = vault("bksec");
    git(&v, &["init", "-q"]);
    git(&v, &["config", "user.email", "t@t"]);
    git(&v, &["config", "user.name", "t"]);
    fs::write(v.join(".secrets/mine.md"), "password: hunter2hunter2\n").unwrap();
    smug(&v, &["backup"], "");
    let files = String::from_utf8_lossy(&git(&v, &["ls-files"]).stdout).to_string();
    assert!(!files.contains("mine.md") && !files.contains(".smug"), "{files}");
});
t!(backup_commits_the_journal, {
    let v = vault("bkjrn");
    git(&v, &["init", "-q"]);
    git(&v, &["config", "user.email", "t@t"]);
    git(&v, &["config", "user.name", "t"]);
    smug(&v, &["write", "a.md", "--expect", "new"], "x");
    smug(&v, &["backup"], "");
    assert!(String::from_utf8_lossy(&git(&v, &["ls-files"]).stdout).contains("history/events.jsonl"));
});
t!(backup_blocks_then_succeeds_after_secret_moved, {
    let v = vault("bkfix");
    git(&v, &["init", "-q"]);
    git(&v, &["config", "user.email", "t@t"]);
    git(&v, &["config", "user.name", "t"]);
    fs::write(v.join("leak.md"), "password: hunter2hunter2\n").unwrap();
    assert_eq!(smug(&v, &["backup"], "").0, 1);
    fs::rename(v.join("leak.md"), v.join(".secrets/leak.md")).unwrap();
    smug(&v, &["backup"], "");
    assert!(git(&v, &["log", "-1"]).status.success());
});

// ---------- search / index ----------
t!(search_without_words_errors, assert_eq!(smug(&vault("snow"), &["search"], "").0, 1));
t!(search_no_match_is_empty_and_ok, {
    let (c, o, _) = smug(&vault("snone"), &["search", "zzzxqj"], "");
    assert!(c == 0 && o.is_empty());
});
t!(search_is_case_insensitive, {
    let v = vault("scase");
    fs::write(v.join("500-knowledge/a.md"), "Tantivy Rocks").unwrap();
    assert!(smug(&v, &["search", "TANTIVY"], "").1.contains("a.md"));
});
t!(search_ranks_better_match_first, {
    let v = vault("srank");
    fs::write(v.join("500-knowledge/weak.md"), "rust once").unwrap();
    fs::write(v.join("500-knowledge/strong.md"), "rust rust rust rust rust").unwrap();
    let o = smug(&v, &["search", "rust"], "").1;
    assert!(o.find("strong.md").unwrap() < o.find("weak.md").unwrap(), "{o}");
});
t!(search_survives_punctuation, {
    let v = vault("spunct");
    fs::write(v.join("500-knowledge/a.md"), "hello world").unwrap();
    assert_eq!(smug(&v, &["search", "hello\"(world)*"], "").0, 0);
});
t!(search_skips_archive, {
    let v = vault("sarch");
    fs::write(v.join("900-archive/old.md"), "uniqueword").unwrap();
    assert!(smug(&v, &["search", "uniqueword"], "").1.is_empty());
});
t!(search_skips_secrets_dir, {
    let v = vault("ssec");
    fs::write(v.join(".secrets/a.md"), "uniqueword").unwrap();
    assert!(smug(&v, &["search", "uniqueword"], "").1.is_empty());
});
t!(search_new_file_found_after_first_index, {
    let v = vault("snew");
    smug(&v, &["search", "x"], "");
    fs::write(v.join("500-knowledge/late.md"), "latecomer").unwrap();
    assert!(smug(&v, &["search", "latecomer"], "").1.contains("late.md"));
});
t!(index_reports_changes, {
    let v = vault("idx2");
    assert!(smug(&v, &["index"], "").1.contains("changed"));
    assert!(smug(&v, &["index"], "").1.contains("(0 changed)"));
});
t!(search_deep_without_qmd_explains, {
    let v = vault("deep");
    let o = Command::new(env!("CARGO_BIN_EXE_smug")).args(["search", "--deep", "x"]).env("SMUG_VAULT", &v).env("PATH", "/nonexistent").output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("QMD"));
});
t!(deleted_state_dir_is_rebuilt, {
    let v = vault("rebuild");
    fs::write(v.join("500-knowledge/a.md"), "persistent").unwrap();
    smug(&v, &["search", "persistent"], "");
    fs::remove_dir_all(v.join(".smug")).unwrap();
    assert!(smug(&v, &["search", "persistent"], "").1.contains("a.md"));
});

// ---------- decisions ----------
macro_rules! bad_ids {
    ($($n:ident => $id:expr),* $(,)?) => {$(
        t!($n, assert_eq!(smug(&vault(stringify!($n)), &["decide", $id, "--source", "s"], "x").0, 1));
    )*};
}
bad_ids! {
    id_rejects_dotdot => "../x",
    id_rejects_slash => "a/b",
    id_rejects_space => "a b",
    id_rejects_dot => "a.b",
    id_rejects_empty => "",
    id_rejects_backslash => "a\\b",
}
t!(decide_without_id_errors, assert_eq!(smug(&vault("dnoid"), &["decide"], "").0, 1));
t!(decide_writes_frontmatter, {
    let v = vault("dfm");
    smug(&v, &["decide", "pg", "--source", "adr-1", "--topic", "db", "--title", "Use PG", "--depends", "a,b"], "because");
    let t = fs::read_to_string(v.join("500-knowledge/decisions/pg.md")).unwrap();
    for w in ["type: decision", "id: pg", "status: active", "topic: db", "source: adr-1", "title: Use PG", "depends_on: [a,b]", "because"] {
        assert!(t.contains(w), "{w} in {t}");
    }
});
t!(decide_refuses_secret_in_body, assert_eq!(smug(&vault("dsec"), &["decide", "x", "--source", "s"], "password: hunter2hunter2").0, 1));
t!(check_unknown_id, {
    let (c, o, _) = smug(&vault("cunk"), &["check", "nope"], "");
    assert!(c == 3 && o.contains("UNKNOWN"), "{o}");
});
t!(check_unsupported_without_source, {
    let v = vault("cuns");
    fs::write(v.join("500-knowledge/d.md"), "---\ntype: decision\nid: d\nstatus: active\n---\nno source").unwrap();
    let (c, o, _) = smug(&v, &["check", "d"], "");
    assert!(c == 3 && o.contains("UNSUPPORTED"), "{o}");
});
t!(check_current_prints_source, {
    let v = vault("ccur");
    smug(&v, &["decide", "d", "--source", "adr-9"], "x");
    assert!(smug(&v, &["check", "d"], "").1.contains("CURRENT") && smug(&v, &["check", "d"], "").1.contains("adr-9"));
});
t!(impact_of_unreferenced_decision, {
    let v = vault("iunref");
    smug(&v, &["decide", "d", "--source", "s"], "x");
    assert!(smug(&v, &["impact", "d"], "").1.contains("nothing depends on d"));
});
t!(impact_follows_wikilinks_and_depends_on, {
    let v = vault("ilinks");
    smug(&v, &["decide", "d", "--source", "s"], "x");
    fs::write(v.join("300-projects/a.md"), "uses [[d]]").unwrap();
    fs::write(v.join("300-projects/b.md"), "---\ndepends_on: [a]\n---\nx").unwrap();
    let o = smug(&v, &["impact", "d"], "").1;
    assert!(o.contains("300-projects/a.md") && o.contains("  300-projects/b.md"), "{o}");
});
t!(impact_handles_cycles, {
    let v = vault("icycle");
    fs::write(v.join("300-projects/a.md"), "---\ndepends_on: [b]\n---\n").unwrap();
    fs::write(v.join("300-projects/b.md"), "---\ndepends_on: [a]\n---\n").unwrap();
    assert_eq!(smug(&v, &["impact", "a"], "").0, 0);
});
t!(at_without_date_errors, assert_eq!(smug(&vault("atnd"), &["at"], "").0, 1));
t!(at_lists_everything_valid_when_no_words, {
    let v = vault("atall");
    smug(&v, &["decide", "a", "--source", "s", "--date", "2026-01-01", "--known-at", "2026-01-01"], "x");
    smug(&v, &["decide", "b", "--source", "s", "--date", "2026-03-01", "--known-at", "2026-03-01"], "y");
    let o = smug(&v, &["at", "2026-02-01"], "").1;
    assert!(o.contains("a [decision]") && !o.contains("b [decision]"), "{o}");
});

// ---------- patch / eval / apply guards ----------
t!(patch_unknown_decision_errors, assert_eq!(smug(&vault("pu"), &["patch", "ghost", "--new-id", "x", "--source", "s"], "x").0, 1));
t!(patch_requires_source, {
    let v = vault("ps");
    smug(&v, &["decide", "a", "--source", "s"], "x");
    assert_eq!(smug(&v, &["patch", "a", "--new-id", "b"], "y").0, 1);
});
t!(patch_applies_nothing, {
    let v = vault("pn");
    smug(&v, &["decide", "a", "--source", "s"], "x");
    smug(&v, &["patch", "a", "--new-id", "b", "--source", "s2"], "y");
    assert!(!v.join("500-knowledge/decisions/b.md").exists());
    assert!(fs::read_to_string(v.join("500-knowledge/decisions/a.md")).unwrap().contains("status: active"));
});
t!(patch_numbers_increase, {
    let v = vault("pnum");
    smug(&v, &["decide", "a", "--source", "s"], "x");
    smug(&v, &["patch", "a", "--new-id", "b", "--source", "s2"], "y");
    smug(&v, &["patch", "a", "--new-id", "c", "--source", "s3"], "z");
    assert!(v.join("patches/001.json").exists() && v.join("patches/002.json").exists());
});
t!(eval_without_cases_errors, assert_eq!(smug(&vault("enc"), &["eval"], "").0, 1));
t!(eval_with_empty_cases_passes, {
    let v = vault("eempty");
    fs::write(v.join("evals/cases.jsonl"), "").unwrap();
    assert_eq!(smug(&v, &["eval"], "").0, 0);
});
t!(eval_leaves_no_simulation_behind, {
    let v = vault("esim");
    smug(&v, &["decide", "a", "--source", "s"], "x");
    fs::write(v.join("evals/cases.jsonl"), "").unwrap();
    smug(&v, &["patch", "a", "--new-id", "b", "--source", "s2"], "y");
    smug(&v, &["eval", "1"], "");
    assert!(!v.join(".smug/sim").exists());
});
t!(apply_unknown_patch_errors, {
    let v = vault("aunk");
    fs::write(v.join("evals/cases.jsonl"), "").unwrap();
    assert_eq!(smug(&v, &["apply", "7"], "").0, 1);
});
t!(apply_bad_number_errors, assert_eq!(smug(&vault("abad"), &["apply", "x"], "").0, 1));
t!(apply_marks_patch_applied, {
    let v = vault("amark");
    smug(&v, &["decide", "a", "--source", "s"], "alpha");
    fs::write(v.join("evals/cases.jsonl"), "").unwrap();
    smug(&v, &["patch", "a", "--new-id", "b", "--source", "s2"], "beta");
    assert_eq!(smug(&v, &["apply", "1"], "").0, 0);
    assert!(fs::read_to_string(v.join("patches/001.json")).unwrap().contains("\"applied\""));
    let old = fs::read_to_string(v.join("500-knowledge/decisions/a.md")).unwrap();
    assert!(old.contains("status: superseded") && old.contains("superseded_by: b") && old.contains("valid_to:"));
    let new = fs::read_to_string(v.join("500-knowledge/decisions/b.md")).unwrap();
    assert!(new.contains("supersedes: a") && new.contains("status: active"));
});
t!(apply_force_without_cases, {
    let v = vault("aforce");
    smug(&v, &["decide", "a", "--source", "s"], "alpha");
    smug(&v, &["patch", "a", "--new-id", "b", "--source", "s2"], "beta");
    assert_eq!(smug(&v, &["apply", "1"], "").0, 1);
    assert_eq!(smug(&v, &["apply", "1", "--force"], "").0, 0);
});
t!(superseded_decision_is_flagged_stale, {
    let v = vault("astale");
    smug(&v, &["decide", "a", "--source", "s"], "alpha");
    fs::write(v.join("evals/cases.jsonl"), "").unwrap();
    smug(&v, &["patch", "a", "--new-id", "b", "--source", "s2"], "beta");
    smug(&v, &["apply", "1"], "");
    assert!(smug(&v, &["check", "a"], "").1.contains("STALE"));
    assert!(smug(&v, &["check", "b"], "").1.contains("CURRENT"));
});

// ---------- negative memory / feedback ----------
t!(fail_requires_method_and_reason, {
    let v = vault("fmr");
    assert_eq!(smug(&v, &["fail", "x", "--method", "m"], "").0, 1);
    assert_eq!(smug(&v, &["fail", "x", "--reason", "r"], "").0, 1);
});
t!(warn_names_version_and_reason, {
    let v = vault("wv");
    smug(&v, &["fail", "f", "--method", "pin the compiler nightly", "--reason", "broke CI", "--version", "1.2"], "");
    let o = smug(&v, &["warn", "should we pin the compiler to nightly"], "").1;
    assert!(o.contains("broke CI") && o.contains("1.2"), "{o}");
});
t!(feedback_rejects_unknown_kind, assert_eq!(smug(&vault("fbk"), &["feedback", "a.md", "meh"], "").0, 1));
t!(feedback_rejects_escape, assert_eq!(smug(&vault("fbe"), &["feedback", "../a.md", "used"], "").0, 1));
t!(feedback_accumulates, {
    let v = vault("fbacc");
    smug(&v, &["feedback", "a.md", "used"], "");
    assert!(smug(&v, &["feedback", "a.md", "harmful"], "").1.contains("utility -2"));
});
t!(feedback_reorders_results, {
    let v = vault("fbord");
    fs::write(v.join("500-knowledge/a.md"), "shared term only once").unwrap();
    fs::write(v.join("500-knowledge/b.md"), "shared term only once").unwrap();
    for _ in 0..3 {
        smug(&v, &["feedback", "500-knowledge/b.md", "useful"], "");
    }
    let o = smug(&v, &["search", "shared"], "").1;
    assert!(o.find("b.md").unwrap() < o.find("a.md").unwrap(), "{o}");
});

// ---------- hooks / setup / context / doctor ----------
t!(hook_requires_subcommand, assert_eq!(smug(&vault("hsub"), &["hook"], "").0, 1));
t!(hook_prompt_silent_without_match, assert!(smug(&vault("hsil"), &["hook", "prompt"], r#"{"prompt":"zzzxqj"}"#).1.is_empty()));
t!(hook_prompt_silent_on_empty_prompt, assert!(smug(&vault("hemp"), &["hook", "prompt"], r#"{"prompt":""}"#).1.is_empty()));
t!(hook_prompt_warns_on_known_failure, {
    let v = vault("hwarn");
    smug(&v, &["fail", "f", "--method", "delete the lockfile", "--reason", "corrupted state"], "");
    assert!(smug(&v, &["hook", "prompt"], r#"{"prompt":"let us delete the lockfile now"}"#).1.contains("corrupted state"));
});
t!(setup_claude_writes_both_hooks, {
    let v = vault("sc");
    assert_eq!(smug(&v, &["setup", "claude"], "").0, 0);
    let j: serde_json::Value = serde_json::from_str(&fs::read_to_string(v.join(".claude/settings.json")).unwrap()).unwrap();
    assert!(j["hooks"]["SessionStart"].is_array() && j["hooks"]["UserPromptSubmit"].is_array());
});
t!(setup_claude_never_overwrites, {
    let v = vault("scow");
    fs::create_dir_all(v.join(".claude")).unwrap();
    fs::write(v.join(".claude/settings.json"), "{\"mine\":true}").unwrap();
    smug(&v, &["setup", "claude"], "");
    assert_eq!(fs::read_to_string(v.join(".claude/settings.json")).unwrap(), "{\"mine\":true}");
});
t!(setup_codex_is_idempotent, {
    let v = vault("scx");
    smug(&v, &["setup", "codex"], "");
    smug(&v, &["setup", "codex"], "");
    assert_eq!(fs::read_to_string(v.join("AGENTS.md")).unwrap().matches("## smug").count(), 1);
});
t!(setup_unknown_target_errors, assert_eq!(smug(&vault("su"), &["setup", "vim"], "").0, 1));
t!(context_prints_companion_files, {
    let v = vault("ctx");
    fs::write(v.join("850-companion/Core.md"), "I am the core").unwrap();
    assert!(smug(&v, &["context"], "").1.contains("I am the core"));
});
t!(context_is_capped, {
    let v = vault("ctxcap");
    fs::write(v.join("850-companion/Core.md"), "x".repeat(20000)).unwrap();
    assert!(smug(&v, &["context"], "").1.chars().count() <= 9500);
});
t!(context_lists_active_decisions_not_stale_ones, {
    let v = vault("ctxdec");
    smug(&v, &["decide", "live", "--source", "s", "--title", "Live one"], "x");
    fs::write(v.join("500-knowledge/decisions/old.md"), "---\ntype: decision\nid: old\nstatus: superseded\ntitle: Old one\n---\n").unwrap();
    let o = smug(&v, &["context"], "").1;
    assert!(o.contains("Live one") && !o.contains("Old one"), "{o}");
});
t!(doctor_ok_on_fresh_vault, assert_eq!(smug(&vault("dok"), &["doctor"], "").0, 0));
t!(doctor_fails_when_gitignore_misses_secrets, {
    let v = vault("dgi");
    fs::write(v.join(".gitignore"), "target/\n").unwrap();
    assert_eq!(smug(&v, &["doctor"], "").0, 1);
});
t!(doctor_reports_conflicted_decisions, {
    let v = vault("dconf");
    smug(&v, &["decide", "a", "--source", "s", "--topic", "db"], "x");
    smug(&v, &["decide", "b", "--source", "s", "--topic", "db"], "y");
    let (c, o, _) = smug(&v, &["doctor"], "");
    assert!(c == 1 && o.contains("CONFLICTED"), "{o}");
});
t!(bench_without_cases_errors, assert_eq!(smug(&vault("bnc"), &["bench"], "").0, 1));
