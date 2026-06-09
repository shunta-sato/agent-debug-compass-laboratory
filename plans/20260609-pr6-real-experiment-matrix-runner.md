# ExecPlan: Real Experiment Matrix Runner

## Purpose / Big Picture

PR6 moves `adc-lab experiment run` from planning-only to real audited execution
for the first safe subset of matrix experiments. The runner must only mark a
trial `completed` when it actually executed supported steps and produced
per-trial evidence artifacts.

The first real execution surface is deliberately narrow:

```text
listed matrix -> supported cpu_load_workers factor -> optional bounded CPU load -> passive observe -> per-trial artifacts -> trial audit
```

Unsupported controlled factors such as `governor` remain `blocked` until their
approval/control/restore workflow is explicitly wired into the matrix runner.

## Scope

In scope:

- `lab.experiment_run.v1` per-trial evidence fields:
  - `artifact_refs`
  - `failure`
  - `started_at_unix_ms`
  - `ended_at_unix_ms`
- Real `adc-lab experiment run` execution for `order=listed`.
- Supported controlled factor: `cpu_load_workers`.
- Per-trial bounded CPU load using PR5 `lab.load_plan.v1` /
  `lab.load_result.v1`.
- Per-trial passive observation artifact.
- Trial-level audit events using operation `experiment.trial`.
- Blocking unsupported controlled factors instead of pretending execution.
- Claim trace that supports only the completed bounded matrix subset and keeps
  fixed-frequency / privileged-control claims blocked.
- Hardware-free local tests and schema fixtures.

Out of scope:

- Privileged governor or cpufreq control inside matrix execution.
- Remote privileged apply.
- Randomized order execution.
- Concurrent observe-while-load execution.
- Fixed-frequency sweep.
- Memory/I/O/GPU/NPU stress.
- Tier 3 sustained thermal or degradation tests.
- Production physical-footprint claims.

## Constraints / Quality Targets

- Default `make verify` remains hardware-free.
- No arbitrary shell and no new root/helper path.
- Matrix execution may only run allowlisted non-privileged steps.
- Unsupported controlled factors must produce `blocked`, not `completed`.
- Failed or safety-aborted load trials must not become supported evidence.
- Per-trial artifact refs must be bounded `artifact://lab/runs/...` refs.
- Warmup/cooldown/repetition/trial count must be bounded by policy.
- `randomized` order remains blocked until seeded reproducible ordering is
  added.

## Dev Workflow Route

- Selected risk route: high.
- Why: PR6 changes runtime behavior, evidence contracts, trial audit semantics,
  and embedded target-local load/observe orchestration.
- Required branches:
  - `execution-plans`: cross-boundary CLI/core/schema/docs/tests change.
  - `error-handling`: unsupported factors, failed trials, and aborted loads
    need explicit domain outcomes.
  - `observability`: trial-level audit and per-trial artifacts are new signals.
  - `embedded-nfr-design`: matrix execution orchestrates target-local load and
    observation bursts.
  - `embedded-hot-path-review`: matrix runner repeats load/observe loops over
    trials.
  - `embedded-observer-effect-review`: per-trial observation can perturb target
    behavior.
  - `embedded-nfr-harness-design`: real matrix execution is part of the future
    resource harness.
  - `embedded-nfr-gate`: final embedded NFR decision before quality gate.
- Non-triggered branches:
  - `architecture-decision-analysis`: roadmap already selected PR6 scope; no
    cross-technology option comparison is needed.
  - `concurrency-core`: PR6 intentionally avoids concurrent observe/load and
    uses the existing bounded CPU load worker model unchanged.
  - `embedded-system-familiarization`: no new target-specific characterization
    is claimed in this PR.

## Requirements / Acceptance

- EARS-AC1: When a non-dry `experiment run` uses an allowlisted
  `cpu_load_workers` matrix, each executed trial shall write per-trial artifacts
  and record their logical artifact refs in `lab.experiment_run.v1`.
- EARS-AC2: When a trial executes load and observe successfully, the trial shall
  have `status=completed`.
- EARS-AC3: When a controlled factor is unsupported, the trial shall have
  `status=blocked`, shall record a failure reason, and shall not claim support.
- EARS-AC4: When a load aborts or a trial step fails, the trial shall have
  `status=failed` and shall keep any generated artifact refs for diagnosis.
- EARS-AC5: Every trial shall emit an `experiment.trial` audit event.
- EARS-AC6: `experiment.run` audit result shall summarize the run as
  `completed`, `blocked`, `failed`, or `planned`.
- EARS-AC7: `randomized` order shall be blocked in real execution until seeded
  reproducible ordering is implemented.
- EARS-AC8: Claim trace shall support only completed bounded non-privileged
  matrix execution and shall keep fixed CPU frequency / privileged-control /
  production claims blocked.

## Context & Orientation

Key files:

- `crates/adc-lab-core/src/experiment.rs`: matrix expansion, planning, and claim
  trace skeleton.
- `crates/adc-lab-core/src/contracts.rs`: `ExperimentTrial` and
  `ExperimentRun`.
- `crates/adc-lab/src/main.rs`: CLI command wiring, artifact writes, SSH
  observe/load helpers, and audit events.
- `schemas/lab.experiment_run.v1.schema.json`: strict experiment run contract.
- `examples/experiments/`: matrix examples.
- `crates/adc-lab/tests/cli.rs`: hardware-free CLI integration tests.
- `docs/architecture/error-handling.md`: failure contract.
- `docs/architecture/observability-plan.md`: audit and artifact signals.
- `reports/resource/*.md`: embedded NFR evidence.

Baseline: merged PR5 at `main` adds bounded CPU load safety monitor and accepts
the built-in CPU load tool only for bounded load plan/result evidence.

## Design

### Supported Execution Subset

Real execution supports only:

```text
order: listed
controlled_factor:
  cpu_load_workers:
    levels: ["0", "1", ...]
```

Levels:

- `0` or `none`: no load step, observe only.
- positive integer: run bounded CPU load with that worker count, then observe.

Observed covariates and uncontrolled confounders are recorded as matrix factors
but do not drive execution. Any other controlled factor blocks the trial.

### Per-Trial Artifacts

Artifacts are written under:

```text
experiments/trials/<trial_id>/load_plan.json
experiments/trials/<trial_id>/load_result.json
experiments/trials/<trial_id>/observation.json
```

Only logical refs are serialized into `ExperimentTrial.artifact_refs`.

### Runtime Bounds

- `warmup_seconds <= 60`
- `cooldown_seconds <= 60`
- `repetitions <= 10`
- expanded trials <= 64
- load duration uses PR5 CPU load bounds
- observe duration is CLI bounded by duration parser and test defaults

### Error Handling

Boundary translation follows "13. Error handling" and "13.1 Basic templates":

- unsupported controlled factor -> trial `blocked`
- randomized order -> trial `blocked`
- load safety abort -> trial `failed`, preserving load result artifact
- filesystem/JSON/SSH failure -> CLI error where evidence cannot be trusted

### Observability

No metrics backend is added. Signals are audit JSONL and per-trial artifacts:

- `experiment.trial`: one audit event per trial.
- `experiment.run`: summary audit event after run/trace artifact write.

### Test Strategy

- Core tests for trial expansion fields and non-dry planning not claiming
  support.
- CLI test for real local bounded matrix with `cpu_load_workers=0,1`.
- CLI test for unsupported governor factor blocked.
- Contract validation for updated schema/golden fixtures.

## Milestones

1. Extend experiment run contract and schema.
2. Add bounded matrix validation and trial metadata in core.
3. Implement controller trial executor and trial audit.
4. Add examples and tests.
5. Update docs, NFR reports, quality gate.
6. Run verification and sensitive scan.
7. Commit, push, and open draft PR.

## Progress (WBS)

- [x] Create PR6 branch and inspect merged PR5 baseline.
- [x] Route dev workflow and create ExecPlan.
- [x] Extend experiment run contracts/schema/goldens.
- [x] Implement safe listed-order trial executor.
- [x] Add examples and tests.
- [x] Update docs and NFR reports.
- [x] Run full verification and sensitive scan.
- [ ] Commit, push, open draft PR.

## Surprises & Discoveries

- The existing runner intentionally emits `not_implemented` for all non-dry
  runs. PR6 must preserve this safety property for unsupported factors while
  allowing completed status only for actually executed supported trials.
- The CLI can safely own the first real execution subset while core retains the
  dry-run/planning contract. This avoids mixing side-effecting target execution
  into the core contract module.

## Decision Log

- 2026-06-09: Limit PR6 real execution to listed order and
  `cpu_load_workers`; keep `governor` and fixed-frequency matrices blocked.
- 2026-06-09: Avoid concurrent observe-while-load in PR6. Sequential
  load-then-observe keeps the first runner auditable and avoids introducing a
  new concurrency design.

## Handoff

Branch: `codex/pr6-real-experiment-matrix-runner`.

Current status: implementation and verification complete; commit/PR handoff
next.

Run:

```sh
make verify
```

Read first:

- `crates/adc-lab-core/src/experiment.rs`
- `crates/adc-lab/src/main.rs`
- `crates/adc-lab/tests/cli.rs`
- `schemas/lab.experiment_run.v1.schema.json`

## Outcomes & Retrospective

- Added the first real experiment matrix execution subset:
  listed-order `cpu_load_workers` trials with bounded load, passive observe,
  per-trial artifact refs, and trial audit.
- Preserved claim boundaries: unsupported controlled factors, randomized order,
  and failed trial steps do not support claims.
- Verification passed:
  - `cargo fmt --all`
  - `make contract`
  - `cargo test -p adc-lab --tests -- --nocapture`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `make verify`
- Sensitive scan over the PR diff found no API keys, passwords, IP addresses,
  email addresses, personal names, or security incident details.
