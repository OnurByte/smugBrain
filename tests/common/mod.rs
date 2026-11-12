#![allow(dead_code)]
use std::{fs, io::Write, path::{Path, PathBuf}, process::{Command, Stdio}};

pub fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("smug-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

pub fn smug(vault: &Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_smug"))
        .args(args).env("SMUG_VAULT", vault)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let _ = c.stdin.take().unwrap().write_all(stdin.as_bytes()); // smug may exit before reading (EPIPE is fine)
    let o = c.wait_with_output().unwrap();
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into())
}

pub fn vault(name: &str) -> PathBuf {
    let d = tmp(name);
    let v = d.join("v");
    assert_eq!(smug(&d, &["init", v.to_str().unwrap()], "").0, 0);
    v
}


pub fn smug_env(vault: &std::path::Path, args: &[&str], stdin: &str, envs: &[(&str, &str)]) -> (i32, String, String) {
    use std::io::Write;
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_smug"));
    cmd.args(args).env("SMUG_VAULT", vault).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut c = cmd.spawn().unwrap();
    let _ = c.stdin.take().unwrap().write_all(stdin.as_bytes());
    let o = c.wait_with_output().unwrap();
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into())
}

/// Talks to `smug mcp` like a harness: newline-delimited JSON-RPC in, responses out.
pub fn mcp(vault: &std::path::Path, flags: &[&str], reqs: &[serde_json::Value]) -> Vec<serde_json::Value> {
    use std::io::Write;
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_smug"))
        .arg("mcp").args(flags).env("SMUG_VAULT", vault)
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn().unwrap();
    let mut i = c.stdin.take().unwrap();
    for r in reqs {
        writeln!(i, "{r}").unwrap();
    }
    drop(i);
    let o = c.wait_with_output().unwrap();
    String::from_utf8_lossy(&o.stdout).lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

pub fn rpc(id: u64, method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

/// One tool call; returns the result object.
pub fn tool(vault: &std::path::Path, flags: &[&str], name: &str, args: serde_json::Value) -> serde_json::Value {
    let r = mcp(vault, flags, &[rpc(1, "tools/call", serde_json::json!({"name": name, "arguments": args}))]);
    r[0]["result"].clone()
}
