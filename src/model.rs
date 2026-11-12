//! ModelRunner: runs a user-supplied model command in a throwaway working directory with a timeout and an output cap.
//! The command can be any program that reads a prompt on stdin and prints an answer (for example `claude -p`).
//! It runs with your privileges: this limits accidents (no vault in its cwd, bounded time and output), it is not a sandbox.
use crate::util::Res;
use std::{
    fs,
    io::{Read, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub const MAX_OUT: usize = 64 * 1024;

pub fn timeout() -> Duration {
    Duration::from_secs(std::env::var("SMUG_MODEL_TIMEOUT").ok().and_then(|s| s.parse().ok()).unwrap_or(120))
}

pub fn ask(cmd: &str, prompt: &str) -> Res<String> {
    let dir = std::env::temp_dir().join(format!("smug-model-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)));
    fs::create_dir_all(&dir)?;
    let r = run(cmd, prompt, &dir);
    let _ = fs::remove_dir_all(&dir);
    r
}

fn run(cmd: &str, prompt: &str, dir: &std::path::Path) -> Res<String> {
    let mut c = if cfg!(windows) { Command::new("cmd") } else { Command::new("sh") };
    c.args([if cfg!(windows) { "/C" } else { "-c" }, cmd]).current_dir(dir).env_remove("SMUG_VAULT").env("SMUG_MODEL_RUN", "1").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    let mut ch = c.spawn()?;
    let mut stdin = ch.stdin.take().unwrap();
    let p = prompt.to_string();
    thread::spawn(move || {
        let _ = stdin.write_all(p.as_bytes());
    });
    let mut out = ch.stdout.take().unwrap();
    let reader = thread::spawn(move || {
        let mut buf = vec![];
        let mut chunk = [0u8; 8192];
        while let Ok(n) = out.read(&mut chunk) {
            if n == 0 {
                break;
            }
            if buf.len() < MAX_OUT {
                buf.extend_from_slice(&chunk[..n.min(MAX_OUT - buf.len())]);
            } // past the cap: keep draining so the child never blocks, discard the rest
        }
        buf
    });
    let t0 = Instant::now();
    let limit = timeout();
    let status = loop {
        if let Some(s) = ch.try_wait()? {
            break s;
        }
        if t0.elapsed() > limit {
            let _ = ch.kill();
            let _ = ch.wait();
            return Err(format!("model timed out after {}s (raise it with SMUG_MODEL_TIMEOUT)", limit.as_secs()).into());
        }
        thread::sleep(Duration::from_millis(20));
    };
    let buf = reader.join().unwrap_or_default();
    if !status.success() {
        return Err(format!("model command failed with {status}").into());
    }
    Ok(String::from_utf8_lossy(&buf).to_string())
}
