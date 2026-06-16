# Workflow Contract Review

## Scope

- PR / branch: `codex/v0241-governor-sweep-retrieval`
- Workflow surfaces:
  - `workflow.collect_plan` for SSH targets
  - generated collect-plan markdown instructions
  - `report validate-run --include-run ...`
  - `report operating-contract --include-run ... --validation ...`
- Generated artifacts:
  - `workflows/collect_plan.v2.json`
  - `workflows/collect_plan.md`
  - `reports/run_validation.v2.json`
  - `reports/target_operating_contract.v2.json`

## Source-of-truth chain

| Stage | Artifact / command | Producer | Consumer | Notes |
| --- | --- | --- | --- | --- |
| workflow authority | `workflow.recommendation` | `adc-lab workflow recommend` | `adc-lab collect plan` / Agent | Authority only; not measurement evidence. |
| executable handoff | `workflow.collect_plan` | `adc-lab collect plan` | Agent/operator | Emits argv-array steps only. |
| target-local governor run | `adc-lab-target-local-<run_id>` | `governor_sweep_prepare` / `approve` / `run` target-local steps | retrieval handoff | Target-local evidence stays on target until explicit retrieval. |
| included governor run | `<primary>/included/target-local-governor-sweep` | `retrieve_target_local_governor_sweep` | `report validate-run`, `report operating-contract` | New v0.2.4.1 producer for the generated include-run consumer path. |
| validation | `reports/run_validation.v2.json` | `report validate-run` | `report operating-contract` | Full-set claims remain blocked/insufficient unless validation is measured. |
| operating contract | `reports/target_operating_contract.v2.json` | `report operating-contract` | suitability / constraints | Consumes validation and the same run set. |

## Generated argv replay

| Step | Execution location | argv | Required env | Expected artifact | Stop/continue |
| --- | --- | --- | --- | --- | --- |
| `governor_sweep_prepare` | `target_local` | `adc-lab control governor-sweep prepare --target local ... --run-dir adc-lab-target-local-<run_id>` | target-local PATH / runner guide | `control.governor_sweep_policy` request | Refusal/contamination stops control claim path. |
| `governor_sweep_approve` | `target_local` | `adc-lab control governor-sweep approve --request ... --out ...` | target-local PATH / runner guide | approved `control.governor_sweep_policy` | Refusal/contamination stops control claim path. |
| `governor_sweep_run` | `target_local` | `adc-lab control governor-sweep run --target local ... --restore-after-each --run-dir adc-lab-target-local-<run_id>` | target-local PATH / helper approval | target-local validation/control/load/restore artifacts | Non-measured evidence is preserved. |
| `prepare_target_local_governor_retrieval_parent` | `operator_handoff` | `mkdir -p <primary>/included` | controller filesystem | included parent | Handoff only, not target evidence. |
| `reset_target_local_governor_retrieval_destination` | `operator_handoff` | `rm -rf <primary>/included/target-local-governor-sweep` | controller filesystem | none | Deletion is scoped to deterministic destination. |
| `retrieve_target_local_governor_sweep` | `operator_handoff` | `scp -r <endpoint>:adc-lab-target-local-<run_id> <primary>/included/target-local-governor-sweep` | SSH/scp access | retrieved include-run directory | Handoff failure blocks downstream full-set validation. |
| `run_validation` | `controller` | `adc-lab report validate-run --run <primary> --include-run <primary>/included/target-local-governor-sweep ...` | controller adc-lab | `report.run_validation` and `GAPS.md` | Insufficient/unknown remains a claim boundary. |
| `operating_contract` | `controller` | `adc-lab report operating-contract --run <primary> --include-run <same> --validation <validation> --strict-fullset` | controller adc-lab | `report.operating_contract` | Strict mode fails closed for missing/non-measured validation. |

## Producer/consumer consistency

| Producer | Artifact | Consumer | Required identity match | Result |
| --- | --- | --- | --- | --- |
| `retrieve_target_local_governor_sweep` | `<primary>/included/target-local-governor-sweep` | `report validate-run --include-run` | exact path match | pass |
| `retrieve_target_local_governor_sweep` | `<primary>/included/target-local-governor-sweep` | `report operating-contract --include-run` | exact path match | pass |
| `report validate-run` | `reports/run_validation.v2.json` | `report operating-contract --validation` | exact generated validation path | pass |
| `retrieve_target_local_workload_demand` | `<primary>/included/target-local-workload-demand/reports/workload_demand_profile.json` | `decide suitability --workload-demand` | exact path match | unchanged pass |
| `constraints self-check` | `reports/constraints_check.v2.json` | final handoff | explicit `--out` path | unchanged pass |

## Run-set / target / workflow identity consistency

| Identity | Producer value | Consumer value | Result |
| --- | --- | --- | --- |
| run set | `<primary>` + `<primary>/included/target-local-governor-sweep` | validation and operating contract use the same generated set | pass |
| workflow id | `target-operating-contract-fullset.v0.2.3` | collect plan and validation refs | unchanged pass |
| target id / class | CLI inputs | collect plan, validation, operating contract | unchanged pass |

## Controller / target-local execution locations

| Step | Expected location | Actual/generated location | Result |
| --- | --- | --- | --- |
| governor prepare/approve/run | target-local for SSH targets | `target_local` with `--target local` | pass |
| governor retrieval prepare/reset/scp | controller operator handoff | `operator_handoff` | pass |
| run validation / operating contract | controller | `controller` | pass |
| local target collect plan | no SSH-only retrieval | tests assert absence | pass |

## Deployment/runtime discovery

| Runtime boundary | Install path | Invocation path | Env/PATH assumption | Preflight | Result |
| --- | --- | --- | --- | --- | --- |
| target-local adc-lab | target release install, normally `~/.local/bin` | target-local guide | `$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin` | unchanged | pass |
| controller retrieval | operator host has `scp` and SSH access | generated argv-array `scp -r` | no filename discovery | explicit source/destination | pass |
| privileged helper | existing governor sweep path | unchanged | approval/helper boundaries unchanged | unchanged | pass |

## Forbidden fallback checks

- filename-order artifact selection: none added; retrieval source and destination are deterministic argv entries.
- mtime/latest/newest artifact inference: none added.
- stale prompt fallback: none added; generated plan remains the executable handoff contract.
- raw co-presence as causal evidence: no relaxation. Retrieval provides the directory consumed by validation, but `report.run_validation` still evaluates typed control/load/restore evidence inside it.

## Claim boundaries

- Workflow authority artifacts: `workflow.recommendation` and
  `workflow.collect_plan` remain not measurement evidence.
- Validation artifacts: `report.run_validation` remains the full-set gate.
- Measurement artifacts: target-local governor artifacts are copied unchanged;
  retrieval does not upgrade their status.
- Blocked claims: production readiness, Pi4/Pi5 selection, 24h sustained safety,
  and real workload performance remain blocked unless future evidence supports
  them.

## Findings

| ID | Severity | Finding | Required fix |
| --- | --- | --- | --- |
| none | n/a | No unresolved workflow-contract findings. | n/a |

## Decision

submit
