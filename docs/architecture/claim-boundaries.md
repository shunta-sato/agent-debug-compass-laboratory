# Claim Boundaries

Observed variation is not a controlled operating-point sweep.

Example:

```text
Observed:
  CPU frequency varied from 600MHz to 1800MHz under default policy.

Allowed claim:
  Frequency variation was observed under target default dynamic policy.

Blocked claim:
  adc-lab verified behavior across all fixed CPU frequencies.

Next evidence needed:
  controlled operating point matrix with fixed frequency or governor-controlled conditions.
```

## Workload And Target Capability Profiles

`lab.workload_profile.v1` defines the workload requirement. It answers "what is
X?" before anyone says "target can run X".

`lab.target_capability_profile.v1` links a target to that workload and to the
run artifacts that were observed. In PR11 these profiles are exploratory
short-smoke evidence only.

Allowed PR11 claims:

```text
- the workload profile defines a shared measurement target
- the target produced passive observation or bounded-load artifacts for that
  workload profile
- the target capability profile records missing evidence and blocked claims
```

Blocked PR11 claims:

```text
- Pi4 is sufficient for the workload
- Pi5 is required for the workload
- Pi4/Pi5 target selection is decided
- battery safe
- sustained production ready
- fixed-frequency behavior verified
- all operating points measured
```

The blocked claims are intentional. PR11 creates the evidence format; later
comparison and suitability contracts decide target selection.

`adc-lab report operating-point` creates `lab.operating_point_coverage.v1`.
Coverage status is explicit:

```text
observational_only:
  passive observe artifacts exist, but no operating point was controlled

controlled_subset:
  at least one allowlisted factor level was executed and has per-trial evidence

controlled_full:
  every declared point in the requested operating envelope was controlled

not_controllable:
  requested points are not currently controllable by adc-lab's safe runner

blocked_unsafe:
  requested points require Tier 3/Tier 4 safety handling before execution
```

In PR7, `cpu_load_workers` can become `controlled_subset` evidence because the
workload level is bounded and audited. CPU governor and fixed frequency remain
blocked until plan/apply/restore is wired into matrix execution. Production
quality claims require target characterization and controlled physical evidence.

## Capability Cost Model

`adc-lab report operating-point` also creates
`lab.capability_cost_model.v1`. This is an architecture evidence packet, not a
benchmark ranking.

Allowed PR8 claims:

```text
- CPU, memory, thermal, cpufreq, and bounded-load surfaces were observed when
  matching artifacts exist.
- CPU can be used as a lab baseline when target inventory exists.
- Bounded load results are partial lab evidence only.
```

Blocked PR8 claims:

```text
- GPU presence means GPU offload is better.
- NPU/DSP offload is supported by this run.
- Bounded CPU load proves production readiness.
- Observed dynamic CPU frequency range is a fixed-frequency sweep.
```

Next evidence needed:

```text
- qualified GPU/NPU/DSP adapters and output schemas
- workload-specific CPU-vs-accelerator cost comparison
- storage/write/flash, wakeup, battery, latency/jitter, and sustained thermal
  evidence
- controlled operating-point matrix for frequency-dependent architecture claims
```
