# Changelog

## 1.0.1 (2026-11-12)

Consistency fixes found in an external review of 1.0.0.

- `apply` is crash-safe: journalled `applying` state, idempotent steps, resumed by `doctor` or a second `apply`.
- An empty or malformed `evals/cases.jsonl` is now an error, not a pass.
- The Tantivy index self-heals (missing, corrupt, truncated or inconsistent index is rebuilt from the Markdown); `doctor` checks it.
- Eval can put a real model in the loop: `smug eval <patch> --model "<command>"` with `answer_contains` / `answer_absent` cases.
- Claude Code `Stop` hook (`smug hook stop`) credits surfaced notes the assistant actually used; `smug setup claude` installs it.
- 250 automated tests.

## 1.0.0 (2026-11-12)

First stable release. The command line and the on-disk formats (frontmatter fields, `history/events.jsonl`, `patches/*.json`, `evals/cases.jsonl`) are now stable.

- Replaced SQLite with redb (state) and Tantivy (search). No C library; pure Rust.
- Decisions with sources, validity and dependencies; `check`, `at`, `impact`.
- Memory Patch workflow: `patch`, `eval`, `apply` (refuses on regression).
- Negative memory (`fail`, `warn`), outcome feedback, `bench`.
- Claude Code session and prompt hooks (`setup claude`); Codex via `AGENTS.md`.
- Prebuilt, checksum-verified binaries for Linux (x86-64, ARM), macOS (Intel, Apple silicon), Windows, FreeBSD and Haiku.
- 223 automated tests.

## 0.2.0

Required compare-and-swap, OS-level lock, symlink-safe writes, persistent index, staged-blob backup scan.

## 0.1.0

First public prototype.
