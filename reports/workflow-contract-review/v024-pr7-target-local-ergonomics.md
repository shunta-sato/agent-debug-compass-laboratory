# Workflow Contract Review

## Scope

- PR / branch: `codex/v024-pr7-target-local-ergonomics`.
- Workflow surfaces:
  - `workflow.collect_plan` payload for SSH target-local execution guidance.
  - Generated collect-plan Agent instructions.
  - SSH target runner failure diagnostics used before target-local collection.
- Generated artifacts:
  - `workflow.collect_plan`
  - generated Markdown collect-plan instructions
  - `schemas/generated/lab.workflow.collect_plan.v2.schema.json`

## Source-of-truth chain

| Stage | Artifact / command | Producer | Consumer | Notes |
| --- | --- | --- | --- | --- |
| workflow authority | `workflow.recommendation` | `adc-lab workflow recommend` | `adc-lab collect plan` / Agent | unchanged; not measurement evidence |
| executable handoff | `workflow.collect_plan` | `adc-lab collect plan` | Agent/operator | now carries `target_local_execution_guide` for SSH target-local steps |
| validation | `report.run_validation` | `adc-lab report validate-run` | `report operating-contract --validation` | unchanged; same run-set identity requirements |
| downstream reports | operating contract, suitability, constraints | existing collect-plan steps | Agent/operator | unchanged claim gates |

## Generated argv replay

| Step | Execution location | argv | Required env | Expected artifact | Stop/continue |
| --- | --- | --- | --- | --- | --- |
| `prepare_target_local_workload_plan_dir` | `target_local` | `["mkdir","-p","adc-lab-target-local-workload-<run>/inputs"]` | guide prepends `$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin` before target-local argv execution | target-local inputs directory | refusal stops workload demand |
| `workload_demand` | `target_local` | `["adc-lab","workload","run","--target","local",...]` | same guide; preserve argv order and quote remote args independently | target-local workload demand | refused/unknown cannot support suitability |
| `governor_sweep_*` | `target_local` for SSH controller workflows | existing argv-array governor sweep steps with `--target local` | same target-local guide | governor control/load/validation artifacts | non-measured evidence preserved but cannot support full-set claims |

## Producer/consumer consistency

| Producer | Artifact | Consumer | Required identity match | Result |
| --- | --- | --- | --- | --- |
| collect plan | `target_local_execution_guide` | generated instructions / Agent | `applies_to_execution_location = target_local` and matching working directory policy | pass |
| target-local workload step | workload demand profile | retrieval and suitability steps | explicit run path from collect plan, not filename order | unchanged pass |
| validation step | `report.run_validation` | operating contract | run set / workflow / target identity | unchanged pass |

## Run-set / target / workflow identity consistency

| Identity | Producer value | Consumer value | Result |
| --- | --- | --- | --- |
| run set | collect plan primary + included runs | validate-run / operating-contract argv | unchanged pass |
| workflow id | `target-operating-contract-fullset.v0.2.3` | collect plan / validation / downstream reports | unchanged pass |
| target id / class | CLI inputs | collect plan / validation / reports | unchanged pass |

## Controller / target-local execution locations

| Step | Expected location | Actual/generated location | Result |
| --- | --- | --- | --- |
| target-local workload setup | target-local | `execution_location = target_local` plus guide | pass |
| workload demand | target-local | `execution_location = target_local`, `--target local` | pass |
| scp staging/retrieval | operator handoff | `execution_location = operator_handoff` | pass |
| reporting / suitability / constraints | controller | `execution_location = controller` | unchanged pass |

## Deployment/runtime discovery

| Runtime boundary | Install path | Invocation path | Env/PATH assumption | Preflight | Result |
| --- | --- | --- | --- | --- | --- |
| target-local adc-lab | user/release install | target-local argv steps | `PATH` prepends `$HOME/.local/bin` before target-local execution | collect-plan guide + Markdown | pass |
| SSH target runner | `/home/<target-user>/.local/bin/adc-lab-target` or allowed absolute path | controller-side SSH runner version/inventory/load checks | `ADC_LAB_TARGET_RUNNER` when default lookup is missing from non-interactive SSH PATH | failure categories: `command_not_found`, `path_missing`, `permission_denied` | pass |
| privileged helper | existing typed local helper only | control refusal / target-local helper path | no remote privileged SSH apply/restore | `helper_unavailable` diagnostic category only preserves boundary | pass |
| version set | run validation | downstream claims | skew blocks full-set measured claims | `version_skew` diagnostic category references validation gaps | pass |

## Forbidden fallback checks

- filename-order artifact selection: no new usage; tests assert generated instructions lack filename-order patterns.
- mtime/latest/newest artifact inference: no new usage.
- stale prompt fallback: generated instructions still prohibit static prompt/harness fallback.
- raw co-presence as causal evidence: no new claim path; guide is handoff metadata, not measurement evidence.

## Claim boundaries

- Workflow authority artifacts: `workflow.recommendation` and `workflow.collect_plan` remain not target measurement evidence.
- Validation artifacts: unchanged; measured validation still required for controlled-governor full-set claims.
- Measurement artifacts: target-local guide does not create measurement evidence.
- Blocked claims: command-not-found, path-missing, permission-denied, helper-unavailable, and version-skew states are diagnostic boundaries, not success states.

## Findings

| ID | Severity | Finding | Required fix |
| --- | --- | --- | --- |
| none | n/a | No workflow-contract findings. | n/a |

## Decision

submit
