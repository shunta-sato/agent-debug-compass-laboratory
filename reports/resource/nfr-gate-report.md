# Embedded NFR Gate Report: adc-lab target runtime

Gate decision: experimental-only

## Findings

- No submit-blocking finding for experimental-only MVP.
- Target55 short-smoke target characterization and resource reports exist.
- PR5 CPU load safety monitor records monitor samples, thermal surface
  availability, operator abort observation, and restore-on-abort status.
- PR5 operator abort behavior is hardware-free verified; it is not target
  thermal, battery, flash, latency, or production evidence.
- PR6 real matrix runner behavior is hardware-free verified for the
  `cpu_load_workers` allowlist and blocked unsupported controlled factors; it
  is not privileged operating-point or production physical-footprint evidence.
- PR7 operating-point coverage is hardware-free verified for read-only
  observational runs and PR6 bounded matrices; it is claim-boundary evidence,
  not new target physical measurement.
- Production physical-footprint claims remain blocked until wakeup, battery/power, flash/storage, jitter, sustained thermal, degraded, and recovery evidence exists.
- `make command-smoke` verifies command wiring only and explicitly reports `resource_metrics_collected=false`; it is not resource evidence.

## Budget Results

| NFR | Budget | Measurement | Result | Evidence |
| --- | ---: | ---: | --- | --- |
| Polling / sampling cadence | 1s command-triggered observation | target55 10s observe | measured for short smoke | `lab/runs/LAB-RUN-target55-idle-only-observe/observations/observe.json` |
| CPU | no always-on default; burst load capped at <=300s, <=available_parallelism workers, operator abort, and experiment trial bounds | idle busy about 0.075%; 2-worker 10s load completed; PR5 operator abort test passed; PR6 matrix real-run test passed | partial | `examples/demos/target55/baselines/resource/idle.json`; `examples/demos/target55/baselines/resource/nominal_workload.json`; `crates/adc-lab/tests/cli.rs` |
| Wakeups | no unbounded default; load safety monitor 100ms only during explicit load | not measured | unknown | `docs/testing/resource-harness.md` |
| RSS / heap | unknown | not measured directly | unknown | `docs/testing/resource-harness.md` |
| Storage writes | no continuous target writes | code review | provisional | `reports/resource/hot-path-review.md` |
| Flash wear | no target write claim | not measured | unknown | `reports/resource/observer-effect-review.md` |
| Battery | no battery claim | not measured | unknown | `requirements/physical_budgets.yaml` |
| Thermal | optional abort for load; result records whether thermal surface was available | target55 short load max 54.53C under 75C abort; PR5 contract records monitor evidence | partial | `examples/demos/target55/reports/operating-envelope/observer_on.json`; `tests/golden/lab.load_result.v1.valid.json` |
| Latency / jitter | no claim | not measured | unknown | `docs/testing/resource-harness.md` |
| Observer overhead | no production overhead claim | target55 short smoke iteration delta about -0.05% | partial | `examples/demos/target55/reports/operating-envelope/observer_on.json` |

## Runtime Mode Classification

| Behavior | Mode | Cadence | Duration bound | Enabled by default? | Result |
| --- | --- | ---: | ---: | --- | --- |
| observe sampler | burst | 1s default | command duration | no | measured for target55 short smoke |
| CPU load | experimental-only burst | worker loop plus 100ms safety monitor | command duration, max 300s | no | measured for target55 short smoke; operator abort contract tested |
| experiment matrix runner | experimental-only burst | listed trial sequence | warmup <=60s, cooldown <=60s, repetitions <=10, expanded trials <=64 | no | PR6 local real-run and blocked-factor tests passed |
| operating-point coverage report | controller-side report | none on target | command lifetime | no | PR7 coverage classification tests passed |
| target runner | command-triggered | none while idle | process lifetime | no | target55 smoke passed |

## Artifact Check

- Target characterization: `examples/demos/target55/docs/target-characterization.md`.
- Target characterization freshness: current for 2026-06-08 short smoke; revisit on OS/kernel/hardware/cooling/workload changes.
- Operating envelope: `examples/demos/target55/docs/operating-envelope.md`, safe smoke only.
- NFR calibration: partial, `requirements/nfr/adc-lab-target-runtime.yaml`.
- Calibration revisit condition status: production calibration still missing.
- NFR matrix: `docs/nfr/adc-lab-target-runtime.md`.
- Physical budget file: `requirements/nfr/adc-lab-target-runtime.yaml`.
- Target profile: `examples/demos/target55/target_profile.yaml`.
- Harness plan: `docs/testing/resource-harness.md`.
- Resource report: `examples/demos/target55/baselines/resource/idle.json`, `examples/demos/target55/baselines/resource/nominal_workload.json`.
- Hot-path report: `reports/resource/hot-path-review.md`.
- Observer-effect report: `reports/resource/observer-effect-review.md`.

## Claims Review

| Claim | Location | Evidence | Decision |
| --- | --- | --- | --- |
| safety-gated lab tooling | README.md | schemas, CLI, audit contracts, target55 smoke | allowed |
| target55 10s 2-worker CPU load completed below 75C abort | `examples/demos/target55/docs/operating-envelope.md` | `examples/demos/target55/reports/operating-envelope/observer_off.json` | allowed target-specific short-smoke claim |
| CPU load operator abort produces structured aborted result | `crates/adc-lab/tests/cli.rs` | hardware-free CLI test and `lab.load_result.v1` schema | allowed contract/runtime claim |
| PR6 matrix runner completes supported `cpu_load_workers` trials and blocks unsupported governor factor | `crates/adc-lab/tests/cli.rs` | hardware-free CLI tests and `lab.experiment_run.v1` schema | allowed contract/runtime claim |
| PR7 operating-point coverage separates observed variation from controlled subset evidence | `crates/adc-lab/tests/cli.rs` | hardware-free CLI tests and `lab.operating_point_coverage.v1` schema | allowed contract/report claim |
| low overhead | none | short smoke insufficient | blocked if introduced |
| battery safe | none | none | blocked if introduced |
| production ready target runtime | none | none | blocked if introduced |

## Unknowns And Limits

- Target55 short smoke cannot prove sustained thermal safety, battery safety, flash safety, or production overhead.
- Target command smoke is available for target55 only through `/home/demo/.local/bin/adc-lab-target`.
- `ADC_LAB_TARGET_RUNNER` is a development override constrained to `adc-lab-target` or allowlisted safe paths; shell fragments are refused.
- Privileged apply/restore is local-target only in this MVP; remote privileged control claims are blocked.
- Tier 3 experiments are not implemented beyond policy/docs.
- Matrix runner does not execute privileged governor or fixed-frequency
  controlled factors in PR6.
- Operating-point coverage can report `controlled_subset` for bounded
  `cpu_load_workers` evidence, but fixed-frequency and governor coverage remain
  blocked until privileged control is wired into trials.

## Handoff To Quality Gate

- Required report path: `reports/resource/nfr-gate-report.md`.
- Submit constraints: experimental-only embedded NFR decision; no production physical-footprint claims.
- Follow-up required: run longer operating-envelope, degraded/recovery, wakeup, storage, battery/power, and jitter measurement before production claims.
