#!/usr/bin/env bash
# Launches two isolated VSCodium windows, each running this extension as a
# development extension. Use it to test multi-user flows (overlays, live
# edits, conflicts) on the same machine without juggling one editor.
#
# Each window gets its own --user-data-dir so accounts, settings, and
# workspace state stay separate. The extension is compiled once up front;
# both windows share the same compiled out/ directory.
set -euo pipefail

EXT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILE_A="$HOME/.config/lightning-git-dev-a"
PROFILE_B="$HOME/.config/lightning-git-dev-b"
# the workspace both dev hosts open so they're already in a real git repo
# for testing overlays, conflicts, and the suggest-resolution flow.
WORKSPACE="$HOME/code/test-public"

if [[ ! -d "$WORKSPACE" ]]; then
  echo "Workspace $WORKSPACE not found; aborting." >&2
  exit 1
fi

echo "Compiling extension at $EXT_DIR"
cd "$EXT_DIR"
npm run compile

echo "Launching session A at $PROFILE_A"
code \
  --user-data-dir "$PROFILE_A" \
  --extensions-dir "$PROFILE_A/extensions" \
  --extensionDevelopmentPath "$EXT_DIR" \
  --new-window \
  "$WORKSPACE" &

echo "Launching session B at $PROFILE_B"
codium \
  --user-data-dir "$PROFILE_B" \
  --extensions-dir "$PROFILE_B/extensions" \
  --extensionDevelopmentPath "$EXT_DIR" \
  --new-window \
  "$WORKSPACE" &

wait
