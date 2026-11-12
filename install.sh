#!/bin/sh
# smugBrain installer: downloads the prebuilt `smug` binary from GitHub Releases, verifies its SHA-256, installs it.
# No Rust, no compiler. Never touches your notes.
#   SMUG_VERSION=v0.2.0   pick a release (default: latest)
#   SMUG_BIN=~/bin        install directory (default: ~/.local/bin)
set -eu
REPO="OnurByte/smugbrain"
BIN="${SMUG_BIN:-$HOME/.local/bin}"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)              T=x86_64-unknown-linux-gnu ;;
  Linux-aarch64|Linux-arm64) T=aarch64-unknown-linux-gnu ;;
  Darwin-x86_64)             T=x86_64-apple-darwin ;;
  Darwin-arm64)              T=aarch64-apple-darwin ;;
  FreeBSD-amd64|FreeBSD-x86_64) T=x86_64-unknown-freebsd ;;
  Haiku-x86_64)              T=x86_64-unknown-haiku ;;
  *) echo "smug: no prebuilt binary for $(uname -s)/$(uname -m). See https://github.com/$REPO/releases" >&2; exit 1 ;;
esac

if [ -n "${SMUG_VERSION:-}" ]; then BASE="${SMUG_BASE:-https://github.com/$REPO/releases/download/$SMUG_VERSION}"
else BASE="${SMUG_BASE:-https://github.com/$REPO/releases/latest/download}"; fi

TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
ASSET="smug-$T.tar.gz"
dl() { if command -v curl >/dev/null 2>&1; then curl -fsSL "$1" -o "$2"; else fetch -qo "$2" "$1"; fi; } # FreeBSD base has fetch, not curl
dl "$BASE/$ASSET" "$TMP/$ASSET" || { echo "smug: no release asset $ASSET at $BASE" >&2; exit 1; }
dl "$BASE/SHA256SUMS" "$TMP/SHA256SUMS"

WANT="$(awk -v f="$ASSET" '$2==f {print $1}' "$TMP/SHA256SUMS")"
if command -v sha256sum >/dev/null 2>&1; then GOT="$(sha256sum "$TMP/$ASSET" | cut -d' ' -f1)"
elif command -v shasum >/dev/null 2>&1; then GOT="$(shasum -a 256 "$TMP/$ASSET" | cut -d' ' -f1)"
else GOT="$(sha256 -q "$TMP/$ASSET")"; fi
[ -n "$WANT" ] && [ "$WANT" = "$GOT" ] || { echo "smug: checksum mismatch, refusing to install" >&2; exit 1; }

tar -xzf "$TMP/$ASSET" -C "$TMP"
mkdir -p "$BIN"
install -m 755 "$TMP/smug" "$BIN/smug"
echo "smug: installed $("$BIN/smug" --version) to $BIN/smug"
case ":$PATH:" in *":$BIN:"*) ;; *) echo "smug: add $BIN to your PATH" ;; esac
echo "smug: next -> smug init my-brain"
