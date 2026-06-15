# Workflow Contract Review

## Scope

- PR / branch: `codex/v024-pr1-evidence-ref-resolution`.
- Workflow surfaces:
  - `report operating-contract --include-run ... --validation ...`
  - `report.evidence_ref_resolution` handoff artifact.
  - Downstream `report.suitability` and `report.constraints` evidence refs
    checked by the shared run-set resolver.
- Generated artifacts:
  - `reports/target_operating_contract.v2.json`
  - `reports/evidence_ref_resolution.v2.json`

## Source-of-truth chain

| Stage | Artifact / command | Producer | Consumer | Notes |
| --- | --- | --- | --- | --- |
| run set | `--run` plus `--include-run` argv | operator / collect plan | `report operating-contract` | No filename-order discovery. |
| validation | `report.run_validation` path from `--validation` | `report validate-run` | operating-contract validation gate | Existing run-set identity checks remain authoritative. |
| operating contract | `report.operating_contract` | `report operating-contract` | suitability / constraints | Controlled-governor claims still require matching validation. |
| evidence-ref resolution | `report.evidence_ref_resolution` | `report operating-contract` | handoff reviewer / future archive checks | Checks refs against the same opened run set. |

## Generated argv replay

| Step | Execution location | argv | Required env | Expected artifact | Stop/continue |
| --- | --- | --- | --- | --- | --- |
| operating_contract | controller | Existing public `adc-lab report operating-contract --run <main> --include-run <included> --validation <path> --strict-fullset --json` | none beyond existing adc-lab runtime | `report.operating_contract`, `report.evidence_ref_resolution` | Existing strict-fullset failure behavior is unchanged. |

## Producer/consumer consistency

| Producer | Artifact | Consumer | Required identity match | Result |
| --- | --- | --- | --- | --- |
| `EvidenceStore::open` | run-set resolution map | `report.evidence_ref_resolution` | logical run id, opened path, artifact URI root, primary/included role | pass |
| `report operating-contract` | `report.operating_contract` evidence refs | resolver report | every `artifact://` ref resolves inside opened run set, non-artifact refs are diagnostic/external | pass |
| `report validate-run` | `report.run_validation` | operating-contract validation gate | run-set id, included refs, workflow id, target id, target class | pass; unchanged |

## Run-set / target / workflow identity consistency

| Identity | Producer value | Consumer value | Result |
| --- | --- | --- | --- |
| run set | `run_set_identity_for_runs` and `EvidenceStore::run_set_resolution_map` | validation gate and resolver report | pass |
| workflow id | existing `target-operating-contract-fullset` v0.2.3 workflow id | validation gate | pass; profile split remains PR 3 |
| target id / class | CLI args and validation payload | operating-contract gate and emitted artifacts | pass |

## Controller / target-local execution locations

| Step | Expected location | Actual/generated location | Result |
| --- | --- | --- | --- |
| evidence ref resolution | controller | controller-only file/path resolution over opened run dirs | pass |
| target-local workload demand | target_local | not changed in PR 1 | deferred to PR 2 |

## Deployment/runtime discovery

| Runtime boundary | Install path | Invocation path | Env/PATH assumption | Preflight | Result |
| --- | --- | --- | --- | --- | --- |
| controller adc-lab | existing | existing CLI invocation | unchanged | `make verify` | pass |
| target-local runner | unchanged | unchanged | unchanged | not touched | not applicable |

## Forbidden fallback checks

- filename-order artifact selection: not introduced.
- mtime/latest/newest artifact inference: not introduced.
- stale prompt fallback: not introduced.
- raw co-presence as causal evidence: not introduced. Resolver checks explicit
  `artifact://` run ids and paths against the opened run set.

## Claim boundaries

- Workflow authority artifacts: unchanged; not target evidence.
- Validation artifacts: still required for controlled-governor full-set gate.
- Measurement artifacts: resolver proves refs are reachable, not that the
  measurements support broader claims.
- Blocked claims: `target.selection.production_ready` remains blocked even when
  matching measured validation removes the missing validation reason.

## Findings

| ID | Severity | Finding | Required fix |
| --- | --- | --- | --- |
| none | none | No workflow-contract blocker for PR 1. | none |

## Decision

submit
