# ExecPlan: Local cpufreq Governor Control MVP

## Purpose / Big Picture

PR4 makes Option A privileged cpufreq governor control usable as a complete
local-only typed workflow:

```text
control plan -> approval artifact -> apply through fixed helper -> restore -> health check
```

The current code already has the safety-critical core: allowlisted
`linux.cpufreq.set_governor`, local-target-only apply/restore, fixed helper
path, approval matching, pre-state capture, restore lease, restore validation,
and read-back verification. PR4 should not broaden privilege. It should close
the operator workflow gaps so approval and restore follow-up are first-class
artifacts instead of hand-written JSON or implicit next steps.

## Scope

In scope:

- `adc-lab control approve --plan <PLAN.json> --approved-by <id>` command.
- Approval artifact generation bound to plan id, canonical plan digest, exact
  operation, target, risk tier, restore requirement, and bounds.
- Audit event for approval artifact generation.
- Restore command writes a post-restore `health-check` artifact and audit event
  when restore succeeds.
- README / architecture docs / getting started updates for the local-only
  plan/approve/apply/restore workflow.
- CLI tests that keep the workflow hardware-free by using `--dry-run` for
  privileged application.

Out of scope:

- Remote privileged apply.
- New sudoers or NOPASSWD behavior.
- Arbitrary helper path override.
- Fixed-frequency sweep.
- Thermal/load experiment execution.
- Real root writes in default verification.

## Constraints / Quality Targets

- Default `make verify` remains hardware-free.
- No Agent root shell and no arbitrary `sudo <path>` escape hatch.
- Approval artifacts are generated from a plan, not free-form operation input.
- Approval generation must reject non-local plans for this MVP, matching apply.
- Apply/restore behavior stays structured JSON with `ControlResult`.
- Restore health check must be diagnostic evidence only; it must not turn a
  failed restore into success.
- Artifact refs must stay bounded as `artifact://lab/runs/...`.

## Dev Workflow Route

- Selected risk route: high.
- Why: this touches privileged target control workflow and user-visible failure
  contracts around approval/apply/restore.
- Required branches:
  - `execution-plans`: complex cross-command workflow; this plan is required.
  - `error-handling`: approval and restore boundary failures must translate to
    structured domain outcomes.
  - `observability`: approval and post-restore health-check artifacts need audit
    signals.
- Non-triggered branches:
  - `architecture-decision-analysis`: Option A and local-only scope are already
    decided.
  - `concurrency-*`: no concurrency changes.
  - embedded NFR design/harness/gate: no new target-local loop, polling,
    always-on behavior, load, or physical budget claim is added.

Relevant error-handling reference headings: "13. Error handling" and "13.1
Basic templates". Boundary translation should keep happy paths visible and turn
filesystem/JSON/control policy failures into domain errors or structured
results.

## Requirements / Acceptance

- EARS-AC1: When an operator runs `adc-lab control approve --plan PLAN.json
  --approved-by operator`, the CLI shall write a `lab.approval_record.v1`
  artifact under the run's `approvals/` directory.
- EARS-AC2: When the approval is generated, it shall include the plan id,
  canonical plan digest, exact operation, plan bounds, target id, risk tier,
  restore requirement, approved actions, and human approver id.
- EARS-AC3: If the plan target is not `local-target`, `control approve` shall
  refuse with a CLI error and shall not create an approval artifact.
- EARS-AC4: When `control apply --dry-run --approval <approval>` is run, audit
  shall include the bounded approval artifact ref.
- EARS-AC5: When `restore` returns `restored`, the controller shall run a
  read-only local health check and store it as a run artifact with an audit
  event.
- EARS-AC6: If restore fails or is only dry-run, health check may be omitted and
  the restore result remains the source of truth.

## Context & Orientation

Key files:

- `crates/adc-lab-core/src/control.rs`: plan validation, approval matching,
  helper path validation, cpufreq backend, apply/restore state machine.
- `crates/adc-lab/src/main.rs`: CLI commands, run artifacts, audit events,
  helper invocation.
- `crates/adc-lab-priv-helper/src/main.rs`: root-side typed apply/restore entry.
- `crates/adc-lab/tests/cli.rs`: hardware-free CLI workflow tests.
- `schemas/lab.approval_record.v1.schema.json`: approval contract.
- `docs/architecture/privilege-model-option-a.md`: privilege boundary.
- `docs/architecture/error-handling.md`: boundary translation rules.
- `docs/architecture/observability-plan.md`: audit and artifact signals.

Existing baseline verified with `make verify` on 2026-06-09 before PR4 edits.

## Design

### Approval generation

Add `ControlCommand::Approve`:

```text
adc-lab control approve --plan <PLAN.json> --approved-by <operator-id>
```

The command reads a `ControlPlan`, validates it with `validate_control_plan`,
refuses non-`local-target`, computes `canonical_plan_digest`, constructs an
`ApprovalRecord`, writes it under `approvals/`, emits `control.approve`, and
prints `ArtifactOutput<ApprovalRecord>`.

It does not accept arbitrary operation fields. The plan remains the only source
for approved operation and bounds.

### Restore health check

After `command_restore` persists a `ControlResult`, if the result status is
`Restored` and the target is `local-target`, run the existing read-only
`health_check(&TargetSpec::parse("local")?)` and write:

```text
health/restore_health_check.json
```

Then append audit operation `health-check.restore`. If the health check itself
fails, the CLI should return an error after the restore result has been
persisted and audited; it must not rewrite the restore result to success or
failure.

### Observability

No metrics/tracing backend is added. Audit JSONL remains the signal. New
operation names:

- `control.approve`
- `health-check.restore`

### Test Strategy

- CLI test: plan -> approve -> dry-run apply, asserting approval artifact,
  approval digest, apply dry-run, and approval audit ref.
- CLI test: remote plan approval is refused and no approval artifact is written.
- CLI test: restore dry-run does not write restore health check.
- Unit/contract tests remain in existing control module for approval matching,
  restore validation, and restore read-back.

## Milestones

1. Add approval generation helper and CLI.
2. Add restore-success health check artifact wiring.
3. Extend CLI tests and docs.
4. Run full verification and sensitive data scan.
5. Commit, push, and open a draft PR.

## Progress (WBS)

- [x] Create PR4 branch and inspect merged PR3 baseline.
- [x] Route dev workflow and create ExecPlan.
- [x] Add approval artifact generation.
- [x] Add restore health-check artifact/audit wiring.
- [x] Update tests.
- [x] Update docs and quality gate report.
- [x] Run full verification and sensitive scan.
- [ ] Commit, push, open draft PR.

## Surprises & Discoveries

- The core cpufreq apply/restore state machine is already present in `main`.
  PR4 can focus on operator workflow completeness and post-restore evidence
  instead of broadening privileged write behavior.
- Run directories pre-create `approvals/`, so the remote approval refusal test
  asserts no approval files were written rather than asserting the directory
  does not exist.
- The restore golden fixture is intentionally remote-target-shaped, so
  hardware-free local restore dry-run tests use a temporary local-target lease.

## Decision Log

- 2026-06-09: Keep PR4 local-only. Remote privileged transport remains deferred.
- 2026-06-09: Add approval generation from plan rather than accepting free-form
  approval input fields.
- 2026-06-09: Post-restore health check is diagnostic evidence; it does not
  alter the persisted `ControlResult`.

## Handoff

Branch: `codex/pr4-local-cpufreq-control`.

Current status: implementation complete and verified; publish next.

Run:

```sh
make verify
```

Read first:

- `crates/adc-lab/src/main.rs`
- `crates/adc-lab-core/src/control.rs`
- `crates/adc-lab/tests/cli.rs`
- `docs/architecture/privilege-model-option-a.md`

## Outcomes & Retrospective

- Added `adc-lab control approve` so approval records are generated from an
  existing plan and bound to plan digest, exact operation, bounds, target, and
  human approver id.
- Added post-restore health-check artifact/audit wiring for successful
  controller restores.
- Added `lab.health_check.v1` schema and golden fixture.
