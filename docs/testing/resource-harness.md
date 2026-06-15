# Resource Harness

## Commands

Command smoke. This verifies command wiring only; it does not collect resource metrics and does not support resource/NFR claims:

```sh
scripts/resource/run-resource-smoke.sh --host-fallback
```

Optional demo target command smoke. This verifies SSH target-runner wiring only unless paired with separate observe/load reports:

```sh
ADC_LAB_TARGET_RUNNER=/home/demo/.local/bin/adc-lab-target scripts/resource/run-resource-smoke.sh --target ssh://target55
```

Platform Operating Contract pressure smoke. These commands collect typed
resource evidence and can be run locally or through an SSH target runner:

```sh
adc-lab pressure run --target ssh://target55 --kind cpu_pressure --duration 5s --workers 1 --abort-temp-c 75 --run-dir lab/runs/LAB-RUN-target55-platform-contract
adc-lab pressure run --target ssh://target55 --kind thermal_pressure --duration 5s --workers 2 --abort-temp-c 75 --run-dir lab/runs/LAB-RUN-target55-platform-contract
adc-lab pressure run --target ssh://target55 --kind memory_pressure --duration 3s --memory-bytes 8388608 --run-dir lab/runs/LAB-RUN-target55-platform-contract
adc-lab pressure run --target ssh://target55 --kind storage_io --duration 3s --storage-bytes 1048576 --run-dir lab/runs/LAB-RUN-target55-platform-contract
adc-lab pressure run --target ssh://target55 --kind network_io --duration 3s --run-dir lab/runs/LAB-RUN-target55-platform-contract
adc-lab pressure run --target ssh://target55 --kind latency_jitter --duration 3s --run-dir lab/runs/LAB-RUN-target55-platform-contract
adc-lab pressure run --target ssh://target55 --kind observer_pressure --duration 3s --run-dir lab/runs/LAB-RUN-target55-platform-contract
adc-lab report operating-contract --run lab/runs/LAB-RUN-target55-platform-contract --target-id target55 --target-class raspberry_pi_4
```

Endpoint-backed bounded network transfer requires an operator-provided receiver
on the controller or LAN. Use a target-local alias in your own environment;
`target55` is only an example target identifier:

```sh
adc-lab pressure run \
  --target ssh://<target-alias> \
  --kind network_io \
  --duration 3s \
  --network-endpoint <controller-host-or-ip>:<port> \
  --network-bytes 1048576 \
  --run-dir lab/runs/LAB-RUN-<target>-network-bounded-transfer
```

Composite coupling evidence requires a composite probe artifact, not just
separate memory, storage, and jitter pressure results:

```sh
adc-lab pressure composite \
  --target ssh://<target-alias> \
  --scenario memory_storage_jitter \
  --duration 3s \
  --memory-bytes 134217728 \
  --storage-bytes 67108864 \
  --run-dir lab/runs/LAB-RUN-<target>-composite-memory-storage-jitter
```

Multi-run aggregation can combine the pressure suite, governor-sweep run,
composite run, and network-transfer run into a candidate pack. The governor
run must include `reports/run_validation.v2.json`; operating-contract claims
do not infer controlled-governor evidence from raw plan, approval, or load
files.

```sh
adc-lab report operating-contract \
  --run lab/runs/LAB-RUN-<target>-platform-contract \
  --include-run lab/runs/LAB-RUN-<target>-governor-sweep \
  --include-run lab/runs/LAB-RUN-<target>-composite-memory-storage-jitter \
  --include-run lab/runs/LAB-RUN-<target>-network-bounded-transfer \
  --validation lab/runs/LAB-RUN-<target>-platform-contract/reports/run_validation.v2.json \
  --strict-fullset \
  --target-id <target-id> \
  --target-class raspberry_pi_4
```

The validation artifact must match the same run set, workflow id, target id,
and target class. A copied validation file from another run set is diagnostic
input only and cannot satisfy controlled-governor full-set claims.

## Required Scenarios

- idle baseline
- default steady state
- bounded burst
- bounded burst with operator abort marker
- bounded burst with thermal surface unavailable or explicitly recorded
- listed-order experiment matrix with `cpu_load_workers=0` and `1`
- unsupported controlled factor matrix blocked without producing completed evidence
- degraded battery_low
- degraded storage_pressure
- observer-on vs observer-off when instrumentation is target-local
- Platform Operating Contract pressure suite:
  - CPU pressure
  - thermal pressure
  - memory pressure
  - bounded storage I/O
  - network interface I/O/counter probe
  - latency/jitter loop
  - observer pressure
  - endpoint-backed bounded network transfer when a receiver is available
  - memory/storage/jitter composite boundary probe
  - resource coupling and target operating contract report

## Report Paths

- Baselines: `baselines/resource/`
- Reports: `reports/resource/`, `reports/operating-envelope/`, `reports/target-characterization/`
- Gate report: `reports/resource/nfr-gate-report.md`
- Platform contract report:
  `reports/target_operating_contract.v2.json`

## Limits

- Target-specific smoke evidence must be regenerated for the target under test;
  v1 demo packs are not part of the maintained verification surface.
- Host fallback proves command wiring and schema behavior only.
- Command smoke reports `resource_metrics_collected=false` and `resource_claims_supported=false`.
- PR5 hardware-free tests verify operator-abort contract behavior; they do not
  measure target thermal, wakeup, power, flash, or latency impact.
- PR6 hardware-free tests verify only the narrow real-run experiment subset:
  listed order, `cpu_load_workers`, bounded load result refs, passive observe
  refs, and per-trial audit. They do not measure production physical footprint.
- Target55 smoke proves only the short bounded target scenarios captured here.
- Platform Operating Contract pressure probes are bounded smoke evidence unless
  repeated under controlled operating points and longer approved soak windows.
- Endpoint-backed bounded transfer is still LAN-confounded and does not prove
  production network cadence, retry/backoff safety, packet loss behavior, or
  target suitability.
- The initial `memory_storage_jitter` composite scenario is phase-based. It can
  record `coupling_evidence_class=composite_measured`, but the chain status
  remains `insufficient` unless the relevant pressure effect is observed.
  Larger memory ladders, concurrent storage/jitter tails, and sustained storage
  cadence remain blocked.
- Manual measurement is required before production physical-footprint claims.
