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
WFUSION_WAIT_MAX="${WFUSION_WAIT_MAX:-15}"
WFUSION_POLL_INTERVAL="${WFUSION_POLL_INTERVAL:-0.1}"

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
  "$WFUSION_ROOT/alerts/wf-alert.json" \
  "$WFUSION_ROOT/alerts/wf-alert.jsonl" \
  "$WFUSION_ROOT/alerts/wf-entity.json" \
  "$WFUSION_ROOT/alerts/wf-semantic.jsonl" \
  "$WFUSION_ROOT/alerts/wf-alert.arrow" \
  "$WFUSION_ROOT/alerts/wf-entity.arrow" \
  "$WFUSION_ROOT/alerts/wf-semantic.arrow" \
  "$WFUSION_ROOT/logs/wfusion.log"

echo "[1/2] running wparse..."
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

echo "[2/2] running wfusion..."
(
  cd "$WFUSION_ROOT"
  "$WFUSION_BIN" run --config wfusion.toml > /tmp/wf_wparse_case_run.log 2>&1 &
  PID=$!
  max_ticks=$(awk "BEGIN { print int(($WFUSION_WAIT_MAX / $WFUSION_POLL_INTERVAL) + 0.999999) }")
  tick=0
  while kill -0 "$PID" >/dev/null 2>&1; do
    if [ -s "$WFUSION_ROOT/alerts/wf-alert.arrow" ]; then
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
if [ ! -s "$WFUSION_ROOT/alerts/wf-alert.arrow" ]; then
  echo "wfusion produced no usable alert output: $WFUSION_ROOT/alerts/wf-alert.arrow"
  echo ""
  echo "tail of wfusion log:"
  tail -n 40 /tmp/wf_wparse_case_run.log || true
  exit 1
fi

ls -lh \
  "$WPARSE_ROOT/data/out_dat/wp-log.arrow" \
  "$WFUSION_ROOT/data/in_dat/wp-log.arrow" \
  "$WFUSION_ROOT/alerts/wf-alert.arrow"
if [ -f "$WFUSION_ROOT/alerts/wf-alert.json" ]; then
  ls -lh "$WFUSION_ROOT/alerts/wf-alert.json"
fi
if [ -f "$WFUSION_ROOT/alerts/wf-entity.arrow" ]; then
  ls -lh "$WFUSION_ROOT/alerts/wf-entity.arrow"
fi
if [ -f "$WFUSION_ROOT/alerts/wf-entity.json" ]; then
  ls -lh "$WFUSION_ROOT/alerts/wf-entity.json"
fi
if [ -f "$WFUSION_ROOT/alerts/wf-semantic.arrow" ]; then
  ls -lh "$WFUSION_ROOT/alerts/wf-semantic.arrow"
fi

if [ "$RUN_DIAGNOSE" = "1" ]; then
  cd "$APP_ROOT"
  WARP_DIAGNOSE_LOG_WFU="$WPARSE_ROOT/data/out_dat/wp-log.arrow" \
  WARP_DIAGNOSE_DEMO_JSON="$WPARSE_ROOT/data/out_dat/wp-log.arrow" \
  WARP_DIAGNOSE_WPARSE_LOG="$WPARSE_ROOT/data/in_dat/wparse.log" \
  WARP_DIAGNOSE_USE_WFUSION=1 \
  WARP_DIAGNOSE_ALERT_WFU_DIR="$WFUSION_ROOT/alerts" \
  WARP_DIAGNOSE_WFUSION_ALERTS="$WFUSION_ROOT/alerts/wf-alert.arrow" \
  cargo run
else
  echo ""
  echo "Run diagnose:"
  echo "cd $APP_ROOT && WARP_DIAGNOSE_LOG_WFU=$WPARSE_ROOT/data/out_dat/wp-log.arrow WARP_DIAGNOSE_WPARSE_LOG=$CASE_ROOT/target_data/raw_log.dat WARP_DIAGNOSE_USE_WFUSION=1 WARP_DIAGNOSE_ALERT_WFU_DIR=$WFUSION_ROOT/alerts cargo run"
fi
