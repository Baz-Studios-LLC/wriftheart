#!/usr/bin/env bash
# Assemble WriftHeart.app around an already-built release binary.
# Usage: packaging/macos-app.sh <path-to-binary> <version> [out-dir]
# The game bakes all art + audio in code, so the bundle is just the (stripped) binary,
# an icon, and an Info.plist — no Resources/assets to copy.
set -euo pipefail
BIN="${1:?usage: macos-app.sh <binary> <version> [out-dir]}"
VERSION="${2:?need a version}"
OUT="${3:-dist}"
HERE="$(cd "$(dirname "$0")" && pwd)"

APP="$OUT/WriftHeart.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BIN" "$APP/Contents/MacOS/wriftheart"
chmod +x "$APP/Contents/MacOS/wriftheart"
strip "$APP/Contents/MacOS/wriftheart" 2>/dev/null || true          # drop debug symbols (huge)

cp "$HERE/WriftHeart.icns" "$APP/Contents/Resources/WriftHeart.icns"
sed "s/__VERSION__/$VERSION/g" "$HERE/Info.plist" > "$APP/Contents/Info.plist"

# Ad-hoc sign so macOS will run it locally without a "damaged" error (unsigned dylib-less
# arm64 binaries otherwise get killed on Apple Silicon). Real signing/notarization is a
# later step; the launcher also strips the download quarantine on install.
codesign --force --deep --sign - "$APP" 2>/dev/null || true

echo "built $APP ($(du -sh "$APP" | cut -f1))"
