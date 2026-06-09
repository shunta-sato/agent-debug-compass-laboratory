# agent-debug-compass-laboratory

`adc-lab` is a safety-gated target familiarization and experiment laboratory for AI agents building embedded and edge software.

The project is separate from Agent Debug Compass Flight Recorder. Flight Recorder is production-oriented, always-on, and lightweight. Laboratory is explicit, bounded, audited, restorable, and allowed to control target operating points only through typed operations.

## North Star

- No Agent root shell.
- No uncontrolled experiment.
- No unapproved irreversible or hard-to-restore operation.
- No unqualified tool becomes evidence.
- No operating-point claim without controlled or explicitly bounded evidence.
- No claim without audit.

## MVP Shape

- Controller CLI: `adc-lab`
- Non-root target runner: `adc-lab-target`
- Option A privileged helper: `adc-lab-priv-helper`
- Core contracts and schemas in `schemas/`
- Run artifacts in `lab/runs/LAB-RUN-*`
- Audit log in every evidence-producing run
- Privileged apply/restore is local-target only in this MVP. Remote read-only inventory, observe, and non-root load are supported; remote privileged helper transport is deferred.
- Privileged helper invocation uses the fixed `/usr/local/libexec/adc-lab-priv-helper` path; there is no public `--helper` override in the controller CLI.
- Experiment matrix execution supports a narrow real-run subset: listed-order
  matrices with the non-privileged `cpu_load_workers` controlled factor. Other
  controlled factors, including `governor` and fixed frequency, are blocked
  until explicit control/apply/restore wiring is added.
- Contract validation is a strict minimal MVP validator for required fields, enums, bounds, and `additionalProperties:false`; it is not claimed to be full Draft 2020-12 coverage.

Target-specific live-run artifacts that are useful as examples live under `examples/demos/`, not under canonical product docs. For example, `examples/demos/target55/` shows a short-smoke Raspberry Pi 4 evidence pack shape.

## Common Commands

```sh
adc-lab inventory --target local
adc-lab toolchain discover --target local
adc-lab tool qualify-inventory --inventory lab/runs/LAB-RUN-.../toolchain/toolchain_inventory.json
adc-lab tool qualify --manifest examples/tools/linux_cpufreq_reader.yaml --tool-version 0.1.0 --tool-sha256 sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa --output-schema examples/tools/linux_cpufreq_reader.output_schema.json --dry-run-output examples/tools/linux_cpufreq_reader.dry_run.json --manual-comparison examples/tools/linux_cpufreq_reader.manual_comparison.json --static-safety-review examples/tools/linux_cpufreq_reader.static_safety_review.txt
adc-lab observe --target local --duration 5s --signals cpu,freq,thermal,memory
adc-lab familiarize read-only --target local --duration 5s --signals cpu,freq,thermal,memory

adc-lab control plan --target local cpu.governor --set performance
adc-lab control approve --plan lab/runs/LAB-RUN-.../plans/PLAN-....json --approved-by operator
adc-lab control apply --plan lab/runs/LAB-RUN-.../plans/PLAN-....json --approval lab/runs/LAB-RUN-.../approvals/APPROVAL-....json --dry-run
adc-lab restore --lease lab/runs/LAB-RUN-.../leases/LEASE-....json --dry-run

adc-lab load cpu --target local --workers 2 --duration 5s --abort-temp-c 75 --operator-abort-file <target-abort-file>
adc-lab experiment run --target local --matrix examples/experiments/pi4_cpu_governor_smoke.yaml --dry-run
adc-lab experiment run --target local --matrix examples/experiments/bounded_load_observe_smoke.yaml --trial-load-duration 1s --trial-observe-duration 0s
adc-lab report pack --run lab/runs/LAB-RUN-...
adc-lab report operating-point --run lab/runs/LAB-RUN-... --target-id local-target
adc-lab health-check --target local
```

For SSH targets, `adc-lab` uses fixed `adc-lab-target` subcommands over SSH. It does not expose arbitrary remote shell.
`ADC_LAB_TARGET_RUNNER` is a development override only and must name `adc-lab-target` from an allowlisted safe path such as `/usr/local/bin/adc-lab-target` or `/home/<user>/.local/bin/adc-lab-target`.

`adc-lab load cpu` is a Tier 1 experimental burst. It is capped by duration and
available parallelism, supports optional thermal abort and operator abort, and
records safety monitor evidence in `lab.load_result.v1`. The operator abort
file path is runtime input only and is not serialized into run artifacts.

`adc-lab experiment run` only marks a trial `completed` when supported
non-privileged steps actually produced per-trial artifacts and audit events.
Unsupported controlled factors are recorded as `blocked`, not completed.

`adc-lab report operating-point` classifies run evidence as
`observational_only`, `controlled_subset`, `controlled_full`,
`not_controllable`, or `blocked_unsafe`. Passive frequency variation remains
observational evidence; it is not a fixed-frequency sweep.

The same report command also writes `lab.capability_cost_model.v1` as an
architecture evidence packet. It records observed CPU/memory/thermal/cpufreq
and bounded-load evidence, but keeps GPU/NPU/DSP/storage/network and production
physical-footprint claims blocked until qualified, target-specific cost
evidence exists. Capability presence is not an architecture recommendation.

## Verification

Use the repository command wrapper:

```sh
make verify
```

This runs format, lint, tests, strict minimal schema fixture validation, contract validation, docs smoke, and command wiring smoke. The smoke command does not by itself support resource/NFR claims.
