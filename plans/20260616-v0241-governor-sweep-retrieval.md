# adc-lab v0.2.4.1 Governor Sweep Retrieval Fix ExecPlan

## Purpose / Big Picture

v0.2.4.1 fixes the release-run blocker discovered by a context-free Codex run
against target55: SSH `collect plan` executed target-local governor sweep steps
but did not emit an operator handoff retrieval step that copies the target-local
governor run into the controller run set. As a result, the same generated plan
that passed `--include-run <run>/included/target-local-governor-sweep` to
`report validate-run` and `report operating-contract` never produced that
directory.

The goal is to make the workflow authority self-consistent: every generated
`--include-run` consumed by validation/reporting must have a preceding
argv-array producer step in the same collect plan, unless it is explicitly
declared as operator-supplied external evidence.

## Scope

In scope:

- Add SSH collect-plan operator handoff steps to retrieve the target-local
  governor-sweep run into `<primary>/included/target-local-governor-sweep`.
- Keep the existing v0.2.4 source-of-truth chain and claim boundaries.
- Add regression tests against the generated collect plan, not only helper
  functions.
- Record RCA, workflow-contract review, verification evidence, and release
  readiness handoff.
- Open a PR against `main`.

Out of scope:

- No target55 repair, install automation, SSH executor, root shell, or arbitrary
  remote command framework.
- No change to governor-sweep approval semantics.
- No relaxation of `report validate-run` or `report operating-contract`
  validation.
- No production readiness, Pi4/Pi5 selection, 24h safety, or real workload
  performance claims.

## Constraints / Quality Targets

- `workflow.collect_plan` remains a handoff/authority artifact, not target
  measurement evidence.
- Steps remain argv arrays; no shell fragments or filename-order discovery.
- Retrieval is controller-side operator handoff plumbing. It must not be counted
  as target measurement evidence.
- The delete/reset step may only target the deterministic retrieved governor
  path, not a parent directory.
- `validate-run` and `operating-contract` must continue to consume identical
  `--include-run` values.
- Local collect plans must not gain SSH-only retrieval steps.
- Verification target: focused collect-plan tests plus `make verify`.

## Context & Orientation

Relevant source:

- `crates/adc-lab-core/src/workflow.rs`: collect-plan generation.
- `crates/adc-lab-core/src/workflow_render.rs`: generated markdown rendering.
- `crates/adc-lab-core/tests/workflow.rs`: core workflow artifact tests.
- `crates/adc-lab/tests/cli.rs`: CLI collect-plan regression tests.

Observed failure evidence:

- target55 rerun root:
  `/home/satoshun/workspace/adc-lab-v024-target55-vanilla-rerun/LAB-RUN-target55-v024-20260616T130553Z`
- `SUPERVISOR_HANDOFF.md` records target/controller/helper versions as `0.2.4`.
- `run_validation.v2.json` status is `insufficient`, with all requested
  governors `unknown` and messages `no control plan for requested governor`.
- `smoke_19_operating_contract.stderr.log` reports:
  `included/target-local-governor-sweep: No such file or directory`.
- `collect_plan.v2.json` includes `governor_sweep_prepare`,
  `governor_sweep_approve`, `governor_sweep_run`, `run_validation`, and
  `operating_contract`, but no `retrieve_target_local_governor_sweep`.

Dev-workflow route:

- Risk route: normal. This is a public Agent-facing workflow behavior fix, but
  bounded to collect-plan generation and tests.
- Required branches: bug-investigation-and-RCA, agent-workflow-contract-review,
  implementation-economy, function-boundary-governor, quality-gate.
- Skipped branches: concurrency, performance, embedded NFR, UI, architecture
  option analysis. This change adds no runtime loop, no new target-local
  primitive, and no architecture option comparison.

## Design

### Root Cause

The v0.2.4 plan generator modeled governor sweep and workload demand as two
target-local evidence producers. During v0.2.4, workload demand received a full
stage/run/retrieve chain:

1. stage workload plan to target
2. run target-local workload demand
3. create included parent
4. reset deterministic destination
5. `scp -r` target-local workload run into the controller run set
6. downstream suitability consumes the explicit retrieved path

Governor sweep received only the target-local prepare/approve/run steps and a
validation note saying the target-local run must be retrieved. The executable
handoff contract did not include the retrieval steps, while downstream
`validate-run` and `operating-contract` were already wired to consume the
retrieved path.

Missing invariant class: generated workflow producer/consumer consistency for
include-run directories.

### Fix

For SSH collect plans, insert a governor retrieval sequence after
`governor_sweep_run` and before `run_validation`:

1. `prepare_target_local_governor_retrieval_parent`
   - `operator_handoff`
   - `mkdir -p <primary>/included`
2. `reset_target_local_governor_retrieval_destination`
   - `operator_handoff`
   - `rm -rf <primary>/included/target-local-governor-sweep`
   - scoped deterministic path only
3. `retrieve_target_local_governor_sweep`
   - `operator_handoff`
   - `scp -r <endpoint>:adc-lab-target-local-<run_id>
     <primary>/included/target-local-governor-sweep`
   - expected path is the retrieved governor include-run directory

The same retrieved path remains the `--include-run` argument for both
`run_validation` and `operating_contract`.

### Test Strategy

Add regression coverage at two levels:

- Core workflow test:
  - SSH collect plan includes the three governor retrieval handoff steps.
  - Step order is `governor_sweep_run` -> prepare/reset/retrieve ->
    `run_validation` -> `operating_contract`.
  - Retrieval command uses argv-array `scp -r` from the target-local run name to
    the exact include-run destination.
  - The include-run consumer path is present in a producer expected path before
    validation.
- CLI collect-plan test:
  - Existing `collect_plan_writes_v2_argv_steps_and_markdown` asserts the exact
    governor retrieval argv and generated markdown mentions the retrieval step.
  - Existing local collect-plan test asserts no SSH-only governor retrieval
    steps are emitted for `--target local`.

Add a workflow-contract review report verifying:

- source-of-truth chain remains explicit
- generated argv replay contains the missing producer step
- producer/consumer table maps retrieved governor path to both downstream
  consumers
- retrieval is handoff plumbing and not target measurement evidence

## Validation & Acceptance

Acceptance criteria:

- A-001: SSH collect plan has a `retrieve_target_local_governor_sweep` step
  before `run_validation`.
- A-002: The retrieved governor path exactly matches all generated
  `--include-run` values.
- A-003: A producer step before validation declares that retrieved include path
  as an expected path.
- A-004: Local collect plans do not emit governor retrieval handoff steps.
- A-005: Generated markdown includes the governor retrieval step and still lacks
  filename-order artifact selection patterns.
- A-006: Focused collect-plan tests and `make verify` pass.

Commands:

- `cargo test -p adc-lab-core collect_plan_steps_are_argv_arrays_and_not_measurement_evidence -- --nocapture`
- `cargo test -p adc-lab --test cli collect_plan_writes_v2_argv_steps_and_markdown collect_plan_local_does_not_emit_include_run_for_validation_or_contract -- --nocapture`
- `cargo test -p adc-lab --test cli collect_plan -- --nocapture`
- `make verify`

## Complexity Budget

- Changed production files target: 1 (`workflow.rs`).
- Changed tests target: 2 (`adc-lab-core/tests/workflow.rs`,
  `adc-lab/tests/cli.rs`).
- New helpers/classes/modules target: 0.
- New indirection layers target: 0.
- Rough line budget: +90 production/test/docs report lines excluding plan and
  review artifacts.

## Function Boundary Decisions

- `target_operating_contract_collect_plan`: keep as the owner of cross-step
  collect-plan path consistency. The fix is a local insertion of additional
  `collect_step_at` values rather than a new abstraction.
- `collect_step_at`: keep. It already owns step construction consistently.
- No new helper will be added unless the repeated workload/governor retrieval
  shape becomes materially larger than expected.

## Design to WBS Coverage Check

| Design deliverable | WBS item |
| --- | --- |
| RCA | WBS 1 |
| Governor retrieval steps | WBS 3 |
| Core workflow regression | WBS 2, WBS 4 |
| CLI regression | WBS 2, WBS 4 |
| Workflow-contract review | WBS 5 |
| Verification and PR | WBS 6, WBS 7 |

## Progress (WBS)

- [x] WBS 0: Create branch from `origin/main` and inspect current failure
  evidence.
- [x] WBS 1: Record initial RCA and development plan.
- [x] WBS 2: Add failing regression expectations for governor retrieval
  producer/consumer consistency.
- [x] WBS 3: Implement SSH governor retrieval handoff steps.
- [x] WBS 4: Update CLI/core tests and generated instruction expectations.
- [x] WBS 5: Add workflow-contract review report.
- [x] WBS 6: Run focused tests and `make verify`.
- [ ] WBS 7: Update Outcomes/Handoff, commit, push, and open PR.

## Surprises & Discoveries

- 2026-06-16: Existing tests asserted identical `--include-run` values for
  validation and operating contract, but not that a preceding producer step
  creates the include path. This allowed a self-consistent consumer pair with no
  producer.
- 2026-06-16: The target55 rerun also exposed a target-local workload PATH
  quoting retry, but that was not the final blocker. The v0.2.4.1 scope remains
  governor-sweep retrieval because workload retrieval already has an executable
  handoff chain and the rerun preserved both attempts.

## Decision Log

- 2026-06-16: Fix the collect-plan producer gap, not downstream validation.
  Rationale: validation correctly rejected missing evidence; relaxing it would
  recreate the Issue #48 class of false full-set claims.
- 2026-06-16: Use `operator_handoff` retrieval steps, not a new remote executor.
  Rationale: v0.2.4 deliberately deferred `collect run`; retrieval remains
  explicit handoff plumbing.
- 2026-06-16: Keep retrieval destination reset scoped to
  `<primary>/included/target-local-governor-sweep`. Rationale: reruns need a
  deterministic layout without broad deletion.
- 2026-06-16: Do not add a new helper abstraction for retrieval steps in this
  patch. Rationale: the immediate fix is small and keeping the existing
  `collect_step_at` call shape avoids a new abstraction while tests protect the
  invariant.

## Verification Log

- Red: `cargo test -p adc-lab-core collect_plan_steps_are_argv_arrays_and_not_measurement_evidence -- --nocapture`
  failed with `collect plan missing full-set skeleton step prepare_target_local_governor_retrieval_parent`.
- Green: `cargo test -p adc-lab-core collect_plan_steps_are_argv_arrays_and_not_measurement_evidence -- --nocapture`
  passed.
- Green: `cargo test -p adc-lab --test cli collect_plan_writes_v2_argv_steps_and_markdown -- --nocapture`
  passed.
- Green: `cargo test -p adc-lab --test cli collect_plan_local_does_not_emit_include_run_for_validation_or_contract -- --nocapture`
  passed.
- Broad focused: `cargo test -p adc-lab --test cli collect_plan -- --nocapture`
  passed: 4 tests.
- Final: `make verify` passed.

## Post-Implementation Economy Audit

| New abstraction | Justification | Decision | Evidence |
| --- | --- | --- | --- |
| none | Existing `collect_step_at` can express the retrieval steps directly. | keep no-new-abstraction | Diff touches one production file; tests cover generated argv. |

Line budget result:

- Production/test code changed in the intended files.
- New helper/class/module count: 0.
- The line delta exceeds the initial +90 rough code/test budget because the
  regression assertions are intentionally explicit. Accepted because the bug
  was caused by under-specified generated-workflow tests.

## Handoff

Current branch: `codex/v0241-governor-sweep-retrieval`.

Status:

- Implementation complete.
- RCA report added at `reports/bug-reports/v0241-governor-sweep-retrieval.md`.
- Workflow-contract review report added at
  `reports/workflow-contract-review/v0241-governor-sweep-retrieval.md` with
  decision `submit`.
- Focused tests and `make verify` passed.
- Known unrelated untracked files exist in the checkout; do not stage them.

Next steps:

1. Commit the intended files only.
2. Push branch and open PR.
3. After merge/release v0.2.4.1, rerun the target55 workflow-authority prompt.

## Outcomes & Retrospective

- v0.2.4 bug class identified: generated include-run consumers without an
  executable producer/retrieval step.
- v0.2.4.1 fix adds explicit governor sweep retrieval handoff steps for SSH
  collect plans.
- Regression tests now verify producer/consumer consistency in generated core
  and CLI collect-plan artifacts.
- No target measurement claims were relaxed.
