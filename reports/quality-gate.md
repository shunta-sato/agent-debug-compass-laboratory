# Quality Gate Report

Gate decision: submit

Findings: 0

## Checks Run

- `make contract` - pass after experiment schema enum fix.
- `make verify` - pass after experiment schema enum fix.
- `make verify` - pass after PII/security redaction.
- `make build-release` - pass after experiment schema enum fix.
- `cargo test --workspace` - pass after public helper override removal.
- `make resource-smoke` - pass after public helper override removal; compatibility alias for command wiring only, `resource_metrics_collected=false`.
- PR-diff sensitive-data scan - pass after redaction; no high-risk findings remained. Residual matches are generic sudo and approval design text.
- `ADC_LAB_TARGET_RUNNER=/home/demo/.local/bin/adc-lab-target timeout 20s scripts/resource/run-resource-smoke.sh --target ssh://target55` - previous pass; not rerun for public helper override removal because this update is local/controller contract hardening.

## Live Discovery

- Rust toolchain: `cargo 1.85.0`, `rustc 1.85.0`.
- Repo command wrapper: `Makefile` targets implemented and verified through `make verify`.
- Schema/config paths: `schemas/*.schema.json`, `tests/golden/*.valid.json`, `COMMANDS.md`.
- Target connection: `ssh://target55`, SSH config maps `target55` to `<target-address>`.
- Target runner: non-root deploy to `/home/demo/.local/bin/adc-lab-target`.
- Target identity: Raspberry Pi 4 Model B Rev 1.5, Debian GNU/Linux 13 (trixie), kernel `6.12.75+rpt-rpi-v8`, aarch64.
- Public redaction placeholders: `<target-address>` and `/home/demo/...` replace live local environment details.

## Triggered Branch Evidence

- ExecPlan - present: `plans/20260608-adc-lab-mvp.md`.
- Project initialization / command wrapper - present: `Makefile`, `COMMANDS.md`.
- Embedded project constitution - present: `docs/00_project_principles.md`, `docs/architecture/resource-discipline.md`, `requirements/physical_budgets.yaml`, `target_profiles/raspberry_pi_4.yaml`, `docs/testing/resource-harness.md`.
- Embedded system familiarization - present: `examples/demos/target55/docs/system-familiarization.md`.
- Embedded target characterization - present: `examples/demos/target55/docs/target-characterization.md`, `examples/demos/target55/target_profile.yaml`, `examples/demos/target55/reports/target-characterization.json`.
- Embedded operating envelope - present: `examples/demos/target55/docs/operating-envelope.md`, `examples/demos/target55/reports/operating-envelope/*.json`.
- Feature-level embedded NFR gate - present: `reports/resource/nfr-gate-report.md` with `experimental-only` decision.
- Hot-path review - present: `reports/resource/hot-path-review.md`.
- Observer-effect review - present: `reports/resource/observer-effect-review.md`.
- Observability - present: `docs/architecture/observability-plan.md`.
- Error handling - present: `docs/architecture/error-handling.md`.
- Function-boundary review - present: `reports/architecture/function-boundary-review.md`.
- Concurrency/thread-safety - present: `reports/concurrency/thread-safety-matrix.md`.

## Exit Criteria Review

- No arbitrary shell or arbitrary sysfs path is accepted by the privileged helper contract.
- The controller CLI exposes no public `--helper` override; privileged apply/restore invoke only the fixed MVP helper path.
- Tier 2 cpufreq control requires approval bound to plan id/digest/operation/bounds, local-target helper binding, restore, typed plan validation, and restore read-back verification.
- Restore leases are validated as untrusted input before writes: schema version, policy segment, governor, optional frequencies, operation id, and restore requirement.
- Audit artifacts are created for evidence-producing controller operations, and Tier 2 apply audit records the submitted approval artifact ref.
- SSH target endpoints reject option/shell injection shapes before invoking `ssh`.
- Non-dry experiment matrix output is `not_implemented` and cannot produce supported claims until real execution exists.
- `lab.experiment_run.v1` now includes `not_implemented` in `trials[].status`, with a contract test covering the non-dry MVP output.
- Report pack artifact refs use bounded `artifact://lab/runs/<run_id>/<relative_path>` refs rather than raw filesystem paths.
- Tool qualification keeps agent-created tools out of evidence until qualification evidence exists.
- Target55 short-smoke claims are supported by live target artifacts.
- Command smoke is explicitly not treated as resource evidence.
- No low-overhead, battery-safe, flash-safe, thermally-safe, or production-ready target-runtime claim remains without measurement evidence; target-runtime NFR status remains experimental-only for production claims.
