# Quality Gate Report

Gate decision: submit

Findings: 0

## Checks Run

- `cargo fmt --all` - pass.
- `make contract` - pass.
- `cargo test -p adc-lab --tests -- --nocapture` - pass.
- `cargo clippy --workspace --all-targets -- -D warnings` - pass.
- `cargo test --workspace` - pass.
- `make verify` - pass.

## Live Discovery

- Rust toolchain: verified through `make verify` with workspace build, fmt,
  clippy, tests, contract validation, docs smoke, and command smoke.
- Repo command wrapper: `Makefile` remains the canonical command surface.
- Schema/config paths: `schemas/lab.approval_record.v1.schema.json`,
  `schemas/lab.control_result.v1.schema.json`,
  `schemas/lab.restore_lease.v1.schema.json`,
  `schemas/lab.health_check.v1.schema.json`, and matching golden fixtures.
- Target connection state: no hardware target required for default
  verification. PR4 tests use generated local plans and `--dry-run` for
  privileged apply/restore paths.
- Artifact/log paths expected from PR4 workflow: `approvals/*.json`,
  `plans/*.result.json`, `leases/*.json`, `health/restore_health_check.json`
  when restore succeeds, and `audit.jsonl` operations `control.approve`,
  `control.apply`, `restore`, and `health-check.restore`.

## Triggered Branch Evidence

- ExecPlan - present:
  `plans/20260609-pr4-local-cpufreq-control-mvp.md`.
- Error handling - present: `docs/architecture/error-handling.md` documents
  approval generation refusal and post-restore health-check semantics.
- Observability - present: `docs/architecture/observability-plan.md` includes
  `control.approve` and `health-check.restore` signals.

## Exit Criteria Review

- `adc-lab control approve` generates `lab.approval_record.v1` from an existing
  plan, not from free-form operation input.
- Approval artifacts are bound to plan id, canonical plan digest, exact
  operation, bounds, target id, risk tier, restore requirement, and human
  approver id.
- Non-local target plans are refused by `control approve`; no approval artifact
  is written.
- `control apply --dry-run --approval ...` records bounded approval artifact refs
  in audit.
- Successful controller restore writes a read-only `lab.health_check.v1`
  artifact and `health-check.restore` audit event.
- Restore dry-run and failed/refused restore do not produce post-restore health
  evidence.
- Default verification remains hardware-free.
- PR4 adds no remote privileged apply, arbitrary helper override, sudoers
  change, fixed-frequency sweep, load generation, or destructive experiment
  behavior.
