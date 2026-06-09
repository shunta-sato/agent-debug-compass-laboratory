# ExecPlan: Controlled Operating Point Coverage

## Purpose / Big Picture

PR7 makes `lab.operating_point_coverage.v1` distinguish passive observed
variation from controlled operating-point evidence. A run that only sampled
frequency or resource signals under the target's current policy must not look
like a fixed-frequency sweep, while a PR6 bounded `cpu_load_workers` matrix can
support only the controlled workload levels it actually executed.

## Scope

In scope:

- Expand `lab.operating_point_coverage.v1` from string lists to structured
  observed points, controlled points, blocked points, and claim boundaries.
- Generate coverage from existing run artifacts:
  - `observations/observe.json`
  - `experiments/experiment_run.json`
- Classify coverage status as:
  - `observational_only`
  - `controlled_subset`
  - `controlled_full`
  - `not_controllable`
  - `blocked_unsafe`
- Keep fixed CPU frequency / governor control claims blocked unless explicit
  controlled evidence exists.
- Add CLI tests and contract fixtures.
- Update claim-boundary, safety, NFR, and gate docs.

Out of scope:

- Adding privileged cpufreq control to matrix execution.
- Fixed-frequency sweep execution.
- Randomized matrix execution.
- New target hardware smoke.
- Production physical-footprint claims.

## Constraints / Quality Targets

- Default `make verify` remains hardware-free.
- Agent-facing refs remain `artifact://lab/runs/...`, never raw local paths.
- Unsupported or missing controlled factors must be represented as blocked
  points, not silently omitted.
- `controlled_full` is allowed by contract but not emitted by PR7.
- No sudo/helper behavior changes.

## Dev Workflow Route

- Selected risk route: high.
- Why: schema/contract shape, CLI report behavior, claim-boundary semantics,
  and embedded evidence interpretation all change.
- Required branches:
  - `execution-plans`: cross-boundary schema/core/CLI/docs/tests work.
  - `function-boundary-governor`: new DTOs and coverage builder helpers change
    function/API boundaries.
  - `error-handling`: missing artifacts and blocked operating points require
    explicit outcomes.
  - `observability`: coverage artifacts and audit signal are evidence.
  - `embedded-nfr-design`: operating-point claims are embedded physical-footprint
    evidence boundaries.
  - `embedded-nfr-gate`: feature-level embedded NFR claim gate before submit.
- Non-triggered branches:
  - `architecture-decision-analysis`: no competing architecture options.
  - `concurrency-core`: no new concurrency.
  - `embedded-hot-path-review`: no new target-local hot path.
  - `embedded-observer-effect-review`: no new observer or sampler behavior.
  - `embedded-nfr-harness-design`: no new measurement harness in PR7.

## Requirements / Acceptance

- EARS-AC1: When a run has only passive observation evidence, operating-point
  coverage shall have `coverage_status=observational_only`.
- EARS-AC2: When a run has completed PR6 `cpu_load_workers` trials, coverage
  shall include controlled points for those workload levels and top-level
  `coverage_status=controlled_subset`.
- EARS-AC3: When a trial is blocked or failed for a controlled factor, coverage
  shall include a blocked point with reason and next evidence needed.
- EARS-AC4: Coverage shall always block fixed-frequency sweep claims unless
  explicit controlled fixed-frequency evidence exists.
- EARS-AC5: Coverage claim boundaries shall separate passive observed variation
  from controlled operating-point evidence.
- EARS-AC6: `report operating-point` shall emit an audit event.

## Context & Orientation

Key files:

- `crates/adc-lab-core/src/contracts.rs`: coverage DTOs.
- `crates/adc-lab-core/src/report.rs`: run artifact inspection and coverage
  builder.
- `crates/adc-lab/src/main.rs`: `report operating-point` command.
- `schemas/lab.operating_point_coverage.v1.schema.json`: strict contract.
- `tests/golden/lab.operating_point_coverage.v1.valid.json`: fixture.
- `crates/adc-lab/tests/cli.rs`: hardware-free report tests.
- `docs/architecture/claim-boundaries.md`: user-facing semantics.

## Design

Coverage is generated from artifacts already present in a lab run.

- Passive observation adds an `observed_point` for current target policy.
- Completed `cpu_load_workers` trials add `controlled_points` for executed
  workload levels.
- Blocked/failed trials add `blocked_points`.
- Fixed CPU frequency remains blocked by default because observed frequency
  movement is not a controlled sweep.

Top-level status:

- any safety-blocked point -> `blocked_unsafe`
- else any controlled point -> `controlled_subset`
- else any observed point -> `observational_only`
- else -> `not_controllable`

## Test Strategy

- Contract fixture validation for the expanded schema.
- Core tests for read-only observation and bounded matrix coverage
  classification.
- CLI tests for `report operating-point` after read-only familiarization and
  after PR6 bounded matrix execution.

## Milestones

1. Expand contracts/schema/golden fixture.
2. Implement coverage builder from run artifacts.
3. Wire CLI report audit and tests.
4. Update docs and NFR/gate evidence.
5. Run full verification and sensitive scan.
6. Commit, push, and open draft PR.

## Progress (WBS)

- [x] Create PR7 branch from merged PR6 main.
- [x] Route dev workflow and create ExecPlan.
- [x] Expand contracts/schema/golden fixture.
- [x] Implement coverage builder from run artifacts.
- [x] Add CLI/core tests.
- [x] Update docs and NFR/gate evidence.
- [x] Run full verification and sensitive scan.
- [ ] Commit, push, open draft PR.

## Surprises & Discoveries

- PR1 already had a minimal `lab.operating_point_coverage.v1` skeleton and
  `report operating-point` command, but the artifact was a fixed provisional
  string list. PR7 can evolve that surface instead of adding a new command.
- Function-boundary routing became triggered after implementation because PR7
  added multiple report-domain helpers and expanded the public coverage DTO.

## Decision Log

- 2026-06-09: Keep PR7 read/report-only. It interprets existing artifacts and
  does not add control, load, or target-side execution.
- 2026-06-09: Emit `controlled_subset` for PR6 `cpu_load_workers` levels because
  workload intensity was controlled, while CPU frequency remains a blocked
  sweep claim.

## Handoff

Branch: `codex/pr7-controlled-operating-point-coverage`.

Current status: implementation and verification complete; commit/PR handoff
next.

Run:

```sh
make verify
```

Read first:

- `crates/adc-lab-core/src/report.rs`
- `crates/adc-lab-core/src/contracts.rs`
- `crates/adc-lab/src/main.rs`
- `schemas/lab.operating_point_coverage.v1.schema.json`

## Outcomes & Retrospective

- Expanded `lab.operating_point_coverage.v1` into structured observed,
  controlled, blocked, and claim-boundary sections.
- `report operating-point` now reads run artifacts and emits
  `observational_only` or `controlled_subset` coverage where appropriate.
- Fixed-frequency and governor claims remain blocked without privileged
  plan/apply/restore evidence.
- Verification passed:
  - `cargo fmt --all`
  - `cargo test -p adc-lab-core operating_point -- --nocapture`
  - `cargo test -p adc-lab --test cli report_operating_point -- --nocapture`
  - `make contract`
  - `cargo test -p adc-lab --tests -- --nocapture`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `make verify`
- Sensitive scan over the PR diff found no API keys, passwords, IP addresses,
  email addresses, personal names, or security incident details.
