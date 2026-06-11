# Quality Gate Report

Gate decision: submit

Findings: 0

## Checks Run

- `cargo check` - pass.
- `cargo test -p adc-lab-core` - pass.
- `cargo test -p adc-lab --test cli` - pass.
- `cargo build --release -p adc-lab` - pass; used for target55 staged
  validation.
- `make verify` - pass.

## Live Discovery

- Repo command surface: `COMMANDS.md` defines `make verify` as the canonical
  gate.
- Existing implementation surfaces inspected:
  `crates/adc-lab/src/main.rs`, `crates/adc-lab-core/src/contracts.rs`,
  `crates/adc-lab-core/src/load.rs`, `crates/adc-lab-core/src/observe.rs`,
  `crates/adc-lab-core/src/run.rs`, `crates/adc-lab-core/src/target.rs`,
  schema fixtures, and CLI tests.
- target55 state checked live: target and controller are `aarch64`; installed
  target55 release remains `/home/satoshun/.local/bin/adc-lab` version
  `0.1.16`.
- Staged validation binary:
  `/home/satoshun/.local/share/adc-lab/runners/20260611-workload-suitability/adc-lab`
  (`sha256=0e084cdde7733ebd2460f0758a01c9fe202cccff62e099df856209117e5b4f10`).
- Target-local workload run:
  `lab/runs/LAB-RUN-target55-local-workload-suitability-20260611T015207Z`.
  The result records `status=completed`, `execution_mode=target_local`, and
  `target_id=target55`.
- SSH workload refusal run:
  `lab/runs/LAB-RUN-workload-ssh-refusal-20260611T015207Z`. The result records
  `status=refused` and
  `reason=remote_workload_execution_not_supported_in_v1`.
- Artifact zip:
  `/mnt/share/target55-local-workload-suitability-20260611.zip`
  (`sha256=bb039e3eeeb36184973b676edaac2aede32fab45b7435d90914b6a075c9f0572`);
  `unzip -t` passed.

## Triggered Branch Evidence

- ExecPlan - present:
  `plans/20260611-local-workload-suitability-loop.md`.
- Embedded system familiarization - present:
  `docs/targets/target55/system-familiarization.md`.
- Embedded NFR gate - present:
  `reports/resource/nfr-gate-report.md`, decision `experimental-only`.
- Function-boundary decision - present:
  `.agents/design-ledger/function-boundaries.md`.
- Observability - present:
  workload run audit artifact, run artifacts, structured refusal result, and
  existing `docs/architecture/observability-plan.md`.

## Exit Criteria Review

- New contracts exist with schemas and golden fixtures:
  `lab.workload_run_plan.v1`, `lab.workload_run_result.v1`,
  `lab.workload_demand_profile.v1`, `lab.suitability_policy.v1`,
  `lab.suitability_decision.v1`, and `lab.design_constraint_pack.v1`.
- `workload run` v1 is local-target only. SSH target workload execution returns
  structured refusal and does not transport executable paths/args to a remote
  shell.
- Workload demand separates process-scoped demand from target-conditioned
  thermal/frequency response and whole-system context.
- Thermal suitability is target-conditioned and non-portable.
- Suitability policy cannot convert unknown evidence to meet; required unknown
  dimensions force `selection_ready=false`.
- Failed/refused/aborted workload evidence cannot produce a meet decision.
- Constraint generation emits JSON and agent-facing Markdown; constraint check
  fails on blocked claim text.
- Target55 validation is representative bounded smoke only. It is not real app
  performance, production readiness, sustained thermal safety, flash-wear
  evidence, or Pi4/Pi5 selection evidence.

## Gate Decision

Submit. The change closes a conservative local-target evidence-to-decision loop
for one bounded representative workload while preserving the remote-execution,
privilege, unknown-is-not-pass, and claim-boundary rules.
