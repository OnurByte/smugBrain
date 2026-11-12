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

