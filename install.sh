#!/bin/sh
# smugBrain installer: copies the empty vault into ./smugbrain (or $1). Never overwrites.
set -eu
DEST="${1:-smugbrain}"
URL="${SMUG_TARBALL:-https://github.com/OnurByte/smugbrain/archive/refs/heads/main.tar.gz}"
[ -e "$DEST" ] && { echo "smug: '$DEST' already exists, refusing to overwrite" >&2; exit 1; }
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
curl -fsSL "$URL" | tar -xz -C "$TMP"
mkdir -p "$DEST"
cp -R "$TMP"/*/template/. "$DEST"/
echo "smug: your brain has been replaced. Open '$DEST' in Obsidian."
