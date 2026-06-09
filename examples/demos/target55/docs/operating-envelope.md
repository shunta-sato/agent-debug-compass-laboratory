# Operating Envelope: target55/adc-lab-target-runtime

This is demo evidence from a real 2026-06-08 `target55` short smoke. It shows the expected artifact shape, not a generic Raspberry Pi operating-envelope claim.

## Purpose

Identify normal and bounded nominal behavior for the initial Raspberry Pi 4 target without crossing into Tier 3 stress or degradation.

## Preconditions

- Target characterization: `examples/demos/target55/docs/target-characterization.md`.
- Safe discovery boundary: read-only observe and 10s CPU load with 2 workers.
- Abort conditions: CPU load abort at 75C.
- Do-not-probe: no sustained thermal stress, OOM, storage pressure, battery drain, reboot/watchdog, firmware, or blackout experiments.

## Scenarios

| Scenario | Workload | Duration | Expected behavior | Safety limit | Report |
| --- | --- | ---: | --- | --- | --- |
| idle | observe only | 10s | low CPU, default freq variation | read-only | `examples/demos/target55/reports/operating-envelope/idle.json` |
| nominal | CPU load, 2 workers | 10s | complete without thermal abort | 75C abort | `examples/demos/target55/reports/operating-envelope/observer_off.json` |
| peak | not run | n/a | n/a | deferred | deferred |
| near_boundary | not run | n/a | n/a | deferred | deferred |
| degraded | not run | n/a | n/a | deferred | deferred |
| observer_off | CPU load only | 10s | complete | 75C abort | `examples/demos/target55/reports/operating-envelope/observer_off.json` |
| observer_on | CPU load plus observe | 10s | complete with bounded observer overhead | 75C abort | `examples/demos/target55/reports/operating-envelope/observer_on.json` |
| recovery | not run | n/a | n/a | deferred | deferred |
| blackout_or_telemetry_loss | not run | n/a | n/a | deferred | deferred |

## Findings

- Normal range: idle-only temp 47.225-49.173C, average frequency 600-1100MHz, aggregate busy about 0.075%.
- Near-boundary indicators: not explored.
- Degradation signals: none observed in bounded scenarios.
- Boundary findings:
  - Signal: thermal.
  - Boundary type: configured abort.
  - Threshold: 75.
  - Unit: Celsius.
  - Confidence: medium for command behavior; low for target envelope.
  - Evidence: `lab/runs/LAB-RUN-target55-load-observer-off`.
- Telemetry blackout:
  - Observed: no.
  - Signal ID: thermal/cpu/frequency/memory.
  - Last seen time: end of 10s observer-on run.
  - Expected cadence ms: 1000.
  - Confidence: medium for short smoke only.
- Observer effect: worker iterations changed by about -0.05% with concurrent observe; load max temp changed from 54.043C to 54.53C. Treat this as a smoke result, not calibrated overhead.
- No-go boundary: Tier 3/4 operations remain prohibited without explicit procedure.
- Safe experimental limits: 10s, 2 CPU workers, 75C abort for this pass.
- Abort conditions observed: none.

## Budget Calibration Inputs

- CPU headroom: bounded 2-worker smoke completed.
- Wakeup headroom: unavailable.
- Memory headroom: idle available memory around 7332200-7334476 KiB.
- Storage write headroom: unavailable.
- Thermal headroom: short load max 54.53C under 75C abort threshold.
- Battery evidence: unavailable.

## Handoff

- nfr-calibration: partial target-specific inputs only.
- nfr-design: measured values can replace host-fallback text where scoped to target55 short smoke.
- nfr-gate: keep production claims blocked; allow target55 short-smoke claims.
