# Embedded Observer-Effect Review: adc-lab target runtime

## Observer

- Component: procfs/sysfs observation, CPU load safety monitor, and run artifact writer.
- Cadence: 1s default during explicit observation command; 100ms safety monitor during explicit CPU load.
- Data captured: CPU ticks, memory availability, cpufreq, thermal readings.
- Storage path: controller run artifacts; target runner writes only stdout in MVP.
- Transmission path: local stdout or fixed SSH command stdout.
- Default/debug/experimental mode: experimental command-triggered burst.

## Perturbation Review

| Vector | Risk | Evidence | Mitigation | Status |
| --- | --- | --- | --- | --- |
| Scheduler / wakeups | added wakeups during observation and load safety monitor | target55 short smoke; wakeups unavailable; PR5 hardware-free operator-abort tests | bounded duration, 1s observe default, 100ms load monitor only during explicit load | partial |
| CPU / allocation | per-sample reads and output allocation | target55 observer-on/off load iterations | no always-on mode | partial |
| Storage writes / flash wear | controller writes artifacts; target load checks optional abort marker metadata | code review and PR5 CLI test | no target continuous writes; abort file path is not serialized into artifacts | provisional |
| Network/radio use | SSH output when remote | target55 SSH smoke | fixed command, bounded output expectation | partial |
| Thermal behavior | observer and safety monitor may perturb load runs | target55 observer-on/off thermal smoke; PR5 safety monitor contract | keep production thermal claims blocked | partial |
| Lock contention / queue pressure | none in MVP observer | code review | no shared queue | provisional |
| Timing / jitter | not measured | missing target evidence | no timing claim | unknown |

## Observer-On vs Observer-Off

- Observer-off scenario: target load without separate observe, `lab/runs/LAB-RUN-target55-load-observer-off`.
- Observer-on scenario: target load with concurrent observe, `lab/runs/LAB-RUN-target55-observer-on-load` and `lab/runs/LAB-RUN-target55-observer-on-observe`.
- Difference: worker iterations changed from 5990418432 observer-off to 5987442688 observer-on, about -0.05%; load max temperature changed from 54.043C to 54.53C.
- What remains unmeasured: wakeups, power, longer-run CPU overhead, thermal recovery, jitter.

## Default-Enable Decision

- Decision: experimental-only.
- Rationale: no always-on observer is enabled; command-triggered observation is bounded and short-smoke measured on target55, but not calibrated for production overhead.
- Required evidence before enabling by default: longer observer-on/off report with wakeups, power, storage, and jitter where relevant.

## Findings

- No submit-blocking observer finding for experimental-only MVP.
- [EOE-001] `target55` - observer-on/off smoke is short and lacks wakeup/power/jitter surfaces - keep production overhead claims blocked.
- [EOE-002] PR5 load safety monitor adds a 100ms check during explicit CPU
  load; this is accepted for experimental burst mode but is not production
  overhead evidence.

## Handoff

- Harness scenario needed: longer observer-on vs observer-off and degraded-mode runs.
- NFR gate blocker: production overhead claims remain blocked.
