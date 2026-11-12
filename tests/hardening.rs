mod common;
use common::*;
use serde_json::{json, Value};
use std::{fs, io::Write, path::Path, process::{Command, Stdio}};

macro_rules! t {
    ($n:ident, $b:expr) => {
        #[test]
        fn $n() {
            $b
        }
    };
}

/// Tiny deterministic generator so fuzz tests are reproducible.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick<'a>(&mut self, xs: &'a [&'a str]) -> &'a str {
        xs[(self.next() as usize) % xs.len()]
    }
}

fn run_in(cwd: &Path, args: &[&str], envs: &[(&str, &str)], stdin: &str) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_smug"));
    c.args(args).current_dir(cwd).env_remove("SMUG_VAULT").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for (k, v) in envs {
        c.env(k, v);
    }
    let mut ch = c.spawn().unwrap();
    let _ = ch.stdin.take().unwrap().write_all(stdin.as_bytes());
    let o = ch.wait_with_output().unwrap();
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into())
}

/// canonicalize() on Windows returns a verbatim `\\?\` path; smug (rightly) hands tools the plain form.
fn canon(p: &Path) -> std::path::PathBuf {
    std::path::PathBuf::from(fs::canonicalize(p).unwrap().to_string_lossy().trim_start_matches(r"\\?\"))
}

fn is_empty_dir(d: &Path) -> bool {
    fs::read_dir(d).unwrap().next().is_none()
}

// ====================== the vault path (found in review) ======================
t!(vault_flag_before_the_command, {
    let v = vault("vf1");
    let other = tmp("vf1-other");
    let (c, _, _) = run_in(&other, &["--vault", v.to_str().unwrap(), "write", "a.md", "--expect", "new"], &[], "x");
    assert_eq!(c, 0);
    assert!(v.join("a.md").exists() && is_empty_dir(&other));
});
t!(vault_flag_after_the_command, {
    let v = vault("vf2");
    let other = tmp("vf2-other");
    assert_eq!(run_in(&other, &["write", "a.md", "--expect", "new", "--vault", v.to_str().unwrap()], &[], "x").0, 0);
    assert!(v.join("a.md").exists());
});
t!(vault_flag_wins_over_the_env_var, {
    let (v, w) = (vault("vf3"), vault("vf3b"));
    run_in(&tmp("vf3-o"), &["--vault", v.to_str().unwrap(), "write", "a.md", "--expect", "new"], &[("SMUG_VAULT", w.to_str().unwrap())], "x");
    assert!(v.join("a.md").exists() && !w.join("a.md").exists());
});
t!(vault_flag_without_a_value_errors, assert_eq!(run_in(&tmp("vf4"), &["search", "--vault"], &[], "").0, 1));
t!(relative_vault_flag_resolves_from_the_cwd, {
    let v = vault("vf5");
    let parent = v.parent().unwrap().to_path_buf();
    assert_eq!(run_in(&parent, &["--vault", "v", "write", "a.md", "--expect", "new"], &[], "x").0, 0);
    assert!(v.join("a.md").exists());
});
t!(mcp_serves_the_vault_it_was_given_from_any_directory, {
    let v = vault("vm1");
    fs::write(v.join("500-knowledge/a.md"), "uniquevaultword").unwrap();
    let other = tmp("vm1-other");
    let req = rpc(1, "tools/call", json!({"name": "smug_recall", "arguments": {"query": "uniquevaultword"}})).to_string();
    let (_, o, _) = run_in(&other, &["mcp", "--vault", v.to_str().unwrap()], &[], &format!("{req}\n"));
    let r: Value = serde_json::from_str(o.lines().next().unwrap()).unwrap();
    assert_eq!(r["result"]["structuredContent"]["notes"][0]["path"], "500-knowledge/a.md");
    assert!(is_empty_dir(&other), "the server must not write into the directory it was started from");
});
t!(mcp_reports_which_vault_it_serves, {
    let v = vault("vm2");
    let r = tool(&v, &[], "smug_recall", json!({"query": "x"}));
    assert_eq!(Path::new(r["structuredContent"]["vault"].as_str().unwrap()), canon(&v));
});
t!(mcp_in_a_non_vault_refuses_to_start_and_touches_nothing, {
    let d = tmp("vm3");
    let (c, _, e) = run_in(&d, &["mcp"], &[], "");
    assert!(c == 1 && e.contains("not a smug vault") && e.contains("--vault"), "{e}");
    assert!(is_empty_dir(&d));
});
t!(commands_refuse_a_non_vault_and_create_nothing, {
    for args in [vec!["write", "a.md", "--expect", "new"], vec!["search", "x"], vec!["decide", "d", "--source", "s"], vec!["doctor"], vec!["index"], vec!["log"], vec!["capture"]] {
        let d = tmp("nv");
        let (c, _, e) = run_in(&d, &args, &[], "x");
        assert_eq!(c, 1, "{args:?}");
        assert!(e.contains("not a smug vault"), "{args:?}: {e}");
        assert!(is_empty_dir(&d), "{args:?} created files in somebody's project");
    }
});
t!(hooks_in_a_non_vault_are_silent_and_touch_nothing, {
    let d = tmp("nvh");
    for sub in ["prompt", "stop", "pre-edit", "session-end", "pre-compact"] {
        let (c, o, e) = run_in(&d, &["hook", sub], &[], r#"{"prompt":"x","session_id":"s","tool_input":{"file_path":"a.rs"}}"#);
        assert!(c == 0 && o.is_empty() && e.is_empty(), "{sub}");
    }
    assert!(is_empty_dir(&d));
});
t!(context_in_a_non_vault_is_silent, {
    let d = tmp("nvc");
    let (c, o, _) = run_in(&d, &["context"], &[], "");
    assert!(c == 0 && o.is_empty() && is_empty_dir(&d));
});
t!(an_existing_obsidian_style_vault_is_recognised, {
    let d = tmp("obs");
    fs::create_dir_all(d.join("850-companion")).unwrap();
    assert_eq!(run_in(&d, &["write", "a.md", "--expect", "new"], &[("SMUG_VAULT", d.to_str().unwrap())], "x").0, 0);
});
fn installed_vault_arg(v: &Path, id: &str, h: &Path, rel: &str) -> String {
    let (c, o, _) = smug_env(v, &["agent", "install", id], "", &[("SMUG_HOME", h.to_str().unwrap())]);
    assert_eq!(c, 0, "{o}");
    json_args(&v.join(rel))
}
fn json_args(p: &Path) -> String {
    let j: Value = serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap();
    assert_eq!(j["mcpServers"]["smug"]["args"][1], "--vault");
    j["mcpServers"]["smug"]["args"][2].as_str().unwrap().to_string()
}
t!(installed_config_carries_the_absolute_vault_path, {
    let v = vault("vi1");
    let got = installed_vault_arg(&v, "cursor", &tmp("vi1h"), ".cursor/mcp.json");
    assert!(Path::new(&got).is_absolute());
    assert_eq!(Path::new(&got), canon(&v));
});
t!(installed_toml_carries_the_vault_path, {
    let (v, h) = (vault("vi2"), tmp("vi2h"));
    smug_env(&v, &["agent", "install", "codex", "--global"], "", &[("SMUG_HOME", h.to_str().unwrap())]);
    let t = fs::read_to_string(h.join(".codex/config.toml")).unwrap();
    assert!(t.contains("--vault") && t.contains(canon(&v).to_str().unwrap()), "{t}");
});
t!(vault_path_with_spaces_works_end_to_end, {
    let d = tmp("vi3");
    let v = d.join("my brain");
    smug(&d, &["init", v.to_str().unwrap()], "");
    let (c, o, _) = smug_env(&v, &["agent", "install", "cursor"], "", &[("SMUG_HOME", d.to_str().unwrap())]);
    assert_eq!(c, 0, "{o}");
    assert!(json_args(&v.join(".cursor/mcp.json")).ends_with("my brain"));
    assert_eq!(smug_env(&v, &["agent", "doctor"], "", &[]).0, 0);
});
t!(unicode_vault_path_works_end_to_end, {
    let d = tmp("vi4");
    let v = d.join("ışık-şemsiye-beyin");
    smug(&d, &["init", v.to_str().unwrap()], "");
    assert_eq!(smug_env(&v, &["agent", "install", "cursor"], "", &[("SMUG_HOME", d.to_str().unwrap())]).0, 0);
    assert_eq!(smug_env(&v, &["agent", "doctor"], "", &[]).0, 0);
});
t!(relative_vault_is_made_absolute_in_the_config, {
    let v = vault("vi5");
    let (c, _, _) = run_in(&v, &["agent", "install", "cursor"], &[("SMUG_VAULT", "."), ("SMUG_HOME", v.parent().unwrap().to_str().unwrap())], "");
    assert_eq!(c, 0);
    assert!(Path::new(&json_args(&v.join(".cursor/mcp.json"))).is_absolute());
});
#[cfg(unix)]
t!(symlinked_vault_is_resolved_in_the_config, {
    let d = tmp("vi6");
    let v = d.join("real");
    smug(&d, &["init", v.to_str().unwrap()], "");
    let link = d.join("link");
    std::os::unix::fs::symlink(&v, &link).unwrap();
    let (c, _, _) = smug_env(&link, &["agent", "install", "cursor"], "", &[("SMUG_HOME", d.to_str().unwrap())]);
    assert_eq!(c, 0);
    assert_eq!(Path::new(&json_args(&link.join(".cursor/mcp.json"))), canon(&v));
});
t!(agent_doctor_checks_the_vault_from_another_directory, {
    let v = vault("vd1");
    let (c, o, _) = smug_env(&v, &["agent", "doctor"], "", &[]);
    assert!(c == 0 && o.contains("serves the right vault from any directory") && o.contains("6 tools"), "{o}");
});
t!(agent_doctor_fails_on_a_non_vault, {
    let d = tmp("vd2");
    let (c, o, _) = smug_env(&d, &["agent", "doctor"], "", &[]);
    assert!(c == 1 && o.contains("FAIL"), "{o}");
});
t!(claude_hooks_carry_the_vault_and_the_binary, {
    let v = vault("vh1");
    smug(&v, &["setup", "claude"], "");
    let s = fs::read_to_string(v.join(".claude/settings.json")).unwrap();
    let canon = canon(&v).to_string_lossy().replace('\\', "\\\\");
    assert!(s.contains("--vault") && s.contains(&canon) && s.contains("hook stop") && s.contains("hook pre-edit"), "{s}");
});
t!(old_style_hooks_are_upgraded_not_duplicated, {
    let v = vault("vh2");
    fs::create_dir_all(v.join(".claude")).unwrap();
    fs::write(v.join(".claude/settings.json"), r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"smug hook stop"}]},{"hooks":[{"type":"command","command":"my-own"}]}]}}"#).unwrap();
    smug(&v, &["setup", "claude"], "");
    let s: Value = serde_json::from_str(&fs::read_to_string(v.join(".claude/settings.json")).unwrap()).unwrap();
    let stop = s["hooks"]["Stop"].as_array().unwrap();
    assert_eq!(stop.len(), 2, "{stop:?}");
    assert!(stop.iter().any(|e| e.to_string().contains("my-own")) && stop.iter().any(|e| e.to_string().contains("--vault")));
    assert!(!stop.iter().any(|e| e["hooks"][0]["command"] == "smug hook stop"));
});
t!(setup_claude_twice_changes_nothing_the_second_time, {
    let v = vault("vh3");
    smug(&v, &["setup", "claude"], "");
    let a = fs::read_to_string(v.join(".claude/settings.json")).unwrap();
    smug(&v, &["setup", "claude"], "");
    assert_eq!(fs::read_to_string(v.join(".claude/settings.json")).unwrap(), a);
});

// ====================== damaged state heals ======================
fn seeded(name: &str) -> std::path::PathBuf {
    let v = vault(name);
    fs::write(v.join("500-knowledge/a.md"), "survivor keyword").unwrap();
    smug(&v, &["write", "b.md", "--expect", "new"], "second note");
    assert!(smug(&v, &["search", "survivor"], "").1.contains("a.md"));
    v
}
t!(garbage_state_file_is_set_aside_and_rebuilt, {
    let v = seeded("dm1");
    fs::write(v.join(".smug/state.redb"), b"this is definitely not a database").unwrap();
    assert!(smug(&v, &["search", "survivor"], "").1.contains("a.md"));
    assert!(v.join(".smug/state.redb.corrupt").exists());
    assert!(smug(&v, &["log", "10"], "").1.contains("b.md"), "the journal replays into the fresh state");
});
t!(truncated_state_file_is_rebuilt, {
    let v = seeded("dm2");
    let p = v.join(".smug/state.redb");
    let len = fs::metadata(&p).unwrap().len();
    fs::OpenOptions::new().write(true).open(&p).unwrap().set_len(len / 3).unwrap();
    assert_eq!(smug(&v, &["doctor"], "").0, 0);
    assert!(smug(&v, &["search", "survivor"], "").1.contains("a.md"));
});
t!(empty_state_file_is_rebuilt, {
    let v = seeded("dm3");
    fs::write(v.join(".smug/state.redb"), b"").unwrap();
    assert!(smug(&v, &["search", "survivor"], "").1.contains("a.md"));
});
t!(a_corrupt_journal_line_is_skipped_and_the_rest_replays, {
    let v = seeded("dm4");
    let j = v.join("history/events.jsonl");
    let t = fs::read_to_string(&j).unwrap();
    let mut lines: Vec<&str> = t.lines().collect();
    lines.insert(1, "{ this line is damaged");
    fs::write(&j, lines.join("\n") + "\n").unwrap();
    fs::remove_dir_all(v.join(".smug")).unwrap();
    let (c, o, _) = smug(&v, &["log", "50"], "");
    assert!(c == 0 && o.contains("b.md"), "{o}");
});
t!(a_truncated_last_journal_line_does_not_stop_new_writes, {
    let v = seeded("dm5");
    let mut f = fs::OpenOptions::new().append(true).open(v.join("history/events.jsonl")).unwrap();
    write!(f, "{{\"id\":999,\"ts\":0,\"kind\":\"inte").unwrap(); // power cut mid-line
    drop(f);
    fs::remove_dir_all(v.join(".smug")).unwrap();
    assert_eq!(smug(&v, &["write", "c.md", "--expect", "new"], "third").0, 0);
    assert_eq!(smug(&v, &["doctor"], "").0, 0);
});
t!(journal_ids_keep_increasing_after_a_state_loss, {
    let v = seeded("dm6");
    fs::remove_dir_all(v.join(".smug")).unwrap();
    smug(&v, &["write", "c.md", "--expect", "new"], "third");
    let ids: Vec<u64> = fs::read_to_string(v.join("history/events.jsonl")).unwrap().lines().filter_map(|l| serde_json::from_str::<Value>(l).ok()).map(|e| e["id"].as_u64().unwrap()).collect();
    assert!(ids.windows(2).all(|w| w[0] < w[1]), "{ids:?}");
});
t!(journal_deleted_but_state_present_keeps_working, {
    let v = seeded("dm7");
    fs::remove_dir_all(v.join("history")).unwrap();
    assert_eq!(smug(&v, &["write", "c.md", "--expect", "new"], "third").0, 0);
    assert_eq!(smug(&v, &["doctor"], "").0, 0);
});
t!(doctor_removes_temp_files_left_by_a_killed_writer, {
    let v = vault("dm8");
    fs::write(v.join("a.smug-tmp-4242"), "half written").unwrap();
    fs::create_dir_all(v.join("300-projects")).unwrap();
    fs::write(v.join("300-projects/b.smug-tmp-1"), "x").unwrap();
    fs::write(v.join("keep-me.md"), "mine").unwrap();
    let o = smug(&v, &["doctor"], "").1;
    assert!(o.contains("2 leftover temp file"), "{o}");
    assert!(!v.join("a.smug-tmp-4242").exists() && !v.join("300-projects/b.smug-tmp-1").exists() && v.join("keep-me.md").exists());
});
#[cfg(unix)]
t!(a_failed_write_leaves_the_file_untouched_and_doctor_aborts_the_intent, {
    use std::os::unix::fs::PermissionsExt;
    let v = vault("dm9");
    fs::create_dir_all(v.join("ro")).unwrap();
    fs::write(v.join("ro/a.md"), "original").unwrap();
    fs::set_permissions(v.join("ro"), fs::Permissions::from_mode(0o555)).unwrap();
    let (c, _, _) = smug(&v, &["write", "ro/a.md", "--force"], "changed");
    fs::set_permissions(v.join("ro"), fs::Permissions::from_mode(0o755)).unwrap();
    if c == 0 {
        return; // running as root: permissions are not enforced, nothing to test
    }
    assert_eq!(fs::read_to_string(v.join("ro/a.md")).unwrap(), "original");
    assert!(smug(&v, &["doctor"], "").1.contains("1 unfinished write(s) reconciled"));
    assert!(smug(&v, &["log", "5"], "").1.contains("abort"));
});
t!(an_unreadable_note_does_not_break_search_or_doctor, {
    let v = vault("dm10");
    fs::write(v.join("500-knowledge/binary.md"), [0u8, 159, 146, 150, 0, 255]).unwrap();
    fs::write(v.join("500-knowledge/ok.md"), "findable").unwrap();
    assert!(smug(&v, &["search", "findable"], "").1.contains("ok.md"));
    assert_eq!(smug(&v, &["doctor"], "").0, 0);
});
t!(invalid_utf8_in_a_note_is_survivable, {
    let v = vault("dm11");
    fs::write(v.join("500-knowledge/latin.md"), b"caf\xe9 survivor").unwrap();
    assert!(smug(&v, &["search", "survivor"], "").1.contains("latin.md"));
});
t!(an_empty_note_is_fine, {
    let v = vault("dm12");
    fs::write(v.join("500-knowledge/empty.md"), "").unwrap();
    assert_eq!(smug(&v, &["search", "anything"], "").0, 0);
    assert_eq!(smug(&v, &["doctor"], "").0, 0);
});
t!(a_very_long_line_and_a_very_long_note_are_fine, {
    let v = vault("dm13");
    fs::write(v.join("500-knowledge/long.md"), format!("{} needleword {}", "x".repeat(2_000_000), "y".repeat(10))).unwrap();
    assert_eq!(smug(&v, &["search", "needleword"], "").0, 0);
});
t!(deeply_nested_notes_are_indexed, {
    let v = vault("dm14");
    let deep = v.join("a/b/c/d/e/f/g/h/i/j");
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("n.md"), "deepneedle").unwrap();
    assert!(smug(&v, &["search", "deepneedle"], "").1.contains("a/b/c/d/e/f/g/h/i/j/n.md"));
});
#[cfg(unix)]
t!(a_symlink_loop_inside_the_vault_does_not_hang, {
    let v = vault("dm15");
    std::os::unix::fs::symlink(&v, v.join("500-knowledge/loop")).unwrap();
    fs::write(v.join("500-knowledge/a.md"), "loopneedle").unwrap();
    assert!(smug(&v, &["search", "loopneedle"], "").1.contains("a.md"));
});
t!(filenames_with_unicode_and_spaces_index_and_search, {
    let v = vault("dm16");
    fs::write(v.join("500-knowledge/şemsiye ışık.md"), "unicodefilename").unwrap();
    assert!(smug(&v, &["search", "unicodefilename"], "").1.contains("şemsiye ışık.md"));
});

// ====================== fuzz ======================
t!(fuzz_mcp_garbage_never_kills_the_server, {
    let v = vault("fz1");
    let mut g = Lcg(7);
    let frags = ["{", "}", "[", "]", "\"", ":", ",", "null", "true", "{\"jsonrpc\":\"2.0\"", "{\"method\":5}", "{\"id\":{},\"method\":\"tools/call\"}", "\\u0000", "é", "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":42}}", "[[[[[[", "{\"params\":{\"arguments\":\"x\"}}"];
    let mut reqs = vec![];
    for _ in 0..300 {
        let n = 1 + g.next() % 6;
        reqs.push((0..n).map(|_| g.pick(&frags)).collect::<Vec<_>>().join(""));
    }
    let mut c = Command::new(env!("CARGO_BIN_EXE_smug")).arg("mcp").env("SMUG_VAULT", &v).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
    let mut i = c.stdin.take().unwrap();
    for r in &reqs {
        writeln!(i, "{}", r.replace('\n', " ")).unwrap();
    }
    writeln!(i, "{}", rpc(99, "ping", json!({}))).unwrap();
    drop(i);
    let o = c.wait_with_output().unwrap();
    assert!(o.status.success(), "the server must survive garbage");
    assert!(String::from_utf8_lossy(&o.stdout).lines().any(|l| l.contains("\"id\":99")), "and still answer the ping at the end");
});
t!(fuzz_mcp_wrong_argument_types_are_tool_errors_not_crashes, {
    let v = vault("fz2");
    for (name, args) in [("smug_recall", json!({"query": 5})), ("smug_recall", json!({"query": null})), ("smug_recall", json!({"query": ["a"]})), ("smug_check", json!({"id": {}})), ("smug_propose", json!({"kind": 1, "id": 2, "text": 3})), ("smug_feedback", json!({"path": [], "kind": {}})), ("smug_recall", json!("not an object")), ("smug_recall", Value::Null)] {
        let r = tool(&v, &[], name, args.clone());
        assert_eq!(r["isError"], true, "{name} {args}");
    }
});
t!(mcp_huge_query_is_handled, {
    let v = vault("fz3");
    let r = tool(&v, &[], "smug_recall", json!({"query": "word ".repeat(200_000)}));
    assert!(r["isError"] == false || r["isError"] == true);
});
t!(mcp_unicode_and_emoji_queries, {
    let v = vault("fz4");
    fs::write(v.join("500-knowledge/u.md"), "ışık şemsiye 🚀 rocket").unwrap();
    let r = tool(&v, &[], "smug_recall", json!({"query": "isik semsiye 🚀"}));
    assert_eq!(r["isError"], false);
    assert!(r["structuredContent"]["notes"][0]["path"].as_str().unwrap().contains("u.md"));
});
t!(mcp_string_and_numeric_ids_are_echoed, {
    let v = vault("fz5");
    let r = mcp(&v, &[], &[json!({"jsonrpc": "2.0", "id": "abc-1", "method": "ping"}), json!({"jsonrpc": "2.0", "id": 7.5, "method": "ping"})]);
    assert_eq!(r[0]["id"], "abc-1");
    assert_eq!(r[1]["id"], 7.5);
});
t!(mcp_batch_array_does_not_crash, {
    let v = vault("fz6");
    let r = mcp(&v, &[], &[json!([{"jsonrpc": "2.0", "id": 1, "method": "ping"}]), rpc(2, "ping", json!({}))]);
    assert!(r.iter().any(|x| x["id"] == 2));
});
t!(mcp_repeated_initialize_is_fine, {
    let v = vault("fz7");
    let r = mcp(&v, &[], &[rpc(1, "initialize", json!({})), rpc(2, "initialize", json!({})), rpc(3, "tools/list", json!({}))]);
    assert_eq!(r.len(), 3);
});
t!(mcp_survives_crlf_line_endings, {
    use std::io::Read;
    let v = vault("fz8");
    let mut c = Command::new(env!("CARGO_BIN_EXE_smug")).arg("mcp").env("SMUG_VAULT", &v).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
    write!(c.stdin.take().unwrap(), "{}\r\n", rpc(1, "ping", json!({}))).unwrap();
    let mut s = String::new();
    c.stdout.take().unwrap().read_to_string(&mut s).unwrap();
    assert!(s.contains("\"id\":1"), "{s}");
});
t!(mcp_blank_lines_are_ignored, {
    let v = vault("fz9");
    let mut c = Command::new(env!("CARGO_BIN_EXE_smug")).arg("mcp").env("SMUG_VAULT", &v).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
    write!(c.stdin.take().unwrap(), "\n\n   \n{}\n\n", rpc(1, "ping", json!({}))).unwrap();
    let o = c.wait_with_output().unwrap();
    assert_eq!(String::from_utf8_lossy(&o.stdout).lines().count(), 1);
});
t!(fuzz_cli_random_args_never_panic, {
    let v = vault("fz10");
    let mut g = Lcg(99);
    let words = ["search", "write", "decide", "check", "at", "impact", "patch", "apply", "eval", "feedback", "hook", "setup", "agent", "propose", "review", "promote", "capture", "--force", "--expect", "--source", "--kind", "../x", "", "a b", "é", "--model", "--timeout", "-1", "99999999999", "--vault"];
    for _ in 0..80 {
        let n = (g.next() % 5) as usize;
        let args: Vec<&str> = (0..n).map(|_| g.pick(&words)).collect();
        let (c, _, e) = run_in(&v, &args, &[("SMUG_VAULT", v.to_str().unwrap())], "");
        assert!(c == 0 || c == 1 || c == 2 || c == 3, "{args:?} -> {c}");
        assert!(!e.contains("internal error") && !e.contains("panicked"), "{args:?}: {e}");
    }
});
t!(fuzz_hook_inputs_never_fail_the_agent, {
    let v = vault("fz11");
    let mut g = Lcg(5);
    let bits = ["{", "}", "\"prompt\":\"hi\"", "\"session_id\":\"s\"", "\"tool_input\":{\"file_path\":\"a.rs\"}", "\"transcript_path\":\"/nonexistent\"", "null", "[", ",", "\"x\":1"];
    for sub in ["prompt", "stop", "pre-edit", "session-end", "pre-compact"] {
        for _ in 0..20 {
            let n = 1 + g.next() % 5;
            let input: String = (0..n).map(|_| g.pick(&bits)).collect::<Vec<_>>().join(",");
            assert_eq!(smug(&v, &["hook", sub], &input).0, 0, "{sub} {input}");
        }
    }
});

// ====================== stress and crash ======================
t!(twelve_writers_and_four_readers_at_once, {
    let v = vault("st1");
    fs::write(v.join("500-knowledge/seed.md"), "stressword").unwrap();
    let mut hs = vec![];
    for i in 0..12 {
        let v = v.clone();
        hs.push(std::thread::spawn(move || smug(&v, &["write", &format!("w{i}.md"), "--expect", "new"], &format!("body {i}")).0));
    }
    for _ in 0..4 {
        let v = v.clone();
        hs.push(std::thread::spawn(move || smug(&v, &["search", "stressword"], "").0));
    }
    let codes: Vec<i32> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(codes.iter().all(|&c| c == 0), "{codes:?}");
    assert_eq!((0..12).filter(|i| v.join(format!("w{i}.md")).exists()).count(), 12);
    let j = fs::read_to_string(v.join("history/events.jsonl")).unwrap();
    assert_eq!(j.lines().filter(|l| l.contains("\"commit\"")).count(), 12);
    assert_eq!(smug(&v, &["doctor"], "").0, 0);
});
t!(three_mcp_servers_and_cli_writes_interleave, {
    let v = vault("st2");
    let mut hs = vec![];
    for k in 0..3 {
        let v = v.clone();
        hs.push(std::thread::spawn(move || {
            let reqs: Vec<Value> = (0..10).map(|i| rpc(i, "tools/call", json!({"name": "smug_propose", "arguments": {"kind": "failure", "id": format!("m{k}-{i}"), "text": "t"}}))).collect();
            mcp(&v, &[], &reqs).iter().filter(|r| r["result"]["isError"] == false).count()
        }));
    }
    let w: Vec<_> = (0..6).map(|i| { let v = v.clone(); std::thread::spawn(move || smug(&v, &["write", &format!("c{i}.md"), "--expect", "new"], "x").0 as usize) }).collect();
    let ok: usize = hs.into_iter().map(|h| h.join().unwrap()).sum();
    let wc: usize = w.into_iter().map(|h| h.join().unwrap()).sum();
    assert_eq!((ok, wc), (30, 0));
    assert_eq!(fs::read_dir(v.join("500-knowledge/candidates")).unwrap().count(), 30);
});
t!(killing_an_mcp_server_mid_session_loses_nothing, {
    let v = vault("st3");
    let mut c = Command::new(env!("CARGO_BIN_EXE_smug")).arg("mcp").env("SMUG_VAULT", &v).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
    let mut i = c.stdin.take().unwrap();
    for n in 0..5 {
        writeln!(i, "{}", rpc(n, "tools/call", json!({"name": "smug_propose", "arguments": {"kind": "failure", "id": format!("k{n}"), "text": "t"}}))).unwrap();
    }
    i.flush().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(150));
    c.kill().unwrap();
    let _ = c.wait();
    assert_eq!(smug(&v, &["doctor"], "").0, 0);
    assert_eq!(smug(&v, &["write", "after.md", "--expect", "new"], "x").0, 0);
});
#[cfg(unix)]
t!(sigkill_during_a_burst_of_writes_leaves_a_consistent_vault, {
    let v = vault("st4");
    let mut kids = vec![];
    for i in 0..30 {
        let mut c = Command::new(env!("CARGO_BIN_EXE_smug")).args(["write", &format!("k{i}.md"), "--force"]).env("SMUG_VAULT", &v).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
        let _ = c.stdin.take().unwrap().write_all(format!("payload {i}").as_bytes());
        kids.push(c);
    }
    std::thread::sleep(std::time::Duration::from_millis(40));
    for c in kids.iter_mut().step_by(2) {
        let _ = c.kill(); // SIGKILL half of them wherever they are
    }
    for mut c in kids {
        let _ = c.wait();
    }
    let (code, o, _) = smug(&v, &["doctor"], "");
    assert_eq!(code, 0, "{o}");
    // no note is ever half written: each is absent or exactly its payload
    for i in 0..30 {
        if let Ok(t) = fs::read_to_string(v.join(format!("k{i}.md"))) {
            assert_eq!(t, format!("payload {i}"), "k{i}.md is partial");
        }
    }
    assert!(smug(&v, &["doctor"], "").1.contains("0 unfinished"), "doctor must leave nothing dangling");
    assert!(fs::read_dir(&v).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().contains(".smug-tmp-")));
});
t!(a_large_vault_is_searched_quickly_and_correctly, {
    let v = vault("st5");
    for i in 0..1500 {
        fs::write(v.join(format!("500-knowledge/n{i}.md")), format!("note {i} lorem ipsum common words filler token{i}")).unwrap();
    }
    let t0 = std::time::Instant::now();
    let first = smug(&v, &["search", "token777"], "");
    assert!(first.1.contains("n777.md"), "{}", first.1);
    let cold = t0.elapsed();
    let t1 = std::time::Instant::now();
    assert!(smug(&v, &["search", "token1234"], "").1.contains("n1234.md"));
    let warm = t1.elapsed();
    assert!(cold.as_secs() < 30 && warm.as_secs() < 5, "cold {cold:?} warm {warm:?}");
});
t!(an_edit_storm_keeps_the_index_exact, {
    let v = vault("st6");
    for round in 0..8 {
        for i in 0..10 {
            fs::write(v.join(format!("500-knowledge/s{i}.md")), format!("round{round} item{i}")).unwrap();
        }
        let o = smug(&v, &["search", &format!("round{round}")], "").1;
        assert_eq!(o.lines().filter(|l| l.ends_with(".md")).count(), 5.min(10), "round {round}: {o}");
        if round > 0 {
            assert!(smug(&v, &["search", &format!("round{}", round - 1)], "").1.is_empty(), "stale round{} text must be gone", round - 1);
        }
    }
});
t!(running_everything_twice_is_idempotent, {
    let v = vault("st7");
    for _ in 0..2 {
        assert_eq!(smug(&v, &["doctor"], "").0, 0);
        assert_eq!(smug(&v, &["index"], "").0, 0);
        assert_eq!(smug(&v, &["setup", "claude"], "").0, 0);
        assert_eq!(smug(&v, &["scan"], "").0, 0);
    }
});
