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

**smugBrain** is a brain upgrade that is, frankly, better than yours. Plain Markdown notes plus one small binary (`smug`) that lets your agents (Claude Code, Codex, Hermes) remember what you did yesterday and never overwrite your notes by accident. No account. No cloud. Nothing to compile.

## Install

Grab a prebuilt binary from the **[Releases page](https://github.com/OnurByte/smugbrain/releases/latest)**, or paste one line. The installer checks the SHA-256 before it installs anything.

macOS and Linux (x86-64, ARM):

```bash
curl -fsSL https://raw.githubusercontent.com/OnurByte/smugbrain/main/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/OnurByte/smugbrain/main/install.ps1 | iex
```

FreeBSD and Haiku have no prebuilt binary yet. With Rust installed:

```bash
cargo install --git https://github.com/OnurByte/smugbrain
```

Then:

```bash
smug init my-brain        # new vault: folders, rules, .gitignore
cd my-brain
smug setup claude         # Claude Code starts every session with your memory
```

## Use

```bash
smug search rust karar                       # find notes (persistent index, ~12 ms)
smug context                                 # what an agent should read first
smug write note.md --expect new < text       # atomic write; --expect <sha256> refuses if the file changed
smug backup                                  # scan staged files, commit, push; stops on any secret
smug doctor                                  # health check + crash recovery
smug log                                     # write history
```

`write` has no blind mode unless you pass `--force`. It refuses paths outside the vault, symlinks anywhere on the path, and content that looks like a key or recovery phrase.

## Numbers

<p align="center"><img src="assets/card-stats.svg" alt="By the numbers" width="880"></p>

## Compare

<p align="center"><img src="assets/card-compare.svg" alt="How we compare" width="880"></p>

## Status

| Done in v0.2 | Not done |
| --- | --- |
| Hash-checked writes, required by default | Memory Patch / impact / eval commands |
| OS-level lock (cannot go stale) | Source, claim and decision data model |
| Symlink-safe, path-escape-safe writes | Semantic search (QMD) behind keyword search |
| Persistent search index | Write history stored in git (it lives in `.smug/` for now) |
| Backup scans exactly what is staged | Real Codex and Hermes hooks (Codex reads `AGENTS.md`) |
| Claude Code session-start hook | Prebuilt FreeBSD and Haiku binaries |
| Tested on Linux, macOS, Windows in CI | Independent benchmark |

Plan: [docs/roadmap.md](docs/roadmap.md).

## License

Public domain ([Unlicense](LICENSE)). Design: NASA Graphics Standards Manual NHB 1430.2 (1976).
