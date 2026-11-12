//! Agent adapter registry and safe installer: merges one `smug` entry into each harness's MCP config without touching
//! anything else, refuses symlinks and unparsable files, supports --dry-run, and can undo itself.
use crate::util::*;
use serde_json::{json, Map, Value};
use std::{fs, path::{Path, PathBuf}};
use toml_edit::{value, Array, DocumentMut, Item, Table};

const NAME: &str = "smug";

#[derive(Clone, Copy, PartialEq, Debug)]
enum Shape {
    Servers { key: &'static str, typed: bool },
    OpenCode,
    Toml,
    PrintOnly,
}

struct Adapter {
    id: &'static str,
    label: &'static str,
    shape: Shape,
    project: Option<&'static str>,
    global: Option<&'static str>,
    detect: Option<&'static str>,
    /// What has actually been verified. Never "live-tested" without a live test.
    support: &'static str,
}

const S: Shape = Shape::Servers { key: "mcpServers", typed: false };

const ALL: &[Adapter] = &[
    Adapter { id: "claude-code", label: "Claude Code", shape: S, project: Some(".mcp.json"), global: Some(".claude.json"), detect: Some(".claude"), support: "MCP config tested, hooks + skill installed, protocol tested with a test client" },
    Adapter { id: "codex", label: "Codex CLI", shape: Shape::Toml, project: None, global: Some(".codex/config.toml"), detect: Some(".codex"), support: "live-tested: Codex CLI 0.167.2, Linux, one session (smug_check over MCP); config tested; AGENTS.md instructions" },
    Adapter { id: "cursor", label: "Cursor", shape: S, project: Some(".cursor/mcp.json"), global: Some(".cursor/mcp.json"), detect: Some(".cursor"), support: "MCP config tested" },
    Adapter { id: "gemini", label: "Gemini CLI", shape: S, project: Some(".gemini/settings.json"), global: Some(".gemini/settings.json"), detect: Some(".gemini"), support: "MCP config tested" },
    Adapter { id: "windsurf", label: "Windsurf", shape: S, project: None, global: Some(".codeium/windsurf/mcp_config.json"), detect: Some(".codeium/windsurf"), support: "MCP config tested" },
    Adapter { id: "kiro", label: "Kiro", shape: S, project: Some(".kiro/settings/mcp.json"), global: Some(".kiro/settings/mcp.json"), detect: Some(".kiro"), support: "MCP config tested" },
    Adapter { id: "vscode", label: "VS Code", shape: Shape::Servers { key: "servers", typed: true }, project: Some(".vscode/mcp.json"), global: None, detect: None, support: "MCP config tested" },
    Adapter { id: "opencode", label: "OpenCode", shape: Shape::OpenCode, project: Some("opencode.json"), global: Some(".config/opencode/opencode.json"), detect: Some(".config/opencode"), support: "live-tested: OpenCode 2.2.7, Linux, one session (smug_check over MCP); config tested" },
    Adapter { id: "zed", label: "Zed", shape: Shape::PrintOnly, project: None, global: None, detect: None, support: "manual: prints the snippet (settings.json allows comments)" },
    Adapter { id: "cline", label: "Cline", shape: Shape::PrintOnly, project: None, global: None, detect: None, support: "manual: prints the snippet" },
];

/// What a tool should run. The vault is passed explicitly: a tool started from another project must not look in its cwd.
fn launch(root: &Path) -> (String, Vec<String>) {
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| "smug".into());
    let vault = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf()).to_string_lossy().trim_start_matches("\\\\?\\").to_string();
    (exe, vec!["mcp".into(), "--vault".into(), vault])
}

fn entry(a: &Adapter, nested: bool, root: &Path) -> Value {
    let (cmd, args) = launch(root);
    let argv: Vec<String> = std::iter::once(cmd.clone()).chain(args.clone()).collect();
    match a.shape {
        Shape::Servers { typed: true, .. } => json!({"type": "stdio", "command": cmd, "args": args}),
        Shape::OpenCode if nested => json!({"type": "local", "command": argv}),
        Shape::OpenCode => json!({"type": "local", "command": argv, "enabled": true}),
        _ => json!({"command": cmd, "args": args}),
    }
}

fn snippet(a: &Adapter, root: &Path) -> String {
    let (cmd, args) = launch(root);
    match a.shape {
        Shape::Toml => format!("[mcp_servers.{NAME}]\ncommand = {}\nargs = {}\n", serde_json::to_string(&cmd).unwrap_or_default(), serde_json::to_string(&args).unwrap_or_default()),
        Shape::OpenCode => serde_json::to_string_pretty(&json!({"mcp": {"servers": {NAME: entry(a, true, root)}}})).unwrap_or_default(),
        Shape::Servers { key, .. } => serde_json::to_string_pretty(&json!({key: {NAME: entry(a, false, root)}})).unwrap_or_default(),
        Shape::PrintOnly => serde_json::to_string_pretty(&json!({NAME: {"command": cmd, "args": args}})).unwrap_or_default(),
    }
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Created(PathBuf),
    Updated(PathBuf),
    Removed(PathBuf),
    Unchanged(PathBuf),
    Refused(String),
}

fn path_for(a: &Adapter, global: bool, home: &Path, repo: &Path) -> Option<PathBuf> {
    if global { a.global.map(|p| home.join(p)) } else { a.project.map(|p| repo.join(p)) }
}

fn edit_json(existing: Option<&str>, a: &Adapter, force: bool, remove: bool, vroot: &Path) -> Result<Option<String>, String> {
    let mut root: Value = match existing {
        Some(t) if !t.trim().is_empty() => serde_json::from_str(t).map_err(|e| format!("not valid JSON ({e}); it may contain comments"))?,
        _ => json!({}),
    };
    let obj = root.as_object_mut().ok_or("top level is not an object")?;
    let (container, nested): (&mut Map<String, Value>, bool) = match a.shape {
        Shape::Servers { key, .. } => (obj.entry(key).or_insert_with(|| json!({})).as_object_mut().ok_or("servers entry is not an object")?, false),
        Shape::OpenCode => {
            let mcp = obj.entry("mcp").or_insert_with(|| json!({}));
            let m = mcp.as_object_mut().ok_or("mcp is not an object")?;
            if m.contains_key(NAME) && !m.contains_key("servers") {
                (mcp.as_object_mut().unwrap(), false) // older flat layout is kept
            } else {
                (mcp.as_object_mut().unwrap().entry("servers").or_insert_with(|| json!({})).as_object_mut().ok_or("mcp.servers is not an object")?, true)
            }
        }
        _ => return Err("not a JSON adapter".into()),
    };
    let want = entry(a, nested, vroot);
    if remove {
        return Ok(container.remove(NAME).map(|_| serde_json::to_string_pretty(&root).unwrap_or_default() + "\n"));
    }
    match container.get(NAME) {
        Some(cur) if *cur == want => return Ok(None),
        Some(_) if !force => return Err("an existing 'smug' entry differs; use --force to replace it".into()),
        _ => {}
    }
    container.insert(NAME.into(), want);
    Ok(Some(serde_json::to_string_pretty(&root).unwrap_or_default() + "\n"))
}

fn edit_toml(existing: Option<&str>, force: bool, remove: bool, root: &Path) -> Result<Option<String>, String> {
    let mut doc: DocumentMut = existing.unwrap_or("").parse().map_err(|e| format!("not valid TOML ({e})"))?;
    let (cmd, args) = launch(root);
    if remove {
        let had = doc.get("mcp_servers").and_then(|t| t.as_table()).is_some_and(|t| t.contains_key(NAME));
        if had {
            doc["mcp_servers"].as_table_mut().unwrap().remove(NAME);
        }
        return Ok(had.then(|| doc.to_string()));
    }
    let servers = doc.entry("mcp_servers").or_insert(Item::Table(Table::new())).as_table_mut().ok_or("mcp_servers is not a table")?;
    servers.set_implicit(true);
    let mut arr = Array::new();
    args.iter().for_each(|a| arr.push(a.as_str()));
    if let Some(cur) = servers.get(NAME).and_then(|i| i.as_table()) {
        let same = cur.get("command").and_then(|v| v.as_str()) == Some(cmd.as_str()) && cur.get("args").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect::<Vec<_>>()) == Some(args.clone());
        if same {
            return Ok(None);
        }
        if !force {
            return Err("an existing [mcp_servers.smug] differs; use --force to replace it".into());
        }
    }
    let mut t = Table::new();
    t["command"] = value(cmd);
    t["args"] = value(arr);
    servers.insert(NAME, Item::Table(t));
    Ok(Some(doc.to_string()))
}

fn run(a: &Adapter, global: bool, home: &Path, repo: &Path, dry: bool, force: bool, remove: bool) -> Outcome {
    if a.shape == Shape::PrintOnly {
        return Outcome::Refused(format!("{} needs a manual edit ({}). Add this:\n{}", a.label, a.support, snippet(a, repo)));
    }
    let Some(path) = path_for(a, global, home, repo) else {
        return Outcome::Refused(format!("{} has no {} config location. Add this by hand:\n{}", a.label, if global { "global" } else { "project" }, snippet(a, repo)));
    };
    if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Outcome::Refused(format!("{} is a symbolic link, left untouched. Edit its target by hand:\n{}", path.display(), snippet(a, repo)));
    }
    let existing = match fs::read_to_string(&path) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Outcome::Refused(format!("cannot read {}: {e}", path.display())),
    };
    let edited = match a.shape {
        Shape::Toml => edit_toml(existing.as_deref(), force, remove, repo),
        _ => edit_json(existing.as_deref(), a, force, remove, repo),
    };
    match edited {
        Err(m) => Outcome::Refused(format!("{}: {m}. Nothing was written. Add this by hand:\n{}", path.display(), snippet(a, repo))),
        Ok(None) => Outcome::Unchanged(path),
        Ok(Some(text)) => {
            if !dry {
                if let Some(d) = path.parent() {
                    let _ = fs::create_dir_all(d);
                }
                let tmp = path.with_extension("smug-tmp");
                if let Err(e) = fs::write(&tmp, &text).and_then(|_| fs::rename(&tmp, &path)) {
                    return Outcome::Refused(format!("write failed: {e}"));
                }
            }
            if remove { Outcome::Removed(path) } else if existing.is_some() { Outcome::Updated(path) } else { Outcome::Created(path) }
        }
    }
}

const SKILL_VERSION: &str = "1";
fn skill_text() -> String {
    format!("---\nname: smug\ndescription: Use smug memory. Recall decisions and past failures before acting, check a remembered decision before relying on it, propose lessons after learning something durable.\nsmug-skill-version: {SKILL_VERSION}\n---\n\n# smug memory\n\nThe project's memory is Markdown managed by `smug`. Use the MCP tools (or the `smug` CLI):\n\n1. **Before a non-trivial change**: `smug_recall` with the task, and `smug_check` on any decision you plan to rely on. Never act on a STALE or CONFLICTED one.\n2. **Before changing a decision**: `smug_impact`, then tell the human. Do not edit decision notes by hand; the human runs `smug patch`.\n3. **When a test fails or you learn something durable**: `smug_propose` (kind `failure` with method and reason, or `decision` with a source). Proposals are inert until a human promotes them.\n4. **Never** write secrets into notes. Never promote your own proposals.\n")
}

fn install_skill(repo: &Path, dry: bool, remove: bool) -> String {
    let p = repo.join(".claude/skills/smug/SKILL.md");
    if remove {
        if !dry {
            let _ = fs::remove_file(&p);
        }
        return format!("skill removed: {}", p.display());
    }
    if fs::read_to_string(&p).is_ok_and(|t| t.contains(&format!("smug-skill-version: {SKILL_VERSION}"))) {
        return format!("skill up to date: {}", p.display());
    }
    if !dry {
        let _ = fs::create_dir_all(p.parent().unwrap());
        let _ = fs::write(&p, skill_text());
    }
    format!("skill written: {}", p.display())
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("SMUG_HOME").or_else(|_| std::env::var("HOME")).or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".into()))
}

pub fn agent(root: &Path, a: &[String]) -> R {
    let sub = a.first().map(String::as_str).unwrap_or("list");
    let rest = a.get(1..).unwrap_or(&[]);
    let (dry, force, global) = (rest.iter().any(|x| x == "--dry-run"), rest.iter().any(|x| x == "--force"), rest.iter().any(|x| x == "--global"));
    let h = home();
    match sub {
        "list" => {
            for ad in ALL {
                let detected = ad.detect.is_some_and(|d| h.join(d).exists());
                println!("{:<12} {:<12} {}{}", ad.id, ad.label, ad.support, if detected { "  [detected]" } else { "" });
            }
            Ok(0)
        }
        "install" | "uninstall" => {
            let remove = sub == "uninstall";
            let want = rest.iter().find(|x| !x.starts_with("--")).map(String::as_str).ok_or("usage: smug agent install <id|detected|all> [--global] [--dry-run] [--force]")?;
            let chosen: Vec<&Adapter> = match want {
                "all" => ALL.iter().collect(),
                "detected" => ALL.iter().filter(|x| x.detect.is_some_and(|d| h.join(d).exists())).collect(),
                id => vec![ALL.iter().find(|x| x.id == id).ok_or(format!("unknown agent '{id}'; run `smug agent list`"))?],
            };
            if chosen.is_empty() {
                println!("no supported coding tool detected in {}", h.display());
            }
            let mut bad = 0;
            for ad in chosen {
                let o = run(ad, global, &h, root, dry, force, remove);
                let line = match &o {
                    Outcome::Created(p) => format!("created {}", p.display()),
                    Outcome::Updated(p) => format!("updated {}", p.display()),
                    Outcome::Removed(p) => format!("removed from {}", p.display()),
                    Outcome::Unchanged(p) => format!("already set up: {}", p.display()),
                    Outcome::Refused(m) => {
                        bad += 1;
                        format!("NOT DONE: {m}")
                    }
                };
                println!("{:<12} {}{}", ad.id, if dry { "(dry run) " } else { "" }, line);
                if ad.id == "claude-code" && !matches!(o, Outcome::Refused(_)) {
                    if remove {
                        println!("{:<12} {}", "", install_skill(root, dry, true));
                    } else {
                        println!("{:<12} {}", "", install_skill(root, dry, false));
                        if dry {
                            println!("{:<12} (dry run) would add smug hooks to {}: SessionStart, UserPromptSubmit, PreToolUse, Stop, SessionEnd, PreCompact (other hooks kept)", "", root.join(".claude/settings.json").display());
                        } else {
                            crate::cmds::setup(root, &["claude".to_string()])?;
                        }
                    }
                }
                if ad.id == "codex" && !remove && !matches!(o, Outcome::Refused(_)) {
                    if dry {
                        println!("{:<12} (dry run) would add a `## smug` section to {} if it has none", "", root.join("AGENTS.md").display());
                    } else {
                        crate::cmds::setup(root, &["codex".to_string()])?;
                    }
                }
            }
            Ok(if bad > 0 { 1 } else { 0 })
        }
        "table" => {
            println!("{}", table());
            Ok(0)
        }
        "doctor" => {
            // Starts the server exactly as a tool would (same command, same --vault) but from a DIFFERENT directory and
            // without SMUG_VAULT: if the vault path were not passed explicitly, this is where it would break.
            // Writes are best-effort: a server that refuses the directory exits before reading, and that is a FAIL, not an I/O error.
            use std::io::{BufRead, BufReader, Write};
            let (cmd, args) = launch(root);
            let want = args[2].clone();
            let mut ch = std::process::Command::new(cmd).args(args).current_dir(std::env::temp_dir()).env_remove("SMUG_VAULT").stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn()?;
            let mut stdin = ch.stdin.take().unwrap();
            let _ = writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"doctor","version":"0"}}}}}}"#);
            let _ = writeln!(stdin, r#"{{"jsonrpc":"2.0","id":2,"method":"tools/list"}}"#);
            let _ = writeln!(stdin, r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"smug_recall","arguments":{{"query":"doctor"}}}}}}"#);
            drop(stdin);
            let (mut tools, mut vault_seen, mut recall_ok) = (0, String::new(), false);
            for l in BufReader::new(ch.stdout.take().unwrap()).lines().map_while(Result::ok) {
                if let Ok(v) = serde_json::from_str::<Value>(&l) {
                    match v["id"].as_u64() {
                        Some(2) => tools = v["result"]["tools"].as_array().map(|t| t.len()).unwrap_or(0),
                        Some(3) => {
                            recall_ok = v["result"]["isError"] == false;
                            vault_seen = v["result"]["structuredContent"]["vault"].as_str().unwrap_or("").to_string();
                        }
                        _ => {}
                    }
                }
            }
            let _ = ch.wait();
            let right_vault = vault_seen == want;
            println!("{} smug mcp answers: {tools} tools", if tools == 6 { "ok  " } else { "FAIL" });
            println!("{} serves the right vault from any directory: {}", if right_vault && recall_ok { "ok  " } else { "FAIL" }, if vault_seen.is_empty() { "(no answer)".into() } else { vault_seen });
            Ok(if tools == 6 && right_vault && recall_ok { 0 } else { 1 })
        }
        _ => Err("usage: smug agent list|install|uninstall|doctor".into()),
    }
}

/// For the README: the harness table, straight from the registry.
pub fn table() -> String {
    ALL.iter().map(|a| format!("| {} | `smug agent install {}` | {} |", a.label, a.id, a.support)).collect::<Vec<_>>().join("\n")
}
