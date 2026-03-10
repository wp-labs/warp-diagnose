#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WFUSION_BIN="${WFUSION_BIN:-$ROOT_DIR/../../../wp-reactor/target/debug/wfusion}"
INPUT="${INPUT:-data/demo.json}"

cd "$ROOT_DIR"

python3 scripts/build_wparse_events.py \
  --input "$INPUT" \
  --output data/wparse_events.ndjson \
  --mode auto

find alerts -name '*.jsonl' -delete

"$WFUSION_BIN" run --config wfusion.toml --metrics --metrics-interval 1s > /tmp/wf_wparse_case_run.log 2>&1 &
PID=$!
sleep 6
kill -INT "$PID" >/dev/null 2>&1 || true
wait "$PID" || true

echo "wfusion log: /tmp/wf_wparse_case_run.log"
wc -l alerts/all.jsonl
sed -n '1,10p' alerts/all.jsonl
