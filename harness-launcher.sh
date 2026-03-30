#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="/Volumes/formac/proj/safebot"
RELEASE_BIN="$PROJECT_DIR/target/release/openclaw-harness"
DEBUG_BIN="$PROJECT_DIR/target/debug/openclaw-harness"

cd "$PROJECT_DIR"

# formac 외장 볼륨 미마운트/이상 상태일 때 데이터 디렉토리 fallback
if [ ! -d "/Volumes/formac" ]; then
  export SAFEBOT_DATA_DIR="$HOME/.openclaw-harness/fallback-data"
  mkdir -p "$SAFEBOT_DATA_DIR"
  echo "[harness-launcher] /Volumes/formac unavailable -> SAFEBOT_DATA_DIR=$SAFEBOT_DATA_DIR" >&2
fi

run_bin() {
  local bin="$1"
  exec "$bin" start --foreground 2>&1
}

if [ -x "$RELEASE_BIN" ]; then
  run_bin "$RELEASE_BIN"
fi

echo "[harness-launcher] release binary missing, trying debug binary..." >&2
if [ -x "$DEBUG_BIN" ]; then
  run_bin "$DEBUG_BIN"
fi

if command -v cargo >/dev/null 2>&1; then
  echo "[harness-launcher] no binary found, building release..." >&2
  cargo build --release >&2
  if [ -x "$RELEASE_BIN" ]; then
    run_bin "$RELEASE_BIN"
  fi

  echo "[harness-launcher] release build did not produce binary, falling back to cargo run" >&2
  exec cargo run --release -- start --foreground 2>&1
fi

echo "[harness-launcher] ERROR: openclaw-harness binary missing and cargo is not installed." >&2
exit 1
