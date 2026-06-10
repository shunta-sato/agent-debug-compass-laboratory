# Target Operating Contract

A Target Operating Contract is a machine-readable summary of what `adc-lab` can and cannot claim about a target.

It is not a benchmark score. It is the evidence boundary that tells an Agent which software designs are supported by measurements, which designs are burst-only, which require degraded mode, and which claims remain blocked.

## Questions answered

```text
What can this target do?
What platform mechanisms affect software performance?
Which operating points were actually controlled?
Which values were merely observed?
What pressure conditions make the target slow down?
Which software patterns are allowed by evidence?
Which patterns are burst-only?
Which patterns require degraded mode?
Which claims are still blocked?
```

## Generated artifacts

`adc-lab report operating-contract` writes:

```text
lab.platform_mechanism_inventory.v1
lab.boundary_probe_plan.v1
lab.resource_coupling_report.v1
lab.target_operating_contract.v1
```

Typical command:

```sh
adc-lab report operating-contract \
  --run lab/runs/LAB-RUN-... \
  --target-id target55 \
  --target-class raspberry_pi_4
```

## Evidence model

`adc-lab` separates:

```text
observed covariates
controlled factors
uncontrolled confounders
pressure ingredients
composite coupling evidence
generic lab rules
measured target rules
evidence-needed rules
```

This prevents common mistakes:

```text
Observed CPU frequency range is not a fixed-frequency sweep.
A pressure probe existing is not the same as a measured platform boundary.
Separate memory, storage, and jitter artifacts are ingredients, not proof that memory pressure causes storage latency.
A generic rule is not the same as a measured target-specific rule.
```

## Contract outputs

A useful target operating contract should identify:

```text
measured mechanisms
boundary evidence
resource-coupling evidence class
allowed patterns
burst-only patterns
degraded-mode triggers
forbidden patterns
blocked claims
next evidence needed
```

## Claim discipline

A Pi4 run may support a narrow statement such as:

```text
On target55, Raspberry Pi 4 completed a 4-worker synthetic CPU load for 300s.
Maximum observed temperature was 72.549C under a 75C abort threshold.
Thermal margin was thin.
Governor control was measured for ondemand/performance/powersave.
Fixed-frequency behavior was not measured.
Memory/cache/storage coupling is still insufficient.
Production readiness is blocked.
```

It still must not claim:

```text
Pi4 is production-ready.
Pi4 is sufficient for workload X.
Pi5 is required for workload Y.
Pi4 is safe for 24h sustained thermal load.
All CPU frequencies were measured.
Memory/cache/storage coupling is fully understood.
Network behavior is characterized.
Real-time latency is guaranteed.
```

No evidence, no claim.
