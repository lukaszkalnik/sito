#!/usr/bin/env bash
# Runs the app's smoke test (src-tauri/src/smoke.rs): with SITO_SMOKE_TEST=1
# it opens the viewer like a click on a photo, closes both windows and exits
# with 0. A freeze while opening a window (v0.6.2) shows up as the timeout.
# Then checks that the window state of both windows was saved.
#
# Usage: smoke-test.sh <app> <.window-state.json>
set -u

app=$1
state=$2
timeout_s=60

SITO_SMOKE_TEST=1 "$app" &
pid=$!
for _ in $(seq $((timeout_s * 10))); do
  kill -0 "$pid" 2>/dev/null || break
  sleep 0.1
done
if kill -0 "$pid" 2>/dev/null; then
  kill -9 "$pid"
  echo "::error::App froze while opening or closing the viewer (no exit within ${timeout_s} s)"
  exit 124
fi
wait "$pid"
code=$?
case $code in
  0) ;;
  2) echo '::error::Viewer window could not be opened' ;;
  3) echo '::error::Viewer window was opened but not shown' ;;
  *) echo "::error::App failed with exit code $code" ;;
esac
if [ "$code" -ne 0 ]; then exit "$code"; fi

for label in main viewer; do
  if ! jq -e --arg l "$label" 'has($l)' "$state" > /dev/null; then
    echo "::error::Window state in $state has no '$label' window"
    exit 1
  fi
done
echo 'Smoke test passed'
