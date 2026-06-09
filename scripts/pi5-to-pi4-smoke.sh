#!/usr/bin/env bash
set -euo pipefail

target="${1:-ssh://pi4}"
run_dir="${2:-lab/runs/LAB-RUN-pi5-pi4-smoke}"
target_id="${TARGET_ID:-pi4-target}"

if [[ -n "${ADC_LAB_BIN:-}" ]]; then
  adc_lab=("$ADC_LAB_BIN")
elif command -v adc-lab >/dev/null 2>&1; then
  adc_lab=(adc-lab)
else
  adc_lab=(cargo run -q -p adc-lab --)
fi

"${adc_lab[@]}" inventory --target "$target" --run-dir "$run_dir" --json
"${adc_lab[@]}" toolchain discover --target "$target" --run-dir "$run_dir" --json
"${adc_lab[@]}" observe --target "$target" --duration 30s --signals cpu,freq,thermal,memory --run-dir "$run_dir" --json
"${adc_lab[@]}" experiment run --target "$target" --matrix examples/experiments/pi4_cpu_governor_smoke.yaml --dry-run --run-dir "$run_dir" --json
"${adc_lab[@]}" report pack --run "$run_dir" --target-id "$target_id" --json
