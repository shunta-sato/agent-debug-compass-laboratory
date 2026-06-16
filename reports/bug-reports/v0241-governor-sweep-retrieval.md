# Bug Report (RCA)

- Title: SSH collect plan omits governor-sweep retrieval before run validation
- Symptom (actual behavior): A v0.2.4 release-binary target55 run generated and
  executed target-local governor sweep steps, then failed at reporting because
  `<primary>/included/target-local-governor-sweep` did not exist.
- Expected behavior: When a collect plan passes a generated `--include-run`
  path to `report validate-run` and `report operating-contract`, the same plan
  must include a preceding argv-array producer or retrieval step for that path.
- Severity/Impact: High for v0.2.4 workflow authority. A context-free Agent
  following the generated plan cannot complete smoke operating-contract
  reporting on an SSH target.
- Environment (versions, platform, config): Controller and target-side
  `adc-lab`, `adc-lab-target`, and `adc-lab-priv-helper` all reported v0.2.4
  during the rerun. Target was `target55`, class `raspberry_pi_4`, SSH workflow.
- Detection (how it was found): Context-free Codex execution of the v0.2.4
  target55 workflow-authority prompt after target binaries were updated.

## Reproduction

- Steps to reproduce:
  1. Use v0.2.4 release binaries.
  2. Generate `adc-lab collect plan --goal target-operating-contract-smoke
     --target ssh://target55 ...`.
  3. Follow the generated argv-array steps.
  4. Observe `run_validation.v2.json` and then `operating_contract`.
- Minimal repro (if available): Generated collect-plan inspection is enough:
  the plan contains `governor_sweep_run`, `run_validation`, and
  `operating_contract`, and both consumers refer to
  `<primary>/included/target-local-governor-sweep`, but there is no preceding
  `retrieve_target_local_governor_sweep` step.
- Frequency: Deterministic for SSH collect plans that require governor
  include-run evidence.

## Evidence

- Logs / stack trace / metrics / traces:
  - `/home/satoshun/workspace/adc-lab-v024-target55-vanilla-rerun/LAB-RUN-target55-v024-20260616T130553Z/SUPERVISOR_HANDOFF.md`
  - `reports/run_validation.v2.json` recorded `status.state = insufficient`,
    `overall_validity = unknown`, and each requested governor had message
    `no control plan for requested governor`.
  - `logs/smoke_19_operating_contract.stderr.log` recorded:
    `included/target-local-governor-sweep: No such file or directory`.
  - `workflows/collect_plan.v2.json` had no
    `retrieve_target_local_governor_sweep` step.
- What changed recently (if known): v0.2.4 added SSH target-local workload
  demand retrieval and persisted constraints self-check. Governor include-run
  parity was previously asserted only between `validate-run` and
  `operating-contract`, not between a producer step and those consumers.

## Root Cause Analysis (Five Whys)

1. Why did `operating_contract` fail?
   - Evidence: It attempted to open
     `<primary>/included/target-local-governor-sweep`, which did not exist.
2. Why did that include-run path not exist?
   - Evidence: The generated collect plan contained no operator handoff step
     that retrieved `adc-lab-target-local-<run_id>` into that path.
3. Why did the generated plan still pass that missing path downstream?
   - Evidence: v0.2.3.1 fixed include-run parity between `validate-run` and
     `operating-contract`, so both consumers agreed on the same path. The
     producer side was not part of that invariant.
4. Why was producer-side retrieval covered for workload but not governor sweep?
   - Evidence: v0.2.4 workload steps explicitly added
     `prepare/reset/retrieve_target_local_workload_demand`; governor sweep
     retained only a validation note saying to retrieve the run before
     controller-side validation.
5. Why did tests miss it?
   - Evidence: Existing tests checked SSH target-local execution location,
     argv-array safety, identical include-run consumer args, and workload
     retrieval. They did not assert that every generated include-run consumed by
     validation/reporting has an earlier producer/retrieval step in the same
     collect plan.

Root cause: missing generated-workflow producer/consumer invariant for
include-run directories.

## Fix

- What changed (summary): Add SSH operator handoff steps to prepare, reset, and
  retrieve the target-local governor-sweep run into the exact include-run path
  consumed by `run_validation` and `operating_contract`.
- Why this fix addresses the root cause: The generated plan now contains a
  concrete argv-array producer for the path it passes to downstream consumers,
  making the source-of-truth chain executable without manual shell harness
  invention.

## Verification

- Tests run:
  - Red: `cargo test -p adc-lab-core collect_plan_steps_are_argv_arrays_and_not_measurement_evidence -- --nocapture`
    failed with `collect plan missing full-set skeleton step prepare_target_local_governor_retrieval_parent`.
  - Green: `cargo test -p adc-lab-core collect_plan_steps_are_argv_arrays_and_not_measurement_evidence -- --nocapture`
    passed.
  - Green: `cargo test -p adc-lab --test cli collect_plan -- --nocapture`
    passed, including SSH collect-plan, characterization-full collect-plan, and local-target negative coverage.
  - Final: `make verify` passed.
- Repro re-run result: Generated-workflow reproduction is closed by tests:
  SSH collect plans now include `retrieve_target_local_governor_sweep` before
  `run_validation`, and the retrieved path exactly matches generated
  `--include-run` consumers.
- Tooling run (if relevant): `make verify` passed build, fmt, clippy,
  generated schema drift, schema ledger, file budgets, workspace tests,
  contract validation, docs smoke, and resource smoke host fallback.

## Prevention

- Prevent: Add regression assertions that a generated SSH collect plan contains
  `retrieve_target_local_governor_sweep` before `run_validation`.
- Detect: Add a generated-workflow invariant in tests: every generated
  governor `--include-run` consumed by validation/reporting is matched by a
  preceding expected path from a producer/retrieval step.
- Mitigate: Keep downstream validation strict. Missing include-run evidence
  remains `insufficient` or fails closed rather than becoming a measured claim.
- Follow-up tasks (with owners / tracking IDs if available): This PR is the
  v0.2.4.1 fix.
- If missed workflow/product contract:
  - Missing invariant class: include-run producer/consumer consistency.
  - Generated-workflow regression: CLI/core collect-plan tests inspect the
    generated artifact.
  - Process update: workflow-contract review report for this PR explicitly
    includes a producer/consumer table for include-run paths.
  - Replay fixture or generated artifact snapshot: the target55 rerun archive
    listed above is the replay evidence; tests use deterministic generated
    collect-plan artifacts.

## Workaround

- Workaround description: An operator could manually copy the target-local run
  into the expected include-run path before validation.
- Risk: That would bypass the workflow authority contract and invite stale or
  path-inferred evidence.
- Removal plan / tracking: Do not rely on the workaround; v0.2.4.1 makes the
  retrieval step explicit in `workflow.collect_plan`.
