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
