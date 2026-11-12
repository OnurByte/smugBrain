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

Eval case: `{"q": "...", "expect": ["path.md"], "absent": ["path.md"], "k": 5, "answer_contains": ["..."], "answer_absent": ["..."]}`. The last two apply only with `--model`.

## Guarantees (each has an automated test)

- With N writers racing on one hash, exactly one wins; the rest get exit code 2 and the file is untouched.
- A process killed mid-write leaves an unfinished intent that `doctor` reconciles; deleting `.smug/` loses nothing because the journal replays.
- A symlink anywhere on a write path, a `..`, an absolute path, `.git`, `.smug` and `.secrets` are refused.
- Secret-shaped content (private keys, tokens, API keys, `password:` lines, 12-24 word recovery phrases) is refused by `write`, skipped by the indexer and blocks `backup`. `backup` scans the staged blobs, so nothing can change between scan and commit.
- `apply` is crash-safe: the patch is journalled as `applying` before any note changes, every step checks whether it already happened, and `doctor` (or `apply` again) finishes an interrupted one. It never leaves two active decisions behind.
- An eval with no cases, or a malformed case line, is an error, never a pass.
- The search index self-heals: if it is missing, corrupt, truncated, or disagrees with the state, it is wiped and rebuilt from the Markdown, on the next search or in `doctor`.
- A stale lock file cannot block you: the lock is an OS advisory lock (on Haiku, which lacks one in std, a lock file that expires after 30 s).

## Agents

One core, two doors: the same functions serve the CLI, the hooks and the MCP server, so there is one index and one set of rules.

- **MCP** (`smug mcp`): `smug_recall`, `smug_check`, `smug_explain`, `smug_impact` are read-only. `smug_propose` writes an inert candidate under `500-knowledge/candidates/`; `smug_feedback` nudges ranking. `--read-only` removes the last two. There is deliberately no tool that promotes, supersedes or applies.
- **Permissions**: agents can read and propose. Humans promote (`smug promote`, which needs a source and refuses topic conflicts), replace (`smug patch` and `apply`) and reject. Candidates are invisible to search, `context`, `check` (reported as CANDIDATE) and failure warnings until promoted.
- **Sessions**: hook state is `.smug/sessions/<first 16 hex of sha256(session id)>/{surfaced.json, receipts.jsonl}`. The raw id never touches the disk. A session can only credit notes that were surfaced to it.
- **Hooks** (Claude Code): `SessionStart` (context), `UserPromptSubmit` (candidates and failure warnings), `PreToolUse` on Edit/Write/MultiEdit (decisions and failed attempts that name the file, via `files:` frontmatter or the file name), `Stop` (credit notes the assistant used), `SessionEnd` and `PreCompact` (capture the transcript to `000-inbox/captures/`, secret-shaped lines redacted).
- **Installer**: merges one `smug` entry into each tool's config with serde_json / toml_edit; it never rewrites other keys, refuses symlinks and files it cannot parse, writes atomically, and can uninstall. Support levels are stated per tool and never say "live-tested" without a live test.
- **ModelRunner**: runs your command through the shell in a fresh temporary directory with `SMUG_VAULT` removed, a timeout (`SMUG_MODEL_TIMEOUT`, default 120 s) and a 64 KB output cap. This limits accidents; it is not a sandbox.
- **Learning loop**: `capture` (or the session hooks) -> `learn --propose` (a model extracts candidates) -> `review` -> `promote` or `reject`.
- **Failure evidence**: `commit`, `file_sha`, `test_result` and `verified` frontmatter; warnings show them.

## Limits

- Search is keyword search (BM25 over Tantivy). `smug search --deep` hands the query to [QMD](https://github.com/tobi/qmd) when it is installed; smug has no embedding model of its own.
- `eval` always checks what retrieval returns. To check what a model answers, pass `--model "<command>"` and add `answer_contains` / `answer_absent` to cases. smug bundles no model, and the command runs through your shell with your privileges.
- `bench` and `eval` run on your own cases; there is no independent public benchmark.
- Every listed tool has automated tests for its config edit, and the MCP server is tested with a scripted client. A live session inside each tool has not been run. Lifecycle hooks exist for Claude Code only.
- The Claude Code hook formats (`additionalContext` for `PreToolUse`, transcript JSONL) follow Anthropic's documented shapes and are tested against synthetic input, not against every Claude Code version.
- Automatic feedback (`smug hook stop`) credits a surfaced note when its file name appears in the assistant's text or tool calls. It is a heuristic. `smug feedback` records it by hand.
- Notes are expected to be UTF-8 text.
