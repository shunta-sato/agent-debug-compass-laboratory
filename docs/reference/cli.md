# CLI Reference

This page keeps the longer command examples out of the README while preserving a reproducible command trail for operators and agents.

Use `COMMANDS.md` as the repository command registry for build, test, lint, and verification commands. This page focuses on `adc-lab` user-facing command examples.

## Public Outputs And Artifact Kinds

Most report and probe outputs are `lab.artifact.v2` envelopes. The file name is
an operator-facing path; the stable contract is the envelope `kind`.

| Command surface | Default or example path | Artifact kind |
| --- | --- | --- |
| `workflow recommend --run-dir ...` | `workflows/recommendation.v2.json` | `workflow.recommendation` |
| `collect plan --out ...` | `workflows/collect_plan.v2.json` | `workflow.collect_plan` |
| `familiarize read-only`, `report pack`, `report operating-point` | `reports/run_report.v2.json` | `report.run` |
| `report validate-run` | `reports/run_validation.v2.json` | `report.run_validation` |
| `report operating-contract` | `reports/target_operating_contract.v2.json` | `report.operating_contract` |
| `report operating-contract` | `reports/evidence_ref_resolution.v2.json` | `report.evidence_ref_resolution` |
| `decide suitability --out ...` | `reports/suitability.v2.json` | `report.suitability` |
| `constraints generate --out ...` | `reports/constraints.v2.json` | `report.constraints` |
| `constraints check-candidate --json`, `constraints self-check --json` | stdout | `report.constraints_check` |
| `load cpu` | `load/cpu.<result_id>.v2.json` | `load` |
| `pressure run` | `pressure/<kind>.<result_id>.v2.json` | `pressure` |
| `pressure composite` | `composite/<scenario>.<result_id>.v2.json` | `composite` |
| `workload run` v2 sidecar | `workload/demand_profile.v2.json` | `workload` |

Compatibility wire artifacts that remain v1-shaped are generated schema
snapshots, not hand-maintained public report schemas. For example,
`reports/workload_demand_profile.json` remains `lab.workload_demand_profile.v1`
so existing workload policy input keeps working while v2 reports consume it
through typed readers.

## Local familiarization

```sh
adc-lab inventory --target local
adc-lab toolchain discover --target local
adc-lab observe --target local --duration 5s --signals cpu,freq,thermal,memory
adc-lab familiarize read-only --target local --duration 5s --signals cpu,freq,thermal,memory
```

These commands produce target inventory, toolchain inventory, passive
observation, run manifest, audit log, and `report.run` claim-boundary evidence.
They do not control privileged operating points.

## Tool qualification

```sh
adc-lab tool qualify-inventory \
  --inventory lab/runs/LAB-RUN-.../toolchain/toolchain_inventory.json

adc-lab tool qualify \
  --manifest examples/tools/linux_cpufreq_reader.yaml \
  --tool-version 0.1.0 \
  --tool-sha256 sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
  --output-schema examples/tools/linux_cpufreq_reader.output_schema.json \
  --dry-run-output examples/tools/linux_cpufreq_reader.dry_run.json \
  --manual-comparison examples/tools/linux_cpufreq_reader.manual_comparison.json \
  --static-safety-review examples/tools/linux_cpufreq_reader.static_safety_review.txt
```

No unqualified tool becomes evidence. Tool qualification records whether a tool can support a claim and where it cannot.

## Privilege provider status

```sh
adc-lab privilege provider-status --target local
```

Provider status is evidence about privilege-provider availability only. It is not permission to grant an Agent a root shell.

## Workflow authority handoff

For Target Operating Contract full-set work, ask adc-lab for the workflow
surface instead of reusing a static prompt or writing a shell harness.

```sh
adc-lab workflow recommend \
  --goal target-operating-contract-fullset \
  --target ssh://target55 \
  --target-id target55 \
  --target-class raspberry_pi_4 \
  --run-dir lab/runs/LAB-RUN-fullset \
  --json

adc-lab agent instructions \
  --goal target-operating-contract-fullset \
  --target ssh://target55 \
  --target-id target55 \
  --target-class raspberry_pi_4 \
  --format codex \
  --out lab/runs/LAB-RUN-fullset/workflows/codex_instructions.md \
  --json

adc-lab collect plan \
  --goal target-operating-contract-fullset \
  --target ssh://target55 \
  --target-id target55 \
  --target-class raspberry_pi_4 \
  --run-dir lab/runs/LAB-RUN-fullset \
  --out lab/runs/LAB-RUN-fullset/workflows/collect_plan.v2.json \
  --agent-instructions-out lab/runs/LAB-RUN-fullset/workflows/collect_plan.md \
  --json
```

`workflow.recommendation` and `workflow.collect_plan` are authority and handoff
artifacts. They are not target measurement evidence. The collect plan contains
argv arrays plus expected artifact kinds and continuation rules; execute those
argv entries as typed commands, and stop rather than inventing missing workflow
surfaces.

## Privileged operating-point workflow

```sh
adc-lab control plan --target local cpu.governor --set performance

adc-lab control approve \
  --plan lab/runs/LAB-RUN-.../plans/PLAN-....json \
  --approved-by operator

adc-lab control apply \
  --plan lab/runs/LAB-RUN-.../plans/PLAN-....json \
  --approval lab/runs/LAB-RUN-.../approvals/APPROVAL-....json \
  --dry-run

adc-lab restore \
  --lease lab/runs/LAB-RUN-.../leases/LEASE-....json \
  --dry-run

adc-lab health-check --target local
```

Remove `--dry-run` only after the fixed-path helper is installed, the operator has reviewed the approval artifact, and restore expectations are understood.

Privileged helper invocation uses the fixed `/usr/local/libexec/adc-lab-priv-helper` path. The controller CLI must not become a public arbitrary-helper or root-shell wrapper.

## Governor sweep workflow

```sh
adc-lab control governor-sweep prepare \
  --target local \
  --governors ondemand,performance,powersave \
  --duration-seconds-max 60 \
  --thermal-celsius-abort 75 \
  --requested-by codex \
  --out lab/runs/LAB-RUN-.../approvals/governor_sweep_policy_request.v2.json \
  --json

adc-lab control governor-sweep approve \
  --request lab/runs/LAB-RUN-.../approvals/governor_sweep_policy_request.v2.json \
  --approved-by operator \
  --out lab/runs/LAB-RUN-.../approvals/governor_sweep_policy.v2.json \
  --json

adc-lab control governor-sweep run \
  --target local \
  --governors ondemand,performance,powersave \
  --approval-policy lab/runs/LAB-RUN-.../approvals/governor_sweep_policy.v2.json \
  --duration-seconds-max 60 \
  --load-workers 2 \
  --load-duration 5s \
  --restore-after-each \
  --json
```

Real sweep apply requires an approved sweep policy artifact. Passing
`--approved-by` directly to `run` does not authorize apply. The command writes
typed control artifacts, audit events, and a `report.run_validation` summary;
non-measured governor evidence exits non-zero after the summary is written
unless `--allow-non-measured` is used for an exploratory dry run.

## Full-set run validation

```sh
adc-lab report validate-run \
  --run lab/runs/LAB-RUN-... \
  --include-run lab/runs/LAB-RUN-target-local-governor-sweep \
  --profile target-operating-contract-fullset \
  --expected-governors ondemand,performance,powersave \
  --workflow-recommendation lab/runs/LAB-RUN-.../workflows/recommendation.v2.json \
  --collect-plan lab/runs/LAB-RUN-.../workflows/collect_plan.v2.json \
  --target-id target55 \
  --target-class raspberry_pi_4 \
  --json
```

The validator writes `reports/run_validation.v2.json` and `reports/GAPS.md`.
It correlates control plans, approvals, apply results, linked load evidence,
restore results, and restore health checks by typed IDs and artifact refs. It
does not infer a controlled-governor measurement from file names or timestamp
order. By default it exits non-zero after writing the artifacts when any
requested governor is not `measured`; pass `--allow-non-measured` only for
exploratory review runs where that failure is expected.
Version skew blocks full-set measured claims by default. `--allow-version-skew`
records an exploratory override in the validation artifact, but it does not
make full-set or production-style claims selection-ready.

## Bounded CPU load

```sh
adc-lab load cpu \
  --target local \
  --workers 2 \
  --duration 5s \
  --abort-temp-c 75 \
  --operator-abort-file <target-abort-file>
```

`adc-lab load cpu` is a Tier 1 experimental burst. It is capped by duration and
available parallelism, supports optional thermal abort and operator abort, and
writes a v2 `kind = load` artifact under `load/`. The payload records the
safety monitor evidence that was previously exposed through the v1 load-result
wire shape.

The operator abort file path is runtime input only and is not serialized into run artifacts.

## Pressure probes

```sh
adc-lab pressure run --target local --kind latency_jitter --duration 1s

adc-lab pressure run \
  --target local \
  --kind memory_pressure \
  --duration 1s \
  --memory-bytes 8388608

adc-lab pressure run \
  --target local \
  --kind storage_io \
  --duration 1s \
  --storage-bytes 1048576

adc-lab pressure run --target local --kind network_io --duration 1s

adc-lab pressure run \
  --target local \
  --kind network_io \
  --duration 1s \
  --network-endpoint 127.0.0.1:9000 \
  --network-bytes 1048576

adc-lab pressure run --target local --kind observer_pressure --duration 1s

adc-lab pressure composite \
  --target local \
  --scenario memory_storage_jitter \
  --duration 3s \
  --memory-bytes 134217728 \
  --storage-bytes 67108864
```

Supported pressure kinds are:

```text
cpu_pressure
thermal_pressure
memory_pressure
storage_io
network_io
latency_jitter
observer_pressure
```

`adc-lab pressure run` writes v2 `kind = pressure` artifacts under
`pressure/<kind>.<result_id>.v2.json`. The probes are command-triggered,
bounded, cleanup-aware, artifact-producing, and claim-bounded. They are
classified as `measured`, `measured_partial`, `not_controllable`,
`unsafe_to_run_with_reason`, or `not_applicable_with_reason` where applicable.

For `network_io`, endpoint visibility and bounded transfer are different evidence classes. Counter-only results are observation evidence. Endpoint attempts without generated bytes remain insufficient for network boundary claims. A completed endpoint-backed transfer records `network_mode=bounded_transfer`, `traffic_generated_bytes`, rx/tx counter deltas, and LAN confounders.

`adc-lab pressure composite` writes v2 `kind = composite` artifacts under
`composite/<scenario>.<result_id>.v2.json`. The initial
`memory_storage_jitter` scenario runs baseline jitter, holds bounded anonymous
memory, runs bounded storage I/O under that held allocation, samples jitter
again, and records a short recovery phase. It can record composite evidence for
that phase-based scenario, but the coupling chain remains insufficient when the
relevant pressure effect is not observed. It does not prove larger memory
ladders, concurrent storage/jitter tails, or sustained storage cadence.

A pressure probe existing does not prove a full platform boundary or composite resource-coupling effect. Composite claims require a composite result artifact, not just separate pressure artifacts.

For more detail, see `docs/reference/pressure-probes.md`.

## Local workload suitability loop

`adc-lab workload run` v1 is local-target only. It does not forward
`executable_path + args` through SSH.

```sh
adc-lab workload run \
  --target local \
  --target-id target-id \
  --execution-mode target-local \
  --plan examples/workloads/pi4_representative_smoke.yaml \
  --run-dir lab/runs/LAB-RUN-workload-... \
  --json
```

The command writes:

```text
workloads/<workload-id>/workload_run_plan.json
workloads/<workload-id>/workload_run_result.json
workloads/<workload-id>/stdout.txt
workloads/<workload-id>/stderr.txt
reports/workload_demand_profile.json
```

`lab.workload_demand_profile.v1` separates:

```text
workload_demand              process-scoped CPU/RSS/I/O/context switches
target_conditioned_response  thermal/frequency/abort response on this target
system_context               whole-system background context
```

Thermal response is target-conditioned and non-portable. It is not workload
demand.

SSH workload execution is refused in v1:

```sh
adc-lab workload run \
  --target ssh://target-host \
  --plan examples/workloads/pi4_representative_smoke.yaml \
  --json
```

The structured result uses
`reason=remote_workload_execution_not_supported_in_v1`.

Create a policy-bound decision from a target evidence run, a target operating
contract, and a workload demand profile:

```sh
adc-lab decide suitability \
  --target-run lab/runs/LAB-RUN-target-contract-... \
  --target-contract lab/runs/LAB-RUN-target-contract-.../reports/target_operating_contract.v2.json \
  --workload-demand lab/runs/LAB-RUN-workload-.../reports/workload_demand_profile.json \
  --policy examples/suitability/pi4-default-policy.yaml \
  --out lab/runs/LAB-RUN-workload-.../reports/suitability.v2.json \
  --json
```

Unknown required dimensions force `selection_ready=false`. Policy cannot
convert unknown evidence to meet.

Generate constraints for implementation agents:

```sh
adc-lab constraints generate \
  --decision lab/runs/LAB-RUN-workload-.../reports/suitability.v2.json \
  --out lab/runs/LAB-RUN-workload-.../reports/constraints.v2.json \
  --agent-instructions-out lab/runs/LAB-RUN-workload-.../reports/agent_constraints.md \
  --json
```

Run the minimal blocked-claim lint:

```sh
adc-lab constraints check-candidate \
  --constraints lab/runs/LAB-RUN-workload-.../reports/constraints.v2.json \
  --path .
```

This check is intentionally small. It fails when blocked claim text appears in
candidate agent-facing content; it is not a full static analyzer. To validate
the generated constraints artifact or generated agent instructions themselves,
use the generated self-check command so the expected `Blocked claims` section is
not treated as downstream positive claim text:

```sh
adc-lab constraints self-check \
  --constraints lab/runs/LAB-RUN-workload-.../reports/constraints.v2.json \
  --path lab/runs/LAB-RUN-workload-.../reports/agent_constraints.md
```

`adc-lab constraints check` remains a compatibility alias and prints a warning.
New Agent workflows should use `check-candidate` or `self-check`.

## Experiment matrix

```sh
adc-lab experiment run \
  --target local \
  --matrix examples/experiments/pi4_cpu_governor_smoke.yaml \
  --dry-run

adc-lab experiment run \
  --target local \
  --matrix examples/experiments/bounded_load_observe_smoke.yaml \
  --trial-load-duration 1s \
  --trial-observe-duration 0s
```

`adc-lab experiment run` only marks a trial `completed` when supported non-privileged steps actually produced per-trial artifacts and audit events. Unsupported controlled factors are recorded as `blocked`, not completed.

## Reports

```sh
adc-lab report pack --run lab/runs/LAB-RUN-...

adc-lab report operating-point \
  --run lab/runs/LAB-RUN-... \
  --target-id local-target

adc-lab report operating-contract \
  --run lab/runs/LAB-RUN-... \
  --target-id local-target \
  --target-class raspberry_pi_4

adc-lab report operating-contract \
  --run lab/runs/LAB-RUN-primary \
  --include-run lab/runs/LAB-RUN-governor-control \
  --include-run lab/runs/LAB-RUN-composite \
  --include-run lab/runs/LAB-RUN-network-transfer \
  --validation lab/runs/LAB-RUN-primary/reports/run_validation.v2.json \
  --strict-fullset \
  --target-id target-id \
  --target-class raspberry_pi_4
```

`adc-lab report operating-point` classifies run evidence as `observational_only`, `controlled_subset`, `controlled_full`, `not_controllable`, or `blocked_unsafe`. Passive frequency variation remains observational evidence; it is not a fixed-frequency sweep.

`adc-lab report operating-contract` writes `lab.artifact.v2` with
`kind = report.operating_contract`. It also writes
`reports/evidence_ref_resolution.v2.json` so handoff review can see whether
the contract evidence refs resolve inside the opened run set or are explicitly
diagnostic/external.
For `report.evidence_ref_resolution`, `status.state = measured` means every
`artifact://` ref in the checked set is resolvable and every non-artifact ref is
explicitly classified as `diagnostic_external`. It does not mean diagnostic or
external refs are machine-resolved artifacts.

The target operating contract tells agents which patterns are allowed by evidence, burst-only, degraded-mode triggers, forbidden without more evidence, or blocked as claims.

With `--include-run`, the v2 evidence store opens all provided run
directories and evaluates the rule table against the combined v2 artifacts. It
does not emit v1 run-set or multi-run compatibility artifacts.
With `--validation`, controlled-governor full-set claims require a matching
`report.run_validation` artifact for the same workflow, run set, target id, and
target class. `--strict-fullset` writes the contract and then exits non-zero
when that validation gate is missing or non-measured.

An operating contract can support scoped lab claims. It cannot say:

```text
Pi4 is sufficient.
Pi5 is required.
This target is production-ready.
```

## SSH targets

For SSH targets, `adc-lab` uses fixed `adc-lab-target` subcommands over SSH. It does not expose arbitrary remote shell.

`ADC_LAB_TARGET_RUNNER` is a development override only and must name `adc-lab-target` from an allowlisted safe path such as:

```text
/usr/local/bin/adc-lab-target
/home/<user>/.local/bin/adc-lab-target
/home/<user>/.local/share/adc-lab/runners/<version>/adc-lab-target
```

The release installer installs user binaries under `~/.local/bin` by default.
Non-interactive SSH may not include that directory in PATH. If the target
runner was installed there, set
`ADC_LAB_TARGET_RUNNER=/home/<target-user>/.local/bin/adc-lab-target` instead
of relying on remote PATH lookup.

Remote read-only inventory, observe, and non-root load are supported. Privileged apply/restore should remain typed, bounded, approved, audited, and restorable; do not grant an Agent a root shell.

## Verification

Use the repository command wrapper:

```sh
make verify
```

This runs format, lint, tests, strict minimal schema fixture validation, contract validation, docs smoke, and command wiring smoke. The smoke command verifies command wiring only. It does not by itself support resource, NFR, Pi4/Pi5 comparison, suitability, or production-readiness claims.
