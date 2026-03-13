#!/usr/bin/env bash
set -euo pipefail

CASE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ROOT="$(cd "$CASE_ROOT/.." && pwd)"
WP_TOOLS_ROOT="$CASE_ROOT/wp-tools"
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
WFUSION_WAIT_MAX="${WFUSION_WAIT_MAX:-15}"
WFUSION_POLL_INTERVAL="${WFUSION_POLL_INTERVAL:-0.1}"

mkdir -p \
  "$CASE_ROOT/target_data" \
  "$WP_TOOLS_ROOT/data/in_dat" \
  "$WP_TOOLS_ROOT/data/logs" \
  "$WP_TOOLS_ROOT/data/out_dat" \
  "$WP_TOOLS_ROOT/data/rescue" \
  "$WP_TOOLS_ROOT/alerts" \
  "$WP_TOOLS_ROOT/logs"

if [ "$INPUT" != "$LOCAL_INPUT" ]; then
  cp "$INPUT" "$LOCAL_INPUT"
fi

rm -f \
  "$WP_TOOLS_ROOT/data/out_dat/default.dat" \
  "$WP_TOOLS_ROOT/data/out_dat/demo.json" \
  "$WP_TOOLS_ROOT/data/out_dat/wp-log.arrow" \
  "$WP_TOOLS_ROOT/data/out_dat/error.dat" \
  "$WP_TOOLS_ROOT/data/out_dat/miss.dat" \
  "$WP_TOOLS_ROOT/data/out_dat/monitor.dat" \
  "$WP_TOOLS_ROOT/data/out_dat/residue.dat" \
  "$WP_TOOLS_ROOT/alerts/wf-alert.json" \
  "$WP_TOOLS_ROOT/alerts/wf-alert.jsonl" \
  "$WP_TOOLS_ROOT/alerts/wf-entity.json" \
  "$WP_TOOLS_ROOT/alerts/wf-semantic.jsonl" \
  "$WP_TOOLS_ROOT/alerts/wf-alert.arrow" \
  "$WP_TOOLS_ROOT/alerts/wf-entity.arrow" \
  "$WP_TOOLS_ROOT/alerts/wf-semantic.arrow" \
  "$WP_TOOLS_ROOT/logs/wfusion.log"

echo "[1/2] running wparse..."
"$WPARSE_BIN" batch --work-root "$WP_TOOLS_ROOT"

if [ ! -f "$WP_TOOLS_ROOT/data/out_dat/wp-log.arrow" ]; then
  echo "missing wparse arrow log: $WP_TOOLS_ROOT/data/out_dat/wp-log.arrow"
  exit 1
fi

echo "[2/2] running wfusion..."
(
  cd "$WP_TOOLS_ROOT"
  "$WFUSION_BIN" run --config wfusion.toml > /tmp/wf_wparse_case_run.log 2>&1 &
  PID=$!
  max_ticks=$(awk "BEGIN { print int(($WFUSION_WAIT_MAX / $WFUSION_POLL_INTERVAL) + 0.999999) }")
  tick=0
  while kill -0 "$PID" >/dev/null 2>&1; do
    if [ -s "$WP_TOOLS_ROOT/alerts/wf-alert.arrow" ]; then
      break
    fi
    if [ "$tick" -ge "$max_ticks" ]; then
      break
    fi
    sleep "$WFUSION_POLL_INTERVAL"
    tick=$((tick + 1))
  done
  kill -INT "$PID" >/dev/null 2>&1 || true
  wait "$PID" || true
)

echo "wfusion log: /tmp/wf_wparse_case_run.log"
if [ ! -s "$WP_TOOLS_ROOT/alerts/wf-alert.arrow" ]; then
  echo "wfusion produced no usable alert output: $WP_TOOLS_ROOT/alerts/wf-alert.arrow"
  echo ""
  echo "tail of wfusion log:"
  tail -n 40 /tmp/wf_wparse_case_run.log || true
  exit 1
fi

ls -lh \
  "$WP_TOOLS_ROOT/data/out_dat/wp-log.arrow" \
  "$WP_TOOLS_ROOT/alerts/wf-alert.arrow"
if [ -f "$WP_TOOLS_ROOT/alerts/wf-alert.json" ]; then
  ls -lh "$WP_TOOLS_ROOT/alerts/wf-alert.json"
fi
if [ -f "$WP_TOOLS_ROOT/alerts/wf-entity.arrow" ]; then
  ls -lh "$WP_TOOLS_ROOT/alerts/wf-entity.arrow"
fi
if [ -f "$WP_TOOLS_ROOT/alerts/wf-entity.json" ]; then
  ls -lh "$WP_TOOLS_ROOT/alerts/wf-entity.json"
fi
if [ -f "$WP_TOOLS_ROOT/alerts/wf-semantic.arrow" ]; then
  ls -lh "$WP_TOOLS_ROOT/alerts/wf-semantic.arrow"
fi

if [ "$RUN_DIAGNOSE" = "1" ]; then
  cd "$APP_ROOT"
  WARP_DIAGNOSE_LOG_WFU="$WP_TOOLS_ROOT/data/out_dat/wp-log.arrow" \
  WARP_DIAGNOSE_DEMO_JSON="$WP_TOOLS_ROOT/data/out_dat/wp-log.arrow" \
  WARP_DIAGNOSE_WPARSE_LOG="$CASE_ROOT/target_data/raw_log.dat" \
  WARP_DIAGNOSE_USE_WFUSION=1 \
  WARP_DIAGNOSE_ALERT_WFU_DIR="$WP_TOOLS_ROOT/alerts" \
  WARP_DIAGNOSE_WFUSION_ALERTS="$WP_TOOLS_ROOT/alerts/wf-alert.arrow" \
  cargo run
else
  echo ""
  echo "Run diagnose:"
  echo "cd $APP_ROOT && WARP_DIAGNOSE_LOG_WFU=$WP_TOOLS_ROOT/data/out_dat/wp-log.arrow WARP_DIAGNOSE_WPARSE_LOG=$CASE_ROOT/target_data/raw_log.dat WARP_DIAGNOSE_USE_WFUSION=1 WARP_DIAGNOSE_ALERT_WFU_DIR=$WP_TOOLS_ROOT/alerts cargo run"
fi
