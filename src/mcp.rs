//! MCP server over stdio (newline-delimited JSON-RPC 2.0). Six tools, one core: the same functions the CLI uses.
//! Read tools never write. `smug_propose` writes only inert candidates; promotion is a human CLI step and is not exposed.
use crate::{learn, mem, store::State, util::*};
use serde_json::{json, Value};
use std::{
    io::{BufRead, Write},
    path::Path,
};

const READ_ONLY: [&str; 4] = ["smug_recall", "smug_check", "smug_explain", "smug_impact"];

fn tool_defs(read_only: bool) -> Value {
    let all = json!([
        {"name":"smug_recall","description":"Find relevant, still-valid notes and decisions for a query. Replaced decisions are never returned. Also warns about things that already failed.","inputSchema":{"type":"object","properties":{"query":{"type":"string"},"k":{"type":"integer","minimum":1,"maximum":20}},"required":["query"]},"annotations":{"readOnlyHint":true}},
        {"name":"smug_check","description":"Is a decision or failure still valid? Returns CURRENT, STALE, CONFLICTED, UNSUPPORTED or UNKNOWN with the reason. Call before acting on a remembered decision.","inputSchema":{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]},"annotations":{"readOnlyHint":true}},
        {"name":"smug_explain","description":"Source, validity dates, what it replaced or was replaced by, and its write history.","inputSchema":{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]},"annotations":{"readOnlyHint":true}},
        {"name":"smug_impact","description":"Every note that depends on a decision, transitively. Use before proposing a change to it.","inputSchema":{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]},"annotations":{"readOnlyHint":true}},
        {"name":"smug_propose","description":"Record something worth remembering as an INERT candidate. It is not memory until a human promotes it.","inputSchema":{"type":"object","properties":{"kind":{"type":"string","enum":["decision","failure"]},"id":{"type":"string","description":"kebab-case id"},"text":{"type":"string"},"source":{"type":"string"},"topic":{"type":"string"},"method":{"type":"string"},"reason":{"type":"string"},"files":{"type":"array","items":{"type":"string"}}},"required":["kind","id","text"]},"annotations":{"readOnlyHint":false,"destructiveHint":false}},
        {"name":"smug_feedback","description":"Record whether a surfaced note was used, useful or harmful. Nudges future ranking.","inputSchema":{"type":"object","properties":{"path":{"type":"string"},"kind":{"type":"string","enum":["used","useful","harmful"]}},"required":["path","kind"]},"annotations":{"readOnlyHint":false,"destructiveHint":false}}
    ]);
    let arr: Vec<Value> = all.as_array().unwrap().iter().filter(|t| !read_only || READ_ONLY.contains(&t["name"].as_str().unwrap_or(""))).cloned().collect();
    Value::Array(arr)
}

fn arg<'a>(a: &'a Value, k: &str) -> Result<&'a str, String> {
    a[k].as_str().ok_or_else(|| format!("missing string argument '{k}'"))
}

fn call(root: &Path, read_only: bool, name: &str, a: &Value) -> Result<Value, String> {
    if read_only && !READ_ONLY.contains(&name) {
        return Err("this server runs with --read-only".into());
    }
    let e = |x: Box<dyn std::error::Error>| x.to_string();
    match name {
        "smug_recall" => {
            let q = arg(a, "query")?;
            let k = a["k"].as_u64().unwrap_or(5).clamp(1, 20) as usize;
            let st = State::open(root).map_err(e)?;
            let hits = mem::search_valid(root, &st, q, k, false).map_err(e)?;
            let warnings = mem::warn_texts(&mem::load_notes(root), q);
            Ok(json!({"notes": hits.iter().map(|h| json!({"path": h.path, "snippet": h.snippet})).collect::<Vec<_>>(), "warnings": warnings,
                "note": if hits.is_empty() { "no relevant memory" } else { "candidates: open the note before relying on it" }}))
        }
        "smug_check" => {
            let id = arg(a, "id")?;
            let (status, why) = mem::check_status(&mem::load_notes(root), id);
            Ok(json!({"id": id, "status": status, "reason": why}))
        }
        "smug_explain" => {
            let id = arg(a, "id")?;
            let notes = mem::load_notes(root);
            let n = notes.iter().find(|n| n.fm.get("id").map(String::as_str) == Some(id)).ok_or("no note with that id")?;
            let st = State::open(root).map_err(e)?;
            let history: Vec<Value> = st.events().map_err(e)?.into_iter().filter(|x| x.path == n.rel && x.kind != "intent").map(|x| json!({"id": x.id, "ts": x.ts, "kind": x.kind})).collect();
            let (status, why) = mem::check_status(&notes, id);
            let f = |k: &str| n.fm.get(k).cloned().unwrap_or_default();
            Ok(json!({"id": id, "path": n.rel, "type": f("type"), "status": status, "reason": why, "source": f("source"), "valid_from": f("valid_from"), "valid_to": f("valid_to"),
                "known_at": f("known_at"), "supersedes": f("supersedes"), "superseded_by": f("superseded_by"), "commit": f("commit"), "verified": f("verified"), "history": history}))
        }
        "smug_impact" => {
            let id = arg(a, "id")?;
            let r = mem::impact_of(&mem::load_notes(root), id);
            Ok(json!({"id": id, "dependents": r.iter().map(|(p, d)| json!({"path": p, "depth": d})).collect::<Vec<_>>()}))
        }
        "smug_propose" => {
            let kind = arg(a, "kind")?;
            let id = arg(a, "id")?;
            let s = |k: &str| a[k].as_str().unwrap_or("").to_string();
            let files = a["files"].as_array().map(|x| x.iter().filter_map(|y| y.as_str()).collect::<Vec<_>>().join(",")).unwrap_or_default();
            let fields = [("topic", s("topic")), ("source", s("source")), ("method", s("method")), ("reason", s("reason")), ("files", if files.is_empty() { String::new() } else { format!("[{files}]") })];
            let st = State::open(root).map_err(e)?;
            match learn::write_candidate(&st, kind, id, &fields, arg(a, "text")?).map_err(e)? {
                Ok(_) => Ok(json!({"candidate": format!("{}/{id}.md", learn::CAND_DIR), "status": "candidate", "next": "a human reviews it with `smug review` and `smug promote`"})),
                Err(m) => Err(m),
            }
        }
        "smug_feedback" => {
            let p = arg(a, "path")?;
            safe_rel(p)?;
            let delta = match arg(a, "kind")? {
                "used" => 1,
                "useful" => 2,
                "harmful" => -3,
                _ => return Err("kind must be used, useful or harmful".into()),
            };
            let st = State::open(root).map_err(e)?;
            st.append("feedback", p, arg(a, "kind")?, None).map_err(e)?;
            Ok(json!({"path": p, "utility": st.add_feedback(p, delta).map_err(e)?}))
        }
        _ => Err(format!("unknown tool '{name}'")),
    }
}

pub fn handle(root: &Path, read_only: bool, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg["method"].as_str().unwrap_or("");
    let reply = |result: Value| id.clone().map(|i| json!({"jsonrpc": "2.0", "id": i, "result": result}));
    let error = |code: i64, m: &str| id.clone().map(|i| json!({"jsonrpc": "2.0", "id": i, "error": {"code": code, "message": m}}));
    match method {
        "initialize" => reply(json!({
            "protocolVersion": msg["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "smug", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "Call smug_recall before decisions, smug_check before acting on a remembered decision, smug_propose after learning something durable. Proposals are inert until a human promotes them."
        })),
        "ping" => reply(json!({})),
        "tools/list" => reply(json!({"tools": tool_defs(read_only)})),
        "tools/call" => {
            let name = msg["params"]["name"].as_str().unwrap_or("");
            match call(root, read_only, name, &msg["params"]["arguments"]) {
                Ok(v) => reply(json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_default()}], "structuredContent": v, "isError": false})),
                Err(m) => reply(json!({"content": [{"type": "text", "text": m}], "isError": true})),
            }
        }
        m if m.starts_with("notifications/") => None,
        _ => error(-32601, &format!("method not found: {method}")),
    }
}

pub fn serve(root: &Path, args: &[String]) -> R {
    let read_only = args.iter().any(|a| a == "--read-only");
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let resp = match serde_json::from_str::<Value>(&line) {
            Ok(m) => handle(root, read_only, &m),
            Err(_) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "parse error"}})),
        };
        if let Some(r) = resp {
            writeln!(out, "{r}")?;
            out.flush()?;
        }
    }
    Ok(0)
}
