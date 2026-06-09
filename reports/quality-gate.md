# Quality Gate Report

Gate decision: submit

Findings: 0

## Checks Run

- `cargo fmt --all` - pass.
- `make contract` - pass.
- `cargo test -p adc-lab --tests -- --nocapture` - pass.
- `cargo clippy --workspace --all-targets -- -D warnings` - pass.
- `cargo test --workspace` - pass.
- `make verify` - pass.
- Secret/PII/security scan over PR diff and changed files - pass. No API keys,
  passwords, IP addresses, email addresses, personal names, or security
  incident details were found in the PR changes.

## Live Discovery

- Rust toolchain: verified through `make verify` with workspace build, fmt
  check, clippy, tests, contract validation, docs smoke, and command smoke.
- Repo command wrapper: `Makefile` remains the canonical command surface.
- Schema/config paths: `schemas/lab.experiment_matrix.v1.schema.json`,
  `schemas/lab.experiment_run.v1.schema.json`,
  `tests/golden/lab.experiment_run.v1.valid.json`, and
  `examples/experiments/bounded_load_observe_smoke.yaml`.
- Target connection state: no hardware target required for default
  verification. PR6 tests run local hardware-free experiment matrix paths.
- Artifact/log paths expected from PR6 workflow:
  `experiments/experiment_run.json`,
  `experiments/trials/<trial_id>/load_plan.json`,
  `experiments/trials/<trial_id>/load_result.json`,
  `experiments/trials/<trial_id>/observation.json`,
  `reports/claim_evidence_trace.json`, and `audit.jsonl` operations
  `experiment.trial` and `experiment.run`.

## Triggered Branch Evidence

- ExecPlan - present:
  `plans/20260609-pr6-real-experiment-matrix-runner.md`.
- Embedded NFR design/gate - present:
  `docs/nfr/adc-lab-target-runtime.md`,
  `requirements/nfr/adc-lab-target-runtime.yaml`,
  `requirements/physical_budgets.yaml`, and
  `reports/resource/nfr-gate-report.md`.
- Embedded hot-path review - present:
  `reports/resource/hot-path-review.md`.
- Embedded observer-effect review - present:
  `reports/resource/observer-effect-review.md`.
- Embedded NFR harness design - present:
  `docs/testing/resource-harness.md`.
- Error handling - present:
  `docs/architecture/error-handling.md`.
- Observability - present:
  `docs/architecture/observability-plan.md`.
- Concurrency/shutdown evidence for existing load worker threads - present:
  `reports/concurrency/thread-safety-matrix.md`.

## Exit Criteria Review

- `lab.experiment_run.v1` records per-trial artifact refs, failure reason, and
  start/end timestamps.
- Real `adc-lab experiment run` execution is limited to listed-order
  `cpu_load_workers` matrices.
- Supported trials produce bounded load and/or passive observation artifacts
  before they can become `completed`.
- Unsupported controlled factors, randomized order, failed loads, and failed
  observations become `blocked` or `failed`, not supported claims.
- Each trial emits an `experiment.trial` audit event. The run emits an
  `experiment.run` audit summary.
- Claim trace supports only completed bounded non-privileged matrix execution
  and keeps fixed-frequency, privileged-control, and production physical
  footprint claims blocked.
- Default verification remains hardware-free.
- PR6 adds no privileged control, sudo helper behavior, cpufreq write,
  fixed-frequency sweep, memory pressure, I/O stress, thermal stress, GPU/NPU
  load, destructive experiment, randomized execution, concurrent
  observe-while-load, or production physical-footprint claim.

## Gate Decision

Submit. The change is experimental-only for embedded NFR purposes, all
production physical-footprint claims remain blocked, and the required contracts,
tests, docs, and scans are present.
