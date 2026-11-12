# Contributing

Thanks for helping. Bug reports, test cases, new platform checks and docs fixes are all welcome.

**Quick path:** fork, branch, `cargo test --release`, open a pull request using the template. Open an issue first for anything that changes a command or a file format.

## Build and test

```bash
cargo test --release          # 420 tests; they spawn the real binary against temp vaults
python3 tools/make_cards.py   # regenerate the README cards
```

- Every behaviour change needs a test. Guarantees in `docs/design.md` each have one.
- Keep the binary dependency-light and pure Rust; no network calls, no telemetry.
- Claims about other projects must come from their own repository and carry a date.

Public domain: by contributing you dedicate your work to the public domain under the Unlicense.
