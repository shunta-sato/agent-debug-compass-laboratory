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
- Target55 smoke proves only the short bounded target scenarios captured here.
- Manual measurement is required before production physical-footprint claims.
