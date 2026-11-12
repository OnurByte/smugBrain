# Design

## Principles

1. **Markdown is the source of truth.** Everything under `.smug/` is disposable and rebuilt on demand.
2. **Nothing changes silently.** Every write goes through one function: secret refusal, symlink and path-escape refusal, SHA-256 compare-and-swap, intent/commit journal, atomic rename.
3. **A decision has a source and a lifetime.** Decisions that lost their source, or that collide on a topic, are flagged. Superseded ones are never returned by search.
4. **Changes are proposed, measured, then applied.** `patch` lists what depends on the old decision, `eval` replays your questions before and after in a throwaway copy, `apply` refuses on regression.

## Stack

One Rust binary. [redb](https://github.com/cberner/redb) holds state (events, document signatures, feedback). [Tantivy](https://github.com/quickwit-oss/tantivy) holds the search index (lower-cased, ASCII-folded, so `isik` finds `ışık`). No SQLite, no C library, no server, no network calls.

## Data

| Path | What | In git |
| --- | --- | --- |
| `*.md` | your notes | yes |
| `500-knowledge/decisions/<id>.md` | decisions | yes |
| `500-knowledge/failures/<id>.md` | things that failed (negative memory) | yes |
| `patches/NNN.json` | proposed and applied Memory Patches | yes |
| `evals/cases.jsonl` | questions the memory must keep answering right | yes |
| `history/events.jsonl` | append-only write journal | yes |
| `.secrets/` | anything secret-shaped | never |
| `.smug/` | redb state, Tantivy index, lock | never, rebuilt |

Decision frontmatter: `type`, `id`, `title`, `status` (`active` / `superseded`), `topic`, `source`, `valid_from`, `valid_to`, `known_at`, `depends_on`, `supersedes`, `superseded_by`. `valid_*` is when it was true in the world; `known_at` is when you learned it. `smug at <date>` honours both.

Eval case: `{"q": "...", "expect": ["path.md"], "absent": ["path.md"], "k": 5}`.

## Guarantees (each has an automated test)

- With N writers racing on one hash, exactly one wins; the rest get exit code 2 and the file is untouched.
- A process killed mid-write leaves an unfinished intent that `doctor` reconciles; deleting `.smug/` loses nothing because the journal replays.
- A symlink anywhere on a write path, a `..`, an absolute path, `.git`, `.smug` and `.secrets` are refused.
- Secret-shaped content (private keys, tokens, API keys, `password:` lines, 12-24 word recovery phrases) is refused by `write`, skipped by the indexer and blocks `backup`. `backup` scans the staged blobs, so nothing can change between scan and commit.
- A stale lock file cannot block you: the lock is an OS advisory lock (on Haiku, which lacks one in std, a lock file that expires after 30 s).

## Limits

- Search is keyword search (BM25 over Tantivy). `smug search --deep` hands the query to [QMD](https://github.com/tobi/qmd) when it is installed; smug has no embedding model of its own.
- `eval` measures what retrieval returns for your questions. It does not call a model, so it cannot tell you what an agent would answer.
- `bench` runs on your own cases; there is no independent public benchmark.
- Claude Code gets native session and prompt hooks. Codex follows `AGENTS.md`; Hermes calls `smug context` and `smug hook prompt`. smug has no native lifecycle hook for them.
- `feedback` is recorded by you or your agent; nothing infers automatically that a note helped.
- Notes are expected to be UTF-8 text.
