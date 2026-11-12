# Contributing

```bash
cargo test --release          # 223 tests; they spawn the real binary against temp vaults
python3 tools/make_cards.py   # regenerate the README cards
```

- Every behaviour change needs a test. Guarantees in `docs/design.md` each have one.
- Keep the binary dependency-light and pure Rust; no network calls, no telemetry.
- Claims about other projects must come from their own repository and carry a date.

Public domain: by contributing you dedicate your work to the public domain under the Unlicense.
