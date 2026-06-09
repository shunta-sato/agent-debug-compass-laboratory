# ExecPlan: Workload Profile And Target Capability Profile

## Purpose / Big Picture

PR11 adds the contract layer that lets adc-lab describe "what workload was
asked of the target" and "what that target has actually demonstrated" in the
same evidence format.

This PR does not decide whether Pi4 is sufficient, whether Pi5 is required, or
which target should be selected. It creates the measuring-stick artifacts that
later comparison and suitability-decision PRs can consume.

## Scope

In scope:

- `lab.workload_profile.v1` contract, schema, golden fixture, and examples.
- `lab.target_capability_profile.v1` contract, schema, golden fixture, and
  examples.
- Target capability profile generation from existing run artifacts.
- CLI report command that writes target capability profile artifacts and audit.
- Pi4 and Pi5 demo profiles in the same schema format, explicitly
  short-smoke/exploratory and not selection evidence.
- Documentation, NFR, observability, and quality-gate updates.

Out of scope:

- Pi4 vs Pi5 comparison decision.
- Suitability decisions such as "Pi4 is sufficient" or "Pi5 is required".
- Long sustained benchmark or production physical-footprint proof.
- New privileged control, root helper behavior, Option B provider runtime, or
  destructive experiment.
- New target-local default background runtime.

## Constraints / Quality Targets

- No Agent root shell.
- No arbitrary shell, helper, adapter, or destructive experiment surface.
- No new privileged control in PR11.
- Generated profiles must keep `selection_ready=false`.
- Short smoke and exploratory evidence must not support production, battery,
  flash, sustained thermal, all-operating-points, or target-selection claims.
- Agent-facing evidence refs must use `artifact://lab/runs/...`, not raw local
  paths.
- `make verify` must remain hardware-free.

## Context & Orientation

Relevant files:

- `crates/adc-lab-core/src/contracts.rs`: serialized DTOs.
- `crates/adc-lab-core/src/report.rs`: existing run pack, operating-point
  coverage, and capability-cost reports.
- `crates/adc-lab-core/src/observe.rs`: observation result samples.
- `crates/adc-lab-core/src/load.rs`: bounded CPU load result contract.
- `crates/adc-lab/src/main.rs`: CLI command dispatch, artifact writes, audit.
- `schemas/` and `tests/golden/`: strict minimal contract validation.
- `examples/demos/target55/`: Pi4 short-smoke normalized summaries.

Discovery notes:

- `plans/_template_execplan.md` is absent; this plan follows `PLANS.md`
  required sections directly.
- PR10 is merged at `a331654`.
- PR6 already supports a narrow non-privileged real matrix for
  `cpu_load_workers`; PR11 can normalize existing load/observe artifacts but
  should not broaden execution behavior.

## Dev Workflow Route

- Risk level: high.
- Why: PR11 adds new public evidence contracts and report behavior used by
  future target-selection workflows.
- Triggered branches:
  - `architecture-decision-analysis`: workload/profile split vs extending
    capability-cost or adding comparison now affects future contract ownership.
  - `function-boundary-governor`: new DTOs, generator, CLI command, and report
    side effects.
  - `error-handling`: malformed workload profiles and malformed run artifacts
    must fail as validation errors rather than produce misleading profiles.
  - `observability`: generated profile artifacts require audit events.
  - `embedded-nfr-design`: physical/NFR claims are represented in profiles, so
    production and selection claims must stay blocked.
  - `embedded-nfr-gate`: required because embedded NFR design is triggered.
  - `quality-gate`: final gate.
- Explicitly not triggered:
  - bug RCA: no regression or incident is under investigation.
  - destructive refactor: no flawed abstraction replacement.
  - concurrency/thread-safety: no new threads, async work, daemon, or socket.
  - UI/C++/Android/ROS2/staged lowering/legacy: not applicable.

## Design

### Contracts

`lab.workload_profile.v1` defines workload identity, class, duration,
requirements, measurement requirements, and a conservative claim boundary.

`lab.target_capability_profile.v1` links a target to a workload and evidence
pack, records observed short-smoke results, lists evidence refs, and separates
supported short-smoke claims from blocked target-selection and production
claims.

### Generator

The generator reads existing run artifacts only:

- `run_manifest.json`
- `observations/observe.json`
- `loads/*.result.json`
- `experiments/trials/*/load_result.json`
- `experiments/trials/*/observation.json`

Missing evidence produces an explicit insufficient profile. Malformed JSON is a
validation error. No target command, load, observe, control, or helper is run by
the profile generator.

### CLI

Add:

```sh
adc-lab report capability-profile \
  --run LAB-RUN-... \
  --target-id target55 \
  --workload examples/workloads/bounded_cpu_load_2_workers_60s.json \
  --json
```

The command writes:

```text
reports/target_capability_profile.<workload_id>.json
```

and appends a Tier 0 audit event:

```text
operation=report.target_capability_profile
operation_id=<workload_id>
result=<capability_status>
```

### Error Handling

- Invalid workload JSON: CLI failure with JSON parse/validation context.
- Missing run artifacts: profile generated with `insufficient_for_selection`,
  `selection_ready=false`, empty/partial observed results, and blocked claims.
- Malformed run artifact: validation error, because producing a profile from
  corrupted evidence would weaken the contract.

Relevant error-handling reference heading: "13. Error handling". The
implementation keeps the happy path as "read workload -> build profile ->
write/audit"; filesystem/JSON failures are translated at the boundary with
context.

### Observability

The profile artifact and audit event are the diagnosable signal. No metrics,
traces, or target-local logs are added.

### Tests

Test list:

- schema fixture validates both new contracts.
- schema rejects unknown workload claim boundary.
- schema rejects unknown target capability status.
- core generator keeps `selection_ready=false` and blocks selection claims.
- core generator extracts bounded load and passive observation metrics from
  existing artifacts.
- CLI writes profile artifact with bounded artifact refs and audit.
- `make verify` remains hardware-free.

## Milestones

1. Add plan and architecture decision record.
2. Add DTOs, schemas, golden fixtures, and examples.
3. Add profile generator and core tests.
4. Add CLI command, artifact write, audit, and CLI tests.
5. Update docs, NFR artifacts, and reports.
6. Run full verification and sensitive-data scan.
7. Commit, push, and open a draft PR.

## Progress (WBS)

- [x] Sync main and create PR11 branch.
- [x] Inspect current contract, observe, load, report, CLI, and target55 demo
      surfaces.
- [x] Route dev workflow and create ExecPlan.
- [x] Add architecture decision record.
- [x] Add core DTOs, schemas, fixtures, and examples.
- [x] Add target capability profile generator.
- [x] Add CLI command and audit.
- [x] Add tests.
- [x] Update docs, NFR, observability, and report artifacts.
- [x] Run verification.
- [x] Run sensitive-data scan.
- [ ] Commit, push, and open draft PR.

## Surprises & Discoveries

- `plans/_template_execplan.md` is referenced by the playbook but is not present
  in this repository.
- Existing target55 demo artifacts are normalized summaries derived from ignored
  raw lab runs; PR11 examples must stay clearly demo/short-smoke.
- Pi5 evidence is not present in the repository, so PR11 must not invent Pi5
  measurements.
- Workload duration is part of the profile contract. Target55's existing 10s
  short-smoke data must not be represented as satisfying the 60s workload
  example, so target capability profiles now record observed load and
  observation durations.

## Decision Log

- 2026-06-09: PR11 will add WorkloadProfile and TargetCapabilityProfile as
  separate contracts, not extend CapabilityCostModel and not add comparison or
  suitability decisions. Rationale: workload requirements and target observed
  behavior need stable identities before any apples-to-apples comparison can be
  credible.

## Validation & Acceptance

Acceptance criteria:

- `lab.workload_profile.v1` defines workload requirements, measurement
  requirements, duration, safety bounds, and claim boundary.
- `lab.target_capability_profile.v1` links target, workload, evidence refs,
  observed results, supported claims, blocked claims, and next evidence needed.
- Pi4 and Pi5 example profiles use the same schema format.
- Profiles label evidence as exploratory/short-smoke and keep selection
  decisions blocked.
- Unsupported claims such as "Pi4 is sufficient", "Pi5 is required", "battery
  safe", "sustained production ready", and "all operating points measured" are
  blocked unless evidence exists.
- `make verify` passes.
- No arbitrary shell, new privileged control surface, or destructive experiment
  is added.

## Handoff

- Branch: `codex/pr11-workload-target-capability-profile`.
- Base commit: `a331654`.
- Current status: implementation, full verification, diff check, and
  sensitive-data scan are complete; PR publishing remains.
- Next steps: commit, push, and open draft PR.
- Expected verification: `cargo fmt --all --check`, targeted core/CLI/contract
  tests, `make contract`, `make verify`, `git diff --check`, sensitive-data
  scan.
- Current verification: targeted core/CLI/schema tests, `make contract`, and
  `make verify`, `git diff --check`, and sensitive-data scan passed.

## Outcomes & Retrospective

PR11 now defines workload profiles and target capability profiles as a
controller-side evidence normalization layer. The implementation keeps
selection readiness false, records observed duration, and blocks Pi4/Pi5
selection claims until later comparison and suitability-decision contracts exist.
