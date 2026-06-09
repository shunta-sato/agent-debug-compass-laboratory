# ExecPlan: adc-lab MVP

## Purpose / Big Picture

Build `agent-debug-compass-laboratory` as `adc-lab`: a safety-gated target familiarization and experiment laboratory for AI agents building embedded and edge software.

The MVP turns ad-hoc shell probing into typed, bounded, restorable, auditable, reproducible experiments. The initial operating model is Pi5 controller to Pi4 Linux target, with adapter seams for Jetson, Android/Snapdragon, macOS, generic embedded Linux, and ROS2 robots.

## Scope

In scope:

- Rust workspace with `adc-lab`, `adc-lab-core`, `adc-lab-target`, and `adc-lab-priv-helper`.
- Strict minimal JSON Schemas and golden fixtures for the lab contracts.
- Local and SSH target inventory skeletons, passive observation, run artifact directories, and audit JSONL.
- Option A privileged helper model with typed operation JSON, no arbitrary shell, structured refusal, cpufreq governor control path, pre-state capture, restore lease, and restore command.
- Bounded CPU load generator with duration and thermal abort boundaries.
- Experiment matrix runner, report packer, claim/evidence trace, tool qualification path, and operating-point report skeletons.
- Embedded project constitution and feature-level resource evidence artifacts with experimental-only NFR gate because no real Pi4/Pi5 measurement is available in this run.
- Local verification: format, lint, unit tests, integration tests, schema fixture validation, contract validation.

Out of scope for this MVP:

- Real Pi4/Pi5 target characterization evidence.
- sudoers NOPASSWD setup.
- firmware flashing, destructive filesystem tests, unsafe radio/network experiments, or always-on production daemon behavior.
- Fully platform-specific schemas for Jetson, Android/Snapdragon, macOS, or ROS2.

## Constraints / Quality Targets

- No Agent root shell.
- No arbitrary shell, arbitrary command, arbitrary sysfs path, arbitrary file write, or arbitrary script path in privileged paths.
- Tier 2+ operations require approval artifact, pre-state capture, restore lease, verification, and audit.
- Audit-less operation output is not evidence.
- Unqualified external or agent-created tools cannot be evidence sources.
- No operating-point claim without controlled or explicitly bounded evidence.
- No production physical-footprint claim without target evidence. This MVP is experimental-only for embedded NFR claims.
- Keep the command wrapper aligned with `COMMANDS.md`: `make verify` is the canonical verification gate.

## Context & Orientation

- Current repo starts from agent instruction files only: `AGENTS.md`, `COMMANDS.md`, `PLANS.md`, `.agents/`.
- `COMMANDS.md` is initialized, but the actual Makefile and Rust workspace are absent.
- Existing `.agents` files are agent-support assets and are not modified by this implementation.
- User decisions:
  - Rust workspace is acceptable.
  - `clap`, `serde`, and `schemars`-style schema discipline are acceptable.
  - MIT license.
  - Schemas start as strict minimal skeletons with `required`, `enum`, `additionalProperties:false`, and golden fixtures.
  - PR1 verification is local only; target smoke can be placeholder.
  - `adc-lab-priv-helper` is skeleton in PR1, but the full MVP includes PR3 behavior.

## Dev-Workflow Routing

- Risk level: high.
- Why: cross-boundary project bootstrap with CLI, schemas, privileged helper, target-local observation/load, audit, restore, concurrency, and embedded physical-footprint claims.
- Escalation trigger: real target execution, sudoers changes, destructive Tier 3/4 operations, always-on runtime, or production physical-footprint claims.

Required branches:

- `execution-plans`: triggered; this file is the living ExecPlan.
- `project-initialization`: triggered by absent command wrapper despite initialized `COMMANDS.md`.
- `embedded-project-constitution`: triggered by new embedded/edge lab project.
- `embedded-nfr-design`: triggered by target-local sampler/load/runner behavior.
- `embedded-nfr-harness-design`: triggered by NFR budgets needing a smoke harness and explicit target-evidence limitations.
- `embedded-hot-path-review`: triggered by observation sampling and CPU load loops.
- `embedded-observer-effect-review`: triggered by target-local observation and measurement paths.
- `embedded-nfr-gate`: triggered before quality gate; expected decision is `experimental-only`.
- `observability`: triggered by audit and diagnosable operation results.
- `error-handling`: triggered by boundary failure translation and structured refused result contracts.
- `function-boundary-governor`: triggered by review-fix helper/API boundary changes for approval binding, target binding, restore failure handling, artifact refs, and SSH runner validation.
- `concurrency-core`: triggered by CPU load worker threads.
- `thread-safety-tooling`: triggered by shared cancellation state in multi-worker load generation; Rust safe-code limitations will be documented.
- `quality-gate`: mandatory final gate.

Skipped branches:

- `architecture-decision-analysis`: not triggered because the user chose Option A and architecture alternatives are not being compared in this run.
- `requirements-engineering`: not triggered as a separate branch because the user provided FR/NFR/PR scope and answered implementation decisions.
- `bug-investigation-and-rca`: no existing bug/regression.
- `destructive-refactor`: no existing abstraction replacement.
- `working-with-legacy-code`: no existing product code.
- UI, Android, ROS2, C++ readability: not touched in this MVP.

## Design

### Boundaries

- `adc-lab-core`: schema models, target clients, local Linux observation, policy, audit, control, restore, load, experiment, reports, and tool qualification.
- `adc-lab`: controller CLI. Creates run artifacts and writes audit events.
- `adc-lab-target`: non-root target-side fixed-command runner for local target inventory, observation, health, and bounded load.
- `adc-lab-priv-helper`: root-owned allowlisted helper. It accepts only typed plan/lease JSON and never accepts shell commands, command strings, arbitrary paths, or script paths.

### Error Handling

Relevant headings from `$error-handling`: "13. Error handling" and "13.1 Basic templates".

- Filesystem, process, schema, JSON/YAML, policy, and privilege errors translate to `LabError`.
- Privileged operation policy failures translate to `ControlResult.status=refused` with `reason_code` and message.
- CLI boundaries return human-readable errors and, where the contract requires it, structured JSON refusal.
- Missing optional target surfaces are reported as unavailable inventory/observation fields, not as panics.

### Observability

- Every CLI operation that creates or changes evidence writes an audit JSONL event with `run_id`, `target_id`, `actor`, operation, risk tier, result, policy version, and artifact refs.
- Correlation identifiers: `run_id`, `plan_id`, `approval_id`, `lease_id`, `result_id`, and `event_id`.
- Audit events are boundary-level only to avoid high-frequency target-local logging.
- Target-local sampling output is bounded by duration, sample interval, and artifact size expectations.

### Concurrency Plan

- Goal (NFR tie-in): bounded CPU load with explicit duration, worker count, thermal abort, and clean shutdown.
- Model: each CPU worker runs an isolated loop; the controller thread monitors time and thermal abort.
- Ownership & shared state map: workers share only `Arc<AtomicBool>` stop flag and local counters returned after join.
- Synchronization strategy: atomics for cancellation; no mutexes, no shared mutable buffers.
- Shutdown/cancellation strategy: monitor sets stop flag at duration, thermal abort, or error; all worker threads are joined before returning.
- Error propagation strategy: setup/monitor errors return `LabError`; worker counters are best-effort diagnostics.
- Observability: load result records worker count, duration, abort reason, maximum observed temperature, and audit result.
- Verification: unit test bounded worker completion; integration tests exercise CLI with short duration; no TSan because this is Rust safe code, not C/C++.

### Embedded NFR / Claim Boundary

- The MVP provides experimental lab tooling, not production low-overhead or battery-safe claims.
- Default target-local observation is bounded and command-triggered, not always-on.
- Resource budgets and reports explicitly mark target evidence missing.
- Host fallback verification proves CLI/schema behavior only, not Pi4/Pi5 physical footprint.

## Validation & Acceptance

Acceptance requires:

- `cargo fmt --all --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- Schema fixture validation passes for every schema file.
- Integration tests cover CLI help, inventory, toolchain discovery, control plan, helper structured refusal, bounded load short run, experiment dry-run, report pack, and tool qualification.
- `make verify` passes.
- `reports/resource/nfr-gate-report.md` exists with `Gate decision: experimental-only`.
- Quality gate returns `submit`.

## Milestones

1. Bootstrap workspace, Makefile, license, README, and command registry.
2. Add schemas, golden fixtures, core models, and contract validation.
3. Add CLI/target/local inventory, observation, audit, and run artifacts.
4. Add Option A helper and control/restore contracts.
5. Add bounded load, experiment matrix, report pack, tool qualification, and operating-point skeletons.
6. Add docs, examples, scripts, embedded constitution and NFR reports.
7. Run full verification, fix findings, update handoff, and complete quality gate.

## Progress (WBS)

- [x] Understand user decisions and set overall GOAL.
- [x] Route through mandatory workflow and create ExecPlan.
- [x] Bootstrap Rust workspace and Makefile.
- [x] Implement core contracts and schemas with golden fixtures.
- [x] Implement CLI and target runner.
- [x] Implement helper/control/restore path.
- [x] Implement load, experiment, report, and tool qualification paths.
- [x] Add embedded constitution, NFR docs/reports, adapter docs, examples, scripts.
- [x] Run verification and address failures.
- [x] Run embedded NFR gate and quality gate.
- [x] Address PR review Must-fix safety findings for privileged control, approval binding, restore failure handling, SSH runner boundary, and artifact refs.
- [x] Re-run full verification after review fixes.
- [x] Push PR update.
- [x] Address second PR review blockers: sudo helper allowlist, restore lease validation/read-back, non-executed experiment claims, external `$ref`, AGENTS/.agents mismatch, approval audit trace, bounds semantics, and SSH endpoint grammar.
- [x] Re-run full verification after second review fixes.
- [x] Push PR update after second review fixes.
- [x] Address follow-up stale-review blockers by removing public `--helper` entirely and expanding restore lease adversarial tests.
- [x] Re-run full verification after public helper override removal.
- [x] Push PR update after public helper override removal.
- [x] Address final experiment schema mismatch by adding `not_implemented` to `lab.experiment_run.v1`.
- [x] Re-run contract and full verification after experiment schema fix.
- [x] Push PR update after experiment schema fix.
- [x] Scan PR diff for sensitive data, contact identifiers, network literals, personal identifiers, and sensitive event details before merge.
- [x] Redact live LAN address, personal home path, and personal repository metadata from PR artifacts.

## Surprises & Discoveries

- `COMMANDS.md` is already marked initialized, but no `Makefile` exists yet. This implementation must make the documented targets real rather than resetting initialization.
- Skill references live under `.agents/skills/*/references/`, not repo-root `references/`.
- The `jsonschema` crate pulled transitive dependencies above local rustc 1.85 MSRV. Contract tests now use a small in-test validator for the strict MVP schema subset.
- Live target discovery found `pi4` was not resolvable, but SSH config contained `target55 -> <target-address>`; target55 is a Raspberry Pi 4 and became the measured target.
- Remote PATH did not include `adc-lab-target`, so the target runner was deployed non-root to `/home/demo/.local/bin/adc-lab-target` and controller SSH calls now support `ADC_LAB_TARGET_RUNNER`.
- PR review found this branch is no longer a PR1 skeleton: because cpufreq privileged control is implemented, merge readiness must use functional-MVP safety criteria. The branch keeps one PR but must harden privileged target binding, approval binding, restore-on-failure behavior, SSH runner path validation, and artifact URI semantics before approval.
- Second PR review found additional functional-MVP blockers: `--helper` allowed arbitrary sudo path, restore leases were not fully validated as untrusted input, non-dry experiment matrix output could imply completed/supported evidence without execution, the strict schema validator silently ignored external `$ref`, and the tracked AGENTS file referenced ignored `.agents` paths.
- Follow-up review repeated stale blocker snippets from before `9ed23af`, but correctly identified that keeping `--helper` as a public CLI option was still weaker than the project promise.
- Final review found one real contract mismatch: runtime emits experiment trial `status=not_implemented`, but `lab.experiment_run.v1.schema.json` did not include that status enum.

## Decision Log

- 2026-06-08: Use `experimental-only` embedded NFR gate for this MVP because no real Pi4/Pi5 physical measurements are available and the user allowed PR1 local-only verification.
- 2026-06-08: Keep SSH execution fixed to `adc-lab-target` subcommands through `std::process::Command`; do not expose arbitrary remote shell.
- 2026-06-08: Keep privileged helper typed around `linux.cpufreq.set_governor` for MVP; additional operations require schema and policy extension.
- 2026-06-08: Use Rust safe worker threads plus `Arc<AtomicBool>` cancellation for bounded CPU load; no C/C++ TSan path is applicable.
- 2026-06-08: Normalize SSH inventory/observe/load target ids on the controller so evidence is attributed to `target55`, not the target runner's local self-name.
- 2026-06-08: Treat PR #1 as "bootstrap + first functional MVP" instead of splitting it after the fact; adopt review Must-fix items as merge blockers.
- 2026-06-08: Use the shortest safe target-binding rule for this PR: privileged apply/restore are local-target only. Remote privileged helper execution is deferred until explicit target-local helper transport is implemented.
- 2026-06-08: Bind human approval to plan id, canonical plan digest, exact operation, and bounds so an approval cannot authorize a different plan with the same action name.
- 2026-06-08: For state-changing apply failure after pre-state capture, attempt immediate restore and return a structured failed result with restore attempt details rather than only a refusal.
- 2026-06-09: Remove public `--helper` from the controller CLI. Privileged apply/restore now use the fixed MVP helper path, and test/dev helper execution calls the helper binary directly without controller `sudo`.
- 2026-06-09: Treat restore leases as untrusted artifacts and validate policy segment, governor, optional frequencies, operation id, and restore requirement before restore, then verify restored governor state by read-back.
- 2026-06-09: Until experiment execution is wired to real control/load/observe steps, non-dry experiment runs are recorded as `not_implemented` and claims remain blocked/provisional.
- 2026-06-09: Keep control plan `bounds` as approval/experiment authorization bounds for this MVP; helper validates approval coverage, while load/experiment phases enforce duration and thermal runtime behavior.
- 2026-06-09: Expand restore validation tests to cover absolute policy paths, governors with newline payloads, bad schema version, non-numeric frequencies, and `restore_required=false`.
- 2026-06-09: Keep `not_implemented` as the explicit experiment trial status for non-dry MVP requests, and add it to the experiment-run schema instead of collapsing it into `blocked`.
- 2026-06-09: Redact PR artifacts before merge by replacing real LAN address and personal home path values with public placeholders, while keeping target55 as a demo target label.

## Handoff

- Branch/commit: `codex/adc-lab-mvp` at PR #1; experiment schema enum fix is implemented and awaiting commit/push.
- Current status: MVP implementation complete, PR opened, first and second review safety fixes applied, final schema mismatch fixed and verified locally.
- Commands run before review fixes: `make verify`, `make build-release`, target55 command smoke, target55 inventory/toolchain/observe/load/observer-on/off smoke.
- Commands run after review fixes: `make verify`, `make build-release`, `make resource-smoke`, target55 command smoke with `ADC_LAB_TARGET_RUNNER=/home/demo/.local/bin/adc-lab-target`.
- Commands run after second review fixes: `cargo test --workspace`, `make verify`, `make build-release`, `make resource-smoke`.
- Commands run after public helper override removal: `cargo test --workspace`, `make verify`, `make build-release`, `make resource-smoke`.
- Commands run after experiment schema fix: `make contract`, `make verify`, `make build-release`.
- Commands run after PII/security redaction: `make verify`, PR-diff sensitive-data scans covering contact identifiers, IPv4 literals, personal identifiers, and sensitive event terms.
- Known risks: target55 short-smoke evidence exists, but sustained thermal, wakeups, battery/power, flash/storage, jitter, degraded, and recovery evidence are still missing; production physical-footprint claims remain blocked.
- Read first: this plan, `README.md`, `crates/adc-lab-core/src/control.rs`, `crates/adc-lab/src/main.rs`, `docs/architecture/privilege-model-option-a.md`, `reports/resource/nfr-gate-report.md`.

## Outcomes & Retrospective

Implemented the adc-lab MVP bootstrap through the requested PR1-PR8 scope and followed up with live target55 characterization. The target55 pass includes non-root target-runner deployment, target inventory/toolchain, idle-only observation, bounded CPU load, observer-on/off smoke, target-specific baseline summaries, operating-envelope summaries, and updated NFR gate evidence.

Verification:

- `make verify`: pass.
- `make build-release`: pass.
- `make resource-smoke`: pass.
- target55 command smoke: previous pass; not rerun for the second review blocker update.

Quality gate:

- Gate decision: submit.
- Findings: 0.
