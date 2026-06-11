# ExecPlan: Local Workload Suitability and Constraint Loop v1

## Purpose / Big Picture

Close the principal architect loop for adc-lab on a local target first:

```text
target evidence + local workload demand profile
  -> suitability decision with margins and unknowns
  -> design constraint pack
  -> agent-facing Markdown constraints
  -> minimal constraint check
```

The first live validation target is target55/Pi4, executed as a target-local
command. SSH may be used only to stage binaries, invoke the target-local
`adc-lab` command, and retrieve artifacts. `workload run --target ssh://...`
must return a structured refusal in v1.

## Scope

In scope:

- `adc-lab workload run --target local --plan <plan.yaml>`.
- Structured refusal for `adc-lab workload run --target ssh://...`.
- `lab.workload_run_plan.v1`.
- `lab.workload_run_result.v1`.
- `lab.workload_demand_profile.v1`.
- Process-scoped demand measurement from `/proc/<pid>/stat`,
  `/proc/<pid>/status`, and `/proc/<pid>/io`.
- Target-conditioned response separated from workload demand.
- System-wide context separated from process demand.
- `lab.suitability_policy.v1`.
- `adc-lab decide suitability`.
- `lab.suitability_decision.v1`.
- Decision input reads target run artifacts, target contract, workload demand,
  and policy.
- `adc-lab constraints generate`.
- `lab.design_constraint_pack.v1`.
- Agent-facing Markdown constraints.
- `adc-lab constraints check`.
- Safe bounded representative workload fixture.
- Schemas, golden fixtures, contract tests, CLI tests, docs, README updates.
- target55 target-local Pi4 validation.
- `make verify`.

Out of scope:

- Remote workload execution over SSH.
- Pi4 vs Pi5 comparison.
- Final target selection across devices.
- Production certification.
- Arbitrary shell workload execution.
- Privileged workload execution.
- Full static analyzer.
- Full real application benchmark suite.
- Android/Jetson/Snapdragon/Mac adapters.
- Power meter, GPU/NPU/DSP integration.

## Constraints / Quality Targets

- Preserve adc-lab as a safety-gated laboratory, not an arbitrary shell wrapper.
- `workload run` v1 is local-target only.
- No shell command string, `sh -c`, or `bash -c` workload plan path.
- Executable identity must record path, working directory, sha256 when
  available, setuid/world-writable checks, and environment policy.
- Process demand must come from process-scoped procfs metrics when available.
- System-wide metrics must not be represented as workload demand.
- Thermal is target-conditioned response, not portable workload demand.
- Suitability decisions read numeric run artifacts plus contract rules; they do
  not infer numbers from target contract prose.
- Unknown cannot become meet by policy.
- Failed/aborted workload demand cannot produce suitability meet.
- Constraints must be emitted as JSON and Markdown suitable for AGENTS.md /
  CLAUDE.md / Codex prompt use.
- `make verify` is the final gate.

## Context & Orientation

Relevant files:

- `crates/adc-lab-core/src/contracts.rs`: DTOs and serialized schemas.
- `crates/adc-lab-core/src/platform_contract.rs`: current pressure/contract
  evidence helpers.
- `crates/adc-lab-core/src/load.rs`: bounded CPU load safety monitor patterns.
- `crates/adc-lab/src/main.rs`: CLI, artifact writes, audit events.
- `crates/adc-lab/tests/cli.rs`: CLI integration tests.
- `schemas/` and `tests/golden/`: JSON schemas and fixtures.
- `examples/workloads/`: existing workload profile examples.
- `docs/reference/cli.md`, `docs/architecture/workload-and-capability-profiles.md`.
- `docs/targets/target55/system-familiarization.md`.
- `reports/resource/nfr-gate-report.md`.

Current facts:

- This branch is stacked on PR #27 (`codex/pi4-reference-pack-live-target55`)
  while PR #27 is still open.
- PR #27 added multi-run operating-contract aggregation, composite
  memory/storage/jitter evidence, and endpoint-backed bounded network transfer.
- target55 has `adc-lab` v0.1.16 installed and helper readiness was previously
  confirmed, but this PR should not require privileged workload execution.
- Existing `lab.workload_profile.v1` describes workload requirements but does
  not execute or measure a workload.
- Existing `report capability-profile` is conservative and does not make
  suitability decisions.

Unknowns:

- The exact process tree aggregation scope for v1. Default decision: parent
  process only, with child accounting explicitly marked unsupported.
- Whether target55 will have the newest branch binary installed during live
  validation. Default decision: stage a user-local debug/release binary without
  replacing the installed release.

## Dev Workflow Route

- Risk level: high.
- Why: new CLI surfaces, new executable runner, process and filesystem I/O,
  procfs measurement, target-local runtime behavior, suitability decisions, and
  agent-facing constraints.
- Required branches:
  - `execution-plans`: this plan.
  - `embedded-system-familiarization`: target evidence and constraints matter.
  - `working-with-legacy-code`: process/time/filesystem/procfs nondeterminism.
  - `function-boundary-governor`: new runner/decision/constraint API surfaces.
  - `error-handling`: failed/aborted/refused semantics and unknown policy.
  - `observability`: artifacts and audit events.
  - `embedded-nfr-design`, `embedded-hot-path-review`,
    `embedded-observer-effect-review`, `embedded-nfr-harness-design`, and
    `embedded-nfr-gate`: target-local workload measurement and observer effect
    are experimental physical-footprint evidence.
  - `quality-gate`: final submission gate.
- Not triggered:
  - Concurrency/thread-safety: v1 can avoid background threads by using bounded
    polling and nonblocking child state checks.
  - UI/C++/Android/ROS/staged-lowering: not applicable.
  - destructive refactor: additive vertical slice.

## Design

### Contracts

Add DTOs and schemas:

- `WorkloadRunPlan`
- `WorkloadRunResult`
- `WorkloadDemandProfile`
- `SuitabilityPolicy`
- `SuitabilityDecision`
- `DesignConstraintPack`

Keep plan YAML-compatible with serde while artifacts are JSON.

### Workload Runner

Implement local-only runner in core:

- Validate target is local.
- Validate executable path:
  - absolute or canonicalized path,
  - no shell executables as plan command,
  - optional sha256 check,
  - reject setuid when requested,
  - reject world-writable.
- Spawn process without a shell.
- Capture stdout/stderr to bounded files.
- Poll `/proc/<pid>/stat`, `/proc/<pid>/status`, `/proc/<pid>/io`.
- Poll system context and thermal/frequency summaries.
- Enforce duration, thermal abort when readable, and optional operator abort
  file.
- Always produce `workload_run_result`.
- Produce degraded `workload_demand_profile` for failed/aborted/refused.

### Suitability Decision

Read:

- `--target-run` for numeric evidence.
- `--target-contract` for rules, blocked claims, unknowns.
- `--workload-demand` for process demand and target-conditioned response.
- `--policy` for required dimensions, thresholds, and margins.

Dimensions for v1:

- CPU
- thermal
- memory
- storage_io
- network_io
- latency_jitter

Required unknown dimension makes `selection_ready=false`.
Unknown never becomes meet.
Thermal is always target-conditioned and non-portable.

### Constraints

Generate:

- `design_constraint_pack.json`
- `agent_constraints.md`

Minimal check:

- Read pack.
- Scan text files under `--path`.
- Fail when blocked claim text appears.
- Keep this as a minimal lint, not a full static analyzer.

## Validation & Acceptance

Test list:

- Schema fixtures validate.
- Workload run rejects SSH target with structured refusal.
- Workload run rejects shell command plan.
- Workload run captures process-scoped CPU/RSS/io for a completed local
  workload.
- Workload result/profile are generated for failed/aborted/refused paths.
- Demand profile separates workload demand from target-conditioned response and
  system context.
- Thermal decision is target-conditioned and non-portable.
- Policy cannot make unknown meet.
- Required unknown dimension yields `selection_ready=false`.
- Failed/aborted demand cannot yield overall meet.
- Constraints generate writes JSON and Markdown.
- Constraints check fails on blocked claim fixture.
- target55 local validation produces `execution_mode=target_local` evidence.
- `make verify` passes.

## Milestones

1. Explore existing schema/CLI/persistence patterns and create this ExecPlan.
2. Add schemas/golden fixtures and DTOs.
3. Implement workload runner and CLI persistence/audit.
4. Implement suitability policy/decision.
5. Implement constraints generate/check.
6. Add examples and docs.
7. Run local tests, target55 target-local validation, package artifacts.
8. Run final verification and open PR.

## Progress (WBS)

- [x] User clarified v1 local-only execution and safe representative workload.
- [x] Created stacked branch `codex/workload-suitability-loop-local`.
- [x] Created this ExecPlan.
- [x] Add schemas and fixtures.
- [x] Implement DTOs and core workload runner.
- [x] Implement CLI commands and artifact/audit persistence.
- [x] Add suitability policy/decision logic.
- [x] Add constraints generation/check.
- [x] Add examples/docs.
- [x] Run local targeted tests.
- [x] Run target55 target-local validation.
- [x] Package target55 artifacts.
- [x] Run `make verify`.
- [ ] Open PR.

## Surprises & Discoveries

- Existing `lab.workload_profile.v1` is a requirement/profile layer, not an
  executable workload runner. The v1 loop adds separate run/result/demand
  contracts instead of expanding that older profile.
- The local runner records direct-child process demand only. Child process-tree
  aggregation remains explicitly unsupported in `data_quality` for v1.
- `decide suitability` scans target run JSON artifacts for numeric evidence and
  uses the target operating contract for blocked rules/unknowns, avoiding
  numeric inference from contract prose.

## Verification Log

- `cargo check` passed after DTO/CLI/core additions.
- `cargo test -p adc-lab-core` passed: 66 unit tests and 25 contract
  validation tests.
- `cargo test -p adc-lab --test cli` passed: 39 CLI integration tests.
- Built current branch `adc-lab` release binary and staged it on target55 at
  `/home/satoshun/.local/share/adc-lab/runners/20260611-workload-suitability/adc-lab`
  without replacing the installed release binary.
- target55 target-local validation run:
  `lab/runs/LAB-RUN-target55-local-workload-suitability-20260611T015207Z`.
  `workload_run_result.status=completed`,
  `execution_mode=target_local`, `target_id=target55`.
- SSH workload transport refusal run:
  `lab/runs/LAB-RUN-workload-ssh-refusal-20260611T015207Z`.
  `workload_run_result.status=refused`,
  `reason=remote_workload_execution_not_supported_in_v1`.
- Suitability decision for the representative workload against the Pi4
  reference evidence pack:
  `overall_decision=fail`, `selection_ready=false` because the representative
  CPU demand exceeded the default policy threshold; thermal and memory met the
  v1 policy margins, while storage/network/latency remain unknown optional
  dimensions.
- Packaged artifacts:
  `/mnt/share/target55-local-workload-suitability-20260611.zip`
  (`sha256=bb039e3eeeb36184973b676edaac2aede32fab45b7435d90914b6a075c9f0572`);
  `unzip -t` passed.
- `make verify` passed after build, fmt check, clippy with `-D warnings`,
  workspace lib/tests, contract validation, docs smoke, and command smoke.

- PR #27 is still open at plan creation time, so this work is a stacked branch
  unless #27 merges before final PR publication.

## Decision Log

- 2026-06-11: Keep workload execution local-only in v1. Rationale: forwarding
  executable + args over SSH would violate the fixed-command target runner
  boundary and approach arbitrary remote command execution.
- 2026-06-11: Use a safe representative workload fixture when no application
  workload is supplied. Rationale: acceptance needs end-to-end measured demand,
  but claims must remain exploratory and non-production.
- 2026-06-11: Mark child process aggregation unsupported in v1 unless the
  implementation can safely aggregate children without races. Rationale:
  parent process demand is enough for the vertical slice and avoids false
  precision.

## Handoff

- Branch: `codex/workload-suitability-loop-local`.
- Base: stacked on PR #27 branch while PR #27 is open.
- Current status: plan created; implementation not started.
- Next steps:
  1. Add DTOs/schemas/goldens.
  2. Add workload runner tests and implementation.
  3. Wire CLI commands.

## Outcomes & Retrospective

Pending.
