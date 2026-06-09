#!/usr/bin/env bash
set -euo pipefail

target="${1:-ssh://pi4}"
run_dir="${2:-lab/runs/LAB-RUN-pi5-pi4-smoke}"

cargo run -q -p adc-lab -- inventory --target "$target" --run-dir "$run_dir" --json
cargo run -q -p adc-lab -- toolchain discover --target "$target" --run-dir "$run_dir" --json
cargo run -q -p adc-lab -- observe --target "$target" --duration 30s --signals cpu,freq,thermal,memory --run-dir "$run_dir" --json
cargo run -q -p adc-lab -- experiment run --target "$target" --matrix examples/experiments/pi4_cpu_governor_smoke.yaml --dry-run --run-dir "$run_dir" --json
cargo run -q -p adc-lab -- report pack --run "$run_dir" --target-id pi4-target55 --json
