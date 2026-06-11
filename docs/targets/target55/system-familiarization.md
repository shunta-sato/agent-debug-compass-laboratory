# Target55 System Familiarization

## Target

- Target ID: `target55`
- Target class: `raspberry_pi_4`
- Board: Raspberry Pi 4 Model B Rev 1.5
- OS: Debian GNU/Linux 13
- Transport: `ssh://target55`
- Runner used for Platform Operating Contract discovery:
  `/home/satoshun/.local/share/adc-lab/runners/20260611-composite/adc-lab-target`
- Existing installed runner preserved:
  `/home/satoshun/.local/bin/adc-lab-target` version `0.1.16`
- Privileged helper readiness:
  `privilege doctor --target local --json` reported `status=ready`,
  helper version `0.1.16`, and `sudo_non_interactive_available=true`.
- Prior review artifact zip:
  `/mnt/share/target55-platform-contract-review-20260610.zip`
  (`sha256=557f9706c17a2ce87631a6aa4804334ff4ff108ad1a705a73290a0f06dab7f2b`)
- Current candidate pack zip:
  `/mnt/share/target55-pi4-reference-pack-v1-candidate-20260611-r2.zip`
  (`sha256=e39efaedc405dc637ce2231518f43d6de84172c1f2e4900442f173ee1274f204`)
- Local workload suitability validation zip:
  `/mnt/share/target55-local-workload-suitability-20260611.zip`
  (`sha256=bb039e3eeeb36184973b676edaac2aede32fab45b7435d90914b6a075c9f0572`)

## Decision Need

Discover the Platform Operating Contract for Pi4/Pi5-style targets: mechanisms,
resource pressure boundaries, coupling chains, recovery/degraded-mode evidence,
and design rules that AI agents must respect before making target performance
or safety claims.

## Artifact Status

| Artifact | Status | Path / Evidence | Freshness / Revisit |
| --- | --- | --- | --- |
| Target characterization | completed | `lab/runs/LAB-RUN-target55-pi4-reference-v1-20260611T0030Z/inventory/target_inventory.json` | Revisit on OS/kernel/firmware/cooling/storage/network change |
| Passive observation | completed | `lab/runs/LAB-RUN-target55-pi4-reference-v1-20260611T0030Z/observations/observe.json` | Revisit when workload or sampling cadence changes |
| Platform mechanism inventory | completed | `lab/runs/LAB-RUN-target55-pi4-reference-v1-20260611T0030Z/reports/platform_mechanism_inventory.json` | Revisit when target surfaces or runner change |
| Boundary probe plan | completed | `lab/runs/LAB-RUN-target55-pi4-reference-v1-20260611T0030Z/reports/boundary_probe_plan.json` | Revisit when new pressure domains or controls are added |
| CPU pressure | completed | `lab/runs/LAB-RUN-target55-pi4-reference-v1-20260611T0030Z/pressure/cpu_pressure.*.result.json` and `lab/runs/LAB-RUN-target55-governor-control-20260611T0030Z/pressure/cpu_pressure.*.result.json` | Short bounded evidence under default and `performance` governor; not long-soak |
| Governor control | completed | `lab/runs/LAB-RUN-target55-governor-control-20260611T0030Z/{plans,approvals,results,leases,health}` | Approved apply/restore evidence exists for governor subset; fixed-frequency behavior remains unmeasured |
| Thermal pressure | completed | `lab/runs/LAB-RUN-target55-pi4-reference-v1-20260611T0030Z/pressure/thermal_pressure.*.result.json` | Short-smoke only; longer soak needs explicit approval/policy |
| Memory pressure | completed | `lab/runs/LAB-RUN-target55-pi4-reference-v1-20260611T0030Z/pressure/memory_pressure.*.result.json` | 128MiB allocation smoke; pressure effect was not observed |
| Storage I/O pressure | completed | `lab/runs/LAB-RUN-target55-pi4-reference-v1-20260611T0030Z/pressure/storage_io.*.result.json` | Bounded tempfile smoke with cleanup; no sustained cadence or flash-wear claim |
| Memory/storage/jitter composite | completed | `lab/runs/LAB-RUN-target55-composite-memory-storage-jitter-20260611T0140Z/composite/memory_storage_jitter.*.result.json` | `coupling_evidence_class=composite_measured`, but result status is `insufficient` because memory phase lacked reclaim/PSI/fault effect and storage/jitter were sequential |
| Network I/O pressure | completed | `lab/runs/LAB-RUN-target55-network-bounded-transfer-20260611T0030Z/pressure/network_io.*.result.json` | Endpoint-backed 1MiB bounded transfer measured; LAN-confounded and no retry/backoff/loss claim |
| Latency/jitter pressure | completed | `lab/runs/LAB-RUN-target55-pi4-reference-v1-20260611T0030Z/pressure/latency_jitter.*.result.json` and governor-control jitter result | Current-condition jitter only; not full pressure matrix |
| Observer pressure | completed | `lab/runs/LAB-RUN-target55-pi4-reference-v1-20260611T0030Z/pressure/observer_pressure.*.result.json` | Bounded observer-off/on smoke; no workload-specific low-overhead claim |
| Resource coupling report | completed | `lab/runs/LAB-RUN-target55-composite-memory-storage-jitter-20260611T0140Z/reports/resource_coupling_report.json` | `memory.reclaim_to_storage_latency` has `coupling_evidence_class=composite_measured` but `status=insufficient`; other chains remain insufficient |
| Multi-run operating contract artifact | completed | `lab/runs/LAB-RUN-target55-pi4-reference-v1-20260611T0030Z/reports/multi_run_operating_contract.json` | `pack_status=platform_operating_contract_candidate`, `contract_status=insufficient` |
| Local workload run | completed | `lab/runs/LAB-RUN-target55-local-workload-suitability-20260611T015207Z/workloads/pi4_representative_smoke/workload_run_result.json` | Target-local run only; `execution_mode=target_local`; representative workload, not real app performance |
| Workload demand profile | completed | `lab/runs/LAB-RUN-target55-local-workload-suitability-20260611T015207Z/reports/workload_demand_profile.json` | Process-scoped direct-child demand; child process aggregation unsupported in v1 |
| Suitability decision | completed | `lab/runs/LAB-RUN-target55-local-workload-suitability-20260611T015207Z/reports/suitability_decision.json` | `overall_decision=fail`, `selection_ready=false` under `pi4_default_policy`; not Pi4/Pi5 selection evidence |
| Agent constraints | completed | `lab/runs/LAB-RUN-target55-local-workload-suitability-20260611T015207Z/reports/agent_constraints.md` | Implementation-agent constraints for this workload/evidence/policy body |
| SSH workload transport | deferred_with_reason | `lab/runs/LAB-RUN-workload-ssh-refusal-20260611T015207Z/workloads/pi4_representative_smoke/workload_run_result.json` | v1 refuses `workload run --target ssh://...` to avoid arbitrary remote command execution |
| Pi5 reference evidence | required_pending | none in this checkout | Required before Pi4/Pi5 contract comparison |
| Battery/power evidence | deferred_with_reason | no power surface used | Deferred because target55 is not a battery-power target in this run |
| Wakeup evidence | deferred_with_reason | no wakeup tool qualified | Deferred until qualified wakeup measurement is added |

## Current Claims

Allowed:

- target55 can produce `lab.resource_pressure_result.v1` artifacts for CPU,
  thermal, memory, storage, network, latency/jitter, and observer pressure.
- target55 can produce `lab.target_operating_contract.v1`; after the review-fix
  semantics this artifact is allowed to be `contract_status=insufficient` when
  pressure artifacts are smoke-only or coupling evidence is ingredients-only.
- pressure and operating-contract artifacts do not use
  `unsupported_by_adc_lab` as a final state.
- target55 has approved apply/restore evidence for the CPU governor subset.
- target55 has one phase-based `memory_storage_jitter` composite result that
  records composite evidence class, but the chain status remains insufficient
  because memory pressure effect was not observed.
- target55 has endpoint-backed bounded network transfer evidence for a 1MiB LAN
  transfer to the controller endpoint.
- target55 has a completed target-local representative workload run with
  process-scoped demand evidence and `execution_mode=target_local`.
- adc-lab v1 refuses SSH workload transport with
  `remote_workload_execution_not_supported_in_v1`.

Blocked:

- Pi4 is sufficient or Pi5 is required for any workload.
- production-ready, battery-safe, flash-safe, thermally-safe, low-overhead, or
  real-time-ish claims.
- sustained 5/15/30 minute thermal claims beyond the current policy/evidence.
- fixed-frequency coverage without approved controllable frequency evidence.
- resident memory budget and memory pressure degradation thresholds; 128MiB did
  not trigger reclaim/PSI/fault evidence on this 8GiB target.
- concurrent storage+jitter tail-latency claims; current composite storage and
  jitter phases are sequential under held memory.
- network production cadence, retry/backoff, packet loss, and target-selection
  claims; the bounded transfer is LAN-confounded.
- representative workload results are not real application performance,
  production readiness, sustained thermal safety, flash-wear evidence, or
  Pi4/Pi5 selection evidence.

## Handoff Status

| Handoff | Status | Next step |
| --- | --- | --- |
| Embedded NFR design | completed | Keep production claims blocked; use `reports/resource/nfr-gate-report.md` |
| Hot-path review | completed | See `reports/resource/hot-path-review.md` |
| Observer-effect review | completed | See `reports/resource/observer-effect-review.md` |
| Harness design | completed | See `docs/testing/resource-harness.md` |
| Pi4 operating contract candidate | completed | Review multi-run contract artifact; contract reference status remains blocked by memory ladder and same-condition coupling evidence |
| Local workload suitability loop | completed | Review workload demand, suitability decision, design constraint pack, and agent constraints; keep `selection_ready=false` unless policy/evidence changes |
| Pi5 operating contract | required_pending | Run same suite on a Pi5 target |

## Verification

- `cargo test --workspace contract_validation -- --nocapture`: pass
- `cargo test -p adc-lab --test cli -- --nocapture`: pass
- `make verify`: pass
