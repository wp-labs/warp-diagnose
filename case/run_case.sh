#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 [-c case_name]"
  echo ""
  echo "Examples:"
  echo "  $0"
  echo "  $0 -c simple"
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CASE_ROOT="$SCRIPT_DIR"
WP_TOOLS_ROOT="$CASE_ROOT/wp-tools"
APP_ROOT="$(cd "$CASE_ROOT/.." && pwd)"
DEFAULT_WFUSION_BIN="$APP_ROOT/../wp-reactor/target/debug/wfusion"

WPARSE_BIN="${WPARSE_BIN:-wparse}"
if [ -x "$DEFAULT_WFUSION_BIN" ]; then
  WFUSION_BIN="${WFUSION_BIN:-$DEFAULT_WFUSION_BIN}"
else
  WFUSION_BIN="${WFUSION_BIN:-wfusion}"
fi

CASE_NAME=""
while getopts ":c:h" opt; do
  case "$opt" in
    c)
      CASE_NAME="$OPTARG"
      ;;
    h)
      usage
      exit 0
      ;;
    :)
      echo "missing value for -$OPTARG" >&2
      usage >&2
      exit 1
      ;;
    \?)
      echo "unknown option: -$OPTARG" >&2
      usage >&2
      exit 1
      ;;
  esac
done
shift $((OPTIND - 1))

if [ "$#" -ne 0 ]; then
  echo "unexpected arguments: $*" >&2
  usage >&2
  exit 1
fi

if [ -n "$CASE_NAME" ] && [[ "$CASE_NAME" == *"/"* ]]; then
  echo "invalid case name: $CASE_NAME" >&2
  exit 1
fi

CASE_SOURCE_ROOT="$CASE_ROOT"
CASE_LABEL="${CASE_NAME:-default}"
if [ -n "$CASE_NAME" ]; then
  CASE_SOURCE_ROOT="$CASE_ROOT/$CASE_NAME"
fi

if [ ! -d "$CASE_SOURCE_ROOT" ]; then
  echo "case directory does not exist: $CASE_SOURCE_ROOT" >&2
  exit 1
fi

INPUT="${INPUT:-$CASE_SOURCE_ROOT/target_data/raw_log.dat}"
if [ ! -f "$INPUT" ]; then
  echo "input log does not exist: $INPUT" >&2
  exit 1
fi

mkdir -p \
  "$CASE_SOURCE_ROOT/data/logs" \
  "$CASE_SOURCE_ROOT/data/out_dat"

rm -f \
  "$CASE_SOURCE_ROOT/data/logs/wparse.log" \
  "$CASE_SOURCE_ROOT/data/logs/wfusion.log" \
  "$CASE_SOURCE_ROOT/data/out_dat/wp-log.arrow" \
  "$CASE_SOURCE_ROOT/data/out_dat/wp-log.json" \
  "$CASE_SOURCE_ROOT/data/out_dat/default.dat" \
  "$CASE_SOURCE_ROOT/data/out_dat/error.dat" \
  "$CASE_SOURCE_ROOT/data/out_dat/miss.dat" \
  "$CASE_SOURCE_ROOT/data/out_dat/monitor.dat" \
  "$CASE_SOURCE_ROOT/data/out_dat/residue.dat" \
  "$CASE_SOURCE_ROOT/data/out_dat/wf-alert.arrow" \
  "$CASE_SOURCE_ROOT/data/out_dat/wf-alert.json" \
  "$CASE_SOURCE_ROOT/data/out_dat/wf-entity.arrow" \
  "$CASE_SOURCE_ROOT/data/out_dat/wf-entity.json" \
  "$CASE_SOURCE_ROOT/data/out_dat/wf-semantic.arrow"

echo "case: $CASE_LABEL"
echo "case root: $CASE_SOURCE_ROOT"
echo "wp-tools root: $WP_TOOLS_ROOT"

echo "[1/2] running wparse..."
(
  cd "$WP_TOOLS_ROOT"
  CASE_PATH="$CASE_SOURCE_ROOT" "$WPARSE_BIN" batch
)

if [ ! -s "$CASE_SOURCE_ROOT/data/out_dat/wp-log.arrow" ]; then
  echo "missing wparse arrow output: $CASE_SOURCE_ROOT/data/out_dat/wp-log.arrow" >&2
  exit 1
fi

echo "[2/2] running wfusion..."
(
  cd "$WP_TOOLS_ROOT"
  CASE_PATH="$CASE_SOURCE_ROOT" "$WFUSION_BIN" run --work-dir .
)

if [ ! -s "$CASE_SOURCE_ROOT/data/out_dat/wf-alert.arrow" ]; then
  echo "missing wfusion alert output: $CASE_SOURCE_ROOT/data/out_dat/wf-alert.arrow" >&2
  exit 1
fi

ls -lh \
  "$CASE_SOURCE_ROOT/data/out_dat/wp-log.arrow" \
  "$CASE_SOURCE_ROOT/data/out_dat/wp-log.json" \
  "$CASE_SOURCE_ROOT/data/out_dat/wf-alert.arrow" \
  "$CASE_SOURCE_ROOT/data/out_dat/wf-alert.json"

if [ -f "$CASE_SOURCE_ROOT/data/out_dat/wf-entity.arrow" ]; then
  ls -lh "$CASE_SOURCE_ROOT/data/out_dat/wf-entity.arrow"
fi
if [ -f "$CASE_SOURCE_ROOT/data/out_dat/wf-entity.json" ]; then
  ls -lh "$CASE_SOURCE_ROOT/data/out_dat/wf-entity.json"
fi
if [ -f "$CASE_SOURCE_ROOT/data/out_dat/wf-semantic.arrow" ]; then
  ls -lh "$CASE_SOURCE_ROOT/data/out_dat/wf-semantic.arrow"
fi
