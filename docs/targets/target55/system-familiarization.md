# Target55 System Familiarization

## Target

- Target ID: `target55`
- Target class: `raspberry_pi_4`
- Board: Raspberry Pi 4 Model B Rev 1.5
- OS: Debian GNU/Linux 13
- Transport: `ssh://target55`
- Runner used for Platform Operating Contract discovery:
  `/home/satoshun/.local/share/adc-lab/runners/20260610-platform-contract/adc-lab-target`
- Existing installed runner preserved:
  `/home/satoshun/.local/bin/adc-lab-target` version `0.1.11`

## Decision Need

Discover the Platform Operating Contract for Pi4/Pi5-style targets: mechanisms,
resource pressure boundaries, coupling chains, recovery/degraded-mode evidence,
and design rules that AI agents must respect before making target performance
or safety claims.

## Artifact Status

| Artifact | Status | Path / Evidence | Freshness / Revisit |
| --- | --- | --- | --- |
| Target characterization | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/inventory/target_inventory.json` | Revisit on OS/kernel/firmware/cooling/storage/network change |
| Passive observation | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/observations/observe.json` | Revisit when workload or sampling cadence changes |
| Platform mechanism inventory | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/reports/platform_mechanism_inventory.json` | Revisit when target surfaces or runner change |
| Boundary probe plan | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/reports/boundary_probe_plan.json` | Revisit when new pressure domains or controls are added |
| CPU pressure | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/pressure/cpu_pressure.*.result.json` | Short-smoke only; repeat for governor/frequency states |
| Thermal pressure | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/pressure/thermal_pressure.*.result.json` | Short-smoke only; longer soak needs explicit approval/policy |
| Memory pressure | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/pressure/memory_pressure.*.result.json` | Revisit with memory ladder and workload-specific resident set |
| Storage I/O pressure | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/pressure/storage_io.*.result.json` | Revisit with write cadence and flash-wear plan |
| Network I/O pressure | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/pressure/network_io.*.result.json` | Revisit with configured bounded LAN endpoint |
| Latency/jitter pressure | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/pressure/latency_jitter.*.result.json` | Revisit under CPU/memory/storage/network pressure combinations |
| Observer pressure | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/pressure/observer_pressure.*.result.json` | Revisit when observer cadence or artifact size changes |
| Resource coupling report | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/reports/resource_coupling_report.json` | Revisit after composite pressure probes |
| Target operating contract | completed | `lab/runs/LAB-RUN-target55-platform-contract-20260610/reports/target_operating_contract.json` | Revisit after Pi5 run and controlled governor/frequency repetitions |
| Pi5 reference evidence | required_pending | none in this checkout | Required before Pi4/Pi5 contract comparison |
| Battery/power evidence | deferred_with_reason | no power surface used | Deferred because target55 is not a battery-power target in this run |
| Wakeup evidence | deferred_with_reason | no wakeup tool qualified | Deferred until qualified wakeup measurement is added |

## Current Claims

Allowed:

- target55 can produce `lab.resource_pressure_result.v1` artifacts for CPU,
  thermal, memory, storage, network, latency/jitter, and observer pressure.
- target55 has a `lab.target_operating_contract.v1` with
  `contract_status=measured_partial`.
- pressure and operating-contract artifacts do not use
  `unsupported_by_adc_lab` as a final state.

Blocked:

- Pi4 is sufficient or Pi5 is required for any workload.
- production-ready, battery-safe, flash-safe, thermally-safe, low-overhead, or
  real-time-ish claims.
- sustained 5/15/30 minute thermal claims beyond the current policy/evidence.
- fixed-frequency coverage without approved controllable frequency evidence.

## Handoff Status

| Handoff | Status | Next step |
| --- | --- | --- |
| Embedded NFR design | completed | Keep production claims blocked; use `reports/resource/nfr-gate-report.md` |
| Hot-path review | completed | See `reports/resource/hot-path-review.md` |
| Observer-effect review | completed | See `reports/resource/observer-effect-review.md` |
| Harness design | completed | See `docs/testing/resource-harness.md` |
| Pi4 operating contract | completed | Review target contract artifact and rerun with longer approved trials |
| Pi5 operating contract | required_pending | Run same suite on a Pi5 target |

## Verification

- `cargo test --workspace contract_validation -- --nocapture`: pass
- `cargo test -p adc-lab --test cli -- --nocapture`: pass
- `make verify`: pass
