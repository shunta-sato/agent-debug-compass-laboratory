# Workflow Contract Review

## Scope

- PR / branch: `codex/v024-pr8-constraints-self-check`.
- Workflow surfaces:
  - `constraints self-check --out`.
  - `workflow.collect_plan` `constraints_self_check` step.
  - Public docs for generated constraints self-check persistence.
- Generated artifacts:
  - `report.constraints_check`
  - `workflow.collect_plan`

## Source-of-truth chain

| Stage | Artifact / command | Producer | Consumer | Notes |
| --- | --- | --- | --- | --- |
| workflow authority | `workflow.recommendation` | `adc-lab workflow recommend` | `adc-lab collect plan` / Agent | unchanged; not measurement evidence |
| executable handoff | `workflow.collect_plan` | `adc-lab collect plan` | Agent/operator | now expects `reports/constraints_check.v2.json` from self-check |
| generated constraints | `report.constraints` and `agent_constraints.md` | `adc-lab constraints generate` | `constraints self-check` | unchanged source for generated negative/explanatory constraints |
| self-check result | `report.constraints_check` | `adc-lab constraints self-check --out ...` | handoff reviewer / Agent | persisted explicitly; no filename-order discovery needed |

## Generated argv replay

| Step | Execution location | argv | Expected artifact | Stop/continue |
| --- | --- | --- | --- | --- |
| `constraints_self_check` | controller | `["adc-lab","constraints","self-check","--constraints",...,"--path",...,"--out","<run>/reports/constraints_check.v2.json","--json"]` | `report.constraints_check` at the explicit `--out` path | failure blocks final handoff completion but does not change measurement validity |

## Producer/consumer consistency

| Producer | Artifact | Consumer | Required identity match | Result |
| --- | --- | --- | --- | --- |
| constraints generate | `report.constraints` | constraints self-check | v2 `report.constraints` envelope | pass |
| constraints self-check | `report.constraints_check` | handoff reviewer / Agent | mode `generated_constraints`; checked path equals generated markdown path | pass |
| collect plan | expected artifact path | Agent/operator | path is generated in argv and `expected_artifact_paths_or_globs` | pass |

## Run-set / target / workflow identity consistency

| Identity | Producer value | Consumer value | Result |
| --- | --- | --- | --- |
| run id | constraints artifact | constraints check artifact and audit event | pass |
| target id | constraints artifact | constraints check artifact and audit event | pass |
| workflow profile | collect plan | downstream constraints step | unchanged pass |

## Forbidden fallback checks

- filename-order artifact selection: no new usage; collect plan carries the exact self-check output path.
- mtime/latest/newest artifact inference: no new usage.
- stale prompt fallback: no new prompt choreography; docs keep workflow authority surfaces.
- raw co-presence as causal evidence: no new claim path; self-check validates generated constraints content only.

## Claim boundaries

- `report.constraints_check` is a constraints validation artifact, not target measurement evidence.
- Generated blocked claims remain allowed in self-check mode because they are negative/explanatory constraints.
- Candidate-content checks remain strict and fail when unsupported positive claim text appears downstream.
- Packaging/handoff completion may depend on the persisted self-check result, but measurement validity is not upgraded by the artifact.

## Findings

| ID | Severity | Finding | Required fix |
| --- | --- | --- | --- |
| none | n/a | No workflow-contract findings. | n/a |

## Decision

submit
