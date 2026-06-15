# Workflow Contract Review

## Scope

- PR / branch: `codex/v024-phase0-characterization`
- Workflow surfaces: v0.2.4 ExecPlan and release workflow-contract review
  process.
- Generated artifacts: none in Phase 0. This report seeds the required
  review artifact family for later implementation PRs.

## Source-of-truth chain

| Stage | Artifact / command | Producer | Consumer | Notes |
| --- | --- | --- | --- | --- |
| Plan | `plans/20260615-v024-target-characterization-lab.md` | Phase 0 | v0.2.4 implementation PRs | Defines requirements, profile compatibility, evidence-ref categories, PR order, and release gate. |
| Review process | `reports/workflow-contract-review/<slug>.md` | Each implementation PR | Reviewers / quality gate | Required for Agent-facing workflow changes. |

## Generated argv replay

| Step | Execution location | argv | Required env | Expected artifact | Stop/continue |
| --- | --- | --- | --- | --- | --- |
| Phase 0 | repository | not applicable | not applicable | plan and review report only | Continue to PR 1 after baseline verification. |

## Producer/consumer consistency

| Producer | Artifact | Consumer | Required identity match | Result |
| --- | --- | --- | --- | --- |
| Phase 0 plan | v0.2.4 requirements / WBS | PR 1-8 implementations | PRs must trace changes to plan requirements and acceptance criteria. | pass |
| Future collect plan | `workflow.collect_plan` | `report validate-run`, `report operating-contract`, `decide suitability`, `constraints self-check` | Explicit run set, workflow id, target id/class, evidence refs, and persisted output paths. | required in later PRs |

## Run-set / target / workflow identity consistency

| Identity | Producer value | Consumer value | Result |
| --- | --- | --- | --- |
| run set | not produced in Phase 0 | not consumed in Phase 0 | not applicable |
| workflow id | profile compatibility decision deferred to implementation, with Phase 0 default Option C | future workflow registry / collect plan | pass as planning constraint |
| target id / class | not produced in Phase 0 | not consumed in Phase 0 | not applicable |

## Controller / target-local execution locations

| Step | Expected location | Actual/generated location | Result |
| --- | --- | --- | --- |
| Phase 0 planning | repository | repository | pass |
| Future SSH workload demand | target_local | not emitted in Phase 0 | required in PR 2 |

## Deployment/runtime discovery

| Runtime boundary | Install path | Invocation path | Env/PATH assumption | Preflight | Result |
| --- | --- | --- | --- | --- | --- |
| v0.2.4 Phase 0 | not applicable | not applicable | no target runtime execution | `origin/main` after PR #62 / `v0.2.3.1` tag verified by git fetch | pass |
| Future target-local steps | `~/.local/bin/adc-lab` and `~/.local/bin/adc-lab-target` expected by v0.2.3.1 guidance | generated target-local argv or safe renderer | PATH guidance / `ADC_LAB_TARGET_RUNNER` must be explicit | required in PR 2 | pending |

## Forbidden fallback checks

- filename-order artifact selection: pass, not introduced.
- mtime/latest/newest artifact inference: pass, not introduced.
- stale prompt fallback: pass, plan requires workflow surfaces and generated
  review reports.
- raw co-presence as causal evidence: pass, plan requires run-set identity and
  evidence-ref resolution.

## Claim boundaries

- Workflow authority artifacts: plan keeps them as authority / handoff, not
  target measurement evidence.
- Validation artifacts: plan keeps strict-fullset success distinct from
  production readiness.
- Measurement artifacts: plan requires stronger target-local workload,
  pressure, network, storage, latency, and composite evidence before reducing
  unknowns.
- Blocked claims: production readiness, Pi4/Pi5 selection, 24h sustained
  safety, real application performance, and full coupling stay blocked unless
  matching evidence exists.

## Findings

| ID | Severity | Finding | Required fix |
| --- | --- | --- | --- |
| none | none | No Phase 0 workflow-contract blocker. | none |

## Decision

submit
