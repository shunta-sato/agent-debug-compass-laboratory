# Quality Gate Report

Gate decision: submit

Findings: 0

## Checks Run

- `cargo check --workspace` - pass.
- `cargo test --workspace contract_validation -- --nocapture` - pass.
- `cargo test -p adc-lab --test cli -- --nocapture` - pass.
- `make verify` - pass.
- `git diff --check` - pass.

## Live Discovery

- Repo command surface: `COMMANDS.md` defines `make verify` as the canonical
  gate.
- Existing implementation surfaces inspected:
  `crates/adc-lab/src/main.rs`, `crates/adc-lab-target/src/main.rs`,
  `crates/adc-lab-core/src/contracts.rs`, `crates/adc-lab-core/src/report.rs`,
  `crates/adc-lab-core/src/load.rs`, `crates/adc-lab-core/src/observe.rs`,
  `crates/adc-lab-core/src/target.rs`, and `crates/adc-lab-core/src/run.rs`.
- Schema/golden validation path inspected:
  `crates/adc-lab-core/tests/contract_validation.rs`, `schemas/`, and
  `tests/golden/`.
- Target55 state checked live:
  target55 is `aarch64`, Raspberry Pi 4 Model B Rev 1.5, Debian 13. Existing
  `/home/satoshun/.local/bin/adc-lab-target` version `0.1.11` was preserved.
- Review-fix staged runner used for live Pi4 execution:
  `/home/satoshun/.local/share/adc-lab/runners/20260610-platform-contract-review/adc-lab-target`
  version `0.1.0`, release build from this worktree.
- Target55 evidence path:
  `lab/runs/LAB-RUN-target55-platform-contract-review-20260610`.
- Review artifact zip:
  `/mnt/share/target55-platform-contract-review-20260610.zip`
  (`sha256=557f9706c17a2ce87631a6aa4804334ff4ff108ad1a705a73290a0f06dab7f2b`).

## Triggered Branch Evidence

- ExecPlan - present:
  `plans/20260610-platform-operating-contract-discovery.md`.
- Requirements engineering - present:
  requirements and acceptance criteria recorded in the ExecPlan.
- Embedded system familiarization - present:
  `docs/targets/target55/system-familiarization.md`.
- Embedded NFR design and gate - present:
  `docs/nfr/adc-lab-target-runtime.md`,
  `requirements/nfr/adc-lab-target-runtime.yaml`, and
  `reports/resource/nfr-gate-report.md`.
- Hot-path review - present:
  `reports/resource/hot-path-review.md`.
- Observer-effect review - present:
  `reports/resource/observer-effect-review.md`.
- Harness design - present:
  `docs/testing/resource-harness.md`.
- Function-boundary decision - present:
  `.agents/design-ledger/function-boundaries.md`.
- Observability - present:
  audit events and run artifacts for `pressure.run` and
  `report.target_operating_contract`; existing observability plan remains the
  architectural signal reference.

## Exit Criteria Review

- New contracts exist with schemas and golden fixtures:
  `lab.platform_mechanism_inventory.v1`, `lab.boundary_probe_plan.v1`,
  `lab.resource_pressure_result.v1`, `lab.resource_coupling_report.v1`, and
  `lab.target_operating_contract.v1`.
- New pressure probes are fixed commands, not arbitrary shell. They are bounded
  by duration and byte ceilings and record cleanup status.
- `unsupported_by_adc_lab` is rejected by schema tests and did not appear in
  target55 run artifacts.
- Target55 Pi4 live run produced pressure results for CPU, thermal, memory,
  storage, network, latency/jitter, and observer pressure.
- Target55 generated a conservative Target Operating Contract with
  `contract_status=insufficient`; pressure artifacts are explicitly classified
  as smoke/current-condition/counter-only where appropriate.
- Resource coupling report chains are `coupling_evidence_class=ingredients_only`
  and `status=insufficient` until composite or phased pressure scenarios are
  implemented and run.
- Memory pressure effect was not observed in the 8MiB target55 allocation smoke,
  so resident-memory and memory/storage coupling claims remain blocked.
- Network I/O without endpoint was classified as `network_mode=counter_only`,
  `not_applicable_with_reason`, and not a network boundary measurement.
- cpufreq sysfs visibility is classified as a visible control surface only; it
  does not become `platform_control_status=measured_partial` unless approved
  apply/restore/health artifacts exist in the same run.
- The change remains experimental-only for embedded NFR purposes. Production,
  battery-safe, flash-safe, thermally-safe, low-overhead, suitability, and
  real-time-ish claims remain blocked unless further evidence is collected.
- Pi5 live evidence is not claimed; Pi5 remains API/schema-supported and
  required pending execution.

## Gate Decision

Submit. The change adds bounded Platform Operating Contract discovery surfaces
and target55 Pi4 smoke evidence while preserving claim boundaries: the generated
contract remains insufficient until pressure effects, bounded network transfer,
and composite coupling evidence exist.
