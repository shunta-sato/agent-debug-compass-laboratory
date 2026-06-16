# Workflow Contract Review

## Scope

- PR / branch: `codex/v024-pr5-pressure-network`
- Workflow surfaces:
  - `adc-lab collect plan --goal target-characterization-full`
  - optional `adc-lab collect plan --network-endpoint <host:port>`
  - generated `workflow.collect_plan` markdown instructions
- Generated artifacts:
  - `workflow.collect_plan`
  - pressure / composite artifacts produced by generated argv steps

## Source-of-truth chain

| Stage | Artifact / command | Producer | Consumer | Notes |
| --- | --- | --- | --- | --- |
| recommendation | `workflow.recommendation` | `workflow recommend` | `collect plan`, operator/Agent | Authority only; not target measurement evidence. |
| plan | `workflow.collect_plan` | `collect plan` | Agent/operator execution | Adds pressure/composite argv steps for `target-characterization-full`. |
| pressure | `pressure` / `composite` artifacts | generated `pressure run` / `pressure composite` argv | `report validate-run`, `report operating-contract` | Existing artifact schemas and conservative predicates are reused. |
| validation | `report.run_validation` | generated plan step | `report operating-contract --validation` | Existing typed refs and run-set identity are preserved. |
| contract | `report.operating_contract` | generated plan step | suitability | PR5 does not relax validation or claim gates. |

## Generated argv replay

| Step | Execution location | argv | Required env | Expected artifact | Stop/continue |
| --- | --- | --- | --- | --- | --- |
| `pressure_latency_jitter` | controller | `["adc-lab","pressure","run","--target",target,"--kind","latency_jitter","--duration","5s","--workers","1","--run-dir",run_dir,"--json"]` | controller `adc-lab`; target transport from `--target` | `pressure/latency_jitter.*.v2.json` | continue on measured/measured_partial/insufficient; stop on refused/contaminated |
| `pressure_observer_pressure` | controller | same shape with `--kind observer_pressure` | same | `pressure/observer_pressure.*.v2.json` | observer pressure is explicit evidence, not zero-overhead proof |
| `pressure_memory_pressure` | controller | same shape with `--kind memory_pressure` | same | `pressure/memory_pressure.*.v2.json` | memory effect must be observed before coupling can advance |
| `pressure_storage_io` | controller | same shape with `--kind storage_io` | same | `pressure/storage_io.*.v2.json` | bounded storage probe only |
| `pressure_cpu_pressure` | controller | same shape with `--kind cpu_pressure --abort-temp-c 75` | same | `pressure/cpu_pressure.*.v2.json` | bounded by duration, worker count, and thermal abort |
| `pressure_thermal_pressure` | controller | same shape with `--kind thermal_pressure --abort-temp-c 75` | same | `pressure/thermal_pressure.*.v2.json` | not 24h sustained safety evidence |
| `pressure_network_counter_only` | controller | same shape with `--kind network_io` and no endpoint | same | `pressure/network_io.*.v2.json` | counter-only evidence cannot support bounded transfer claims |
| `pressure_network_endpoint_backed` | controller | same shape with `--kind network_io --network-endpoint <host:port> --network-bytes 1048576` | explicit receiver endpoint | `pressure/network_io.*.v2.json` | emitted only when configured; still insufficient unless bounded transfer completes |
| `composite_memory_storage_jitter` | controller | `["adc-lab","pressure","composite","--target",target,"--scenario","memory_storage_jitter","--duration","5s","--workers","1","--run-dir",run_dir,"--json"]` | controller `adc-lab`; target transport from `--target` | `composite/memory_storage_jitter.*.v2.json` | coupling claims remain blocked unless composite artifact is measured |

## Producer/Consumer Consistency

| Producer | Artifact | Consumer | Required identity match | Result |
| --- | --- | --- | --- | --- |
| `collect plan` | `workflow.collect_plan` | operator/Agent, `report validate-run --collect-plan` | explicit path generated in plan | pass |
| pressure steps | `pressure/<kind>.<result_id>.v2.json` | rules engine, operating contract | same planned run dir / included run set | pass |
| composite step | `composite/memory_storage_jitter.<result_id>.v2.json` | rules engine, operating contract | same planned run dir / included run set | pass |
| `report validate-run` | `report.run_validation` | `report operating-contract --validation` | existing run-set/target/profile checks | unchanged/pass |

## Run-Set / Target / Workflow Identity Consistency

| Identity | Producer value | Consumer value | Result |
| --- | --- | --- | --- |
| run set | `planned_run_dir` in `workflow.collect_plan` | every pressure/composite argv `--run-dir`; validation `--run` | pass |
| workflow id | `target-operating-contract-fullset.v0.2.3` | validation/contract chain | unchanged/pass |
| target id / class | `--target-id`, `--target-class` inputs | recommendation, collect plan, validation, operating contract | unchanged/pass |
| endpoint | optional `collect plan --network-endpoint` | endpoint-backed network argv only | pass; counter-only step never receives endpoint |

## Controller / Target-Local Execution Locations

| Step | Expected location | Actual/generated location | Result |
| --- | --- | --- | --- |
| pressure/composite steps | controller orchestrated typed CLI command | `execution_location: controller` | pass |
| SSH governor sweep | target-local, existing behavior | unchanged | pass |
| workload demand | controller or target-local depending on target transport | unchanged | pass |

## Forbidden Fallback Checks

- filename-order artifact selection: absent from generated instructions and tests.
- mtime/latest/newest artifact inference: absent.
- endpoint-backed network: requires explicit `--network-endpoint`; no rx/tx counter inference.
- composite coupling: requires measured composite evidence; artifact presence alone is not enough.

## Claim Boundaries

- Workflow authority artifacts remain not measurement evidence.
- Counter-only `network_io` is expected to be not_applicable or insufficient for
  bounded transfer claims.
- Endpoint-backed network remains insufficient unless the bounded transfer
  completes and records generated bytes.
- Memory/storage coupling claims remain blocked unless memory pressure effect and
  measured `memory_storage_jitter` composite evidence are both present.
- Thermal pressure and 300s load evidence remain bounded characterization, not
  24h sustained safety evidence.

## Findings

| ID | Severity | Finding | Required fix |
| --- | --- | --- | --- |
| none | n/a | PR5 uses existing typed pressure primitives and preserves conservative rules for endpoint-backed network and composite coupling claims. | n/a |

## Decision

submit pending verification
