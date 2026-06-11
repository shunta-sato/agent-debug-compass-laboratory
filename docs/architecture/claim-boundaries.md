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

## Workload Suitability

Workload suitability is a policy decision over a target operating contract,
workload demand profile, and suitability policy. It answers one scoped
target/workload/policy question; it does not rank targets.

Allowed claims:

```text
- the workload profile defines a shared measurement target
- the target produced v2 evidence artifacts for that workload context
- the suitability artifact records meet / marginal / fail / unknown under the
  named policy
```

Blocked claims:

```text
- Pi4 is sufficient for the workload
- Pi5 is required for the workload
- battery safe
- sustained production ready
- fixed-frequency behavior verified
- all operating points measured
```

The blocked claims are intentional. The claim catalog keeps stable IDs and
next-evidence guidance so constraints checks can reject unsupported text.

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

## Operating Contract Rules

`adc-lab report operating-contract` writes a v2
`kind = report.operating_contract` artifact from Rust rule tables.

Allowed claims:

```text
- the named rule matched the required evidence kinds
- blocked claims remain blocked when required evidence is missing
- next evidence is catalog-backed and auditable
```

Blocked claims:

```text
- pressure smoke alone proves production readiness
- separate probes prove same-condition resource coupling
- observed dynamic CPU frequency range is a fixed-frequency sweep
```

Next evidence needed:

```text
- paired or composite evidence for coupling claims
- sustained thermal and recovery evidence for production claims
- approved control/apply/restore evidence for controlled operating points
```
