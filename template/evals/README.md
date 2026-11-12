# Evals

One JSON per line in `cases.jsonl`:

    {"q": "which database do we use", "expect": ["500-knowledge/decisions/postgres.md"], "absent": ["500-knowledge/decisions/sqlite.md"], "k": 5}

An empty file is an error, not a pass. Add `"answer_contains": [...]` or `"answer_absent": [...]` and run with `--model "<command>"` to check what a model answers. `smug eval` runs them; `smug eval <patch>` runs them before and after a Memory Patch and flags regressions.
