#!/usr/bin/env bash
set -euo pipefail

target="${TARGET:-${1:-}}"
duration="${DURATION:-30s}"
run_dir="${RUN_DIR:-lab/runs/LAB-RUN-readonly-familiarization-$(date +%s)}"

if [[ -z "$target" ]]; then
  echo "usage: TARGET=ssh://pi4 $0" >&2
  echo "   or: $0 ssh://pi4" >&2
  exit 2
fi

cargo run -q -p adc-lab -- familiarize read-only \
  --target "$target" \
  --duration "$duration" \
  --signals cpu,freq,thermal,memory \
  --run-dir "$run_dir" \
  --json
