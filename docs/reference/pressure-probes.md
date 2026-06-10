# Pressure Probes Reference

`adc-lab pressure run` creates bounded `lab.resource_pressure_result.v1` artifacts.

Pressure probes are:

```text
command-triggered
bounded
cleanup-aware
artifact-producing
claim-bounded
```

A pressure probe existing does not automatically mean a platform mechanism or coupling effect was fully measured.

For example:

```text
memory allocation smoke != memory pressure boundary
network counter visibility != network operating contract
storage tempfile smoke != flash safety
idle jitter loop != real-time behavior under pressure
```

## Supported pressure kinds

```text
cpu_pressure
thermal_pressure
memory_pressure
storage_io
network_io
latency_jitter
observer_pressure
```

## Examples

Latency / jitter smoke:

```sh
adc-lab pressure run --target local --kind latency_jitter --duration 1s
```

Bounded memory allocation pressure:

```sh
adc-lab pressure run \
  --target local \
  --kind memory_pressure \
  --duration 1s \
  --memory-bytes 8388608
```

Bounded temporary storage I/O:

```sh
adc-lab pressure run \
  --target local \
  --kind storage_io \
  --duration 1s \
  --storage-bytes 1048576
```

Network counter / endpoint visibility smoke:

```sh
adc-lab pressure run --target local --kind network_io --duration 1s
```

Observer pressure smoke:

```sh
adc-lab pressure run --target local --kind observer_pressure --duration 1s
```

## Status semantics

Pressure results are classified as `measured`, `measured_partial`, `not_controllable`, `unsafe_to_run_with_reason`, or `not_applicable_with_reason` where applicable.

The result status is intentionally stricter than command success. A successful command may still describe only partial evidence when the probe did not control enough factors to support a broader claim.

## Claim boundaries

Use pressure probes as ingredients for an operating contract, not as standalone proof of production suitability.

Examples of blocked conclusions from pressure probes alone:

```text
The target is safe for 24h sustained load.
Memory pressure causes storage latency on this target.
Network behavior is characterized.
Real-time latency is guaranteed.
Flash wear is safe for production.
```

Generate a target operating contract after collecting relevant evidence:

```sh
adc-lab report operating-contract \
  --run lab/runs/LAB-RUN-... \
  --target-id target55 \
  --target-class raspberry_pi_4
```
