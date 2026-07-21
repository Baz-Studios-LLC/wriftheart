#!/bin/bash
# Double-click this in Finder to (re)build and launch the RUST port of WriftHeart.
# Runs the RELEASE profile: Claude builds + boot-verifies release after every change,
# so that binary is always current — your launch reuses it and starts near-instantly
# (and release runs faster in-game). If something somehow isn't built yet, cargo
# compiles it first (~a minute). Close the game window to quit; double-click again
# to restart. Leave the little Terminal window alone while you play.
cd "$(dirname "$0")"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
export PATH="$HOME/.cargo/bin:$PATH"
cargo run --release
status=$?
if [ $status -ne 0 ]; then
  echo ""
  echo "WriftHeart exited with an error (code $status) - the message above says why."
  echo "Press RETURN to close this window."
  read -r
fi
