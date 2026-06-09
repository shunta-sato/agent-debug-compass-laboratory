# Architecture Decision Analysis: Option B Privilege Provider

## 1. Decision Question

- Deciding: how PR10 should introduce the future Option B systemd/Unix-socket
  privilege provider boundary.
- Not deciding: systemd unit contents, socket protocol framing, daemon
  lifecycle, remote privileged apply, or replacing the Option A sudo helper.

## 2. Context And Constraints

- Current state: Option A uses a fixed root-owned helper path,
  `/usr/local/libexec/adc-lab-priv-helper`, invoked by the controller only for
  typed operations.
- Constraints:
  - No Agent root shell.
  - No arbitrary helper path.
  - Privileged apply/restore remains local-target only.
  - Option B must not become active by accident.
  - Hardware-free `make verify` must remain the default gate.
- Open uncertainties:
  - final Unix socket protocol.
  - systemd unit hardening profile.
  - root-provider ownership, versioning, and upgrade procedure.
  - target-specific physical footprint of any future daemon.

## 3. Quality Drivers

| Driver | Scenario | Metric / threshold | Verification |
| --- | --- | --- | --- |
| Security boundary | Agent asks for provider status or future control | no arbitrary shell/helper path; Option B disabled by default | CLI tests and schema fixture |
| Operability | Operator reviews a lab run | provider posture is visible as an artifact and audit event | CLI test checks artifact and audit |
| Rollout safety | Option B design lands before daemon implementation | no systemd unit, socket listener, or root process added in PR10 | diff review and NFR gate |
| Maintainability | Future provider code needs a stable contract | provider kind, transport, availability, and allowed operations are typed | contract validation |
| Physical footprint | Embedded target has no new always-on work in PR10 | no target-local daemon or polling loop added | NFR report and code review |

## 4. Candidate Options

| Option | Summary | Assumptions |
| --- | --- | --- |
| A | Keep only the Option A sudo helper and defer Option B entirely | current docs are enough until implementation begins |
| B | Implement a systemd/Unix-socket provider now | root daemon hardening, protocol, and lifecycle can be reviewed in one PR |
| C | Add provider status contract now; keep Option A active and Option B planned/disabled | visible contract reduces future risk without activating a daemon |

## 5. Risk / Tradeoff Analysis

| Option | Benefits | Risks | Tradeoffs | Sensitivity Points |
| --- | --- | --- | --- | --- |
| A | smallest change; no new code path | no machine-readable provider posture; future Option B may arrive as a large risk jump | slowest route to Option B | acceptable only if provider work is not near-term |
| B | validates real transport early | large security and NFR review surface; possible root daemon before contracts mature | fastest but riskiest | depends on systemd hardening, protocol, and target footprint evidence |
| C | auditable posture, small blast radius, no privileged behavior change | status schema may need revision when daemon protocol is designed | adds a contract before implementation | good if future PR can evolve schema compatibly |

## 6. Decision

- Chosen direction: Option C.
- Rationale: PR10 should make the privilege provider boundary visible and
  reviewable before any root daemon exists. This preserves the Option A safety
  boundary while reducing the risk of a later large, ambiguous Option B change.
- Rejected options:
  - Option A only: leaves no provider artifact for audit or future migration
    checks.
  - Option B implementation now: too much security and physical-footprint risk
    before provider status, protocol, and hardening contracts are reviewed.

## 7. Verification Tasks

- Tests: add core and CLI tests for provider status, disabled Option B, artifact
  refs, and audit event.
- Benchmarks: none in PR10 because no runtime daemon is added.
- Migration checks: verify Option A helper path and apply/restore behavior are
  unchanged.
- Monitoring / observability: provider status is an audit-backed run artifact.
- Rollback / fallback: removing the CLI/status contract returns the system to
  Option A-only behavior; no target service cleanup is required.
- Dependency or boundary checks: verify no systemd unit, Unix socket server, or
  new sudo path is introduced.

## 8. Handoffs

- requirements-engineering: not required; roadmap scope is specific.
- observability: record `privilege.provider_status` audit signal.
- embedded-target-characterization: deferred; Option B runtime is not enabled.
- embedded-system-familiarization: deferred; no target capability decision is
  being made.
- embedded-nfr-design: record no new target-local runtime and block production
  physical-footprint claims.
- error-handling: disabled/planned provider is artifact state, not command
  failure.
- code-smells-and-antipatterns: not required unless implementation introduces
  broad coupling.
- quality-gate: verify architecture record, NFR report, ExecPlan, commands, and
  sensitive-data scan.
