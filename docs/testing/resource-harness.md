# Resource Harness

## Commands

Command smoke. This verifies command wiring only; it does not collect resource metrics and does not support resource/NFR claims:

```sh
scripts/resource/run-resource-smoke.sh --host-fallback
```

Optional demo target command smoke. This verifies SSH target-runner wiring only unless paired with separate observe/load reports:

```sh
ADC_LAB_TARGET_RUNNER=/home/demo/.local/bin/adc-lab-target scripts/resource/run-resource-smoke.sh --target ssh://target55
```

## Required Scenarios

- idle baseline
- default steady state
- bounded burst
- bounded burst with operator abort marker
- bounded burst with thermal surface unavailable or explicitly recorded
- listed-order experiment matrix with `cpu_load_workers=0` and `1`
- unsupported controlled factor matrix blocked without producing completed evidence
- degraded battery_low
- degraded storage_pressure
- observer-on vs observer-off when instrumentation is target-local

## Report Paths

- Baselines: `baselines/resource/`
- Reports: `reports/resource/`, `reports/operating-envelope/`, `reports/target-characterization/`
- Gate report: `reports/resource/nfr-gate-report.md`

## Limits

- Demo target smoke passed for `target55` after non-root deployment of `adc-lab-target` to `/home/demo/.local/bin/adc-lab-target`; the tracked demo pack lives under `examples/demos/target55/`.
- Host fallback proves command wiring and schema behavior only.
- Command smoke reports `resource_metrics_collected=false` and `resource_claims_supported=false`.
- PR5 hardware-free tests verify operator-abort contract behavior; they do not
  measure target thermal, wakeup, power, flash, or latency impact.
- PR6 hardware-free tests verify only the narrow real-run experiment subset:
  listed order, `cpu_load_workers`, bounded load result refs, passive observe
  refs, and per-trial audit. They do not measure production physical footprint.
- Target55 smoke proves only the short bounded target scenarios captured here.
- Manual measurement is required before production physical-footprint claims.
