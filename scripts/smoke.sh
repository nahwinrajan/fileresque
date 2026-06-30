#!/usr/bin/env bash
#
# smoke.sh — runtime smoke check for the FileResque frontend.
#
# Purpose: catch the class of failure that unit tests cannot — a blank page or a
# crash-on-mount. It boots the SvelteKit dev server, loads it in headless Chrome,
# and asserts that (1) the root element actually mounted and (2) the console is
# free of uncaught errors. Produces artifacts (screenshot + console log) so a QA
# sign-off can reference proof, not a checkbox.
#
# SCOPE: this drives the dev server in a browser, NOT the Tauri shell. It verifies
# the frontend renders without JS errors. It does NOT exercise Tauri IPC — commands
# like get_disks reject in a plain browser and fall through to their error UI, which
# is expected and is not a console error. Full end-to-end (real backend) is a
# separate, heavier WebDriver concern.
#
# Exit codes: 0 = pass, 1 = smoke failure, 2 = environment error (no Chrome).
set -euo pipefail
# Job control: the backgrounded dev server becomes its own process-group leader
# (PGID == its PID), so cleanup can kill the whole tree — bun *and* the vite
# child that actually binds the port — instead of just the subshell wrapper.
set -m

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${ROOT}/.smoke"
URL="http://localhost:5173/"
ROOT_MARKER='app-shell'   # stable root class rendered by src/routes/+page.svelte
CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"

mkdir -p "$ARTIFACT_DIR"

if [ ! -x "$CHROME" ]; then
  echo "✗ smoke: Chrome not found at '$CHROME'"
  echo "  set CHROME=/path/to/chrome (or chromium / Edge) and re-run"
  exit 2
fi

# ── Boot the dev server ──────────────────────────────────────────────────────
# `exec` so bun replaces the subshell and inherits its PID/PGID; vite is then a
# child in the same process group.
( cd "$ROOT" && exec bun dev ) >"$ARTIFACT_DIR/dev-server.log" 2>&1 &
DEV_PID=$!

cleanup() {
  # Signal the whole process group (negative PID) so bun and its vite child both
  # exit; fall back to the bare PID if the group send is rejected.
  kill -TERM -- "-${DEV_PID}" 2>/dev/null || kill -TERM "$DEV_PID" 2>/dev/null || true
  # Belt-and-suspenders: free port 5173 if anything still lingers on it.
  local stragglers
  stragglers="$(lsof -ti tcp:5173 2>/dev/null || true)"
  [ -n "$stragglers" ] && kill -TERM $stragglers 2>/dev/null || true
  wait "$DEV_PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

echo "• smoke: waiting for dev server at $URL"
for _ in $(seq 1 60); do
  if curl -sf -o /dev/null "$URL"; then break; fi
  if ! kill -0 "$DEV_PID" 2>/dev/null; then
    echo "✗ smoke: dev server exited before becoming ready"
    cat "$ARTIFACT_DIR/dev-server.log"
    exit 1
  fi
  sleep 0.5
done

# ── Load in headless Chrome: DOM on stdout, console on stderr ────────────────
DOM="$("$CHROME" --headless --disable-gpu --dump-dom --enable-logging=stderr \
  --virtual-time-budget=8000 "$URL" 2>"$ARTIFACT_DIR/console.log")"
"$CHROME" --headless --disable-gpu --window-size=1280,800 \
  --screenshot="$ARTIFACT_DIR/app.png" --virtual-time-budget=8000 "$URL" \
  >/dev/null 2>&1 || true

# ── Assertions ───────────────────────────────────────────────────────────────
FAIL=0

if echo "$DOM" | grep -q "$ROOT_MARKER"; then
  echo "✓ smoke: root element '.$ROOT_MARKER' mounted"
else
  echo "✗ smoke: root element '.$ROOT_MARKER' NOT found — app did not mount (blank page)"
  FAIL=1
fi

ERRORS="$(grep 'CONSOLE' "$ARTIFACT_DIR/console.log" 2>/dev/null \
  | grep -iE 'Uncaught|TypeError|ReferenceError|SyntaxError' || true)"
if [ -z "$ERRORS" ]; then
  echo "✓ smoke: no uncaught console errors"
else
  echo "✗ smoke: uncaught console errors detected:"
  echo "$ERRORS" | sed 's/^/    /'
  FAIL=1
fi

echo "  artifacts → $ARTIFACT_DIR/app.png · console.log · dev-server.log"
if [ "$FAIL" -eq 0 ]; then
  echo "✓ smoke: PASS"
else
  echo "✗ smoke: FAIL"
fi
exit "$FAIL"
