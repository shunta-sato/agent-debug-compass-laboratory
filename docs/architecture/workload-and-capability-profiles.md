# Workload And Target Capability Profiles

PR11 adds the first target-selection foundation layer, but it does not make
target-selection decisions.

## Contracts

`lab.workload_profile.v1` defines what a workload asks for:

- `workload_id`
- workload class
- duration
- safety and measurement requirements
- claim boundary

`lab.target_capability_profile.v1` defines what a target has demonstrated for
that workload:

- `target_id`
- `workload_id`
- run/evidence refs
- observed short-smoke results
- supported claims
- blocked claims
- next evidence needed
- `selection_ready`

## Boundary

Allowed PR11 claims:

```text
- a workload profile defines the same measurement target for Pi4 and Pi5
- a target capability profile links observed artifacts to that workload id
- evidence is exploratory or short-smoke only
- missing evidence and blocked claims are explicit
```

Blocked PR11 claims:

```text
- Pi4 is sufficient for a workload
- Pi5 is required for a workload
- Pi4 is faster/slower/better than Pi5
- target is battery safe
- target is sustained production ready
- all operating points were measured
- fixed-frequency behavior was verified
```

Those claims require later comparison and suitability decision contracts.

## CLI

```sh
adc-lab report capability-profile \
  --run lab/runs/LAB-RUN-... \
  --target-id target55 \
  --workload examples/workloads/bounded_cpu_load_2_workers_60s.json \
  --json
```

The command reads existing run artifacts only. It does not run observe, load,
privileged control, helper apply, SSH commands, or destructive experiments.

The output artifact is:

```text
reports/target_capability_profile.<workload_id>.json
```

and the audit event is:

```text
operation=report.target_capability_profile
operation_id=<workload_id>
risk_tier=tier0_read_only_observation
```

## Examples

Workload definitions:

- `examples/workloads/idle_observe_60s.json`
- `examples/workloads/bounded_cpu_load_2_workers_60s.json`

Exploratory profiles:

- `examples/demos/pi4/target_capability_profile.bounded_cpu_load_2_workers_60s.json`
- `examples/demos/pi5/target_capability_profile.bounded_cpu_load_2_workers_60s.json`

The Pi4 example is derived from the target55 short-smoke demo. The Pi5 example
is intentionally evidence-pending and contains no invented measurements.

## Follow-Up Layers

The local suitability v1 layer adds:

- `lab.workload_run_plan.v1` for a bounded local workload plan.
- `lab.workload_run_result.v1` for completed / failed / aborted / refused run
  outcomes.
- `lab.workload_demand_profile.v1` for process-scoped demand, separated from
  target-conditioned response and system context.
- `lab.suitability_policy.v1` for required dimensions, thresholds, and margin
  rules.
- `lab.suitability_decision.v1` for workload-specific meet / marginal / fail /
  unknown decisions.
- `lab.design_constraint_pack.v1` and agent-facing Markdown constraints.

This v1 layer is local-target only. `adc-lab workload run --target ssh://...`
returns a structured refusal because forwarding `executable_path + args` over
SSH would become arbitrary remote command execution. A future remote workload
layer needs staged allowlisted paths, sha256 verification, no shell, and
target-side manifest validation.

Still future:

- `lab.target_comparison.v1` for apples-to-apples Pi4/Pi5 comparison.
- Final target selection across devices.

Until same-suite comparison evidence exists, Pi4/Pi5 target-selection claims
remain blocked.
