# Workflow Contract Review

## Scope

- PR / branch: `codex/v024-pr4-deep-cpu-thermal`
- Workflow surfaces:
  - `adc-lab workflow recommend --goal target-characterization-full`
  - `adc-lab collect plan --goal target-characterization-full`
  - generated `workflow.collect_plan` markdown instructions
- Generated artifacts:
  - `workflow.recommendation`
  - `workflow.collect_plan`

## Source-of-truth chain

| Stage | Artifact / command | Producer | Consumer | Notes |
| --- | --- | --- | --- | --- |
| recommendation | `workflow.recommendation` | `workflow recommend` | `collect plan`, operator/Agent | Authority only; not target measurement evidence. |
| plan | `workflow.collect_plan` | `collect plan` | Agent/operator execution | Adds CPU/thermal characterization argv steps for `target-characterization-full`. |
| validation | `report.run_validation` | later generated plan step | `report operating-contract --validation` | Existing typed refs and run-set identity are preserved. |
| contract | `report.operating_contract` | generated plan step | suitability | PR4 does not relax validation or claim gates. |
| suitability / constraints | generated downstream steps | generated plan steps | Agent handoff | Existing explicit paths remain unchanged. |

## Generated argv replay

| Step | Execution location | argv | Required env | Expected artifact | Stop/continue |
| --- | --- | --- | --- | --- | --- |
| `observe_baseline_60s` | controller | `["adc-lab","observe","--target",target,"--duration","60s","--sample-interval","1s","--run-dir",run_dir,"--json"]` | installed controller `adc-lab`; target transport from `--target` | `observation` | continue on measured/insufficient; stop on refused/unknown |
| `observe_baseline_300s` | controller | `["adc-lab","observe","--target",target,"--duration","300s","--sample-interval","1s","--run-dir",run_dir,"--json"]` | installed controller `adc-lab`; target transport from `--target` | `observation` | continue on measured/insufficient; stop on refused/unknown |
| `cpu_ladder_1_worker_60s` | controller | `["adc-lab","load","cpu","--target",target,"--workers","1","--duration","60s","--abort-temp-c","75","--run-dir",run_dir,"--json"]` | load support and thermal surface when available | `load` | continue on measured/insufficient; stop on refused/contaminated |
| `cpu_ladder_2_worker_60s` | controller | same as above with `--workers 2` | load support and thermal surface when available | `load` | continue on measured/insufficient; stop on refused/contaminated |
| `cpu_ladder_4_worker_60s` | controller | same as above with `--workers 4` | load support and thermal surface when available | `load` | continue on measured/insufficient; stop on refused/contaminated |
| repeatability/cooldown | controller | three `--workers 4 --duration 60s --abort-temp-c 75` load steps with 60s observe cooldowns | same as ladder | `load` / `observation` | contaminated load cannot feed downstream claims |
| `sustained_bounded_load_300s` | controller | `["adc-lab","load","cpu","--target",target,"--workers","4","--duration","300s","--abort-temp-c","75","--run-dir",run_dir,"--json"]` | load support and thermal abort | `load` | claim gate is `sustained_300s_not_24h_safety` |
| `cooldown_after_sustained_load` | controller | `["adc-lab","observe","--target",target,"--duration","120s","--sample-interval","1s","--run-dir",run_dir,"--json"]` | installed controller `adc-lab`; target transport from `--target` | `observation` | cooldown context only |

## Producer/consumer consistency

| Producer | Artifact | Consumer | Required identity match | Result |
| --- | --- | --- | --- | --- |
| CPU/thermal steps | `load`, `observation` artifacts in the planned run dir | `report validate-run`, `report operating-contract` | same `--run`, `target_id`, workflow profile | pass |
| `collect plan` | `workflow.collect_plan` | `report validate-run --collect-plan` | explicit path generated in plan | pass |
| `report validate-run` | `report.run_validation` | `report operating-contract --validation` | existing run-set/target/profile checks | unchanged/pass |
| `constraints` | `report.constraints` and markdown | `constraints self-check` | explicit generated paths | unchanged/pass |

## Run-set / target / workflow identity consistency

| Identity | Producer value | Consumer value | Result |
| --- | --- | --- | --- |
| run set | `planned_run_dir` in `workflow.collect_plan` | every generated CPU/thermal argv `--run-dir`; validation `--run` | pass |
| workflow id | `target-operating-contract-fullset.v0.2.3` | validation/contract chain | unchanged/pass |
| target id / class | `--target-id`, `--target-class` inputs | recommendation, collect plan, validation, operating contract | unchanged/pass |

## Controller / Target-Local Execution Locations

| Step | Expected location | Actual/generated location | Result |
| --- | --- | --- | --- |
| CPU/thermal observe/load | controller orchestrated typed CLI command | `execution_location: controller` | pass |
| SSH governor sweep | target-local, existing PR2/3 behavior | unchanged | pass |
| workload demand | controller or target-local depending on target transport | unchanged | pass |

## Deployment/Runtime Discovery

| Runtime boundary | Install path | Invocation path | Env/PATH assumption | Preflight | Result |
| --- | --- | --- | --- | --- | --- |
| controller `adc-lab` | repository/release binary | generated argv starts with `adc-lab` | PATH or explicit operator invocation | `capability_check`, inventory, toolchain steps precede claims | pass |
| SSH target runner | release default may be `~/.local/bin/adc-lab-target` | existing SSH command path / `ADC_LAB_TARGET_RUNNER` guidance | non-interactive SSH PATH may omit `~/.local/bin` | unchanged from v0.2.3.1 diagnostics | pass |

## Forbidden Fallback Checks

- filename-order artifact selection: absent from generated instructions and tests.
- mtime/latest/newest artifact inference: absent.
- stale prompt fallback: unchanged prohibition from agent instructions.
- raw co-presence as causal evidence: unchanged validation chain; CPU/thermal primitive artifacts do not bypass `report.run_validation`.

## Claim Boundaries

- Workflow authority artifacts: recommendation and collect plan remain not measurement evidence.
- Validation artifacts: unchanged source-of-truth link before operating contract.
- Measurement artifacts: CPU/thermal load/observation steps are bounded evidence only.
- Blocked claims: `sustained_bounded_load_300s` carries `sustained_300s_not_24h_safety`; operating rules still require at least 900s plus thermal pressure effect before sustained thermal soak can even become provisional.

## Findings

| ID | Severity | Finding | Required fix |
| --- | --- | --- | --- |
| none | n/a | No unresolved workflow-contract findings. | n/a |

## Decision

submit
