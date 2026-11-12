<p align="center"><img src="assets/banner.svg" alt="smugBrain: local-first memory for AI coding agents" width="880"></p>

<p align="center">
<a href="https://github.com/OnurByte/smugbrain/releases/latest"><img src="assets/badge-release.svg" alt="release"></a>
<a href="https://github.com/OnurByte/smugbrain/actions/workflows/ci.yml"><img src="assets/badge-tests.svg" alt="tests"></a>
<img src="assets/badge-license.svg" alt="public domain">
<img src="assets/badge-rust.svg" alt="written in Rust">
<img src="assets/badge-mcp.svg" alt="speaks MCP">
<img src="assets/badge-nocloud.svg" alt="no account, no cloud">
</p>

# smugBrain: local-first memory for AI coding agents

**smugBrain** gives Claude Code, Codex, Cursor, Gemini CLI and other MCP tools a memory you own: plain Markdown notes plus one small binary, `smug`. It records decisions **with their sources**, never returns a decision that has been replaced, reminds your agent of **what already failed before it edits a file**, shows you **what a change will break before you make it**, and refuses to apply it if your own test questions start failing. No account. No cloud. No telemetry. Nothing to compile. Public domain.

Better than your brain. Objectively.

## See it work

Output from a real run, trimmed to the lines that matter. Your agent decided on Postgres in March and you moved to CockroachDB in June. Most memory tools keep handing the agent the March note. smug does not:

```console
$ smug decide postgres --source adr-7 --topic db --files src/database.rs < why.txt
$ smug check postgres
CURRENT  postgres  source: adr-7

$ smug patch postgres --new-id cockroach --source adr-9 < why-we-moved.txt
Memory Patch #1: postgres -> cockroach  (proposed, nothing applied)

$ smug apply 1
FIXED     before fail after pass which database do we use
1 case(s), 0 regression(s)
applied patch 1: postgres superseded by cockroach

$ smug check postgres
STALE  postgres  superseded by cockroach on 2026-11-12
```

From here on `smug search` and the agent's `smug_recall` never return the Postgres note again; `smug search --history` and `smug at <date>` still can.

And when a fix has already failed once, the agent hears about it at the moment it opens that file, not three hours later:

```console
$ smug fail pool-cache --method "cache the pool" --reason "leaked connections" --file src/database.rs
$ smug warn "let's cache the pool"
WARN pool-cache: 'cache the pool' failed before because leaked connections (...)

# what Claude Code is told before it edits src/database.rs:
smug memory for src/database.rs (check before editing; the code may have changed):
- FAILED BEFORE [pool-cache] cache the pool: leaked connections (verified: no) [UNSUPPORTED]
```

## Install

### Prompt install

Paste this into your AI agent (Claude Code, Codex, Cursor, Gemini CLI...). It does the rest and asks before it changes anything.

```text
Install smugBrain on this computer from https://github.com/OnurByte/smugbrain.
Detect my operating system, download the matching binary from the latest GitHub release,
verify it against SHA256SUMS, and put it on my PATH. Then run `smug init my-brain`.
Next run `smug agent install detected --dry-run` and show me exactly what it would change;
only run it for real after I say yes. Never overwrite existing files, never ask me for secrets,
and finish by telling me what you installed, what you skipped, and what I should do next.
```

### Command install

Each block has a copy button in its top-right corner on GitHub. The installer verifies the SHA-256 before it installs anything.

<img src="assets/os-macos.svg" height="28" alt="macOS"> <img src="assets/os-linux.svg" height="28" alt="Linux"> **macOS, Linux (x86-64, ARM) and Haiku**

```bash
curl -fsSL https://raw.githubusercontent.com/OnurByte/smugbrain/main/install.sh | sh
```

<img src="assets/os-freebsd.svg" height="28" alt="FreeBSD"> **FreeBSD** (uses the built-in `fetch`; with `curl` installed the line above works too)

```bash
fetch -qo - https://raw.githubusercontent.com/OnurByte/smugbrain/main/install.sh | sh
```

<img src="assets/os-windows.svg" height="28" alt="Windows"> **Windows** (PowerShell)

```powershell
irm https://raw.githubusercontent.com/OnurByte/smugbrain/main/install.ps1 | iex
```

Or download a binary from the **[Releases page](https://github.com/OnurByte/smugbrain/releases/latest)**: Linux, macOS (Intel and Apple silicon), Windows, FreeBSD and Haiku, each listed in `SHA256SUMS`.

## Connect your coding tool

```bash
smug init my-brain          # your memory: folders, rules, .gitignore
cd my-brain
smug agent install detected # connects every supported tool it finds on this computer
```

Restart your coding tool, then ask it: *"Use smug to recall what we decided about the database."* Nothing is overwritten: smug merges one entry into each tool's config, refuses files it cannot parse, and `--dry-run` shows what it would do first, including the Claude Code hooks it would add.

### Already have an Obsidian vault?

Skip `smug init` (it refuses a folder that is not empty, so it cannot overwrite your notes). smug works in any folder that has an `850-companion/` directory:

```bash
cd my-existing-vault
printf '.smug/\n.secrets/*\n' >> .gitignore   # index and secrets stay off GitHub
smug doctor                                    # tells you what is missing
smug agent install claude-code --dry-run       # read it, then run it without --dry-run
```

Your own hooks and settings stay where they are. At session start smug reads `850-companion/Rules.md` (or `Kurallar.md`), `Core.md`, `Last-Session.md` and `Threads.md`, in that order of priority, and stays under 9,500 characters, below the point where Claude Code cuts a session-start message: rules, identity, the newest handoff card and open threads arrive first, older sessions last.

### Works with

<p align="center"><img src="assets/card-clients.svg" alt="Works with Claude Code, Codex CLI, Cursor, Gemini CLI, Windsurf, Kiro, VS Code, OpenCode, Zed, Cline and any MCP client" width="880"></p>

Copy the line for your tool:

```bash
smug agent install claude-code
```

```bash
smug agent install codex --global
```

```bash
smug agent install cursor
```

```bash
smug agent install gemini
```

Also: `windsurf --global`, `kiro`, `vscode`, `opencode`. Zed and Cline print a snippet to paste. Any other MCP tool: use the command `smug mcp`.

<details>
<summary><b>What "supported" means here</b></summary>

**Live-tested** (a real session of the tool calling smug over MCP, on Linux, 2026-11-12): **Codex CLI 0.167.2** and **OpenCode 2.2.7**. Every setup tool above also has automated tests for how smug edits its config, and the MCP server is tested with a scripted client (`smug agent doctor` runs that check on your machine, from a different directory, to prove the vault path is right). **Claude Code** was not live-tested: its CLI was not logged in where we tested. Lifecycle hooks (session start, before-edit reminders, capture) are installed for Claude Code only; Codex runs hooks too, and smug does not install them there yet. `smug agent list` prints the exact test level per tool.

</details>

<details>
<summary><b>What your agent can do (six tools)</b></summary>

**`smug_recall`** (relevant, still-valid memory), **`smug_check`** (is this decision still valid?), **`smug_explain`** (source, dates, history), **`smug_impact`** (what depends on it), **`smug_propose`** (save something worth remembering), **`smug_feedback`** (did it help?). Four are read-only. `smug_propose` only writes an **inert candidate**: nothing becomes memory until *you* run `smug review` and `smug promote`. Start the server with `smug mcp --read-only` to take the write tools away entirely.

</details>

## Numbers

Your agent's memory should not be a black box you rent. With smugBrain it is a folder of ordinary files that belong to you, backed by numbers you can reproduce on your own machine.

<p align="center"><img src="assets/card-stats.png" alt="By the numbers" width="880"></p>

**What that means for you:** you never create an account, never pay, and never lose your notes if we disappear. If a password or key ends up in a note, the backup stops before it reaches GitHub. And every number above is something you can re-run yourself.

## Compare

Other tools remember things. smugBrain also makes sure the memory is still true, still safe and still worth trusting before your agent acts on it. Here is how it compares with avenoxbeyin, obsidian-second-brain, hafiza-os, Graphiti and mem0. Facts come from each project's own repository and README, checked 2026-11-12. They lead in places and the card says where.

<p align="center"><img src="assets/card-compare.png" alt="How smugBrain compares with avenoxbeyin, hafiza-os, Graphiti and mem0" width="880"></p>

**In plain words:** it installs as one file, it works offline, it will not hand your agent a decision you already replaced, and it shows you what a change will break before you make it. smugBrain is also public domain, so there is nothing to license and nothing to ask permission for.

<details>
<summary><b>What it does: the commands</b></summary>

```bash
smug search rust karar                 # find notes; replaced decisions never come back
smug decide postgres --source adr-7 --topic db --files src/database.rs < why.txt
smug check postgres                    # CURRENT / STALE / CONFLICTED / UNSUPPORTED
smug fail pool-cache --method "cache the pool" --reason "leaked connections" --file src/database.rs --commit head --test-result "3 failed" --verified yes
smug warn "let's cache the pool"       # it has failed before: why, in which commit, was it verified
smug at 2026-11-03 database            # what was valid and known on that day
```

**The learning loop (nothing becomes memory without you)**

```bash
smug capture --transcript session.jsonl    # a session, secret-shaped lines redacted
smug learn 000-inbox/captures/x.md --model "claude -p" --propose   # candidates, all inert
smug review                                # what is waiting, with conflicts flagged
smug promote use-redb --source adr-9       # you decide; refuses conflicts and missing sources
```

**Change a decision safely**

```bash
smug patch postgres --new-id cockroach --source adr-9 < why.txt   # proposes, applies nothing
smug impact postgres                   # every note that depends on it
smug eval 1 --model "claude -p"        # your questions, before and after, with a real model
smug eval --ab --model "claude -p"     # memory on vs memory off: what is it actually worth?
smug apply 1                           # refuses if a question that passed now fails
```

Everyday safety: `smug write note.md --expect <sha256|new>` (atomic; refuses if the file changed under you), `smug backup` (scans exactly what is staged, stops on any secret), `smug doctor` (health check, crash recovery, index repair), `smug bench`, `smug receipts` (what each agent session was shown and used). Full list: run `smug` with no arguments.

</details>

<details>
<summary><b>FAQ</b></summary>

**What problem does it solve?** AI agents forget, and worse, they confidently use decisions that were replaced months ago, or retry a fix that failed last week. smugBrain keeps every decision and every failed attempt with its source, commit and test result, and refuses to hand a stale one back.

**Does it need the cloud, an account or an API key?** No. It makes no network calls. Your notes are Markdown files in a folder.

**Can two agents use the same memory at once?** Yes. Each agent session keeps its own state (hashed session id), writes are hash-checked so one cannot silently overwrite another, and every write is journalled.

**Can my agent change my memory by itself?** It can propose, never promote. Promotion, supersession and patch-apply are human CLI steps, and are not exposed over MCP at all.

**Does it work with Obsidian?** Yes. The vault is a normal Obsidian-compatible folder; `[[wikilinks]]` count as dependencies in `smug impact`.

**Can it test what a model would answer?** Yes, with your own model command: add `answer_contains` / `answer_absent` to eval cases and run `smug eval 1 --model "claude -p"`. The model runs in a throwaway directory with a timeout and an output cap, never inside your vault. `smug eval --ab` compares the same model with and without your memory.

**What if smug crashes halfway through a change?** Every step is journalled and idempotent. `smug doctor` (or re-running `smug apply`) finishes an interrupted apply; a damaged search index is rebuilt from the Markdown automatically.

**Is it safe to back up?** `smug backup` scans exactly what is staged and stops on private keys, tokens, API keys, `password:` lines, environment assignments such as `OPENAI_API_KEY=...` or `export DB_PASSWORD=...`, and recovery phrases. Anything secret-shaped belongs in `.secrets/`, which is never committed or indexed.

**Is it really a single binary?** Yes: pure Rust (redb for state, Tantivy for search), no Python, no Node, no C library, no server.

**Does it search by meaning?** Built-in search is keyword search with Turkish-friendly folding (`isik` finds `ışık`). `smug search --deep` hands the query to [QMD](https://github.com/tobi/qmd) for semantic search when you have it installed.

</details>

<details>
<summary><b>Under the hood</b></summary>

Markdown notes. State in [redb](https://github.com/cberner/redb), search in [Tantivy](https://github.com/quickwit-oss/tantivy). Every write is journalled to `history/events.jsonl`, which lives in git: delete `.smug/` and nothing is lost. Guarantees, data formats, permissions and limits: [docs/design.md](docs/design.md). Release history: [CHANGELOG.md](CHANGELOG.md).

</details>

<details>
<summary><b>Limits (read this before you rely on it)</b></summary>

- The model in `eval` is whatever command you supply; smug bundles none and the command runs with your privileges (throwaway directory, timeout and output cap, but not a sandbox).
- `bench` and `eval` run on your own cases; there is no independent public benchmark.
- Hooks are installed for Claude Code only; Codex has hooks of its own that smug does not wire yet. Other tools get the MCP tools.
- Automatic feedback is a heuristic: a surfaced note counts as used when its file name appears in what the assistant wrote or did.
- Only Codex CLI and OpenCode were live-tested over MCP (one session each); see "What supported means here". In Claude Code the prompt hook and the skill were seen working in a real session (Linux, 2026-11-12); its MCP tools were not exercised in that session.

</details>

## Contributing

Pull requests, bug reports and test cases are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md). Good first contributions: a test for a platform quirk, a new secret pattern with a test, a live check of a tool in the table.

<a href="https://github.com/OnurByte/smugbrain/graphs/contributors"><img src="https://contrib.rocks/image?repo=OnurByte/smugbrain" alt="Contributors"></a>

## License

Public domain ([Unlicense](LICENSE)). Security: [SECURITY.md](SECURITY.md). Design: NASA Graphics Standards Manual NHB 1430.2 (1976). Logos identify compatibility and are not endorsements; each belongs to its owner.
