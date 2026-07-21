#!/usr/bin/env bash
# ONE command for local play-testing on the Mac: build release + assemble WriftHeart.app
# in dist/, so you launch from Finder/Dock with NO terminal window (a bare unix binary
# double-clicked from Finder opens Terminal; the .app doesn't).
#
#   ./build-app.sh          build + assemble
#   ./build-app.sh open     ...and launch it
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release
VERSION="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
packaging/macos-app.sh target/release/wriftheart "$VERSION" dist
[ "${1:-}" = "open" ] && open dist/WriftHeart.app || true
