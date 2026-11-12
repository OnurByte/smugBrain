mod common;
use common::*;
use serde_json::{json, Value};
use std::{fs, path::Path};

macro_rules! t {
    ($n:ident, $b:expr) => {
        #[test]
        fn $n() {
            $b
        }
    };
}

fn home(name: &str) -> std::path::PathBuf {
    tmp(&format!("home-{name}"))
}

fn install(v: &Path, h: &Path, args: &[&str]) -> (i32, String, String) {
    let mut a = vec!["agent"];
    a.extend(args);
    smug_env(v, &a, "", &[("SMUG_HOME", h.to_str().unwrap())])
}

fn json_of(p: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap()
}

// ====================== MCP server ======================
t!(mcp_initialize_identifies_smug, {
    let r = mcp(&vault("mi"), &[], &[rpc(1, "initialize", json!({"protocolVersion": "2025-03-26", "capabilities": {}}))]);
    assert_eq!(r[0]["result"]["serverInfo"]["name"], "smug");
    assert_eq!(r[0]["result"]["protocolVersion"], "2025-03-26");
    assert!(r[0]["result"]["capabilities"]["tools"].is_object());
});
t!(mcp_lists_exactly_six_tools, {
    let r = mcp(&vault("ml"), &[], &[rpc(1, "tools/list", json!({}))]);
    let names: Vec<&str> = r[0]["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["smug_recall", "smug_check", "smug_explain", "smug_impact", "smug_propose", "smug_feedback"]);
});
t!(mcp_every_tool_has_a_schema_and_description, {
    let r = mcp(&vault("msch"), &[], &[rpc(1, "tools/list", json!({}))]);
    for t in r[0]["result"]["tools"].as_array().unwrap() {
        assert!(t["inputSchema"]["type"] == "object" && !t["description"].as_str().unwrap().is_empty(), "{t}");
    }
});
t!(mcp_read_only_hides_write_tools, {
    let r = mcp(&vault("mro"), &["--read-only"], &[rpc(1, "tools/list", json!({}))]);
    assert_eq!(r[0]["result"]["tools"].as_array().unwrap().len(), 4);
});
t!(mcp_read_only_refuses_a_forced_propose, {
    let v = vault("mro2");
    let r = tool(&v, &["--read-only"], "smug_propose", json!({"kind": "failure", "id": "x", "text": "t"}));
    assert_eq!(r["isError"], true);
    assert!(!v.join("500-knowledge/candidates/x.md").exists());
});
t!(mcp_read_only_refuses_feedback, assert_eq!(tool(&vault("mro3"), &["--read-only"], "smug_feedback", json!({"path": "a.md", "kind": "used"}))["isError"], true));
t!(mcp_ping_answers, assert!(mcp(&vault("mp"), &[], &[rpc(1, "ping", json!({}))])[0]["result"].is_object()));
t!(mcp_notifications_get_no_reply, {
    let r = mcp(&vault("mn"), &[], &[json!({"jsonrpc": "2.0", "method": "notifications/initialized"}), rpc(2, "ping", json!({}))]);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0]["id"], 2);
});
t!(mcp_unknown_method_is_an_error, assert_eq!(mcp(&vault("mum"), &[], &[rpc(1, "frobnicate", json!({}))])[0]["error"]["code"], -32601));
t!(mcp_garbage_line_is_a_parse_error, {
    use std::io::Write;
    let v = vault("mg");
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_smug")).arg("mcp").env("SMUG_VAULT", &v).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();
    writeln!(c.stdin.take().unwrap(), "this is not json").unwrap();
    let o = c.wait_with_output().unwrap();
    assert!(String::from_utf8_lossy(&o.stdout).contains("-32700"));
});
t!(mcp_unknown_tool_is_a_tool_error, assert_eq!(tool(&vault("mut"), &[], "smug_nope", json!({}))["isError"], true));
t!(mcp_missing_argument_is_a_tool_error, assert_eq!(tool(&vault("mma"), &[], "smug_check", json!({}))["isError"], true));
t!(mcp_recall_returns_valid_notes_and_warnings, {
    let v = vault("mr");
    smug(&v, &["decide", "pg", "--source", "adr-1", "--title", "Postgres"], "we use postgres for storage");
    smug(&v, &["fail", "f1", "--method", "run migrations at boot", "--reason", "locked the table"], "");
    let r = tool(&v, &[], "smug_recall", json!({"query": "postgres storage, should we run migrations at boot"}));
    assert_eq!(r["isError"], false);
    assert!(r["structuredContent"]["notes"].as_array().unwrap().iter().any(|n| n["path"].as_str().unwrap().contains("pg.md")));
    assert!(r["structuredContent"]["warnings"][0].as_str().unwrap().contains("locked the table"));
});
t!(mcp_recall_says_so_when_nothing_matches, {
    let r = tool(&vault("mr0"), &[], "smug_recall", json!({"query": "zzzxqj"}));
    assert_eq!(r["structuredContent"]["note"], "no relevant memory");
});
t!(mcp_recall_hides_replaced_decisions, {
    let v = vault("mrh");
    smug(&v, &["decide", "a", "--source", "s", "--date", "2026-01-01"], "alphaword");
    fs::write(v.join("500-knowledge/decisions/a.md"), fs::read_to_string(v.join("500-knowledge/decisions/a.md")).unwrap().replace("status: active", "status: superseded")).unwrap();
    assert!(tool(&v, &[], "smug_recall", json!({"query": "alphaword"}))["structuredContent"]["notes"].as_array().unwrap().is_empty());
});
t!(mcp_check_reports_status, {
    let v = vault("mc");
    smug(&v, &["decide", "d", "--source", "adr"], "x");
    assert_eq!(tool(&v, &[], "smug_check", json!({"id": "d"}))["structuredContent"]["status"], "CURRENT");
    assert_eq!(tool(&v, &[], "smug_check", json!({"id": "nope"}))["structuredContent"]["status"], "UNKNOWN");
});
t!(mcp_explain_shows_source_dates_and_history, {
    let v = vault("me");
    smug(&v, &["decide", "d", "--source", "adr-3", "--date", "2026-02-02"], "x");
    let r = &tool(&v, &[], "smug_explain", json!({"id": "d"}))["structuredContent"];
    assert_eq!(r["source"], "adr-3");
    assert_eq!(r["valid_from"], "2026-02-02");
    assert!(!r["history"].as_array().unwrap().is_empty());
});
t!(mcp_explain_unknown_id_is_a_tool_error, assert_eq!(tool(&vault("me0"), &[], "smug_explain", json!({"id": "ghost"}))["isError"], true));
t!(mcp_impact_lists_dependents, {
    let v = vault("mim");
    smug(&v, &["decide", "d", "--source", "s"], "x");
    fs::write(v.join("300-projects/a.md"), "uses [[d]]").unwrap();
    let r = &tool(&v, &[], "smug_impact", json!({"id": "d"}))["structuredContent"]["dependents"];
    assert_eq!(r[0]["path"], "300-projects/a.md");
});
t!(mcp_propose_writes_an_inert_candidate, {
    let v = vault("mpr");
    let r = tool(&v, &[], "smug_propose", json!({"kind": "decision", "id": "use-redb", "text": "uniquecandidateword is stored in redb", "source": "chat", "topic": "state"}));
    assert_eq!(r["isError"], false, "{r}");
    assert!(v.join("500-knowledge/candidates/use-redb.md").exists());
    assert!(!v.join("500-knowledge/decisions/use-redb.md").exists());
    assert!(smug(&v, &["search", "uniquecandidateword"], "").1.is_empty(), "candidates are not searchable memory");
    assert!(tool(&v, &[], "smug_recall", json!({"query": "uniquecandidateword"}))["structuredContent"]["notes"].as_array().unwrap().is_empty());
    assert_eq!(tool(&v, &[], "smug_check", json!({"id": "use-redb"}))["structuredContent"]["status"], "CANDIDATE");
});
t!(mcp_propose_refuses_secrets, assert_eq!(tool(&vault("mps"), &[], "smug_propose", json!({"kind": "failure", "id": "x", "text": "password: hunter2hunter2"}))["isError"], true));
t!(mcp_propose_refuses_bad_ids, {
    for id in ["../x", "a b", "", "a/b"] {
        assert_eq!(tool(&vault("mpb"), &[], "smug_propose", json!({"kind": "failure", "id": id, "text": "t"}))["isError"], true, "{id}");
    }
});
t!(mcp_propose_refuses_a_bad_kind, assert_eq!(tool(&vault("mpk"), &[], "smug_propose", json!({"kind": "law", "id": "x", "text": "t"}))["isError"], true));
t!(mcp_propose_cannot_overwrite, {
    let v = vault("mpo");
    tool(&v, &[], "smug_propose", json!({"kind": "failure", "id": "x", "text": "first"}));
    assert_eq!(tool(&v, &[], "smug_propose", json!({"kind": "failure", "id": "x", "text": "second"}))["isError"], true);
    assert!(fs::read_to_string(v.join("500-knowledge/candidates/x.md")).unwrap().contains("first"));
});
t!(mcp_has_no_promote_tool, {
    let r = mcp(&vault("mnp"), &[], &[rpc(1, "tools/list", json!({}))]);
    let names: Vec<&str> = r[0]["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.iter().all(|n| !n.contains("promote") && !n.contains("supersede") && !n.contains("apply") && !n.contains("patch")), "{names:?}");
});
t!(mcp_feedback_updates_utility, {
    let v = vault("mf");
    let r = tool(&v, &[], "smug_feedback", json!({"path": "500-knowledge/a.md", "kind": "useful"}));
    assert_eq!(r["structuredContent"]["utility"], 2);
});
t!(mcp_feedback_rejects_escapes_and_kinds, {
    let v = vault("mfe");
    assert_eq!(tool(&v, &[], "smug_feedback", json!({"path": "../a.md", "kind": "used"}))["isError"], true);
    assert_eq!(tool(&v, &[], "smug_feedback", json!({"path": "a.md", "kind": "meh"}))["isError"], true);
});
t!(mcp_does_not_hold_the_vault_lock_between_calls, {
    use std::io::{BufRead, BufReader, Write};
    let v = vault("mlock");
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_smug")).arg("mcp").env("SMUG_VAULT", &v).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();
    let mut i = c.stdin.take().unwrap();
    let mut o = BufReader::new(c.stdout.take().unwrap());
    writeln!(i, "{}", rpc(1, "tools/call", json!({"name": "smug_recall", "arguments": {"query": "x"}}))).unwrap();
    let mut l = String::new();
    o.read_line(&mut l).unwrap();
    assert_eq!(smug(&v, &["write", "a.md", "--expect", "new"], "x").0, 0, "the CLI must still work while an agent session is connected");
    drop(i);
    let _ = c.wait();
});
t!(mcp_serves_many_calls_in_one_session, {
    let v = vault("mmany");
    let reqs: Vec<Value> = (1..=20).map(|i| rpc(i, "ping", json!({}))).collect();
    assert_eq!(mcp(&v, &[], &reqs).len(), 20);
});

// ====================== agent installer ======================
t!(agent_list_shows_ten_tools, {
    let o = install(&vault("al"), &home("al"), &["list"]).1;
    for w in ["claude-code", "codex", "cursor", "gemini", "windsurf", "kiro", "vscode", "opencode", "zed", "cline"] {
        assert!(o.contains(w), "{w}");
    }
});
t!(agent_list_never_claims_live_testing, assert!(!install(&vault("al2"), &home("al2"), &["list"]).1.contains("live-tested")));
t!(agent_unknown_id_errors, assert_eq!(install(&vault("au"), &home("au"), &["install", "emacs"]).0, 1));
t!(agent_install_needs_a_target, assert_eq!(install(&vault("an"), &home("an"), &["install"]).0, 1));
t!(agent_claude_project_config_is_created, {
    let (v, h) = (vault("ac"), home("ac"));
    assert_eq!(install(&v, &h, &["install", "claude-code"]).0, 0);
    let j = json_of(&v.join(".mcp.json"));
    assert_eq!(j["mcpServers"]["smug"]["args"], json!(["mcp"]));
    assert!(j["mcpServers"]["smug"]["command"].as_str().unwrap().contains("smug"));
});
t!(agent_claude_also_gets_hooks_and_skill, {
    let (v, h) = (vault("acs"), home("acs"));
    install(&v, &h, &["install", "claude-code"]);
    let s = json_of(&v.join(".claude/settings.json"));
    for e in ["SessionStart", "UserPromptSubmit", "PreToolUse", "Stop", "SessionEnd", "PreCompact"] {
        assert!(s["hooks"][e].is_array(), "{e}");
    }
    assert_eq!(s["hooks"]["PreToolUse"][0]["matcher"], "Edit|Write|MultiEdit");
    let skill = fs::read_to_string(v.join(".claude/skills/smug/SKILL.md")).unwrap();
    assert!(skill.starts_with("---\nname: smug") && skill.contains("smug-skill-version: 1") && skill.contains("Never promote"));
});
t!(agent_merge_keeps_other_servers, {
    let (v, h) = (vault("am"), home("am"));
    fs::write(v.join(".mcp.json"), r#"{"mcpServers":{"other":{"command":"x"}},"keep":1}"#).unwrap();
    install(&v, &h, &["install", "claude-code"]);
    let j = json_of(&v.join(".mcp.json"));
    assert!(j["mcpServers"]["other"].is_object() && j["mcpServers"]["smug"].is_object() && j["keep"] == 1);
});
t!(agent_install_is_idempotent, {
    let (v, h) = (vault("ai"), home("ai"));
    install(&v, &h, &["install", "cursor"]);
    let before = fs::read_to_string(v.join(".cursor/mcp.json")).unwrap();
    let o = install(&v, &h, &["install", "cursor"]).1;
    assert!(o.contains("already set up"), "{o}");
    assert_eq!(fs::read_to_string(v.join(".cursor/mcp.json")).unwrap(), before);
});
t!(agent_dry_run_writes_nothing, {
    let (v, h) = (vault("ad"), home("ad"));
    let o = install(&v, &h, &["install", "claude-code", "--dry-run"]).1;
    assert!(o.contains("dry run"), "{o}");
    assert!(!v.join(".mcp.json").exists() && !v.join(".claude/skills").exists() && !v.join(".claude/settings.json").exists());
});
t!(agent_conflicting_entry_needs_force, {
    let (v, h) = (vault("af"), home("af"));
    fs::write(v.join(".mcp.json"), r#"{"mcpServers":{"smug":{"command":"somewhere-else"}}}"#).unwrap();
    assert_eq!(install(&v, &h, &["install", "claude-code"]).0, 1);
    assert_eq!(json_of(&v.join(".mcp.json"))["mcpServers"]["smug"]["command"], "somewhere-else");
    assert_eq!(install(&v, &h, &["install", "claude-code", "--force"]).0, 0);
    assert_ne!(json_of(&v.join(".mcp.json"))["mcpServers"]["smug"]["command"], "somewhere-else");
});
t!(agent_invalid_json_is_left_alone_with_a_snippet, {
    let (v, h) = (vault("aj"), home("aj"));
    fs::write(v.join(".mcp.json"), "{ // comment\n").unwrap();
    let (c, o, _) = install(&v, &h, &["install", "claude-code"]);
    assert!(c == 1 && o.contains("Nothing was written") && o.contains("mcpServers"), "{o}");
    assert_eq!(fs::read_to_string(v.join(".mcp.json")).unwrap(), "{ // comment\n");
});
#[cfg(unix)]
t!(agent_symlinked_config_is_refused, {
    let (v, h) = (vault("asl"), home("asl"));
    fs::write(v.join("real.json"), "{}").unwrap();
    std::os::unix::fs::symlink(v.join("real.json"), v.join(".mcp.json")).unwrap();
    assert_eq!(install(&v, &h, &["install", "claude-code"]).0, 1);
    assert_eq!(fs::read_to_string(v.join("real.json")).unwrap(), "{}");
});
t!(agent_codex_writes_toml_and_keeps_the_rest, {
    let (v, h) = (vault("acx"), home("acx"));
    fs::create_dir_all(h.join(".codex")).unwrap();
    fs::write(h.join(".codex/config.toml"), "# my config\nmodel = \"gpt\"\n\n[mcp_servers.other]\ncommand = \"x\"\n").unwrap();
    assert_eq!(install(&v, &h, &["install", "codex", "--global"]).0, 0);
    let t = fs::read_to_string(h.join(".codex/config.toml")).unwrap();
    assert!(t.contains("# my config") && t.contains("model = \"gpt\"") && t.contains("[mcp_servers.other]") && t.contains("[mcp_servers.smug]") && t.contains("args = [\"mcp\"]"), "{t}");
    assert!(fs::read_to_string(v.join("AGENTS.md")).unwrap().contains("## smug"));
});
t!(agent_codex_toml_idempotent, {
    let (v, h) = (vault("acx2"), home("acx2"));
    install(&v, &h, &["install", "codex", "--global"]);
    let first = fs::read_to_string(h.join(".codex/config.toml")).unwrap();
    assert!(install(&v, &h, &["install", "codex", "--global"]).1.contains("already set up"));
    assert_eq!(fs::read_to_string(h.join(".codex/config.toml")).unwrap(), first);
});
t!(agent_codex_has_no_project_config, {
    let (v, h) = (vault("acx3"), home("acx3"));
    let (c, o, _) = install(&v, &h, &["install", "codex"]);
    assert!(c == 1 && o.contains("[mcp_servers.smug]"), "{o}");
});
t!(agent_codex_invalid_toml_refused, {
    let (v, h) = (vault("acx4"), home("acx4"));
    fs::create_dir_all(h.join(".codex")).unwrap();
    fs::write(h.join(".codex/config.toml"), "this = = broken").unwrap();
    assert_eq!(install(&v, &h, &["install", "codex", "--global"]).0, 1);
    assert_eq!(fs::read_to_string(h.join(".codex/config.toml")).unwrap(), "this = = broken");
});
t!(agent_gemini_shape, {
    let (v, h) = (vault("ag"), home("ag"));
    install(&v, &h, &["install", "gemini"]);
    assert!(json_of(&v.join(".gemini/settings.json"))["mcpServers"]["smug"]["command"].is_string());
});
t!(agent_vscode_uses_servers_with_type, {
    let (v, h) = (vault("avs"), home("avs"));
    install(&v, &h, &["install", "vscode"]);
    let j = json_of(&v.join(".vscode/mcp.json"));
    assert_eq!(j["servers"]["smug"]["type"], "stdio");
});
t!(agent_opencode_new_layout, {
    let (v, h) = (vault("aoc"), home("aoc"));
    install(&v, &h, &["install", "opencode"]);
    let j = json_of(&v.join("opencode.json"));
    assert_eq!(j["mcp"]["servers"]["smug"]["type"], "local");
    assert_eq!(j["mcp"]["servers"]["smug"]["command"][1], "mcp");
});
t!(agent_opencode_keeps_the_old_flat_layout, {
    let (v, h) = (vault("aoc2"), home("aoc2"));
    fs::write(v.join("opencode.json"), r#"{"mcp":{"smug":{"type":"local","command":["old"],"enabled":true}}}"#).unwrap();
    install(&v, &h, &["install", "opencode", "--force"]);
    let j = json_of(&v.join("opencode.json"));
    assert!(j["mcp"]["smug"]["command"][1] == "mcp" && j["mcp"]["servers"].is_null());
});
t!(agent_windsurf_is_global_only, {
    let (v, h) = (vault("aw"), home("aw"));
    assert_eq!(install(&v, &h, &["install", "windsurf"]).0, 1);
    assert_eq!(install(&v, &h, &["install", "windsurf", "--global"]).0, 0);
    assert!(h.join(".codeium/windsurf/mcp_config.json").exists());
});
t!(agent_kiro_project_config, {
    let (v, h) = (vault("ak"), home("ak"));
    install(&v, &h, &["install", "kiro"]);
    assert!(v.join(".kiro/settings/mcp.json").exists());
});
t!(agent_zed_and_cline_print_a_snippet, {
    for id in ["zed", "cline"] {
        let (c, o, _) = install(&vault("az"), &home("az"), &["install", id]);
        assert!(c == 1 && o.contains("\"smug\"") && o.contains("manual"), "{id}: {o}");
    }
});
t!(agent_detected_with_nothing_installed_is_a_no_op, {
    let (c, o, _) = install(&vault("adt"), &home("adt"), &["install", "detected"]);
    assert!(c == 0 && o.contains("no supported coding tool detected"), "{o}");
});
t!(agent_detected_finds_what_is_there, {
    let (v, h) = (vault("adt2"), home("adt2"));
    fs::create_dir_all(h.join(".cursor")).unwrap();
    install(&v, &h, &["install", "detected"]);
    assert!(v.join(".cursor/mcp.json").exists() && !v.join(".gemini").exists());
});
t!(agent_uninstall_removes_only_smug, {
    let (v, h) = (vault("aun"), home("aun"));
    fs::write(v.join(".cursor-seed"), "").ok();
    fs::create_dir_all(v.join(".cursor")).unwrap();
    fs::write(v.join(".cursor/mcp.json"), r#"{"mcpServers":{"other":{"command":"x"}}}"#).unwrap();
    install(&v, &h, &["install", "cursor"]);
    install(&v, &h, &["uninstall", "cursor"]);
    let j = json_of(&v.join(".cursor/mcp.json"));
    assert!(j["mcpServers"]["other"].is_object() && j["mcpServers"]["smug"].is_null());
});
t!(agent_uninstall_claude_removes_the_skill, {
    let (v, h) = (vault("aun2"), home("aun2"));
    install(&v, &h, &["install", "claude-code"]);
    install(&v, &h, &["uninstall", "claude-code"]);
    assert!(!v.join(".claude/skills/smug/SKILL.md").exists());
});
t!(agent_uninstall_when_absent_is_unchanged, assert!(install(&vault("aun3"), &home("aun3"), &["uninstall", "cursor"]).1.contains("already set up")));
t!(agent_doctor_gets_six_tools_from_the_server, {
    let (c, o, _) = install(&vault("adoc"), &home("adoc"), &["doctor"]);
    assert!(c == 0 && o.contains("6 tools"), "{o}");
});
t!(agent_table_is_markdown_rows, {
    let o = install(&vault("atb"), &home("atb"), &["table"]).1;
    assert_eq!(o.lines().filter(|l| l.starts_with("| ")).count(), 10);
});

// ====================== hooks ======================
t!(setup_claude_merges_into_existing_settings, {
    let v = vault("sm");
    fs::create_dir_all(v.join(".claude")).unwrap();
    fs::write(v.join(".claude/settings.json"), r#"{"theme":"dark","hooks":{"Stop":[{"hooks":[{"type":"command","command":"my-own-hook"}]}]}}"#).unwrap();
    assert_eq!(smug(&v, &["setup", "claude"], "").0, 0);
    let s = json_of(&v.join(".claude/settings.json"));
    assert_eq!(s["theme"], "dark");
    let stop = s["hooks"]["Stop"].to_string();
    assert!(stop.contains("my-own-hook") && stop.contains("smug hook stop"));
});
t!(setup_claude_is_idempotent, {
    let v = vault("sm2");
    smug(&v, &["setup", "claude"], "");
    let a = fs::read_to_string(v.join(".claude/settings.json")).unwrap();
    assert!(smug(&v, &["setup", "claude"], "").1.contains("already set up"));
    assert_eq!(fs::read_to_string(v.join(".claude/settings.json")).unwrap(), a);
});
t!(setup_claude_refuses_broken_settings, {
    let v = vault("sm3");
    fs::create_dir_all(v.join(".claude")).unwrap();
    fs::write(v.join(".claude/settings.json"), "{ nope").unwrap();
    assert_eq!(smug(&v, &["setup", "claude"], "").0, 1);
    assert_eq!(fs::read_to_string(v.join(".claude/settings.json")).unwrap(), "{ nope");
});
fn pre_edit(v: &Path, file: &str) -> String {
    smug(v, &["hook", "pre-edit"], &json!({"tool_name": "Edit", "tool_input": {"file_path": file}}).to_string()).1
}
t!(pre_edit_reminds_of_a_past_failure_for_that_file, {
    let v = vault("pe");
    smug(&v, &["fail", "f", "--method", "cache the pool", "--reason", "leaked connections", "--files", "src/database.rs", "--commit", "abc123", "--test-result", "3 failed", "--verified", "yes"], "");
    let o = pre_edit(&v, "src/database.rs");
    let j: Value = serde_json::from_str(o.trim()).unwrap();
    let msg = j["hookSpecificOutput"]["additionalContext"].as_str().unwrap();
    assert_eq!(j["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    for w in ["FAILED BEFORE", "leaked connections", "verified: yes", "commit abc123", "3 failed", "CURRENT"] {
        assert!(msg.contains(w), "{w} in {msg}");
    }
});
t!(pre_edit_is_silent_for_unrelated_files, assert!(pre_edit(&{ let v = vault("pe2"); smug(&v, &["fail", "f", "--method", "m", "--reason", "r", "--files", "src/a.rs"], ""); v }, "src/zzz.rs").is_empty()));
t!(pre_edit_matches_by_path_suffix, {
    let v = vault("pe3");
    smug(&v, &["fail", "f", "--method", "m", "--reason", "r", "--files", "src/database.rs"], "");
    assert!(pre_edit(&v, "src/database.rs").contains("FAILED BEFORE"));
    assert!(pre_edit(&v, "./src/database.rs").contains("FAILED BEFORE"));
});
t!(pre_edit_shows_active_decisions_for_the_file, {
    let v = vault("pe4");
    smug(&v, &["decide", "pg", "--source", "adr", "--title", "Postgres only", "--files", "src/database.rs"], "x");
    assert!(pre_edit(&v, "src/database.rs").contains("DECISION [pg] Postgres only"));
});
t!(pre_edit_ignores_superseded_decisions, {
    let v = vault("pe5");
    fs::create_dir_all(v.join("500-knowledge/decisions")).unwrap();
    fs::write(v.join("500-knowledge/decisions/old.md"), "---\ntype: decision\nid: old\nstatus: superseded\nfiles: [src/database.rs]\n---\nx").unwrap();
    assert!(pre_edit(&v, "src/database.rs").is_empty());
});
t!(pre_edit_ignores_unreviewed_candidates, {
    let v = vault("pe6");
    smug(&v, &["propose", "c", "--kind", "failure", "--method", "m", "--reason", "r", "--files", "src/database.rs"], "");
    assert!(pre_edit(&v, "src/database.rs").is_empty());
});
t!(pre_edit_survives_garbage_and_missing_paths, {
    let v = vault("pe7");
    assert_eq!(smug(&v, &["hook", "pre-edit"], "not json").0, 0);
    assert_eq!(smug(&v, &["hook", "pre-edit"], r#"{"tool_input":{}}"#).0, 0);
});
t!(pre_edit_records_a_receipt, {
    let v = vault("pe8");
    smug(&v, &["fail", "f", "--method", "m", "--reason", "r", "--files", "a/b.rs"], "");
    smug(&v, &["hook", "pre-edit"], &json!({"session_id": "S1", "tool_input": {"file_path": "a/b.rs"}}).to_string());
    assert!(smug(&v, &["receipts"], "").1.contains("edit-guard 1"));
});
t!(fail_stores_commit_file_hash_and_verification, {
    let v = vault("ff");
    fs::write(v.join("code.rs"), "fn main() {}").unwrap();
    smug_env(&v, &["fail", "f", "--method", "m", "--reason", "r", "--file", "code.rs", "--commit", "deadbee", "--test-result", "2 failed"], "", &[]);
    let t = fs::read_to_string(v.join("500-knowledge/failures/f.md")).unwrap();
    assert!(t.contains("commit: deadbee") && t.contains("test_result: 2 failed") && t.contains("verified: no") && t.contains("files: [code.rs]"), "{t}");
});
t!(fail_file_sha_matches_the_file, {
    let v = vault("ff2");
    let cwd_file = v.join("code.rs");
    fs::write(&cwd_file, "abc").unwrap();
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_smug")).args(["fail", "f", "--method", "m", "--reason", "r", "--file", "code.rs"]).current_dir(&v).env("SMUG_VAULT", &v).stdin(std::process::Stdio::null()).output().unwrap();
    assert!(o.status.success());
    assert!(fs::read_to_string(v.join("500-knowledge/failures/f.md")).unwrap().contains("file_sha: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"));
});
t!(fail_rejects_a_bad_verified_value, assert_eq!(smug(&vault("ff3"), &["fail", "f", "--method", "m", "--reason", "r", "--verified", "maybe"], "").0, 1));
t!(warn_mentions_verification, {
    let v = vault("fw");
    smug(&v, &["fail", "f", "--method", "pin the compiler nightly", "--reason", "broke CI", "--verified", "yes", "--commit", "abc"], "");
    let o = smug(&v, &["warn", "should we pin the compiler to nightly"], "").1;
    assert!(o.contains("verified yes") && o.contains("commit abc"), "{o}");
});

// ====================== sessions ======================
fn stop_input(v: &Path, session: &str, said: &str) -> String {
    let p = v.join(format!("tr-{session}.jsonl"));
    fs::write(&p, format!("{}\n", json!({"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": said}]}}))).unwrap();
    json!({"session_id": session, "transcript_path": p.to_str().unwrap()}).to_string()
}
fn prompt(v: &Path, session: &str, text: &str) {
    smug(v, &["hook", "prompt"], &json!({"session_id": session, "prompt": text}).to_string());
}
t!(two_sessions_do_not_overwrite_each_others_surfaced_notes, {
    let v = vault("ss");
    fs::write(v.join("500-knowledge/alpha.md"), "alphatopic content").unwrap();
    fs::write(v.join("500-knowledge/beta.md"), "betatopic content").unwrap();
    prompt(&v, "A", "tell me about alphatopic");
    prompt(&v, "B", "tell me about betatopic");
    smug(&v, &["hook", "stop"], &stop_input(&v, "A", "I read alpha.md"));
    let log = smug(&v, &["log", "20"], "").1;
    assert!(log.contains("alpha.md") && !log.contains("beta.md"), "{log}");
});
t!(a_session_cannot_credit_a_note_another_session_surfaced, {
    let v = vault("ss2");
    fs::write(v.join("500-knowledge/alpha.md"), "alphatopic content").unwrap();
    prompt(&v, "A", "alphatopic");
    smug(&v, &["hook", "stop"], &stop_input(&v, "B", "mentions alpha.md but B never saw it surfaced"));
    assert!(!smug(&v, &["log", "20"], "").1.contains("feedback"));
});
t!(session_dirs_hash_the_id, {
    let v = vault("ss3");
    prompt(&v, "super-secret-session-id", "anything at all");
    let names: Vec<String> = fs::read_dir(v.join(".smug/sessions")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    assert!(names.len() == 1 && !names[0].contains("super") && names[0].len() == 16, "{names:?}");
});
t!(no_session_id_uses_the_default_session, {
    let v = vault("ss4");
    smug(&v, &["hook", "prompt"], r#"{"prompt":"anything"}"#);
    assert!(v.join(".smug/sessions/default").exists());
});
t!(receipts_count_per_session, {
    let v = vault("ss5");
    fs::write(v.join("500-knowledge/alpha.md"), "alphatopic").unwrap();
    prompt(&v, "A", "alphatopic");
    smug(&v, &["hook", "stop"], &stop_input(&v, "A", "alpha.md"));
    let o = smug(&v, &["receipts"], "").1;
    assert!(o.contains("surfaced 1") && o.contains("used 1"), "{o}");
});
t!(receipts_empty_vault, assert!(smug(&vault("ss6"), &["receipts"], "").1.contains("no sessions")));
t!(eight_concurrent_sessions_keep_their_own_state, {
    let v = vault("ss7");
    for i in 0..8 {
        fs::write(v.join(format!("500-knowledge/n{i}.md")), format!("topic{i}word body")).unwrap();
    }
    let hs: Vec<_> = (0..8).map(|i| { let v = v.clone(); std::thread::spawn(move || prompt(&v, &format!("S{i}"), &format!("topic{i}word"))) }).collect();
    hs.into_iter().for_each(|h| h.join().unwrap());
    assert_eq!(fs::read_dir(v.join(".smug/sessions")).unwrap().count(), 8);
    for i in 0..8 {
        let d = fs::read_dir(v.join(".smug/sessions")).unwrap().flatten().find(|e| fs::read_to_string(e.path().join("surfaced.json")).unwrap_or_default().contains(&format!("n{i}.md"))).expect("each session surfaced its own note");
        assert!(!fs::read_to_string(d.path().join("surfaced.json")).unwrap().contains(&format!("n{}.md", (i + 1) % 8)));
    }
});
fn end_input(v: &Path, session: &str) -> String {
    let p = v.join("t.jsonl");
    let lines = [json!({"type": "user", "message": {"role": "user", "content": "please use postgres"}}), json!({"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": "ok, decided: postgres.\npassword: hunter2hunter2\ndone"}]}})];
    fs::write(&p, lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n")).unwrap();
    json!({"session_id": session, "transcript_path": p.to_str().unwrap()}).to_string()
}
t!(session_end_captures_the_transcript_redacted, {
    let v = vault("se");
    assert_eq!(smug(&v, &["hook", "session-end"], &end_input(&v, "A")).0, 0);
    let f = fs::read_dir(v.join("000-inbox/captures")).unwrap().flatten().next().unwrap().path();
    let t = fs::read_to_string(f).unwrap();
    assert!(t.contains("user: please use postgres") && t.contains("decided: postgres") && t.contains("[redacted") && !t.contains("hunter2"), "{t}");
});
t!(pre_compact_captures_too_and_a_second_capture_overwrites, {
    let v = vault("se2");
    smug(&v, &["hook", "pre-compact"], &end_input(&v, "A"));
    smug(&v, &["hook", "session-end"], &end_input(&v, "A"));
    assert_eq!(fs::read_dir(v.join("000-inbox/captures")).unwrap().count(), 1);
});
t!(capture_without_transcript_is_silent, assert_eq!(smug(&vault("se3"), &["hook", "session-end"], "{}").0, 0));
t!(capture_cli_redacts_stdin, {
    let v = vault("se4");
    let (c, o, _) = smug(&v, &["capture", "--label", "manual"], "hello\npassword: hunter2hunter2\nbye");
    assert_eq!(c, 0);
    let t = fs::read_to_string(v.join(o.trim())).unwrap();
    assert!(t.contains("hello") && !t.contains("hunter2"));
});
t!(capture_cli_refuses_empty_input, assert_eq!(smug(&vault("se5"), &["capture"], "  \n").0, 1));
t!(captures_are_not_decisions_and_do_not_leak_secrets_into_search, {
    let v = vault("se6");
    smug(&v, &["capture"], "token: abcdefgh12345678\nvisibleword");
    assert!(smug(&v, &["search", "abcdefgh12345678"], "").1.is_empty());
});

// ====================== learning loop ======================
t!(propose_creates_an_inert_candidate, {
    let v = vault("lp");
    assert_eq!(smug(&v, &["propose", "c1", "--kind", "decision", "--source", "chat"], "use redb").0, 0);
    assert!(fs::read_to_string(v.join("500-knowledge/candidates/c1.md")).unwrap().contains("status: candidate"));
    assert!(!v.join("500-knowledge/decisions/c1.md").exists());
});
t!(propose_duplicate_conflicts, {
    let v = vault("lp2");
    smug(&v, &["propose", "c1", "--kind", "decision"], "a");
    assert_eq!(smug(&v, &["propose", "c1", "--kind", "decision"], "b").0, 2);
});
t!(propose_needs_a_kind, assert_eq!(smug(&vault("lp3"), &["propose", "c1"], "a").0, 1));
t!(review_lists_candidates_and_flags_problems, {
    let v = vault("lr");
    smug(&v, &["decide", "live", "--source", "s", "--topic", "db"], "x");
    smug(&v, &["propose", "c1", "--kind", "decision", "--topic", "db"], "no source and a conflict");
    let o = smug(&v, &["review"], "").1;
    assert!(o.contains("NO SOURCE") && o.contains("CONFLICTS with live"), "{o}");
});
t!(review_when_empty, assert!(smug(&vault("lr2"), &["review"], "").1.contains("no candidates")));
t!(promote_a_decision, {
    let v = vault("lm");
    smug(&v, &["propose", "c1", "--kind", "decision", "--source", "adr-5", "--topic", "state"], "use redb");
    assert_eq!(smug(&v, &["promote", "c1"], "").0, 0);
    let d = fs::read_to_string(v.join("500-knowledge/decisions/c1.md")).unwrap();
    assert!(d.contains("type: decision") && d.contains("status: active") && d.contains("source: adr-5") && d.contains("promoted_from"), "{d}");
    assert!(fs::read_to_string(v.join("500-knowledge/candidates/c1.md")).unwrap().contains("status: promoted"));
    assert_eq!(smug(&v, &["check", "c1"], "").0, 0);
});
t!(promote_refuses_a_decision_without_a_source, {
    let v = vault("lm2");
    smug(&v, &["propose", "c1", "--kind", "decision"], "x");
    assert_eq!(smug(&v, &["promote", "c1"], "").0, 1);
    assert!(!v.join("500-knowledge/decisions/c1.md").exists());
    assert_eq!(smug(&v, &["promote", "c1", "--source", "meeting"], "").0, 0);
});
t!(promote_refuses_a_topic_conflict_and_points_to_patch, {
    let v = vault("lm3");
    smug(&v, &["decide", "live", "--source", "s", "--topic", "db"], "x");
    smug(&v, &["propose", "c1", "--kind", "decision", "--source", "s2", "--topic", "db"], "y");
    let (c, _, e) = smug(&v, &["promote", "c1"], "");
    assert!(c == 1 && e.contains("smug patch live"), "{e}");
    assert_eq!(smug(&v, &["promote", "c1", "--allow-conflict"], "").0, 0);
});
t!(promote_a_failure_keeps_its_fields, {
    let v = vault("lm4");
    smug(&v, &["propose", "f1", "--kind", "failure", "--method", "retry forever", "--reason", "hammered api", "--files", "src/net.rs"], "details");
    assert_eq!(smug(&v, &["promote", "f1"], "").0, 0);
    let f = fs::read_to_string(v.join("500-knowledge/failures/f1.md")).unwrap();
    assert!(f.contains("method: retry forever") && f.contains("reason: hammered api") && f.contains("files: [src/net.rs]") && f.contains("verified: no"), "{f}");
    assert!(smug(&v, &["warn", "should we retry forever here"], "").1.contains("hammered api"));
});
t!(promote_twice_errors, {
    let v = vault("lm5");
    smug(&v, &["propose", "c1", "--kind", "decision", "--source", "s"], "x");
    smug(&v, &["promote", "c1"], "");
    assert_eq!(smug(&v, &["promote", "c1"], "").0, 1);
});
t!(promote_unknown_candidate_errors, assert_eq!(smug(&vault("lm6"), &["promote", "ghost"], "").0, 1));
t!(reject_marks_and_blocks_promotion, {
    let v = vault("lj");
    smug(&v, &["propose", "c1", "--kind", "decision", "--source", "s"], "x");
    assert_eq!(smug(&v, &["reject", "c1", "--reason", "wrong"], "").0, 0);
    let t = fs::read_to_string(v.join("500-knowledge/candidates/c1.md")).unwrap();
    assert!(t.contains("status: rejected") && t.contains("rejected_reason: wrong"));
    assert_eq!(smug(&v, &["promote", "c1"], "").0, 1);
    assert!(smug(&v, &["review"], "").1.contains("no candidates"));
});
t!(candidates_never_reach_context_or_search_or_failure_warnings, {
    let v = vault("lc");
    smug(&v, &["propose", "f1", "--kind", "failure", "--method", "drop the production table", "--reason", "oops"], "candidateonlyword");
    assert!(smug(&v, &["warn", "drop the production table"], "").1.is_empty());
    assert!(smug(&v, &["search", "candidateonlyword"], "").1.is_empty());
    assert!(!smug(&v, &["context"], "").1.contains("f1"));
});
t!(learn_requires_a_model_and_propose, {
    let v = vault("ln");
    fs::write(v.join("000-inbox/c.md"), "x").unwrap();
    assert_eq!(smug(&v, &["learn", "000-inbox/c.md", "--propose"], "").0, 1);
    assert_eq!(smug(&v, &["learn", "000-inbox/c.md", "--model", "cat"], "").0, 1);
});
#[cfg(unix)]
mod fake_model {
    use super::*;
    const MODEL: &str = r#"printf '%s\n' '{"kind":"decision","id":"use-redb","text":"State lives in redb.","topic":"state"}' '{"kind":"failure","id":"retry-loop","text":"it spun","method":"retry forever","reason":"hammered the API","files":["src/net.rs"]}' 'not json' '{"kind":"law","id":"bad","text":"x"}' '{"kind":"decision","id":"../evil","text":"x"}'"#;
    t!(learn_proposes_candidates_from_a_capture, {
        let v = vault("lf");
        smug(&v, &["capture", "--label", "s1"], "we decided to use redb");
        let cap = fs::read_dir(v.join("000-inbox/captures")).unwrap().flatten().next().unwrap().path();
        let rel = format!("000-inbox/captures/{}", cap.file_name().unwrap().to_string_lossy());
        let (c, o, _) = smug(&v, &["learn", &rel, "--model", MODEL, "--propose"], "");
        assert_eq!(c, 0, "{o}");
        assert!(o.contains("2 candidate(s) proposed, 2 skipped"), "{o}");
        assert!(v.join("500-knowledge/candidates/use-redb.md").exists() && v.join("500-knowledge/candidates/retry-loop.md").exists());
        assert!(fs::read_to_string(v.join("500-knowledge/candidates/use-redb.md")).unwrap().contains("source: capture:"));
        assert!(!v.join("500-knowledge/decisions/use-redb.md").exists(), "learn must never create memory");
    });
    t!(learn_survives_a_model_that_says_nothing, {
        let v = vault("lf2");
        fs::write(v.join("000-inbox/c.md"), "x").unwrap();
        let (c, o, _) = smug(&v, &["learn", "000-inbox/c.md", "--model", "true", "--propose"], "");
        assert!(c == 0 && o.contains("0 candidate(s)"), "{o}");
    });
    t!(learn_reports_a_failing_model, {
        let v = vault("lf3");
        fs::write(v.join("000-inbox/c.md"), "x").unwrap();
        assert_eq!(smug(&v, &["learn", "000-inbox/c.md", "--model", "false", "--propose"], "").0, 1);
    });
    t!(learn_does_not_let_the_model_write_secrets, {
        let v = vault("lf4");
        fs::write(v.join("000-inbox/c.md"), "x").unwrap();
        let m = r#"echo '{"kind":"failure","id":"s","text":"password: hunter2hunter2","method":"m","reason":"r"}'"#;
        let (_, o, _) = smug(&v, &["learn", "000-inbox/c.md", "--model", m, "--propose"], "");
        assert!(o.contains("0 candidate(s)") && !v.join("500-knowledge/candidates/s.md").exists(), "{o}");
    });
    t!(model_runs_outside_the_vault, {
        let v = vault("mo");
        smug(&v, &["decide", "d", "--source", "s"], "dbword");
        fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"dbword\",\"answer_absent\":[\"CLAUDE.md\"]}\n").unwrap();
        // `ls` lists the model's working directory; if it were the vault it would print CLAUDE.md
        assert_eq!(smug(&v, &["eval", "--model", "ls"], "").0, 0);
    });
    t!(model_times_out, {
        let v = vault("mo2");
        fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"x\",\"answer_contains\":[\"y\"]}\n").unwrap();
        let (c, _, e) = smug_env(&v, &["eval", "--model", "sleep 5"], "", &[("SMUG_MODEL_TIMEOUT", "1")]);
        assert!(c == 1 && e.contains("timed out"), "{e}");
    });
    t!(model_output_is_capped_not_hung, {
        let v = vault("mo3");
        fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"x\",\"answer_contains\":[\"y\"]}\n").unwrap();
        assert_eq!(smug(&v, &["eval", "--model", "yes | head -c 400000"], "").0, 0);
    });
    t!(timeout_flag_is_accepted, {
        let v = vault("mo4");
        fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"x\",\"answer_contains\":[\"y\"]}\n").unwrap();
        let (c, _, e) = smug(&v, &["eval", "--model", "sleep 5", "--timeout", "1"], "");
        assert!(c == 1 && e.contains("timed out"), "{e}");
    });
    // ---- memory on vs off
    t!(ab_measures_the_lift_memory_gives, {
        let v = vault("ab");
        smug(&v, &["decide", "pg", "--source", "adr", "--title", "database"], "we use postgres for storage");
        fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"which database do we use\",\"answer_contains\":[\"we use postgres\"]}\n").unwrap();
        // `cat` repeats its prompt, so only the memory-on prompt contains the note text
        let (c, o, _) = smug(&v, &["eval", "--ab", "--model", "cat"], "");
        assert_eq!(c, 0, "{o}");
        assert!(o.contains("memory on: 1/1") && o.contains("memory off: 0/1") && o.contains("lift: +1"), "{o}");
    });
    t!(ab_fails_when_memory_hurts, {
        let v = vault("ab2");
        smug(&v, &["decide", "pg", "--source", "adr"], "polluting note text");
        fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"polluting\",\"answer_absent\":[\"polluting note text\"]}\n").unwrap();
        let (c, o, _) = smug(&v, &["eval", "--ab", "--model", "cat"], "");
        assert!(c == 1 && o.contains("lift: -"), "{o}");
    });
}
t!(ab_needs_a_model, {
    let v = vault("ab3");
    fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"x\",\"answer_contains\":[\"y\"]}\n").unwrap();
    assert_eq!(smug(&v, &["eval", "--ab"], "").0, 1);
});
t!(ab_needs_answer_cases, {
    let v = vault("ab4");
    fs::write(v.join("evals/cases.jsonl"), "{\"q\":\"x\"}\n").unwrap();
    assert_eq!(smug(&v, &["eval", "--ab", "--model", "cat"], "").0, 1);
});
