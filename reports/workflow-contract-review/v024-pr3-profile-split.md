# Workflow Contract Review

## Scope

- PR / branch: `codex/v024-pr3-profile-split`.
- Workflow surfaces:
  - `workflow recommend` profile resolution and generated
    `workflow.recommendation` payload.
  - `agent instructions` profile metadata and claim-boundary rendering.
  - `collect plan` profile metadata and generated `report validate-run`
    argv.
  - `report validate-run` requested/effective profile payload fields.
  - Operating-contract validation gate, run-report summary, and rules predicate
    consumption of supported validation profiles.

## Source-of-Truth Chain

| Stage | Artifact / command | Producer | Consumer | Notes |
| --- | --- | --- | --- | --- |
| recommendation | `workflow.recommendation` | `adc-lab workflow recommend --goal <profile>` | Agent / collect plan author | Authority artifact only; not target measurement evidence. |
| collect plan | `workflow.collect_plan` | `adc-lab collect plan --goal <profile>` | Agent / operator | Carries `goal`, `effective_profile`, `profile_depth`, coverage, boundary, and safety caps. |
| run validation | `report.run_validation` | collect-plan `run_validation` argv | operating-contract gate / rules / run report | Uses `--profile <effective_profile>`; legacy fullset requires explicit depth before generation. |
| operating contract | `report.operating_contract` | collect-plan `operating_contract` argv | suitability | Still requires matching run-set identity and measured validation before controlled-governor claims. |
| suitability / constraints | `report.suitability` / `report.constraints` | existing downstream commands | Agent handoff | Profile split does not relax downstream selection readiness or constraints semantics. |

## Profile Contract

| Requested profile | Required depth flag | Effective validation profile | Result |
| --- | --- | --- | --- |
| `target-operating-contract-smoke` | optional, must be `smoke` if supplied | `target-operating-contract-smoke` | pass |
| `target-characterization-full` | optional, must be `characterization-full` if supplied | `target-characterization-full` | pass |
| `target-operating-contract-fullset` | required: `smoke` or `characterization-full` | selected by depth | compatibility only; warning emitted |
| unsupported profile | n/a | n/a | fail closed |

## Generated Argv Replay

| Producer | Generated argv element | Consumer expectation | Result |
| --- | --- | --- | --- |
| smoke collect plan | `report validate-run --profile target-operating-contract-smoke` | validation payload records effective smoke profile | pass |
| characterization-full collect plan | `report validate-run --profile target-characterization-full` | validation payload records full characterization profile | pass |
| legacy fullset collect plan with depth | `workflow recommend --profile-depth <depth>` and `validate-run --profile <effective>` | old fullset name cannot silently choose depth | pass |
| operating-contract step | `--validation reports/run_validation.v2.json --strict-fullset` | gate checks run set, workflow id, target id/class, supported profile, and measured validity | pass |

## Producer / Consumer Consistency

| Producer | Field / artifact | Consumer | Required match | Result |
| --- | --- | --- | --- | --- |
| profile resolver | `effective_profile` | collect-plan `run_validation` argv | exact profile string | pass |
| `report validate-run` | `payload.profile` | operating-contract gate | supported profile name | pass |
| `report validate-run` | `payload.validation_profile` | operating-contract gate / rules | smoke, characterization-full, or legacy v0.2.3 compatibility | pass |
| validation artifact | `subject_run_set_id` / `included_run_refs` | operating-contract gate | current run set identity | unchanged, pass |
| validation artifact | `workflow_id` | operating-contract gate | `target-operating-contract-fullset.v0.2.3` workflow family | unchanged, pass |

## Claim Boundaries

- `workflow.recommendation` and `workflow.collect_plan` remain authority and
  handoff artifacts, not target measurement evidence.
- Smoke profile explicitly says it is not deep target characterization and
  cannot support production, selection, 24h safety, or Pi4/Pi5 sufficiency
  claims.
- Characterization-full profile is still bounded laboratory evidence; it does
  not by itself authorize production readiness, 24h safety, battery safety, or
  target selection.
- Legacy `target-operating-contract-fullset` cannot be used without an explicit
  depth choice.

## Forbidden Fallback Checks

- No filename-order artifact selection introduced.
- No mtime/latest/newest inference introduced.
- No hand-written shell harness fallback introduced.
- No raw primitive control/load artifact is promoted to a controlled-governor
  claim without matching `report.run_validation`.

## Verification Evidence

- `cargo test -p adc-lab --test cli workflow_ -- --nocapture`
- `cargo test -p adc-lab --test cli collect_plan_ -- --nocapture`
- `cargo test -p adc-lab --test cli validate_run -- --nocapture`
- `cargo test -p adc-lab --test cli operating_contract -- --nocapture`
- `python3 scripts/ci/check-file-budgets.py --enforce`

- `cargo test -p adc-lab-core -- --nocapture`
- `cargo test -p adc-lab --test cli -- --nocapture`
- `make schemas-check`
- `make docs-smoke`
- `make verify`

## Findings

| ID | Severity | Finding | Required fix |
| --- | --- | --- | --- |
| none | none | No workflow-contract blocker for PR 3. | none |

## Decision

submit
