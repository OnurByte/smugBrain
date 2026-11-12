<p align="center"><img src="assets/banner.svg" alt="smugBrain" width="880"></p>

<p align="center">
<a href="https://github.com/OnurByte/smugbrain/releases/latest"><img src="https://img.shields.io/github/v/release/OnurByte/smugbrain?color=D92B2F" alt="Release"></a>
<a href="https://github.com/OnurByte/smugbrain/actions/workflows/ci.yml"><img src="https://github.com/OnurByte/smugbrain/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
<img src="https://img.shields.io/badge/Rust-000?logo=rust&logoColor=white" alt="Rust">
<img src="https://img.shields.io/badge/license-public%20domain-D92B2F" alt="Public domain">
<img src="https://img.shields.io/badge/macOS-000?logo=apple&logoColor=white" alt="macOS">
<img src="https://img.shields.io/badge/Linux-000?logo=linux&logoColor=white" alt="Linux">
<img src="https://img.shields.io/badge/Windows-0078D6" alt="Windows">
<img src="https://img.shields.io/badge/FreeBSD-AB2B28?logo=freebsd&logoColor=white" alt="FreeBSD">
<img src="https://img.shields.io/badge/Haiku-F58220" alt="Haiku">
</p>

**smugBrain** is a brain upgrade that is, frankly, better than yours. Plain Markdown notes plus one small binary (`smug`) that gives your agents (Claude Code, Codex, Hermes) a memory that knows where every fact came from, notices when a decision has been replaced, and shows you what a change will break **before** it happens. No account. No cloud. Nothing to compile.

## Install

Prebuilt binaries are on the **[Releases page](https://github.com/OnurByte/smugbrain/releases/latest)**. Or paste one line; the installer checks the SHA-256 before it installs anything.

macOS, Linux (x86-64, ARM) and Haiku:

```bash
curl -fsSL https://raw.githubusercontent.com/OnurByte/smugbrain/main/install.sh | sh
```

FreeBSD (uses the built-in `fetch`; with `curl` installed the line above works too):

```bash
fetch -qo - https://raw.githubusercontent.com/OnurByte/smugbrain/main/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/OnurByte/smugbrain/main/install.ps1 | iex
```

Then:

```bash
smug init my-brain        # new vault: folders, rules, .gitignore
cd my-brain
smug setup claude         # Claude Code starts every session with your memory
```

## What it does

```bash
smug search rust karar                 # find notes; replaced decisions never come back
smug decide postgres --source adr-7 --topic db < why.txt    # a decision, with its source
smug check postgres                    # CURRENT / STALE / CONFLICTED / UNSUPPORTED
smug fail retry-loop --method "retry forever" --reason "hammered the API" < /dev/null
smug warn "let's retry forever"        # it has failed before, here is why
smug at 2026-11-03 database            # what was valid and known on that day
```

**Change a decision safely**

```bash
smug patch postgres --new-id cockroach --source adr-9 < why.txt   # proposes, applies nothing
smug impact postgres                   # every note that depends on it
smug eval 1                            # your questions, before and after the patch
smug apply 1                           # refuses if a question that passed now fails
```

Everyday safety: `smug write note.md --expect <sha256|new>` (atomic; refuses if the file changed under you), `smug backup` (scans exactly what is staged, stops on any secret), `smug doctor` (health check and crash recovery), `smug bench` (recall, MRR and latency on your own cases). Full list: `smug` with no arguments.

## Numbers

<p align="center"><img src="assets/card-stats.svg" alt="By the numbers" width="880"></p>

## Compare

<p align="center"><img src="assets/card-compare.svg" alt="How we compare" width="880"></p>

## Under the hood

One Rust binary. Your notes are Markdown. State is [redb](https://github.com/cberner/redb), search is [Tantivy](https://github.com/quickwit-oss/tantivy) (lower-cased and ASCII-folded, so `isik` finds `ışık`). Every write is journalled to `history/events.jsonl`, which lives in git: delete `.smug/` and nothing is lost. No SQLite, no C library, no server.

## What is not done

- Semantic search is not built in. `smug search --deep` hands the query to [QMD](https://github.com/tobi/qmd) if you have it.
- `eval` checks what retrieval returns for your questions, not what a model answers. There is no model-in-the-loop test yet.
- The benchmark is self-run on your own cases. There is no independent benchmark.
- Codex and Hermes have no native lifecycle hooks in smug; Codex follows `AGENTS.md`, Hermes calls the commands.
- Feedback on which notes helped is manual (`smug feedback`).

Plan and status: [docs/roadmap.md](docs/roadmap.md).

## License

Public domain ([Unlicense](LICENSE)). Design: NASA Graphics Standards Manual NHB 1430.2 (1976).
