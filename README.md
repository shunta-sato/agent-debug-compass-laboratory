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
- Experiment matrix execution is dry-run/planning only in this MVP. Non-dry runs are recorded as `not_implemented` and cannot support claims until real control/load/observe execution is wired.
- Contract validation is a strict minimal MVP validator for required fields, enums, bounds, and `additionalProperties:false`; it is not claimed to be full Draft 2020-12 coverage.

Target-specific live-run artifacts that are useful as examples live under `examples/demos/`, not under canonical product docs. For example, `examples/demos/target55/` shows a short-smoke Raspberry Pi 4 evidence pack shape.

## Common Commands

```sh
adc-lab inventory --target local
adc-lab toolchain discover --target local
adc-lab observe --target local --duration 5s --signals cpu,freq,thermal,memory

adc-lab control plan --target local cpu.governor --set performance
adc-lab control apply --plan lab/runs/LAB-RUN-.../plans/PLAN-....json --dry-run
adc-lab restore --lease lab/runs/LAB-RUN-.../leases/LEASE-....json --dry-run

adc-lab load cpu --target local --workers 2 --duration 5s --abort-temp-c 75
adc-lab experiment run --target local --matrix examples/experiments/pi4_cpu_governor_smoke.yaml --dry-run
adc-lab report pack --run lab/runs/LAB-RUN-...
adc-lab health-check --target local
```

For SSH targets, `adc-lab` uses fixed `adc-lab-target` subcommands over SSH. It does not expose arbitrary remote shell.
`ADC_LAB_TARGET_RUNNER` is a development override only and must name `adc-lab-target` from an allowlisted safe path such as `/usr/local/bin/adc-lab-target` or `/home/<user>/.local/bin/adc-lab-target`.

## Verification

Use the repository command wrapper:

```sh
make verify
```

This runs format, lint, tests, strict minimal schema fixture validation, contract validation, docs smoke, and command wiring smoke. The smoke command does not by itself support resource/NFR claims.
