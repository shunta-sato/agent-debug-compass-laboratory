#!/usr/bin/env bash
set -euo pipefail

mode=""
target="local"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host-fallback)
      mode="host-fallback"
      shift
      ;;
    --target)
      mode="target"
      target="${2:?missing target value}"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$mode" ]]; then
  echo "usage: $0 --host-fallback | --target ssh://pi4" >&2
  exit 2
fi

if [[ "$mode" == "host-fallback" ]]; then
  output="$(cargo run -q -p adc-lab -- health-check --target local)"
  if ! grep -q '"status": "ok"' <<<"$output"; then
    printf '%s\n' "$output" >&2
    echo "resource smoke host fallback: health-check degraded" >&2
    exit 1
  fi
  echo "command smoke host fallback: ok resource_metrics_collected=false resource_claims_supported=false"
  exit 0
fi

output="$(cargo run -q -p adc-lab -- health-check --target "$target")"
if ! grep -q '"status": "ok"' <<<"$output"; then
  printf '%s\n' "$output" >&2
  echo "resource smoke target ${target}: health-check degraded" >&2
  exit 1
fi
echo "command smoke target ${target}: ok resource_metrics_collected=false resource_claims_supported=false"
