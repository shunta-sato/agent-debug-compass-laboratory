# ExecPlan: Bounded Load and Safety Monitor

## Purpose / Big Picture

PR5 makes `adc-lab load cpu` a bounded, abortable, auditable Tier 1
experiment primitive. It does not broaden privileged control. The load command
must remain an explicit, short-lived burst with duration and worker bounds,
thermal abort support, operator abort support, and machine-readable result
evidence that distinguishes a completed load from a safety-aborted load.

The outcome is:

```text
operator command -> bounded load plan -> safety-monitored execution -> load result -> audit
```

## Scope

In scope:

- CPU load safety monitor metadata in `lab.load_plan.v1`.
- CPU load safety monitor evidence in `lab.load_result.v1`.
- Operator abort by target-local abort file for local and SSH target-runner
  execution.
- Restore-on-abort status field for CPU load, fixed to `not_required` because
  PR5 CPU load does not mutate target state.
- Built-in CPU load qualification moves from deferred/rejected to accepted for
  bounded load plan/result evidence only.
- Core and CLI tests for operator abort, duration/worker bounds, and artifact
  path hygiene.
- NFR, hot-path, observer-effect, and gate docs updated to reflect PR5 claims.

Out of scope:

- Privileged apply/restore changes.
- cpufreq write changes.
- Fixed-frequency sweep.
- Memory, I/O, GPU, NPU, or thermal stress.
- Sustained/degraded/recovery thermal experiments.
- Production physical-footprint claims.
- A real restore hook for state-changing experiments.

## Constraints / Quality Targets

- Default `make verify` remains hardware-free.
- No unbounded load: duration remains capped at 300s, workers remain capped to
  available parallelism.
- Operator abort path is runtime input only and is not persisted in
  Agent-facing JSON artifacts.
- Safety monitor result must record whether thermal surface was available,
  whether operator abort was observed, how many monitor samples ran, and what
  restore-on-abort status applies.
- No new root path, sudo helper behavior, or destructive test.
- Audit must record each load result.

## Dev Workflow Route

- Selected risk route: high.
- Why: PR5 changes target-local hot-path behavior and safety-abort contracts for
  experimental load generation.
- Required branches:
  - `execution-plans`: this crosses CLI, target runner, core contracts, schemas,
    tests, and NFR evidence.
  - `embedded-nfr-design`: bounded load has physical CPU/thermal footprint.
  - `embedded-hot-path-review`: CPU worker loop and monitor loop are hot paths.
  - `embedded-observer-effect-review`: monitor sampling can perturb the target.
  - `embedded-nfr-harness-design`: PR5 defines how a future harness can prove
    resource claims.
  - `error-handling`: abort reasons and worker failures are boundary outcomes.
  - `observability`: safety monitor evidence and audit events must be
    diagnosable.
  - `embedded-nfr-gate` and `quality-gate`: final submit readiness.
- Non-triggered branches:
  - `architecture-decision-analysis`: roadmap already chose PR5 scope.
  - `concurrency-core`: the existing worker-thread model is not redesigned; PR5
    only adds abort observation around it.

## Requirements / Acceptance

- EARS-AC1: When `adc-lab load cpu` creates a plan, the plan shall include a
  `safety_monitor` object with monitor interval, thermal abort setting,
  operator abort enablement, and restore-on-abort policy.
- EARS-AC2: When a CPU load completes or aborts, the result shall include a
  `safety_monitor` object with sample count, thermal surface availability,
  operator abort observation, and restore-on-abort status.
- EARS-AC3: If the operator abort file exists before or during the load, the
  load shall stop, return `status=aborted`, and set
  `abort_reason=operator_abort`.
- EARS-AC4: The operator abort file path shall not be serialized into
  `lab.load_plan.v1` or `lab.load_result.v1`.
- EARS-AC5: SSH target runner load shall accept the same operator abort option
  and enforce it on the target side.
- EARS-AC6: Contract validation shall pass for updated load plan/result golden
  fixtures.
- EARS-AC7: Default verification shall remain hardware-free.
- EARS-AC8: The built-in CPU load tool shall be accepted as evidence only for
  bounded load plan/result artifacts and shall not support production physical
  NFR claims.

## Context & Orientation

Key files:

- `crates/adc-lab-core/src/load.rs`: CPU load plan creation and worker/monitor
  loop.
- `crates/adc-lab-core/src/contracts.rs`: `LoadPlan` and `LoadResult`.
- `crates/adc-lab/src/main.rs`: controller CLI and SSH target-runner call.
- `crates/adc-lab-target/src/main.rs`: fixed-command non-root target runner.
- `schemas/lab.load_plan.v1.schema.json`: load plan contract.
- `schemas/lab.load_result.v1.schema.json`: load result contract.
- `crates/adc-lab/tests/cli.rs`: hardware-free CLI integration tests.
- `docs/nfr/adc-lab-target-runtime.md`: NFR matrix.
- `reports/resource/*.md`: hot-path, observer-effect, and NFR gate reports.

Existing baseline on this branch comes from merged PR4.

## Design

### Runtime Options

Keep runtime-only inputs out of Agent-facing contracts:

```text
CpuLoadRuntimeOptions {
  operator_abort_file: Option<PathBuf>
}
```

The plan stores `operator_abort_enabled=true/false`, but not the path.

### Safety Monitor Contract

`LoadPlan` gains:

```text
safety_monitor:
  sample_interval_ms: 100
  thermal_abort_c: null | number
  operator_abort_enabled: bool
  restore_on_abort: not_required
```

`LoadResult` gains:

```text
safety_monitor:
  sample_interval_ms: 100
  samples: integer
  thermal_surface_available: bool
  operator_abort_observed: bool
  restore_on_abort_status: not_required
```

`not_required` is intentional for PR5 CPU load because the operation has no
target state restore lease. Future state-changing experiment runners can add
`attempted` / `succeeded` / `failed` when a restore hook is attached.

### Abort Handling

The monitor loop checks, in order:

1. operator abort file existence
2. thermal surface and abort threshold
3. deadline

On abort, the stop flag is set, workers join, and the result records the abort
reason. Worker panic still fails the command because the execution result cannot
be trusted.

### Observability

No metrics backend is added. PR5 observability is the load result JSON plus the
existing `load.cpu` audit event.

### Test Strategy

- Core unit test: operator abort file pre-created causes `operator_abort`.
- Core unit test: completed load records safety monitor fields.
- CLI integration test: local load operator abort writes plan/result/audit and
  does not serialize the abort file path.
- Schema/golden fixture validation covers new strict fields and enums.

## Milestones

1. Add plan and runtime contract types.
2. Implement operator abort and safety monitor result collection.
3. Wire CLI and target runner.
4. Update schemas, golden fixtures, and tests.
5. Update NFR/resource docs.
6. Run verification and sensitive scan.
7. Commit, push, and open draft PR.

## Progress (WBS)

- [x] Create PR5 branch and inspect merged PR4 baseline.
- [x] Route dev workflow and create ExecPlan.
- [x] Add load safety monitor contracts.
- [x] Implement operator abort runtime support.
- [x] Wire controller CLI and target runner.
- [x] Update schemas and golden fixtures.
- [x] Add core and CLI tests.
- [x] Update built-in CPU load qualification policy.
- [x] Update NFR/resource docs and quality reports.
- [x] Run full verification and sensitive scan.
- [ ] Commit, push, open draft PR.

## Surprises & Discoveries

- PR4 already has a bounded CPU load implementation with duration, worker, and
  thermal abort checks. PR5 can focus on making safety monitor evidence explicit
  and adding operator abort.
- PR3 intentionally rejected built-in load evidence until bounded load safety
  evidence existed. PR5 closes that deferred policy for the built-in CPU load
  tool while keeping external and agent-created load tools unqualified.

## Decision Log

- 2026-06-09: Store operator abort enablement/status in contracts, but do not
  serialize the raw abort file path.
- 2026-06-09: Keep restore-on-abort as `not_required` for CPU load because PR5
  does not mutate target state.
- 2026-06-09: Accept `adc-lab-builtin-cpu-load` as builtin evidence only for
  bounded load plan/result artifacts; production NFR claims remain blocked.

## Handoff

Branch: `codex/pr5-bounded-load-safety-monitor`.

Current status: implementation, docs, verification, and sensitive scan complete;
publish next.

Run:

```sh
make verify
```

Read first:

- `crates/adc-lab-core/src/load.rs`
- `crates/adc-lab-core/src/contracts.rs`
- `crates/adc-lab/src/main.rs`
- `crates/adc-lab-target/src/main.rs`
- `crates/adc-lab/tests/cli.rs`

## Outcomes & Retrospective

- Added `safety_monitor` contract fields to `lab.load_plan.v1` and
  `lab.load_result.v1`.
- Added runtime-only operator abort file support for controller-local and SSH
  target-runner CPU load.
- Added safety monitor result evidence for samples, thermal surface
  availability, operator abort observation, and restore-on-abort status.
- Kept CPU load bounded by duration, available parallelism, optional thermal
  abort, and optional operator abort.
- Accepted `adc-lab-builtin-cpu-load` as builtin evidence only for bounded load
  plan/result artifacts.
- Verified with `cargo fmt --all`, `make contract`,
  `cargo test -p adc-lab --tests -- --nocapture`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `make verify`, and PR diff sensitive-data scans.
