# Quality Gate Report

Gate decision: submit

Findings: 0

## Checks Run

- `cargo fmt --all` - pass.
- `cargo test -p adc-lab-core operating_point -- --nocapture` - pass.
- `cargo test -p adc-lab --test cli report_operating_point -- --nocapture` -
  pass.
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
- Schema/config paths:
  `schemas/lab.operating_point_coverage.v1.schema.json`,
  `tests/golden/lab.operating_point_coverage.v1.valid.json`, and
  `examples/experiments/bounded_load_observe_smoke.yaml`.
- Target connection state: no hardware target required for default
  verification. PR7 tests use local hardware-free read/report paths and the
  existing bounded matrix test path.
- Artifact/log paths expected from PR7 workflow:
  `reports/operating_point_coverage.json`,
  `reports/capability_cost_model.json`, and `audit.jsonl` operation
  `report.operating_point`.

## Triggered Branch Evidence

- ExecPlan - present:
  `plans/20260609-pr7-controlled-operating-point-coverage.md`.
- Function boundary review - present:
  `reports/architecture/function-boundary-review.md`.
- Error handling - present:
  `docs/architecture/error-handling.md`.
- Observability - present:
  `docs/architecture/observability-plan.md`.
- Embedded NFR design/gate - present:
  `docs/nfr/adc-lab-target-runtime.md`,
  `requirements/nfr/adc-lab-target-runtime.yaml`,
  `requirements/physical_budgets.yaml`, and
  `reports/resource/nfr-gate-report.md`.
- Hot-path and observer-effect reports - present:
  `reports/resource/hot-path-review.md` and
  `reports/resource/observer-effect-review.md`.

## Exit Criteria Review

- `lab.operating_point_coverage.v1` now records explicit coverage statuses:
  `observational_only`, `controlled_subset`, `controlled_full`,
  `not_controllable`, and `blocked_unsafe`.
- Read-only observation runs produce `observational_only` coverage, with fixed
  CPU frequency sweep claims blocked.
- Completed PR6 `cpu_load_workers` trials produce `controlled_subset` coverage
  for bounded workload levels only.
- Blocked/failed trials become `blocked_points` with reason and next evidence.
- `report operating-point` emits `report.operating_point` audit.
- Claim boundaries keep passive observed frequency movement separate from
  controlled fixed-frequency or governor evidence.
- Default verification remains hardware-free.
- PR7 adds no privileged control, sudo helper behavior, cpufreq write,
  fixed-frequency sweep, load generation, target-local runtime, destructive
  experiment, or production physical-footprint claim.

## Gate Decision

Submit. The change is report/contract-only for operating-point evidence
classification. It improves claim boundaries without adding target-local
runtime or broadening production physical-footprint claims.
