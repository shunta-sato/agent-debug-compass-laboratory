# Workflow Contract Review

## Scope

- PR / branch: `codex/v024-pr2-target-local-workload`.
- Workflow surfaces:
  - `workflow.collect_plan` SSH target-local workload demand steps.
  - Generated collect-plan Agent instructions for target-local workload handoff.
  - `decide suitability` consumption of the retrieved workload demand path.
  - Suitability CPU / memory claim gates for clean versus degraded workload
    demand.

## Source-of-Truth Chain

| Stage | Artifact / command | Producer | Consumer | Notes |
| --- | --- | --- | --- | --- |
| collect plan | `workflow.collect_plan` | `adc-lab collect plan --target ssh://...` | Agent / operator | No filename-order discovery. |
| workload plan staging | `<primary>/inputs/workload_run_plan.yaml` -> `<target-run>/inputs/workload_run_plan.yaml` | explicit `mkdir -p` + `scp` argv steps | target-local workload demand | No prose-only staging or hand-written harness. |
| target-local workload demand | `adc-lab workload run --target local --execution-mode target-local ...` | target-local argv step | retrieval handoff | Runs on target-local adc-lab, not controller ssh workload mode. |
| retrieval preparation | `<primary>/included` + cleaned `<primary>/included/target-local-workload-demand` | explicit `mkdir -p` + scoped `rm -rf` argv steps | retrieval handoff | Rerun layout is deterministic; cleanup is limited to the retrieved workload path. |
| retrieval | `scp -r <endpoint>:adc-lab-target-local-workload-<run> <primary>/included/target-local-workload-demand` | operator handoff | suitability step | Retrieval is handoff plumbing, not target measurement evidence. |
| suitability | `adc-lab decide suitability --workload-demand <primary>/included/target-local-workload-demand/reports/workload_demand_profile.json` | controller argv step | constraints | Consumes explicit path from collect plan, not discovered files. |

## Generated Argv Replay

| Step | Execution location | argv | Required env | Expected artifact | Stop/continue |
| --- | --- | --- | --- | --- | --- |
| `prepare_target_local_workload_plan_dir` | `target_local` | `mkdir -p adc-lab-target-local-workload-<run>/inputs` | target-local working directory convention | target-local inputs directory | Refusal blocks target-local workload demand. |
| `stage_target_local_workload_plan` | `operator_handoff` | `scp <primary>/inputs/workload_run_plan.yaml <endpoint>:adc-lab-target-local-workload-<run>/inputs/workload_run_plan.yaml` | SSH/scp access to target endpoint | target-local workload plan path | Refusal blocks target-local workload demand. |
| `workload_demand` | `target_local` | `adc-lab workload run --target local --execution-mode target-local --plan adc-lab-target-local-workload-<run>/inputs/workload_run_plan.yaml --run-dir adc-lab-target-local-workload-<run> --json` | target-local adc-lab on PATH; collect instructions include `~/.local/bin` PATH guidance | `workload` / target-local `reports/workload_demand_profile.json` | Refused or unknown demand cannot support suitability. |
| `prepare_target_local_workload_retrieval_parent` | `operator_handoff` | `mkdir -p <primary>/included` | controller filesystem access to the primary run dir | included-run parent directory | Refusal blocks retrieval only. |
| `reset_target_local_workload_retrieval_destination` | `operator_handoff` | `rm -rf <primary>/included/target-local-workload-demand` | controller filesystem access to the primary run dir | no output artifact; deletion target is fixed by collect plan | Refusal blocks retrieval; delete is limited to the deterministic included workload path. |
| `retrieve_target_local_workload_demand` | `operator_handoff` | `scp -r <endpoint>:adc-lab-target-local-workload-<run> <primary>/included/target-local-workload-demand` | SSH/scp access to target endpoint | retrieved `reports/workload_demand_profile.json` | Handoff failure blocks downstream suitability, not prior measurement validity. |
| `suitability` | `controller` | `adc-lab decide suitability --workload-demand <primary>/included/target-local-workload-demand/reports/workload_demand_profile.json ...` | controller adc-lab | `report.suitability` | Unknown/degraded dimensions keep `selection_ready=false`. |

## Producer / Consumer Consistency

| Producer | Artifact | Consumer | Required identity / path match | Result |
| --- | --- | --- | --- | --- |
| controller workload plan input | `<primary>/inputs/workload_run_plan.yaml` | `stage_target_local_workload_plan` | exact controller input path is in staging argv | pass |
| `stage_target_local_workload_plan` | `<target-run>/inputs/workload_run_plan.yaml` | `workload_demand` | staged target path equals workload `--plan` path | pass |
| `workload_demand` target-local step | target-local workload run dir | retrieval step | target-local source run dir is deterministic from collect-plan run id | pass |
| retrieval preparation steps | `<primary>/included` and cleaned `<primary>/included/target-local-workload-demand` | retrieval step | parent exists and destination cannot pre-exist with layout-changing semantics | pass |
| retrieval step | `<primary>/included/target-local-workload-demand/reports/workload_demand_profile.json` | `decide suitability` | exact retrieved path appears in `--workload-demand` argv | pass |
| `workload run` refused/degraded output | `lab.workload_demand_profile.v1` | suitability CPU / memory decision | degraded profile cannot produce `meet` or `selection_ready=true` | pass |

## Run-Set / Target / Workflow Identity Consistency

| Identity | Producer value | Consumer value | Result |
| --- | --- | --- | --- |
| workflow id | `target-operating-contract-fullset.v0.2.3` | unchanged collect-plan payload | pass |
| target id | collect-plan `--target-id` and workload argv `--target-id` | workload demand profile and suitability target | pass |
| run path | target-local run dir and retrieved included path | suitability explicit `--workload-demand` | pass |

## Controller / Target-Local Execution Locations

| Step | Expected location | Actual/generated location | Result |
| --- | --- | --- | --- |
| prepare target workload plan dir | target-local | `execution_location = target_local`, `mkdir -p`, `requires_target_local = true` | pass |
| stage workload plan | operator handoff | `execution_location = operator_handoff`, `scp` argv array | pass |
| workload demand | target-local | `execution_location = target_local`, `--target local`, `requires_target_local = true` | pass |
| prepare retrieval parent | operator handoff | `execution_location = operator_handoff`, `mkdir -p <primary>/included` | pass |
| reset retrieval destination | operator handoff | `execution_location = operator_handoff`, `rm -rf` limited to `<primary>/included/target-local-workload-demand` | pass |
| retrieval | operator handoff | `execution_location = operator_handoff`, `scp` argv array | pass |
| suitability | controller | `execution_location = controller`, consumes retrieved path | pass |

## Deployment / Runtime Discovery

| Runtime boundary | Install path | Invocation path | Env/PATH assumption | Preflight | Result |
| --- | --- | --- | --- | --- | --- |
| target-local adc-lab | existing release/user install | target-local argv step | collect instructions prepend `~/.local/bin` to PATH for target-local steps | existing generated instruction tests | pass |
| SSH retrieval | operator SSH/scp access | `scp -r` argv | endpoint comes from validated `ssh://` target spec | collect-plan CLI tests | pass |

## Forbidden Fallback Checks

- filename-order artifact selection: not introduced.
- mtime/latest/newest artifact inference: not introduced.
- stale prompt fallback: not introduced.
- controller-side `workload run --target ssh://...` as suitability evidence:
  removed from SSH collect plan.
- raw co-presence as causal evidence: not introduced; suitability consumes the
  explicit retrieved workload demand path.

## Claim Boundaries

- `workflow.collect_plan` is still a handoff contract, not measurement evidence.
- Retrieval by `scp` is handoff plumbing, not target evidence.
- Synthetic workload demand is workload characterization evidence, not real
  application performance evidence.
- Degraded or refused workload demand is preserved as evidence but cannot
  support CPU/memory `meet` or `selection_ready=true`.
- Production readiness remains blocked by downstream operating-contract rules.

## Findings

| ID | Severity | Finding | Required fix |
| --- | --- | --- | --- |
| none | none | No workflow-contract blocker for PR 2. | none |

## Decision

submit
