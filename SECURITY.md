# Security

smug runs locally, makes no network calls and has no telemetry.

**Report a vulnerability** privately through GitHub: Security tab, "Report a vulnerability" on this repository. Please include a minimal reproduction. Expect an answer within a week.

In scope: path escape or symlink writes outside the vault, a secret reaching the index, a journal, a commit or a prompt, a way to overwrite a note without the hash check, tampering with the installer or the checksums.

Supported: the latest release.

Every release asset is listed in `SHA256SUMS`; `install.sh` and `install.ps1` refuse to install on a mismatch.
