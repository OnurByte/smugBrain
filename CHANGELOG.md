# Changelog

## 1.1.3 (2026-11-13)

Four ideas adapted from avenoxbeyin 3.13.0, plus the fix that unblocks the release build.

- **Session context is ordered by priority.** `smug context` now prints rules first, then identity, the newest handoff card and open threads; older sessions go last. The 9,500-character budget cuts the tail, so a long `Last-Session.md` can no longer push your rules or open threads out of the session.
- `smug context` reads `850-companion/Kurallar.md` when there is no `Rules.md`, so existing Turkish vaults get their rules without renaming anything.
- **The secret guard catches prefixed environment assignments:** `OPENAI_API_KEY=...`, `export DB_PASSWORD=...`, `set X_SECRET=...`, `$env:GITHUB_TOKEN = "..."`. Config names such as `PUBLIC_API_KEY_NAME=` and `${PLACEHOLDER}` values are not flagged.
- **`smug write` refuses to create a note with a name that cannot travel between machines:** Windows device names (`CON`, `NUL`, `COM1`...), any of `<>:"|?*`, control characters, a trailing dot or space, or a component over 255 bytes. Existing notes are not affected.
- `smug agent doctor` in a directory that is not a vault now prints `FAIL` and exits 1 every time. It used to die with a broken-pipe error when the server exited before the probe was written, which failed the Linux release build of 1.1.2 (so 1.1.2 never shipped binaries).
- Tests: 441 (was 427).

## 1.1.2 (2026-11-12)

Bug-hunt release: six real bugs, each with a regression test.

- Search no longer fails on a bare `AND`, `OR` or `NOT`; a query error also no longer wipes and rebuilds the index.
- Notes with CRLF line endings (git `autocrlf` on Windows) now have their frontmatter read.
- `impact` follows `[[id#heading]]` links.
- `smug patch` numbers patches by the highest existing number, so deleting an earlier patch no longer blocks new ones.
- `--date` and `--known-at` must be `YYYY-MM-DD`.
- A NUL byte can no longer hide a secret from the write guard: `smug write` and proposals refuse NUL content.
- A second capture in the same session (pre-compact, then session-end) appends instead of overwriting the first.
- Tests sweep temp directories left by earlier runs.

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
