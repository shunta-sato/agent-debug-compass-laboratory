# CLI Reference

This page keeps the longer command examples out of the README while preserving a reproducible command trail for operators and agents.

Use `COMMANDS.md` as the repository command registry for build, test, lint, and verification commands. This page focuses on `adc-lab` user-facing command examples.

## Local familiarization

```sh
adc-lab inventory --target local
adc-lab toolchain discover --target local
adc-lab observe --target local --duration 5s --signals cpu,freq,thermal,memory
adc-lab familiarize read-only --target local --duration 5s --signals cpu,freq,thermal,memory
```

These commands produce target inventory, toolchain inventory, passive observation, run manifest, audit log, and claim-boundary evidence. They do not control privileged operating points.

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

## Bounded CPU load

```sh
adc-lab load cpu \
  --target local \
  --workers 2 \
  --duration 5s \
  --abort-temp-c 75 \
  --operator-abort-file <target-abort-file>
```

`adc-lab load cpu` is a Tier 1 experimental burst. It is capped by duration and available parallelism, supports optional thermal abort and operator abort, and records safety monitor evidence in `lab.load_result.v1`.

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
adc-lab pressure run --target local --kind observer_pressure --duration 1s
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

`adc-lab pressure run` writes `lab.resource_pressure_result.v1` artifacts under `pressure/`. The probes are command-triggered, bounded, cleanup-aware, artifact-producing, and claim-bounded. They are classified as `measured`, `measured_partial`, `not_controllable`, `unsafe_to_run_with_reason`, or `not_applicable_with_reason` where applicable.

A pressure probe existing does not prove a full platform boundary or composite resource-coupling effect.

For more detail, see `docs/reference/pressure-probes.md`.

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

adc-lab report capability-profile \
  --run lab/runs/LAB-RUN-... \
  --target-id local-target \
  --workload examples/workloads/bounded_cpu_load_2_workers_60s.json
```

`adc-lab report operating-point` classifies run evidence as `observational_only`, `controlled_subset`, `controlled_full`, `not_controllable`, or `blocked_unsafe`. Passive frequency variation remains observational evidence; it is not a fixed-frequency sweep.

The same report command also writes `lab.capability_cost_model.v1` as an architecture evidence packet. It records observed CPU/memory/thermal/cpufreq and bounded-load evidence, but keeps GPU/NPU/DSP/storage/network and production physical-footprint claims blocked until qualified, target-specific cost evidence exists.

`adc-lab report operating-contract` writes:

* `lab.platform_mechanism_inventory.v1`
* `lab.boundary_probe_plan.v1`
* `lab.resource_coupling_report.v1`
* `lab.target_operating_contract.v1`

The target operating contract tells agents which patterns are allowed by evidence, burst-only, degraded-mode triggers, forbidden without more evidence, or blocked as claims.

`adc-lab report capability-profile` writes `lab.target_capability_profile.v1` for a specific `lab.workload_profile.v1`. It reads existing run artifacts only and keeps `selection_ready=false` while capability profiles are exploratory.

A capability profile can say a target produced short-smoke artifacts for a workload. It cannot say:

```text
Pi4 is sufficient.
Pi5 is required.
This target is production-ready.
```

See also `docs/architecture/workload-and-capability-profiles.md`.

## SSH targets

For SSH targets, `adc-lab` uses fixed `adc-lab-target` subcommands over SSH. It does not expose arbitrary remote shell.

`ADC_LAB_TARGET_RUNNER` is a development override only and must name `adc-lab-target` from an allowlisted safe path such as:

```text
/usr/local/bin/adc-lab-target
/home/<user>/.local/bin/adc-lab-target
/home/<user>/.local/share/adc-lab/runners/<version>/adc-lab-target
```

Remote read-only inventory, observe, and non-root load are supported. Privileged apply/restore should remain typed, bounded, approved, audited, and restorable; do not grant an Agent a root shell.

## Verification

Use the repository command wrapper:

```sh
make verify
```

This runs format, lint, tests, strict minimal schema fixture validation, contract validation, docs smoke, and command wiring smoke. The smoke command verifies command wiring only. It does not by itself support resource, NFR, Pi4/Pi5 comparison, suitability, or production-readiness claims.
