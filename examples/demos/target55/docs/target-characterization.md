# Target Characterization: target55

This is demo evidence from a real 2026-06-08 `target55` short smoke. It is not canonical product documentation and does not support production physical-footprint claims.

## Target Identity

- Characterized at: 2026-06-08, from `lab/runs/LAB-RUN-target55-characterization-live`.
- Target profile version: `examples/demos/target55/target_profile.yaml`.
- Target class: Raspberry Pi 4.
- Hardware: Raspberry Pi 4 Model B Rev 1.5, 4 CPU cores, 8008356 KiB memory.
- OS/runtime: Debian GNU/Linux 13 (trixie), kernel `6.12.75+rpt-rpi-v8`, `aarch64`.
- Power source: unknown.
- Storage: assumed SD from target class; not measured.
- Deployment mode: lab-triggered target runner deployed to `/home/demo/.local/bin/adc-lab-target`.

## Measurement Context

- Workload fingerprint: idle-only observe, bounded CPU load with 2 workers for 10s, observer-on load comparison.
- Measurement method: `adc-lab` controller over SSH using `ADC_LAB_TARGET_RUNNER=/home/demo/.local/bin/adc-lab-target`.
- Measurement duration seconds: 10s per scenario.
- Observer state: idle observer-on; load observer-off and observer-on scenarios captured separately.
- Environment:
  - Ambient temp: unknown.
  - Power mode: unknown.
  - Battery state: not applicable / unavailable.
  - Governor: default dynamic policy observed; privileged control not applied.
  - Thermal state: normal for tested scenarios; max observed 54.53C.

## Measurement Surfaces

| Surface | Available? | Tool/path | Privilege | Limit |
| --- | --- | --- | --- | --- |
| CPU | yes | `/proc/stat` | none | aggregate ticks only |
| Memory | yes | `/proc/meminfo` | none | availability only |
| Wakeups | no | unavailable | n/a | not measured |
| Disk writes | no | unavailable in MVP | n/a | no flash-wear claim |
| Battery | no | unavailable | n/a | no battery claim |
| Thermal | yes | `/sys/class/thermal/thermal_zone*/temp` | none | one thermal zone observed |
| CPU frequency | yes | `/sys/devices/system/cpu/cpufreq` | read none, write helper | control not applied |
| Latency/jitter | no | unavailable | n/a | no jitter claim |

## Workload Catalog

| Workload | Description | Command | Risk | Notes |
| --- | --- | --- | --- | --- |
| idle | 10s observe only | `adc-lab observe --target ssh://target55 --duration 10s` | Tier 0 | baseline only |
| nominal | 2 worker CPU load for 10s | `adc-lab load cpu --workers 2 --duration 10s --abort-temp-c 75` | Tier 1 | bounded, completed |
| observer_on | observe and load concurrently | two bounded `adc-lab` commands | Tier 1 | used for observer-effect smoke |
| peak | not run | n/a | deferred | avoid unbounded stress |
| degraded | not run | n/a | deferred | safety constraints not characterized |

## Baselines

| Scenario | Report path | Summary | Confidence |
| --- | --- | --- | --- |
| idle | `examples/demos/target55/baselines/resource/idle.json` | 10s, 0.075% aggregate busy, 47.225-49.173C, 600-1100MHz | medium |
| nominal | `examples/demos/target55/baselines/resource/nominal_workload.json` | 2 workers, 10s, max 54.043C, no abort | medium |

## Constraints

- Do-not-probe: no Tier 3 sustained stress, battery drain, storage pressure, OOM, watchdog, reboot, or blackout test yet.
- Safety constraints: CPU load bounded to 10s and 75C abort during this pass.
- Missing signals: wakeups, storage writes, flash wear, battery, latency/jitter, ambient temperature.
- Target-specific unknowns: power supply state, cooling setup, sustained thermal behavior, recovery/cooldown curve.

## Confidence

- Overall confidence: medium for identity and short bounded smoke; low for production NFR budgets.
- Confidence per surface: CPU/memory/thermal/frequency medium; wakeups/storage/battery/jitter unknown.
- Why: live target evidence exists, but only short scenarios were run.
- Staleness: revisit on OS/kernel change, hardware/cooling change, runner change, or workload change.
- Revisit when: privileged control is installed, sustained load is needed, or production physical claims are proposed.

## Handoff

- operating-envelope-discovery: completed for safe idle/nominal/observer smoke; degraded/near-boundary deferred.
- nfr-calibration: partial inputs available; production calibration blocked by missing signals and short duration.
- nfr-design: update to target-specific partial evidence while keeping no-measurement-no-claim boundaries.
