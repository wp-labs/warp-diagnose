#!/usr/bin/env bash
set -euo pipefail

CASE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ROOT="$(cd "$CASE_ROOT/.." && pwd)"
WPARSE_ROOT="$CASE_ROOT/wparse"
WFUSION_ROOT="$CASE_ROOT/wfusion"
DEFAULT_WFUSION_BIN="$APP_ROOT/../wp-reactor/target/debug/wfusion"
WPARSE_BIN="${WPARSE_BIN:-wparse}"
if [ -x "$DEFAULT_WFUSION_BIN" ]; then
  WFUSION_BIN="${WFUSION_BIN:-$DEFAULT_WFUSION_BIN}"
else
  WFUSION_BIN="${WFUSION_BIN:-wfusion}"
fi
INPUT="${INPUT:-$CASE_ROOT/target_data/raw_log.dat}"
LOCAL_INPUT="$CASE_ROOT/target_data/raw_log.dat"
RUN_DIAGNOSE="${RUN_DIAGNOSE:-0}"

mkdir -p \
  "$CASE_ROOT/target_data" \
  "$WPARSE_ROOT/data/in_dat" \
  "$WPARSE_ROOT/data/logs" \
  "$WPARSE_ROOT/data/out_dat" \
  "$WPARSE_ROOT/data/rescue" \
  "$WFUSION_ROOT/alerts" \
  "$WFUSION_ROOT/data/in_dat" \
  "$WFUSION_ROOT/logs"

if [ "$INPUT" != "$LOCAL_INPUT" ]; then
  cp "$INPUT" "$LOCAL_INPUT"
fi

rm -f \
  "$WPARSE_ROOT/data/out_dat/default.dat" \
  "$WPARSE_ROOT/data/out_dat/demo.json" \
  "$WPARSE_ROOT/data/out_dat/wp-log.arrow" \
  "$WPARSE_ROOT/data/out_dat/error.dat" \
  "$WPARSE_ROOT/data/out_dat/miss.dat" \
  "$WPARSE_ROOT/data/out_dat/monitor.dat" \
  "$WPARSE_ROOT/data/out_dat/residue.dat" \
  "$WFUSION_ROOT/data/in_dat/wp-log.arrow" \
  "$WFUSION_ROOT/alerts/wf-alert.jsonl" \
  "$WFUSION_ROOT/alerts/wf-semantic.jsonl" \
  "$WFUSION_ROOT/alerts/wf-alert.arrow" \
  "$WFUSION_ROOT/alerts/wf-semantic.arrow" \
  "$WFUSION_ROOT/logs/wfusion.log"

"$WPARSE_BIN" batch --work-root "$WPARSE_ROOT"

if [ ! -f "$WPARSE_ROOT/data/out_dat/wp-log.arrow" ]; then
  echo "missing wparse arrow log: $WPARSE_ROOT/data/out_dat/wp-log.arrow"
  exit 1
fi

cp "$WPARSE_ROOT/data/out_dat/wp-log.arrow" "$WFUSION_ROOT/data/in_dat/wp-log.arrow"

if [ ! -f "$WFUSION_ROOT/data/in_dat/wp-log.arrow" ]; then
  echo "missing wfusion arrow seed: $WFUSION_ROOT/data/in_dat/wp-log.arrow"
  exit 1
fi

(
  cd "$WFUSION_ROOT"
  "$WFUSION_BIN" run --config wfusion.toml > /tmp/wf_wparse_case_run.log 2>&1 &
  PID=$!
  sleep 6
  kill -INT "$PID" >/dev/null 2>&1 || true
  wait "$PID" || true
)

echo "wfusion log: /tmp/wf_wparse_case_run.log"
if [ ! -f "$WFUSION_ROOT/alerts/wf-alert.jsonl" ]; then
  echo "missing wfusion alert jsonl: $WFUSION_ROOT/alerts/wf-alert.jsonl"
  exit 1
fi

if [ -x "$APP_ROOT/target/debug/wf_alert_json_to_arrow" ]; then
  "$APP_ROOT/target/debug/wf_alert_json_to_arrow" \
    "$WFUSION_ROOT/alerts/wf-alert.jsonl" \
    "$WFUSION_ROOT/alerts/wf-alert.arrow"

  if [ -f "$WFUSION_ROOT/alerts/wf-semantic.jsonl" ]; then
    "$APP_ROOT/target/debug/wf_alert_json_to_arrow" \
      "$WFUSION_ROOT/alerts/wf-semantic.jsonl" \
      "$WFUSION_ROOT/alerts/wf-semantic.arrow"
  fi
else
  RUSTC_WRAPPER= cargo run --quiet --manifest-path "$APP_ROOT/Cargo.toml" --bin wf_alert_json_to_arrow -- \
    "$WFUSION_ROOT/alerts/wf-alert.jsonl" \
    "$WFUSION_ROOT/alerts/wf-alert.arrow"

  if [ -f "$WFUSION_ROOT/alerts/wf-semantic.jsonl" ]; then
    RUSTC_WRAPPER= cargo run --quiet --manifest-path "$APP_ROOT/Cargo.toml" --bin wf_alert_json_to_arrow -- \
      "$WFUSION_ROOT/alerts/wf-semantic.jsonl" \
      "$WFUSION_ROOT/alerts/wf-semantic.arrow"
  fi
fi

if [ ! -f "$WFUSION_ROOT/alerts/wf-alert.arrow" ]; then
  echo "missing converted alert arrow: $WFUSION_ROOT/alerts/wf-alert.arrow"
  exit 1
fi

ls -lh \
  "$WPARSE_ROOT/data/out_dat/wp-log.arrow" \
  "$WFUSION_ROOT/data/in_dat/wp-log.arrow" \
  "$WFUSION_ROOT/alerts/wf-alert.jsonl" \
  "$WFUSION_ROOT/alerts/wf-alert.arrow"
if [ -f "$WFUSION_ROOT/alerts/wf-semantic.jsonl" ]; then
  ls -lh "$WFUSION_ROOT/alerts/wf-semantic.jsonl"
fi
if [ -f "$WFUSION_ROOT/alerts/wf-semantic.arrow" ]; then
  ls -lh "$WFUSION_ROOT/alerts/wf-semantic.arrow"
fi

if [ "$RUN_DIAGNOSE" = "1" ]; then
  cd "$APP_ROOT"
  WARP_DIAGNOSE_DEMO_JSON="$WPARSE_ROOT/data/out_dat/wp-log.arrow" \
  WARP_DIAGNOSE_WPARSE_LOG="$WPARSE_ROOT/data/in_dat/wparse.log" \
  WARP_DIAGNOSE_USE_WFUSION=1 \
  WARP_DIAGNOSE_WFUSION_ALERTS="$WFUSION_ROOT/alerts/wf-alert.arrow" \
  cargo run
else
  echo ""
  echo "Run diagnose:"
  echo "cd $APP_ROOT && WARP_DIAGNOSE_DEMO_JSON=$WPARSE_ROOT/data/out_dat/wp-log.arrow WARP_DIAGNOSE_WPARSE_LOG=$CASE_ROOT/target_data/raw_log.dat WARP_DIAGNOSE_USE_WFUSION=1 WARP_DIAGNOSE_WFUSION_ALERTS=$WFUSION_ROOT/alerts/wf-alert.arrow cargo run"
fi
