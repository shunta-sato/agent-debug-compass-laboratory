# Embedded System Familiarization: target55/adc-lab-target-runtime

This is demo evidence from a real 2026-06-08 `target55` short smoke. It demonstrates how a familiarization pack can look and keeps all production physical-footprint claims blocked.

## 1. Goal

- System to understand: Raspberry Pi 4 target `target55` controlled from adc-lab controller over SSH.
- Software decision depending on this understanding: whether adc-lab can replace host-fallback evidence with bounded target evidence for inventory, observation, load, and observer-effect smoke.
- Work type: new target and target-local runtime characterization.
- Production/resource-safety claims requested: none; target-specific short-smoke claims only.

## 2. Artifact Status

| Artifact | Status | Path | Required? | Freshness / revisit condition | Missing/provisional/deferred reason |
| --- | --- | --- | --- | --- | --- |
| target characterization | current | `examples/demos/target55/docs/target-characterization.md` | yes | revisit on OS/kernel/hardware/cooling/runner change | short smoke only |
| operating envelope | provisional | `examples/demos/target55/docs/operating-envelope.md` | yes | revisit before near-boundary/degraded claims | degraded and recovery deferred |
| calibrated NFRs | provisional | `requirements/nfr/adc-lab-target-runtime.yaml` | yes | revisit before production claims | no battery/wakeup/flash/jitter evidence |
| hardware capability map | provisional | this file section 6 | yes | revisit when more surfaces are added | accelerator/counters not mapped |
| workload map | current for safe smoke | this file section 5 | yes | revisit for new workloads | peak/degraded deferred |
| bottleneck/margin map | provisional | this file section 8 | yes | revisit after longer runs | only short thermal/load smoke |
| architecture constraints | current | this file section 10 | yes | revisit when default daemon mode is added | no always-on mode in MVP |
| NFR gate report | current | `reports/resource/nfr-gate-report.md` | yes | revisit with more evidence | production claims blocked |

## 3. Artifact Freshness

- Target characterization current because: live target55 inventory and smoke were captured in this session.
- Operating envelope current because: safe idle, observer-off, and observer-on bounded scenarios were captured.
- Calibrated NFRs current because: target-specific short-smoke evidence is now referenced, with explicit limits.
- Hardware capability map current because: procfs, thermal, cpufreq, CPU, memory surfaces were discovered.
- Workload map current because: idle and bounded CPU load are documented.
- Bottleneck/margin map current because: short thermal margin is documented.
- Architecture constraints current because: no arbitrary shell, no always-on target daemon, bounded target runner path remain valid.
- Revisit when target hardware changes: yes.
- Revisit when OS/kernel/runtime changes: yes.
- Revisit when workload profile changes: yes.
- Revisit when power mode changes: yes.
- Revisit when measurement method changes: yes.

## 4. Target Identity

- Target class: Raspberry Pi 4.
- Hardware: Raspberry Pi 4 Model B Rev 1.5.
- CPU / cores / governors: 4 cores; cpufreq sysfs present; default dynamic policy observed.
- Memory: 8008356 KiB total.
- Storage: SD assumed, write behavior not measured.
- Power source: unknown.
- Thermal surfaces: one thermal zone.
- OS/runtime: Debian GNU/Linux 13 (trixie), Linux `6.12.75+rpt-rpi-v8`, aarch64.
- Kernel/driver constraints: cpufreq control requires privileged helper; not applied in this pass.
- Accelerator / GPU / NPU / DSP: not characterized.
- I/O buses: not characterized.
- Real-time constraints: not characterized.

## 5. Workload Map

| Workload | Description | Normal? | Peak? | Boundary? | Risk | Report |
| --- | --- | --- | --- | --- | --- | --- |
| idle | 10s passive observation | yes | no | no | Tier 0 | `examples/demos/target55/baselines/resource/idle.json` |
| nominal | 2-worker CPU load for 10s | yes for smoke | no | no | Tier 1 | `examples/demos/target55/baselines/resource/nominal_workload.json` |
| observer_on | 2-worker load plus observe | yes for smoke | no | no | Tier 1 | `examples/demos/target55/reports/operating-envelope/observer_on.json` |
| peak | not run | no | yes | possible | deferred | n/a |
| degraded | not run | no | no | yes | deferred | n/a |
| recovery | not run | no | no | yes | deferred | n/a |

## 6. Hardware Capability Map

| Capability | Available? | Measurement surface | Software lever | Risk | Architecture implication | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| CPU frequency scaling | yes | sysfs cpufreq | typed helper only for writes | privileged control risk | no raw sysfs writer | inventory |
| thermal zones | yes | sysfs thermal | load abort monitor | observer effect | keep abort bound | idle/load reports |
| battery state | no | unavailable | none | unsupported claim | block battery-safe claim | target characterization |
| storage write budget | no | unavailable | controller artifacts only | flash wear unknown | block flash-safe claim | target characterization |
| accelerator/NPU/GPU | unknown | none | none | unsupported claim | adapter future work | deferred |
| scheduler / real-time | unknown | none | none | jitter unknown | no real-time claim | deferred |
| hardware counters | unknown | none | none | missing precision | use procfs only | deferred |

## 7. Operating Envelope Summary

- Normal range: idle temp 47.225-49.173C, frequency 600-1100MHz, aggregate busy about 0.075%.
- Near-boundary indicators: not explored.
- Degradation signals: none observed in 10s bounded smoke.
- Telemetry/logging blackout: none observed in 10s observer-on run.
- Recovery behavior: not measured.
- No-go boundary: Tier 3/4 remain prohibited without explicit procedure.
- Observer effect: short-smoke worker iteration delta about -0.05%; thermal max +0.487C vs observer-off.

## 8. Bottleneck and Margin Map

| Resource | Current baseline | Near-boundary | Margin | Dominant risk | Evidence |
| --- | ---: | ---: | ---: | --- | --- |
| CPU | idle aggregate busy about 0.075% | not explored | unknown | short sample only | idle baseline |
| memory | 7332200-7334476 KiB available idle | not explored | unknown | no memory pressure test | idle baseline |
| wakeups | unavailable | unavailable | unknown | no wakeup surface | missing |
| flash writes | unavailable | unavailable | unknown | no flash-wear estimate | missing |
| battery | unavailable | unavailable | unknown | no battery surface | missing |
| thermal | 47.225-49.173C idle; 54.53C short load max | 75C abort configured | about 20.47C below configured abort in short smoke | sustained thermal unknown | load reports |
| latency/jitter | unavailable | unavailable | unknown | no jitter surface | missing |

## 9. NFR Calibration Inputs

- CPU budget source: target55 short idle/load smoke.
- memory budget source: target55 idle memory availability only.
- wakeup budget source: unavailable.
- battery budget source: unavailable.
- flash budget source: unavailable.
- thermal budget source: target55 short idle/load smoke only.
- latency/jitter budget source: unavailable.

## 10. Architecture Constraints

- Must: keep target operations typed, bounded, audited, and command-triggered.
- Must not: use raw root shell, arbitrary sysfs writes, unbounded stress, or audit-less evidence.
- Should: keep target runner deployable as non-root user-local binary.
- Allowed only in burst mode: observation sampling, CPU load.
- Experimental only: target runtime physical-footprint claims beyond short-smoke target55 behavior.
- Deferred until measured: battery, flash, wakeups, jitter, sustained thermal, recovery, degraded modes.

## 11. Claims Blocked By Missing Evidence

| Claim | Missing evidence | Allowed wording/status |
| --- | --- | --- |
| hardware-efficient | calibrated target envelope and bottleneck map | target-specific short-smoke only |
| low-overhead | observer-on/off across longer workloads and wakeup/CPU overhead | blocked |
| battery-safe | battery/power measurement | blocked |
| flash-safe | storage write and flash-wear evidence | blocked |
| thermally-safe | sustained thermal and recovery envelope | blocked; short-smoke stayed below 75C abort |
| production-ready | target characterization across normal/degraded/recovery plus NFR calibration | blocked |

## 12. Handoffs

| Handoff | Status | Required? | Evidence path | Blocker? | Notes |
| --- | --- | --- | --- | --- | --- |
| embedded-project-constitution | completed | yes | `docs/00_project_principles.md` | no | existing |
| embedded-target-characterization | completed | yes | `examples/demos/target55/docs/target-characterization.md` | no | short smoke |
| embedded-operating-envelope-discovery | completed | yes | `examples/demos/target55/docs/operating-envelope.md` | no | degraded deferred |
| embedded-nfr-calibration | deferred_with_reason | yes | `requirements/nfr/adc-lab-target-runtime.yaml` | no | partial inputs only |
| embedded-nfr-design | completed | yes | `docs/nfr/adc-lab-target-runtime.md` | no | updated with target evidence |
| architecture-decision-analysis | not_needed | no | n/a | no | no architecture option comparison |
| embedded-nfr-harness-design | completed | yes | `docs/testing/resource-harness.md` | no | target runner path now works |
| embedded-hot-path-review | completed | yes | `reports/resource/hot-path-review.md` | no | no production claim |
| embedded-observer-effect-review | completed | yes | `reports/resource/observer-effect-review.md` | no | short smoke measured |
| embedded-nfr-gate | completed | yes | `reports/resource/nfr-gate-report.md` | no | production claims blocked |
| quality-gate | completed | yes | `reports/quality-gate.md` | no | `make verify` and target55 smoke passed |
