# Changelog

## 1.1.1 (2026-11-12)

Fixes from an external review of 1.1.0, plus a much larger test suite.

- **The vault path is now explicit.** `smug agent install` writes `--vault <absolute path>` into every tool's MCP config and into the Claude Code hooks, and there is a global `--vault <dir>` option. A tool started from another project can no longer look in (or write into) the wrong directory.
- **smug refuses to run in a directory that is not a vault** (no `850-companion/` folder, no journal): no stray `.smug/` or `history/` in your projects; hooks stay silent there; `smug mcp` exits with a clear message.
- `smug agent doctor` now starts the server like a tool would, from a different directory with no environment variable, and checks it serves the right vault.
- A damaged or truncated `state.redb` is set aside and rebuilt from the journal; `doctor` removes temp files left by killed writers.
- Older smug hook entries are upgraded in place, never duplicated; your own hooks are untouched.
- **Live-tested:** Codex CLI 0.167.2 and OpenCode 2.2.7 each called smug over MCP in a real session.
- Tests: fuzzing of the MCP server, CLI arguments, hook inputs and the parsers; SIGKILL bursts; 12 writers + 4 readers; vault paths with spaces, Unicode, symlinks, relative paths; corrupt state, journal and index files.


## 1.1.0 (2026-11-12)

The agent layer. smug now meets agents where they work.

- **MCP server** (`smug mcp [--read-only]`): six tools (`smug_recall`, `smug_check`, `smug_explain`, `smug_impact`, `smug_propose`, `smug_feedback`). Writes are limited to inert candidates; promotion is human-only and not exposed.
- **Agent installer** (`smug agent list|install|uninstall|doctor`): Claude Code, Codex CLI, Cursor, Gemini CLI, Windsurf, Kiro, VS Code, OpenCode, plus snippets for Zed and Cline. Safe config merge, `--dry-run`, `--force`, refuses symlinks and unparsable files, uninstall. Claude Code also gets hooks (merged, never overwritten) and a versioned skill.
- **Edit-time memory**: `smug hook pre-edit` reminds the agent of decisions and failed attempts recorded for a file before it edits it.
- **Evidence on failures**: `smug fail` records commit, file hash, test result and verification state.
- **Session isolation**: hook state lives under `.smug/sessions/<hash>/`; two agents on one vault no longer overwrite each other. `smug receipts`.
- **Learning loop**: `capture`, `learn --propose`, `review`, `promote`, `reject`; `SessionEnd` and `PreCompact` capture sessions with secret-shaped lines redacted. Nothing becomes memory without a human.
- **Model eval, isolated**: the model command runs in a throwaway directory with a timeout and an output cap. `smug eval --ab` measures the same model with memory on and off.
- 420 automated tests (a few Unix-only ones are skipped on Windows).

## 1.0.1 (not released separately)

Folded into 1.1.0: crash-safe `apply`, empty or malformed eval is an error, self-healing search index, model-in-the-loop eval, Stop-hook feedback.

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
