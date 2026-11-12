# Roadmap

Source: the author's own comparison of the author's prototype vault against another open-source memory system, 2026-11. No independent benchmark of the two exists yet; claims below are plans, not results.

## Principle

Markdown is the source of truth. SQLite is operational state and an index: it can be deleted and rebuilt. A Markdown file and a SQLite transaction cannot commit atomically, so every file write goes: intent journal → hash check → atomic rename → verified commit record. Crash recovery reconciles unfinished writes.

## Stack (decided)

| Layer | Choice | Why |
| --- | --- | --- |
| Engine | Rust | One binary, strong types, safe concurrency |
| CLI | `clap` | `smug` commands |
| State + history | SQLite (`rusqlite`) | Transactions, revisions |
| Fast search | SQLite FTS5 | No extra service |
| Semantic search | QMD (kept) | Already works; BM25 + vectors + rerank |
| Graph | SQLite edges + `petgraph` | Evidence-backed links only |
| Format | Markdown + YAML frontmatter | Obsidian-compatible |

Not in v1: separate graph DB, always-on daemon, mandatory cloud, new GUI, autonomous browsing, support for eight agents.

## Phases

| # | Phase | Done when |
| --- | --- | --- |
| 01 | Measure | Same questions, same data, both systems; 100+ queries; secret-canary and crash tests |
| 02 | Rust core | Two agents write at once without data loss; recovers from a kill -9 |
| 03 | Retrieval V3 | L0 exact id/alias/file → L1 FTS5 + Turkish normalisation → L2 QMD hybrid. Superseded facts filtered after retrieval, not just down-weighted |
| 04 | Decisions | Facts have validity, decisions have reasons. Edges go candidate → verified → active → superseded |
| 05 | Ideas + capture | Small adapters: URL, repo, page text, file, transcript. Fetching and synthesising are separate steps |
| 06 | Migrate | Rust runs read-only in shadow mode beside the prototype, diffs reported, then writes move over; prototype retired only after rollback is proven |

## Benchmark contract

Recall@5, MRR, nDCG@5 · false-positive and abstention rate · stale-claim suppression · secret canary, path escape, unauthorised write · crash, concurrency, idempotency, restore · p50/p95 latency, RAM, CPU, context size · cold-session and compaction handoff.

Acceptance gates: zero data loss, zero secret leakage, correct source references, no retrieval regression versus the prototype. Goal: p95 < 250 ms for warm keyword queries on small and medium vaults (a target, not a measurement). The model-backed search never runs on every prompt.

## Status (2026-11-12, v0.3.0)

Stack: one Rust binary. redb for state, Tantivy for search (Turkish-friendly: lower-cased and ASCII-folded), an append-only journal in `history/events.jsonl` that lives in git and rebuilds the database if `.smug/` is lost. No SQLite, no C library.

Done:
- Safe writes: SHA-256 compare-and-swap required, OS lock, symlink/path-escape/secret refusal, intent/commit journal, crash recovery in `doctor`.
- Search: persistent incremental index, hard filter that drops superseded decisions, `--history` to see them, `--deep` hands off to QMD for semantic search when installed.
- Decisions with sources, validity and dependencies (`decide`, `check`, `at`, `impact`).
- Memory Patch (`patch`), behavioural eval before and after (`eval`), gated `apply`.
- Negative memory (`fail`, `warn`), outcome feedback (`feedback`), `bench` (recall@k, MRR, latency on your own cases).
- Agents: Claude Code session and prompt hooks (`setup claude`), Codex via `AGENTS.md`, Hermes via `smug context` and `smug hook prompt`.
- Prebuilt binaries and checksum-verified installers.

Not done, stated plainly:
- Semantic search is not built in (QMD adapter only).
- `eval` checks what retrieval returns, not what a model answers. There is no model-in-the-loop behavioural test yet.
- The benchmark is self-run on your own cases. There is no independent, public benchmark.
- Codex and Hermes have no native lifecycle hooks in smug; they follow the instructions in `AGENTS.md` or call the commands.
- Outcome feedback is manual (`smug feedback`); nothing detects automatically whether an agent used a note.
