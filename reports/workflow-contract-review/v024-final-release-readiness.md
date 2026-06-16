# Workflow Contract Review

## Scope

- PR / branch: `codex/v024-final-readiness`.
- Workflow surfaces:
  - v0.2.4 release-readiness handoff.
  - target55 rerun readiness for generated workflow authority prompts and
    collect plans.
  - final artifact review criteria for v0.2.4 target characterization packs.
- Generated artifacts:
  - `workflow.recommendation`
  - `workflow.collect_plan`
  - `report.run_validation`
  - `report.operating_contract`
  - `report.suitability`
  - `report.constraints`
  - `report.constraints_check`

## Source-of-truth chain

| Stage | Artifact / command | Producer | Consumer | Notes |
| --- | --- | --- | --- | --- |
| workflow authority | `workflow.recommendation` | `adc-lab workflow recommend` | `adc-lab agent instructions` / `adc-lab collect plan` | authority artifact only, not target measurement evidence |
| executable handoff | `workflow.collect_plan` | `adc-lab collect plan` | Agent/operator | argv-array steps, explicit expected artifacts, target-local execution guide |
| validation | `report.run_validation` | `adc-lab report validate-run` | `report operating-contract --validation` | run-set, workflow, target id, and target class must match |
| operating contract | `report.operating_contract` and `report.evidence_ref_resolution` | `adc-lab report operating-contract` | `decide suitability` / reviewer | controlled-governor claims require matching validation |
| suitability | `report.suitability` | `adc-lab decide suitability` | `constraints generate` | required unknown/fail dimensions keep `selection_ready=false` |
| constraints | `report.constraints` and generated markdown | `adc-lab constraints generate` | `constraints self-check` | blocked claims are negative/explanatory constraints |
| self-check | `report.constraints_check` | `adc-lab constraints self-check --out` | reviewer / handoff archive | persisted at explicit path; no stdout scraping required |

## Generated argv replay

| Step | Execution location | argv | Required env | Expected artifact | Stop/continue |
| --- | --- | --- | --- | --- | --- |
| target55 smoke plan generation | controller | `adc-lab collect plan --goal target-operating-contract-smoke --target ssh://target55 --target-id target55 --target-class raspberry_pi_4 --expected-governors ondemand,performance ...` | none beyond installed controller `adc-lab` | `workflow.collect_plan`, generated instructions | generation failure blocks rerun |
| target55 characterization-full plan generation | controller | `adc-lab collect plan --goal target-characterization-full --target ssh://target55 --target-id target55 --target-class raspberry_pi_4 --expected-governors ondemand,performance [--network-endpoint host:port] ...` | none beyond installed controller `adc-lab`; endpoint optional | `workflow.collect_plan`, generated instructions | generation failure blocks rerun |
| target-local execution steps | target-local via operator handoff | argv arrays from `workflow.collect_plan` | `PATH` prepends `$HOME/.local/bin`; controller may set `ADC_LAB_TARGET_RUNNER=/home/<target-user>/.local/bin/adc-lab-target` | target-local workload/control artifacts | refused/contaminated evidence preserved but cannot support full-set claims |
| final constraints self-check | controller | `adc-lab constraints self-check --constraints ... --path ... --out ... --json` | none | `report.constraints_check` | failure blocks final handoff completion, not measurement validity |

## Producer/consumer consistency

| Producer | Artifact | Consumer | Required identity match | Result |
| --- | --- | --- | --- | --- |
| `collect plan` | `workflow.collect_plan` | Agent/operator and `report validate-run --collect-plan` | workflow id, goal/effective profile, target id, target class | pass by generated plan contract |
| `report validate-run` | `report.run_validation` | `report operating-contract --validation` | subject run set, included runs, workflow id, target id, target class | pass by PR3/PR5 gates |
| `report operating-contract` | operating contract and resolution report | `decide suitability` / reviewer | opened run set and evidence refs must resolve or be classified | pass by PR1/PR6 gates |
| `decide suitability` | `report.suitability` | `constraints generate` | workload/target/policy identity | pass by PR6 gates |
| `constraints generate` | `report.constraints` and markdown | `constraints self-check` | constraints id and blocked claim catalog terms | pass by PR8 gates |

## Run-set / target / workflow identity consistency

| Identity | Producer value | Consumer value | Result |
| --- | --- | --- | --- |
| run set | collect plan primary + deterministic included runs | validate-run / operating-contract / evidence-ref resolver | pass |
| workflow id | `target-operating-contract-fullset.v0.2.3` family plus `effective_profile` | recommendation, collect plan, validation, operating contract | pass |
| target id / class | `target55` / `raspberry_pi_4` for target55 rerun | all generated workflow and report commands | pass |

## Controller / Target-Local Execution Locations

| Step | Expected location | Actual/generated location | Result |
| --- | --- | --- | --- |
| workflow recommendation / collect plan | controller | controller | pass |
| governor sweep and workload demand for SSH target | target-local | `execution_location = target_local`, `--target local` inside target-local argv | pass by PR2/PR7 gates |
| staging/retrieval | operator handoff | `execution_location = operator_handoff` | pass |
| reporting / suitability / constraints | controller | controller | pass |

## Deployment/runtime discovery

| Runtime boundary | Install path | Invocation path | Env/PATH assumption | Preflight | Result |
| --- | --- | --- | --- | --- | --- |
| controller `adc-lab` | release or local build | controller shell | `adc-lab` on controller PATH | `adc-lab version` / release manifest before target55 rerun | ready |
| target-local `adc-lab` | target user install | target-local argv steps | `$HOME/.local/bin` must be available in target-local execution environment | collect-plan target-local guide | ready |
| SSH target runner | `/home/<target-user>/.local/bin/adc-lab-target` | controller SSH runner calls | set `ADC_LAB_TARGET_RUNNER` when non-interactive SSH PATH misses installer default | runner version diagnostic | ready |
| privileged helper | existing typed helper only | target-local control steps | no remote privileged shell or arbitrary helper path | control refusal / helper diagnostics | unchanged safe boundary |

## Forbidden fallback checks

- filename-order artifact selection: blocked; workflow uses explicit refs,
  paths, digests, and expected artifact paths.
- mtime/latest/newest artifact inference: blocked by docs guard and generated
  instructions.
- stale prompt fallback: blocked; generated instructions require installed
  workflow surfaces and stop on capability/version mismatch.
- raw co-presence as causal evidence: blocked; validation and downstream
  reports require typed run-set identity and evidence refs.

## Claim boundaries

- Workflow authority artifacts: not measurement evidence.
- Validation artifacts: controlled-governor full-set validation evidence only,
  not production readiness or target selection.
- Measurement artifacts: bounded by measured profile, duration, endpoint,
  policy, and target-local execution result.
- Blocked claims: production readiness, Pi4/Pi5 selection, 24h sustained
  safety, real application performance, broad network/storage/latency
  guarantees, and composite coupling remain blocked unless matching evidence is
  produced by a later run.

## Release Gate Status

| Gate | Status | Evidence / next action |
| --- | --- | --- |
| `make verify` | pass | local final PR verification |
| `make schemas-check` | pass | local final PR verification |
| `make docs-smoke` | pass | local final PR verification |
| final workflow-contract review | pass | this report, decision `submit` |
| release binary / `SHA256SUMS` / manifest | deferred to release cut | run after this final readiness PR merges |
| target55 smoke profile run | deferred to operator/runtime gate | use generated workflow surfaces from the release binary |
| target55 characterization-full dry or bounded review | deferred to operator/runtime gate | run only if duration/risk budget permits |
| artifact review criteria | ready | criteria recorded in ExecPlan; target55 run must answer each item or mark blocked/deferred |

## Target55 Rerun Readiness

Minimum smoke-generation command after release binaries are installed:

```sh
adc-lab collect plan \
  --goal target-operating-contract-smoke \
  --target ssh://target55 \
  --target-id target55 \
  --target-class raspberry_pi_4 \
  --expected-governors ondemand,performance \
  --run-dir lab/runs/LAB-RUN-target55-v024-smoke \
  --out lab/runs/LAB-RUN-target55-v024-smoke/workflows/collect_plan.v2.json \
  --agent-instructions-out lab/runs/LAB-RUN-target55-v024-smoke/workflows/collect_plan.md \
  --json
```

For SSH runner discovery, set this when the non-interactive SSH PATH cannot see
the release-installed runner:

```sh
export ADC_LAB_TARGET_RUNNER=/home/<target-user>/.local/bin/adc-lab-target
```

Do not run a static prompt or hand-written shell harness when generated
workflow surfaces are available.

## Findings

| ID | Severity | Finding | Required fix |
| --- | --- | --- | --- |
| none | n/a | No workflow-contract findings. | n/a |

## Decision

submit
