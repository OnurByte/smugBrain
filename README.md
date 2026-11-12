<p align="center"><img src="assets/banner.svg" alt="smugBrain" width="880"></p>

<p align="center">
<img src="https://img.shields.io/badge/Rust-000?logo=rust&logoColor=white" alt="Rust">
<img src="https://img.shields.io/badge/license-public%20domain-D92B2F" alt="Public domain">
<img src="https://img.shields.io/badge/macOS-000?logo=apple&logoColor=white" alt="macOS">
<img src="https://img.shields.io/badge/Linux-000?logo=linux&logoColor=white" alt="Linux">
<img src="https://img.shields.io/badge/FreeBSD-AB2B28?logo=freebsd&logoColor=white" alt="FreeBSD">
<img src="https://img.shields.io/badge/Windows-0078D6" alt="Windows">
<img src="https://img.shields.io/badge/Haiku-F58220" alt="Haiku">
</p>

**smugBrain** is a brain upgrade that is, frankly, better than yours. Plain Markdown notes plus one small Rust binary (`smug`) that lets your agents (Claude Code, Codex, Hermes) remember what you did yesterday. No account. No cloud.

## Install

Empty vault, no Rust needed. macOS, Linux, FreeBSD, Haiku (needs `sh`, `curl`, `tar`):

```bash
curl -fsSL https://raw.githubusercontent.com/OnurByte/smugbrain/main/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/OnurByte/smugbrain/main/install.ps1 | iex
```

The `smug` command (any system with Rust and a C compiler):

```bash
cargo install --git https://github.com/OnurByte/smugbrain
```

Only tested on Linux so far. Windows, FreeBSD, Haiku and macOS: should work, unverified.

## Use

```bash
smug init my-brain        # new vault
smug search rust karar    # find notes
smug context              # what an agent should read first (pipe it into a session-start hook)
smug write note.md --expect new < text   # atomic write; refuses if the file changed under you
smug scan                 # secret guard
smug backup               # scan, commit, push; stops if anything looks like a secret
smug doctor               # health check + crash recovery
smug log                  # write history
```

## Numbers

<p align="center"><img src="assets/card-stats.svg" alt="By the numbers" width="880"></p>

## Compare

<p align="center"><img src="assets/card-compare.svg" alt="How we compare" width="880"></p>

## Roadmap

| Step | Status |
| --- | --- |
| Rust core: init, search, context, write, log, scan, backup, doctor | done, v0.1 |
| Semantic search (QMD) behind keyword search | 0% |
| Facts with validity, decisions with reasons | 0% |
| Capture adapters: URL, repo, file, video | 0% |
| Benchmark against others, 100+ queries | 0% |

Plan and acceptance gates: [docs/roadmap.md](docs/roadmap.md).

## Tests

```bash
cargo test --release     # path-escape and secret-pattern tests
```

The numbers card was measured by hand: 8 parallel `smug write --expect <hash>` calls on one note (1 accepted, 7 refused), and `smug search` on 2,000 generated notes.

## License

Public domain ([Unlicense](LICENSE)). Do whatever. Design: NASA Graphics Standards Manual NHB 1430.2 (1976).
