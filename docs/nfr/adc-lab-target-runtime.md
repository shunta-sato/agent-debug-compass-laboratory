# Embedded NFR Matrix: adc-lab target runtime

## Target-Class Assumptions

- Target class: Raspberry Pi 4 initial target, generic embedded Linux future.
- Power state: unknown.
- Deployment mode: lab-triggered experiment, not always-on production daemon.
- Default mode: no background target-local runtime.
- Burst mode: bounded observation or CPU load command.
- Degraded modes: experimental-only until degraded and recovery behavior are characterized.
- PR5 CPU load safety monitor: 100ms monitor loop during explicit CPU load
  only; records thermal surface availability, operator abort observation,
  sample count, and restore-on-abort status.
- Measurement surfaces available in the demo: target55 procfs CPU/memory, sysfs thermal, sysfs cpufreq read surface. Demo artifacts live under `examples/demos/target55/`.
- Measurement surfaces unavailable: wakeups, storage writes, battery/power, latency/jitter, sustained thermal recovery.

## NFR Matrix

| NFR | Default budget | Burst budget | Measurement | Merge rule | Status |
| --- | ---: | ---: | --- | --- | --- |
| Polling / sampling cadence | no always-on default | bounded by command duration | demo target55 1s observe smoke | sub-100ms default blocks submit | demo-measured for short smoke |
| CPU | no always-on default | bounded by worker count, duration, and operator abort | demo target55 idle/load smoke; PR5 hardware-free operator-abort tests | default over budget blocks submit | demo-partial |
| Wakeups | no always-on default | bounded by sample interval or 100ms load safety monitor during explicit load | not measured | unbounded wakeups block submit | unknown |
| RSS / heap | unknown | unknown | not measured directly | unbounded growth blocks submit | unknown |
| Hot-path allocation | no default hot path | load loop avoids shared allocation | code review | per-sample allocation needs evidence | provisional |
| Storage writes | no continuous default writes | run artifacts on controller | code review | continuous target writes need budget | provisional |
| Flash wear | no target write claim | unknown | not measured | missing estimate blocks production claim | unknown |
| Battery | unknown | unknown | not measured | battery_unknown is not AC | unknown |
| Network/radio | no hidden background use | SSH fixed commands only | demo target55 smoke over SSH | hidden background use blocks battery-safe claim | provisional |
| Thermal | no default heat claim | thermal abort supported and safety monitor records whether thermal surface was available | demo target55 short load smoke; PR5 contract tests | feature-caused thermal rise needs degraded mode | demo-partial |
| Latency / jitter | no production latency claim | unknown | not measured | missed timing budget blocks submit | unknown |
| Observer overhead | no production overhead claim | short observer-on/off comparison | demo target55 short smoke | uncalibrated overhead limits claims | demo-partial |

## Runtime Mode Classification

| Behavior | Mode | Cadence | Duration bound | Enabled by default? | Required evidence |
| --- | --- | ---: | ---: | --- | --- |
| `adc-lab observe` sampling | burst | >= 1s default | command duration | no | demo target55 observer-effect smoke captured |
| `adc-lab load cpu` | experimental-only burst | worker loop plus 100ms safety monitor | command duration, max 300s | no | demo target55 thermal/load smoke captured; PR5 operator-abort path hardware-free verified |
| `adc-lab-target` | command-triggered | none while idle | process lifetime | no daemon | demo target55 smoke passed |

## Degraded-Mode Policy

| Condition | Required behavior | Evidence |
| --- | --- | --- |
| battery_low | operator must avoid battery claims; future target adapter may refuse or reduce duty cycle | missing |
| memory_pressure | bounded outputs; fail rather than grow unbounded | tests and review |
| thermal_pressure | CPU load supports abort temperature and records whether thermal surface was available | demo target55 short smoke plus PR5 safety monitor contract |
| storage_pressure | controller artifacts; no target continuous writes in MVP | review |
| measurement_unavailable | mark experimental-only and block production claims | this document |

## Measurement Plan

- Target command smoke: `ADC_LAB_TARGET_RUNNER=/home/demo/.local/bin/adc-lab-target scripts/resource/run-resource-smoke.sh --target ssh://target55`.
- Host fallback command smoke: `scripts/resource/run-resource-smoke.sh --host-fallback`.
- Command smoke does not collect resource metrics; resource/NFR claims require separate observe/load artifacts.
- PR5 safety monitor verification is hardware-free in unit/CLI tests. It proves
  contract and operator-abort behavior, not target thermal safety.
- Demo evidence pack: `examples/demos/target55/`.
- Demo baseline: `examples/demos/target55/baselines/resource/`.
- Demo report path: `examples/demos/target55/reports/operating-envelope/`, `examples/demos/target55/reports/target-characterization.json`.
- Missing evidence: wakeups, battery/power, storage writes, flash wear, latency/jitter, sustained thermal/recovery/degraded envelope.

## No-Measurement-No-Claim List

Claims not allowed until measured:

- low overhead
- battery safe
- thermally safe
- flash safe
- production ready
