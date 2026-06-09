# ExecPlan: Option B Privilege Provider Status Skeleton

## Purpose / Big Picture

PR10 introduces an auditable privilege-provider boundary for the future
systemd/Unix-socket Option B design without enabling a root daemon, systemd
unit, socket listener, or new privileged transport.

The goal is to make the current privilege posture machine-readable:

- Option A sudo helper remains the only active provider.
- Option B systemd/Unix-socket provider is represented as planned and disabled.
- Provider status is a Tier 0 report artifact with audit.
- No privileged behavior changes in this PR.

## Scope

In scope:

- `lab.privilege_provider_status.v1` contract, schema, and golden fixture.
- Core provider status model and builder.
- CLI command that writes provider status into a run artifact and audit.
- Architecture decision record comparing Option A, Option B, and a no-daemon
  status-only step.
- Documentation and NFR evidence updates showing Option B is not active.
- Tests for contract validation and CLI artifact/audit output.

Out of scope:

- systemd service or socket unit.
- Unix-domain socket server/client.
- root daemon process.
- remote privileged apply.
- NOPASSWD sudoers changes.
- cpufreq control behavior changes.
- new target-local default runtime.

## Constraints / Quality Targets

- No Agent root shell.
- No arbitrary helper path or command execution.
- No new privileged runtime in PR10.
- Option B must be visibly disabled by default.
- Provider status artifacts must use `artifact://lab/runs/...` refs.
- Provider status command is Tier 0 read-only reporting.
- `make verify` must remain hardware-free.

## Context & Orientation

Relevant files:

- `crates/adc-lab-core/src/control.rs`: Option A fixed helper constants and
  helper path validation.
- `crates/adc-lab/src/main.rs`: CLI command dispatch, run artifacts, audit.
- `crates/adc-lab-core/src/contracts.rs`: serialized DTOs.
- `schemas/` and `tests/golden/`: strict minimal contract fixtures.
- `docs/architecture/privilege-model-option-a.md`: current Option A model.
- `docs/architecture/safety-model.md`: risk-tier policy.
- `docs/architecture/observability-plan.md`: audit signal expectations.

Discovery notes:

- `plans/_template_execplan.md` is still absent; this plan follows `PLANS.md`
  required sections directly.
- PR9 main is merged at `be91b13`.
- Current controller apply/restore still uses only
  `/usr/local/libexec/adc-lab-priv-helper`.

## Dev Workflow Route

- Risk level: high.
- Why: privilege architecture and safety claims are cross-boundary, even though
  PR10 avoids a runtime daemon.
- Triggered branches:
  - `architecture-decision-analysis`: Option A vs Option B vs status-only
    staging affects security, operability, rollout, and physical-footprint
    risk.
  - `function-boundary-governor`: new core DTO/builder and CLI command
    boundary.
  - `error-handling`: provider unavailable/disabled status must be domain data,
    not ambiguous command failure.
  - `observability`: provider status must be auditable.
  - `embedded-nfr-design`: Option B describes a future target-local root
    provider; PR10 must record that no runtime is enabled.
  - `embedded-nfr-gate`: required because embedded NFR design is triggered.
  - `quality-gate`: final gate.
- Explicitly not triggered:
  - concurrency/thread-safety: no daemon, socket server, threads, or async work.
  - bug RCA: no regression under investigation.
  - destructive refactor: no replacement migration.
  - UI/C++/Android/ROS2/staged lowering/legacy: not applicable.

## Design

### Contract

Add `PrivilegeProviderStatus` with:

- active provider id.
- provider list.
- provider kind enum.
- availability enum.
- transport enum.
- root boundary description.
- endpoint description.
- allowed operations.
- approval/audit/restore booleans.
- default-enabled flag.
- safety notes.

The Option B provider entry is present, but has:

- `availability = "planned_disabled"`
- `default_enabled = false`
- no operations allowed yet
- endpoint path as a design identifier, not an active socket claim

### CLI

Add:

```sh
adc-lab privilege provider-status --target local --json
```

It creates or opens a run, writes:

```text
privilege/privilege_provider_status.json
```

and appends a Tier 0 audit event:

```text
operation=privilege.provider_status
result=recorded
```

### Error Handling

- Invalid target uses existing `TargetSpec::parse` errors.
- Option B disabled is represented in the artifact, not as a CLI failure.
- Artifact write and audit errors use existing filesystem context.

### Observability

Provider status is observable through the run artifact plus audit event. No
metrics/traces are added, and no target-local logging is introduced.

### Tests

Test list:

- schema fixture validates `lab.privilege_provider_status.v1`.
- schema rejects unknown provider availability.
- core provider status marks Option A active and Option B disabled.
- CLI writes provider status artifact and audit.
- CLI output uses artifact refs and does not include raw run paths in
  Agent-facing refs.

## Milestones

1. Add plan and architecture decision record.
2. Add core DTOs, provider status builder, schema, and fixture.
3. Add CLI command, artifact write, and audit event.
4. Add docs/NFR/report updates.
5. Run full verification and sensitive-data scan.
6. Commit, push, and open a draft PR.

## Progress (WBS)

- [x] Sync main and create PR10 branch.
- [x] Inspect current control, CLI, contract, audit, and NFR surfaces.
- [x] Route dev workflow and create ExecPlan.
- [x] Add architecture decision record.
- [x] Implement provider status core contract.
- [x] Add schema and fixture.
- [x] Add CLI command and tests.
- [x] Update architecture docs, safety docs, observability, error handling, and
      NFR artifacts.
- [x] Run verification.
- [x] Run sensitive-data scan.
- [x] Commit and open draft PR.

## Surprises & Discoveries

- `plans/_template_execplan.md` is referenced by the playbook but is not present
  in this repository.
- PR6 already moved experiment matrix beyond dry-run for a narrow non-privileged
  `cpu_load_workers` subset, so PR10 must not assume experiment execution is
  planning-only.
- The existing quality gate report is a rolling report and was updated from
  PR9 to PR10 rather than creating a separate file.

## Decision Log

- 2026-06-09: PR10 will be status/contract only for Option B. Rationale:
  introducing a root daemon or Unix socket before the provider status contract
  is auditable would skip the safety boundary this phase is meant to establish.

## Validation & Acceptance

Acceptance criteria:

- `adc-lab privilege provider-status --target local --json` writes a provider
  status artifact and audit event.
- The artifact says Option A is active and Option B is planned/disabled.
- Contract fixture validation covers the new schema.
- No new privileged apply/restore transport is enabled.
- Full `make verify` passes.
- Sensitive-data scan over the PR diff finds no API keys, passwords, IP
  addresses, email addresses, personal names, or incident details beyond
  existing allowlisted product identifiers.

## Handoff

- Branch: `codex/pr10-option-b-privilege-provider`.
- Commit: `11f6fa7` before this handoff update.
- Draft PR: https://github.com/shunta-sato/agent-debug-compass-laboratory/pull/10.
- Current status: implementation, verification, push, and draft PR creation are
  complete.
- Next steps: review PR #10 and merge when approved.
- Expected verification: `cargo fmt --all`, targeted core/CLI tests,
  `make contract`, `make verify`, `git diff --check`, sensitive-data scan.
- Current verification: `cargo fmt --all --check`, targeted privilege core/CLI
  tests, `make contract`, `make verify`, `git diff --check`, and
  high-confidence sensitive-data scan passed.

## Outcomes & Retrospective

PR10 now exposes provider posture as contract-backed Tier 0 evidence. Option B
is visible to agents and operators as planned-disabled, while Option A remains
the only active privileged transport.
