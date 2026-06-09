# Quality Gate Report

Gate decision: submit

Findings: 0

## Checks Run

- `make contract` - pass.
- `cargo test --workspace` - pass.
- `make verify` - pass.
- `TARGET=local DURATION=0s RUN_DIR=<temp>/LAB-RUN-readonly-script-smoke scripts/targets/pi5-to-pi4-readonly-familiarization.sh` - pass; emitted run manifest ref, familiarization pack ref, and `observational_read_only` status.

## Live Discovery

- Rust toolchain: verified through `make verify` with workspace build, fmt, clippy, tests, contract validation, docs smoke, and command smoke.
- Repo command wrapper: `Makefile` remains the canonical command surface.
- Schema/config paths: `schemas/lab.run_manifest.v1.schema.json`, `schemas/lab.familiarization_pack.v1.schema.json`, `tests/golden/*.valid.json`.
- Target connection state: no hardware target required for default verification. Pi5-to-Pi4 read-only smoke is documented and runnable only when `TARGET` is provided.
- Artifact/log paths expected from read-only familiarization: `run_manifest.json`, `reports/familiarization_pack.json`, `reports/claim_evidence_trace.json`, `inventory/target_inventory.json`, `toolchain/toolchain_inventory.json`, `observations/observe.json`, and `audit.jsonl`.

## Triggered Branch Evidence

- ExecPlan - present: `plans/20260609-pr2-run-manifest-readonly-familiarization.md`.
- Observability - present: `docs/architecture/observability-plan.md` updated with `familiarize.read_only`, `report.claim_trace`, `run_manifest.write`, and `report.pack` audit signals.

## Exit Criteria Review

- `lab.run_manifest.v1` exists with strict minimal schema and golden fixture.
- `lab.familiarization_pack.v1` now records `pack_status`, supported claims, blocked claims, and next evidence needed.
- `adc-lab familiarize read-only` generates a single audited read-only run with manifest, pack, claim trace, inventory, toolchain inventory, and passive observation.
- Artifact references in generated value objects use bounded `artifact://lab/runs/...` refs.
- Audit events are emitted for inventory, toolchain discovery, observe, claim trace generation, run manifest write, and pack generation.
- Claim trace blocks fixed-frequency, load, thermal-safety, battery/flash-safety, low-overhead, and production-readiness claims.
- Default verification remains hardware-free.
- PR2 adds no privileged control, sudo apply, cpufreq write, CPU load generation, or destructive experiment behavior.
