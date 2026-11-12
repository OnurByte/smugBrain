//! smug: plain-Markdown memory for agents. Markdown is truth; .smug/ is disposable state.
mod cmds;
mod index;
mod mem;
mod store;
mod util;
mod write;

use std::{env, process::exit};
use util::*;

fn main() {
    // quiet exit when a pipe closes early (`smug doctor | head`)
    std::panic::set_hook(Box::new(|i| {
        let m = i.to_string();
        if m.contains("Broken pipe") {
            exit(0);
        }
        eprintln!("smug: internal error: {m}");
    }));
    let a: Vec<String> = env::args().skip(1).collect();
    let rest = a.get(1..).unwrap_or(&[]);
    let root = vault();
    let r = match a.first().map(String::as_str) {
        Some("init") => cmds::init(rest),
        Some("scan") => cmds::scan(&root),
        Some("backup") => cmds::backup(&root),
        Some("write") => cmds::write(&root, rest),
        Some("log") => cmds::history(&root, rest),
        Some("search") => cmds::search(&root, rest),
        Some("index") => cmds::reindex(&root),
        Some("context") => cmds::context(&root),
        Some("doctor") => cmds::doctor(&root),
        Some("setup") => cmds::setup(&root, rest),
        Some("hook") => cmds::hook(&root, rest),
        Some("decide") => mem::decide(&root, rest),
        Some("fail") => mem::fail(&root, rest),
        Some("check") => mem::check(&root, rest),
        Some("warn") => mem::warn(&root, rest),
        Some("impact") => mem::impact(&root, rest),
        Some("patch") => mem::patch(&root, rest),
        Some("eval") => mem::eval(&root, rest),
        Some("apply") => mem::apply(&root, rest),
        Some("at") => mem::at(&root, rest),
        Some("feedback") => mem::feedback(&root, rest),
        Some("bench") => mem::bench(&root, rest),
        Some("--version" | "version") => {
            println!("smug {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        }
        _ => {
            println!(include_str!("usage.txt"));
            Ok(0)
        }
    };
    match r {
        Ok(c) => exit(c),
        Err(e) => {
            eprintln!("smug: {e}");
            exit(1)
        }
    }
}
