# Embedded Hot-Path Review: adc-lab target runtime

## Hot Path

- Entry point: `adc-lab observe`, `adc-lab-target observe`, `adc-lab load cpu`.
- Cadence or trigger: command-triggered; observation default interval is 1s; CPU load loop is bounded by duration.
- Default mode: no always-on target-local loop.
- Burst mode: explicit command.
- Target class: Raspberry Pi 4 initial target, generic embedded Linux future.

## Per-Iteration Cost

| Cost source | Present? | Allowed budget | Evidence | Risk | Required change |
| --- | --- | --- | --- | --- | --- |
| Allocation | low in load loop; observation allocates per sample | 0 per default steady-state iteration | code review | acceptable because no default steady state | measure before production claim |
| Serialization/parsing | output serialization after bounded run | 0 per high-frequency default iteration unless measured | code review | acceptable | keep outside high-frequency loop |
| Filesystem / flash I/O | procfs/sysfs reads during observation | 0 continuous default writes unless budgeted | code review | observer effect unknown | target observer comparison |
| Directory scan | thermal/cpufreq discovery per sample | 0 per default iteration | code review | possible overhead | cache or measure if cadence increases |
| Network/radio use | SSH fixed commands only with runner path allowlist | 0 hidden background use unless budgeted | code review and CLI test | acceptable | no background radio |
| Blocking syscall | procfs/sysfs reads | 0 in sub-100ms loop unless justified | code review | acceptable at 1s default | no sub-100ms default |
| Lock/queue operation | none | bounded wait and bounded queue | code review | low | none |
| O(n) data-structure operation | factor expansion in cold path | 0 per high-frequency iteration unless bounded | tests | low | none |

## Cadence Decision

- Default cadence: no always-on cadence; observation default 1s when explicitly run.
- Burst cadence: 1s default, user-bounded duration; CPU load duration is capped at 300s by default policy and worker count cannot exceed available parallelism.
- Why this cadence is needed: enough for coarse CPU/frequency/thermal smoke.
- Event-driven or coalesced alternative: future adapters can cache surfaces if needed.
- Measurement required: yes, before overhead claims.

## Findings

- No submit-blocking hot-path finding for experimental-only MVP.

## Handoff

- NFR budget impact: experimental-only until target smoke and observer-effect comparison exist.
- Harness scenario needed: observer-on vs observer-off and bounded burst.
- Gate blocker: production physical-footprint claims remain blocked.
