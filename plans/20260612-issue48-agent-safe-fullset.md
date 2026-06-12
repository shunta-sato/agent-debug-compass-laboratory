# Issue #48 Agent-Safe Full-Set Execution

## Purpose / Big Picture

Execution GOAL:

Make adc-lab's Target / Platform Operating Contract full-set collection
agent-safe and self-validating. An Agent must not need shell-level artifact
selection such as `find ... PLAN-*.json | tail -n 1` to run governor-control
evidence collection, and adc-lab must not let a final summary imply measured
governor evidence when plan, approval, apply, load, restore, and health-check
evidence do not match the requested governor.

Source issue:

- GitHub issue #48:
  <https://github.com/shunta-sato/agent-debug-compass-laboratory/issues/48>
- Issue title: "P0: Make adc-lab agent-safe full-set execution
  self-validating".
- Triggering evidence: v0.2.1 target55 full-set execution produced useful
  artifacts, but the governor-control harness selected plan / approval
  artifacts by filename order and could pair a label with the wrong controlled
  factor. adc-lab safely refused a mismatched approval, but the surrounding
  workflow still allowed an overbroad "helper apply/restore did run" summary.

## Scope

In scope:

- A high-level governor sweep workflow that owns plan / approval / apply /
  bounded load / restore / restore verification / health-check ordering.
- A structured run validator for target-operating-contract full-set runs.
- Per-governor evidence validity states with explicit evidence refs and gaps.
- Full-set summary semantics that distinguish valid measured evidence from
  refused, contaminated, insufficient, not applicable, and unknown evidence.
- `constraints check` semantics that do not fail generated constraints merely
  because the generated "Blocked claims" section names blocked claims in a
  negative context.
- Documentation and examples that teach Agents the high-level workflow and
  validator instead of filename heuristics.

Out of scope:

- Relaxing Tier 2 safety controls.
- Giving Agents a root shell or arbitrary helper path.
- Remote privileged apply / restore.
- Treating insufficient, refused, unknown, or contaminated evidence as pass.
- Proving target55 physical production suitability from hardware-free tests.

## Problem Frame

- Problem owner: adc-lab CLI and evidence-kernel workflow, not only the
  external Agent harness.
- Current pain / evidence: issue #48 documents wrong plan / approval selection
  by filename order and a summary that could overclaim governor evidence.
- Desired outcome: adc-lab itself generates or enforces the safe full-set
  execution path and emits a machine-readable validity summary.
- Solution-first risk: adding a wrapper script would preserve the fragile
  shell-harness problem; the fix belongs in typed CLI/core contracts.
- Non-goals: no safety relaxation, no root shell, no conversion of blocked
  evidence into measured evidence.
- Proceed to requirements/spec: yes.

## Requirements

| ID | Priority | Requirement | Acceptance criteria | Verification method |
|---|---|---|---|---|
| R-001 | Must | When a user requests a governor full-set sweep, adc-lab shall execute or dry-run the ordered typed workflow for each requested governor without shell-level plan / approval discovery. | No public docs or examples require `find`, `sort`, or filename-order selection for governor evidence. Each requested governor has a per-governor summary with plan, approval, apply, load, restore, and health-check refs or explicit gaps. | CLI integration tests with synthetic run dirs; docs grep; `make verify`. |
| R-002 | Must | When plan, approval, control result, restore lease, load, or health-check evidence does not match the requested governor, adc-lab shall mark that governor as not measured and explain the mismatch. | Mismatched approval plan ID, approval digest, plan desired governor, applied governor, failed/refused apply, failed restore, missing health-check, and post-failure load contamination are each detected. | Core validator unit tests using fixtures plus CLI `report validate-run` tests. |
| R-003 | Must | While summarizing a full-set run, adc-lab shall use the validity vocabulary `measured`, `measured_partial`, `insufficient`, `refused`, `contaminated`, `not_applicable`, and `unknown`. | A governor label is `measured` only when plan, approval, apply, applied state, bounded load, restore, and health-check all match the requested governor. | Snapshot/schema tests for the validation artifact; rule tests proving contaminated/refused evidence cannot support controlled-governor claims. |
| R-004 | Must | If a run contains blocked / unknown / insufficient evidence, adc-lab shall surface those gaps in a structured artifact and a human-readable `GAPS.md` or equivalent. | `report validate-run` writes a v2 validation artifact and a gaps document. Missing or invalid full-set evidence is visible without inspecting raw JSON by hand. | CLI tests and docs examples. |
| R-005 | Should | `constraints check` shall distinguish downstream candidate content from generated constraints self-checking. | Checking generated constraints does not fail solely because the generated "Blocked claims" section contains blocked claim text in a warning/negative context. Candidate-content checks still fail on unsupported positive claims. | Unit/CLI tests for both modes. |
| R-006 | Must | When a high-level governor sweep attempts real apply, adc-lab shall require approval evidence equivalent to the existing plan-review-approve-apply path. | `--approved-by` alone cannot authorize real sweep apply. Non-dry-run sweep apply requires an out-of-band preapproved sweep policy artifact or another artifact that binds target, requested governors, bounds, expiry, and approving actor before apply starts. | Safety invariant tests; CLI tests proving `--approved-by` alone refuses real apply. |
| R-007 | Must | When a bounded load is used as controlled-governor evidence, adc-lab shall record an explicit causal link to the matching control result or applied operating-point snapshot. | New load evidence produced by the sweep contains a control-result ref or applied-governor snapshot. Existing v0.2.1-style load artifacts without that link are classified as `unknown` or `contaminated`, never `measured` for a requested governor. | Core validator tests for linked and unlinked load artifacts; schema tests for the new field. |

## Constraints / Quality Targets

- Preserve the North Star: no Agent root shell, no uncontrolled experiment, no
  unapproved hard-to-restore operation, no unqualified tool evidence, no claim
  without audit.
- Reuse existing control safety semantics: approval digest binding,
  `approval_mismatch` refusal, restore lease validation, helper allowlist, and
  SSH shell-fragment refusal must stay green.
- The high-level sweep is typed orchestration over existing primitives, not a
  shell wrapper.
- A high-level sweep must not collapse human review into `--approved-by`.
  `--approved-by` is identity metadata only unless accompanied by a prior
  approval artifact that binds the sweep scope.
- The sweep must not silently continue as success after `refused`,
  `approval_mismatch`, failed apply, failed restore, or missing health-check.
- Load evidence can support controlled-governor claims only when it has an
  explicit control-result or applied-governor evidence link. Timestamp or
  filename order is not a valid causal proof.
- Hardware-free CI must verify semantics with fake fixtures/backends; live
  target55 execution is optional validation evidence, not a merge prerequisite.
- All new public artifacts must be generated-schema checked and covered by
  `make schemas-check`.
- Every new public schema-versioned artifact must receive a
  `schemas/schema-ledger.tsv` row in the same PR that introduces it.
- Docs/examples must not teach `find`, `sort`, `tail`, or filename-order
  selection for plan / approval / control artifacts. A docs-smoke or test grep
  must enforce this after the docs update lands.
- `make verify` remains the final gate for each PR.

## Context & Orientation

Relevant current code:

- `crates/adc-lab/src/main.rs`: CLI type definitions and dispatch.
- `crates/adc-lab/src/commands/control.rs`: current plan / approve / apply /
  restore CLI primitives and restore health-check persistence.
- `crates/adc-lab-core/src/control.rs`: control plan, approval matching,
  helper path validation, apply/restore state machine, and cpufreq backend.
- `crates/adc-lab/src/commands/load.rs`: bounded load v2 artifact persistence.
- `crates/adc-lab/src/commands/report.rs`: operating-contract and run-report
  command surfaces.
- `crates/adc-lab/src/commands/constraints.rs` and
  `crates/adc-lab-core/src/suitability.rs`: generated constraints and
  blocked-claim scanning.
- `crates/adc-lab-core/src/contracts.rs`: v1 control DTOs and generated-schema
  DTO source.
- `docs/reference/cli.md`, `docs/testing/resource-harness.md`, and
  `docs/getting-started/pi4-pi5-measurement-prompt.md`: Agent/operator
  command examples that must stop teaching shell artifact selection.

Current facts:

- `control apply` already refuses mismatched approval records through
  `approval_matches`.
- `restore` persists `health/restore_health_check.json` only after a restored
  result.
- `ControlResult` records `status`, `refusal`, optional restore lease, and
  restore attempt, but it does not currently expose a rich per-governor
  validation summary.
- `constraints check` currently scans candidate paths for blocked claim text
  without understanding generated constraints document structure.
- `v0.2.1` is tagged at the current planning baseline.

## Design

### Dev Workflow Route

- Risk route: high. Rationale: the plan changes claim-producing validation
  semantics and will later touch Tier 2 control orchestration.
- Default lane: required. Acceptance criteria, Test List, implementation
  economy budget, and design-balance responsibility map are recorded here.
- Required branches: `test-driven-development`, `design-balance`,
  `implementation-economy`, ExecPlan updates, and final `quality-gate`.
- Triggered but scoped in this phase: observability/audit design is recorded in
  this plan; no live target or embedded NFR measurement is claimed.
- Verification depth: focused core/CLI/schema/safety tests plus full
  `make verify` before each PR.

### Phase 0-2 Test List

- [x] mismatched approval/control result for a requested governor validates as
      `refused`, not `measured`.
- [x] applied/restored control plus unlinked v0.2.1-style load validates as
      `contaminated`.
- [x] `report validate-run` writes `report.run_validation` and `GAPS.md`.
- [x] generated schema drift and schema-ledger check include
      `report.run_validation` and `control.governor_sweep_policy`.
- [x] non-dry-run sweep self-approval refusal is captured as a safety invariant
      seed before Phase 3 implementation.

### Phase 3 Test List

- [x] non-dry-run `control governor-sweep run` with `--approved-by` but no
      approved policy refuses before helper invocation.
- [x] `control governor-sweep prepare` writes a requested sweep policy
      artifact that cannot authorize real apply.
- [x] `control governor-sweep approve` converts a requested policy into an
      approved policy with a scope-bound digest.
- [x] approved-policy dry-run sweep writes typed per-governor plan, approval,
      dry-run result, audit, validation artifact, and gaps.
- [x] policy scope mismatch (target/governor/bounds/expiry/digest) refuses
      before helper invocation.

### Phase 0-2 Responsibility Map

| Unit | Name | Responsibility sentence | Reason to change | Dependency direction |
|---|---|---|---|---|
| core module | `run_validation` | Correlate run artifacts into conservative full-set validation payloads. | Validation semantics or required evidence changes. | Depends on contracts/evidence/run context; CLI depends on it. |
| CLI command | `report validate-run` | Persist validation artifact, gaps markdown, and audit for an existing run. | User-facing report workflow changes. | Depends on core validator and command common helpers. |
| DTO payload | `RunValidationPayload` | Define the v2 validation artifact schema. | Public validation contract changes. | Schema generator and validator depend on it. |
| DTO payload | `GovernorSweepPolicyPayload` | Define future preapproved sweep policy shape. | Sweep approval scope changes. | Schema generator now; Phase 3 control workflow later. |
| load payload extension | `LoadPayload` causal fields | Carry explicit control-result or operating-point evidence links. | Controlled-load evidence model changes. | Produced by load/sweep, consumed by validator. |

Layout decision: keep validation in core and persistence in CLI. Rejected
alternative: a CLI-only validator, because operating-contract rules and future
sweeps need the same correlation semantics.

### Phase 0-2 Complexity Budget

- Changed files target: 8-14 including generated schemas and tests.
- New modules target: 1 core module (`run_validation`).
- New helpers/wrappers target: up to 4 local scan/correlation helpers inside
  the module; no generic filesystem framework.
- Production Rust budget: +350 to +550 lines; tests +120 to +220 lines.
- Abstraction rule: keep the validator data-oriented; do not introduce a
  second evidence store or bespoke query DSL.

### Phase 3 Responsibility Map

| Unit | Name | Responsibility sentence | Reason to change | Dependency direction |
|---|---|---|---|---|
| CLI command group | `control governor-sweep` | Orchestrate prepare / approve / run for a requested governor set. | Sweep workflow semantics change. | Depends on existing control/load/report helpers and core policy DTOs. |
| policy payload | `GovernorSweepPolicyPayload` | Bind target, governors, bounds, expiry, approver, and digest for sweep authorization. | Sweep approval contract changes. | Produced/validated by CLI; schema generator depends on it. |
| control command helpers | sweep helpers in `commands/control.rs` | Persist typed artifacts and enforce policy scope before helper invocation. | CLI orchestration safety changes. | Depends on existing control primitives; no new helper backend. |

Layout decision: keep Phase 3 in `commands/control.rs` instead of adding a new
command module. Rejected alternative: a separate sweep module, because the
implementation must reuse private control persistence/helper boundaries and
should not create a second control orchestration layer yet.

### Phase 3 Complexity Budget

- Changed files target: 5-9 including tests, docs, plan, and generated schema.
- New modules target: 0; keep orchestration inside existing control command
  boundary.
- New helpers target: local policy normalization/validation/persistence
  helpers only.
- Production Rust budget: +180 to +320 lines; tests +120 to +220 lines.
- Abstraction rule: no new backend abstraction until a fake non-dry-run sweep
  is needed for Phase 4 summary semantics.

### Artifact Model

Add a v2 validation artifact, tentatively:

- kind: `report.run_validation`
- default path: `reports/run_validation.v2.json`
- markdown gaps path: `reports/GAPS.md`
- profile: `target-operating-contract-fullset`

Add a v2 preapproved sweep policy artifact, tentatively:

- kind: `control.governor_sweep_policy`
- default path: `approvals/governor_sweep_policy.v2.json`
- purpose: bind the human-approved sweep scope before any non-dry-run helper
  apply.
- fields: target id, requested governors, operation bounds, expiry, approving
  actor, policy digest, and created-at timestamp.

Core payload shape should include:

- `profile`
- `requested_governors`
- `governor_results[]`
- `overall_status`
- `gaps[]`
- `evidence_refs[]`
- `audit_refs[]`

Each governor result should include:

- requested governor
- actual planned governor
- approval status and approval mismatch details
- apply status and refusal code
- applied-state validation status
- bounded-load status and contamination reason when applicable
- restore status
- health-check status
- final validity status from the required vocabulary
- evidence refs and next evidence

### Validity Vocabulary

Avoid adding a third top-level status dialect. The v2 envelope `status` remains
the artifact-level health/status contract. The validation payload may define a
payload-local `validity` enum for per-governor evidence validity because it
needs two classifications that do not map cleanly to the generic envelope
states:

- `contaminated`: evidence exists but cannot be causally tied to the requested
  operating point, or was collected after a refused/mismatched control step.
- `unknown`: the validator lacks enough explicit evidence to determine whether
  the requested operating point was active.

Mapping rule:

- payload `measured` -> envelope `Status::Measured` for an all-measured
  validation artifact;
- payload `measured_partial` / mixed non-fatal states -> envelope
  `Status::MeasuredPartial`;
- payload `insufficient` / `unknown` -> envelope `Status::Insufficient`;
- payload `not_applicable` -> envelope `Status::NotApplicable { reason }`;
- payload `refused` -> envelope `Status::Refused { code, message }` when the
  validation artifact as a whole is a refusal;
- payload `contaminated` -> envelope `Status::UnsafeBlocked` or
  `Status::Insufficient` depending on whether the run attempted an unsafe or
  untrusted claim. Phase 0 must lock this mapping in schema tests before
  implementation.

Decision rule: if the payload enum grows beyond validation-only semantics, stop
and either extend the shared envelope `Status` or reuse an existing status
instead of creating a second rule/status DSL.

### Approval Model

The sweep must remove shell artifact guessing without weakening Tier 2 human
approval. Adopt the safe default:

- `control governor-sweep` non-dry-run apply requires an out-of-band
  preapproved sweep policy artifact.
- The policy artifact binds target id, allowed governor set, operation bounds,
  expiry, approving actor, and policy digest.
- `--approved-by` alone may be used only for a prepare/review artifact or
  dry-run output. It cannot authorize real apply.
- Existing plan / approval / apply remains supported for manually reviewed
  single-step control. The sweep can internally create per-governor control
  plans, but those plans must be covered by the preapproved sweep policy before
  any helper apply.

Safety invariant seed:

- a non-dry-run sweep with only `--approved-by operator` refuses before helper
  invocation and writes a structured refusal;
- a sweep policy with the wrong target, missing governor, expired timestamp,
  narrower bounds, or mismatched digest refuses before helper invocation.

### Load / Control Causality Model

Controlled-governor load evidence must have a direct evidence link. For new
sweep output, each load artifact used as controlled evidence must include one
of:

- `control_result_ref` pointing to the matching successful applied control
  result; or
- an `operating_point_snapshot` with the applied governor and policy state
  captured immediately before the load.

Timestamp order, directory order, and file names are not causal proof. Existing
v0.2.1 runs that lack the new causal link can still be validated for
read-only/pressure evidence, but governor-specific load evidence is
`unknown` or `contaminated` unless another explicit artifact proves the control
state.

### Governor Sweep Workflow

Add a high-level command under `control`, tentatively:

```sh
adc-lab control governor-sweep prepare \
  --target local \
  --governors ondemand,performance,powersave \
  --load-workers 4 \
  --load-duration 60s \
  --abort-temp-c 75 \
  --approved-by operator \
  --out lab/runs/.../approvals/governor_sweep_policy_request.v2.json

adc-lab control governor-sweep \
  --target local \
  --governors ondemand,performance,powersave \
  --load-workers 4 \
  --load-duration 60s \
  --abort-temp-c 75 \
  --restore-after-each \
  --approval-policy lab/runs/.../approvals/governor_sweep_policy.v2.json \
  --run-dir lab/runs/...
```

Exact CLI shape may change, but it must satisfy:

- no caller-provided plan / approval filename discovery for the normal path;
- no hidden approval bypass: a real apply path requires a preapproved sweep
  policy artifact, not only `--approved-by`;
- every step writes typed artifacts and audit events;
- load runs only after the corresponding requested governor is successfully
  applied and verified, and the load artifact receives a causal control link;
- restore and health-check run after each applied governor when
  `--restore-after-each` is set;
- non-measured validity exits non-zero by default after writing the partial
  summary and gaps evidence; an explicit opt-out may be added for exploratory
  collection, but it must not change the artifact validity state.

Implementation should prefer a core orchestrator with an injectable cpufreq
backend so tests do not need real `/sys` writes or sudo.

### Run Validator

Add a report command, tentatively:

```sh
adc-lab report validate-run \
  --run lab/runs/... \
  --profile target-operating-contract-fullset \
  --expected-governors ondemand,performance,powersave \
  --json
```

The validator reads existing run artifacts, correlates by typed IDs and
logical refs, and emits `report.run_validation`. It must reject inference from
filename order. The correlation key order is:

1. requested governor from sweep request or `--expected-governors`;
2. `ControlPlan.plan_id` and desired state;
3. `ApprovalRecord.approved_plan_id` and digest;
4. `ControlResult.plan_id`, status, refusal, and restore lease;
5. restore result and restore-health audit;
6. bounded-load artifacts that are causally attached to the successful
   governor step, not merely present in the run directory.

Loads after refused or mismatched control are `contaminated` for that requested
governor unless the validator can prove they were collected under a valid
matching apply.

### Operating Contract / Summary Semantics

The validator artifact becomes the source for controlled-governor full-set
claims. A controlled governor must not become `measured` in summaries or
operating-contract claims unless the validator says that governor is measured.
If no validation artifact exists for the full-set profile, full-set controlled
governor claims remain `unknown` or `insufficient`, not inferred from raw load
or plan files.

### Constraints Check Semantics

Keep candidate-content lint strict, but stop treating generated constraints as
candidate implementation content. Preferred direction:

- keep current candidate scan as the default behavior for user-supplied paths;
- add an explicit self-check mode or command for generated constraints artifacts
  that validates structure and source linkage without flagging the generated
  blocked-claims section itself;
- update docs to say candidate checks are for downstream agent-facing content,
  not for the generated constraint artifact's own explanatory warnings.

### Observability / Audit

New operations should append audit events:

- `control.governor_sweep`
- `control.governor_sweep.step` or one event per existing primitive plus a
  sweep summary event
- `report.validate_run`

No claim-producing validation or sweep artifact is valid without an audit
event.

### Error Handling

- Refused, failed, contaminated, insufficient, and unknown states are data, not
  panics.
- Malformed JSON, symlink/escape paths, invalid schemas, and impossible
  correlations are CLI errors because evidence cannot be trusted.
- By default, non-measured requested governor validity exits non-zero after the
  validation artifact and gaps are written.
- An explicit exploratory opt-out may allow zero exit for partial collection,
  but the JSON summary and gaps must still carry the non-measured states.

## Validation & Acceptance

Required local gates for every implementation PR:

```bash
make verify
```

Additional focused gates by phase:

```bash
cargo test -p adc-lab-core --test safety_invariants -- --nocapture
cargo test -p adc-lab --test safety_invariants -- --nocapture
cargo test -p adc-lab --test cli -- --nocapture
cargo test --workspace contract_validation -- --nocapture
make schemas-check
make docs-smoke
```

Acceptance is complete only when:

- no normal full-set docs/examples use shell filename heuristics for plan /
  approval selection, and a repo check prevents those examples from returning;
- a fixture reproducing the issue #48 mismatch is marked refused or
  contaminated, never measured;
- real governor sweep apply refuses without a preapproved sweep policy artifact
  even when `--approved-by operator` is supplied;
- new sweep load artifacts carry an explicit control-result ref or applied
  governor snapshot; v0.2.1-style unlinked load artifacts are unknown or
  contaminated for controlled-governor claims;
- a successful synthetic governor sweep produces per-governor measured states;
- `report validate-run` emits a v2 validation artifact and gaps document;
- new public validation / sweep policy artifacts are generated-schema checked
  and present in `schemas/schema-ledger.tsv`;
- operating-contract/full-set summary claims consume validation state or remain
  insufficient/unknown;
- generated constraints can be self-checked without false failure, while
  candidate content still fails on blocked positive claims.

## Milestones / PR Plan

Phase 0: Characterization and contract design.

- Add issue #48 fixture(s) with mismatched plan / approval / governor labels.
- Define validation status vocabulary and schema contract.
- Lock the approval model: non-dry-run sweep apply requires preapproved sweep
  policy evidence; `--approved-by` alone cannot authorize apply.
- Lock the load/control causality model: new sweep loads require
  `control_result_ref` or applied-governor snapshot; old unlinked loads are
  unknown/contaminated for controlled-governor claims.
- Decide and test the payload `validity` to envelope `Status` mapping.
- Add failing tests for contamination, missing health-check, and generated
  constraints self-check semantics.
- Add safety invariant seeds for sweep approval refusal before helper
  invocation.
- Add schema-ledger tasks for the new validation and sweep-policy artifacts.

Phase 1: Core run-validation engine.

- Implement typed artifact scanning and correlation in core.
- Emit validation payload and gaps model from fixtures.
- Keep CLI unchanged except test-only scaffolding if possible.

Phase 2: `report validate-run` CLI.

- Add command, output artifact, gaps markdown, audit event, and docs skeleton.
- Verify issue #48 fixture through CLI.

Phase 3: `control governor-sweep` workflow.

- Add typed high-level orchestration for local governor sweeps.
- Add dry-run / fake-backend tests for safe CI coverage.
- Ensure load is skipped or contaminated after refused/mismatched control.
- Ensure non-measured requested governors produce a non-zero exit by default
  after artifacts are written.

Phase 4: Summary / operating-contract validity awareness.

- Ensure controlled-governor claims require validation artifact evidence.
- Update run report / operating-contract rule tests for refused and
  contaminated governor evidence.

Phase 5: `constraints check` mode split.

- Add generated constraints self-check semantics or an explicit documented mode.
- Preserve strict candidate-content checking.

Phase 6: Docs, examples, and release readiness.

- Update CLI reference, resource harness, Pi4/Pi5 prompt, and README pointers.
- Remove any remaining Agent instructions that require artifact filename
  guessing.
- Add a docs-smoke or test grep preventing shell filename-heuristic examples
  for plan / approval / control artifact selection from returning.
- Record final outcomes, target miss decisions, and verification evidence.

Phase risk / size forecast:

| Phase | Risk | Expected changed surface | Notes |
|---|---|---|---|
| Phase 0 | Medium | tests, contracts draft, schema ledger draft, plan | Characterization and safety decisions only; no runtime apply path change. |
| Phase 1 | Medium | core validator module, fixtures, generated schema | Read-only artifact correlation; high semantic importance but no target writes. |
| Phase 2 | Medium | report CLI, audit, docs skeleton | New claim-producing report command; must preserve no-claim-without-audit. |
| Phase 3 | High | control CLI/core orchestrator, safety invariants, load artifact linkage | Tier 2 workflow orchestration; helper invocation must remain bounded and approved. |
| Phase 4 | Medium | run-report / operating-contract rules | Claim semantics change; must remain conservative. |
| Phase 5 | Medium | constraints checker and CLI tests | Public lint semantics split; no target runtime effect. |
| Phase 6 | Low | docs/examples/checks | Documentation and regression guard. |

## Progress (WBS)

- [x] Read GitHub issue #48 and capture source problem.
- [x] Inspect current control, report, constraints, and docs surfaces.
- [x] Create this ExecPlan and execution GOAL.
- [x] Phase 0: Add characterization tests for mismatched approval and
      unlinked load evidence.
- [ ] Phase 0: Add generated constraints self-check characterization before
      Phase 5.
- [x] Phase 0: Define validation artifact schema and status vocabulary.
- [x] Phase 0: Record approval-model decision.
- [x] Phase 0: Add safety invariant seed that sweep apply cannot self-approve
      with `--approved-by` before Phase 3 implementation.
- [x] Phase 0: Record load/control causality decision and fixture old unlinked
      loads as unknown/contaminated.
- [x] Phase 0: Define generated schema / ledger rows for
      `report.run_validation` and `control.governor_sweep_policy`.
- [x] Phase 1: Implement core run-validation engine.
- [x] Phase 2: Add `report validate-run`.
- [x] Phase 3: Add `control governor-sweep`.
- [x] Phase 3: Make non-measured sweep results non-zero by default with an
      explicit exploratory opt-out only if justified.
- [ ] Phase 4: Wire validation into full-set summaries / operating-contract
      claims.
- [ ] Phase 5: Split constraints check semantics.
- [ ] Phase 6: Add docs/examples grep guard against plan / approval filename
      heuristics.
- [ ] Phase 6: Update docs/examples and close final outcomes.

## Design -> WBS Coverage Check

| Design item | WBS coverage |
|---|---|
| `report.run_validation` artifact and `GAPS.md` | Phase 0, Phase 1, Phase 2 |
| `control.governor_sweep_policy` artifact and approval semantics | Phase 0, Phase 3 |
| load/control causal evidence link | Phase 0, Phase 1, Phase 3 |
| validation payload enum to envelope `Status` mapping | Phase 0, Phase 1 |
| high-level `control governor-sweep` | Phase 3 |
| validation-aware full-set summary / operating-contract semantics | Phase 4 |
| constraints generated self-check semantics | Phase 5 |
| docs/examples replacing shell heuristics and grep guard | Phase 6 |
| audit events for new claim-producing artifacts | Phase 2, Phase 3 |

No named design deliverable is deferred.

## Surprises & Discoveries

- `gh issue view` cannot run in this environment because `gh` is not
  authenticated; issue #48 was read through the public GitHub page.
- The current checkout baseline is `27204c1`, tagged `v0.2.1`.
- `plans/_template_execplan.md` is not present on this branch, so this plan was
  created directly against the `PLANS.md` required sections.
- Existing `ControlResult` proves status and restore lease linkage but may not
  be rich enough to prove an applied-state snapshot by itself. Phase 0 must
  decide whether validator evidence can rely on existing verified apply status
  plus lease applied state, or whether the control result needs an explicit
  post-apply state snapshot.
- Plan review identified two safety-critical Phase 0 decisions: sweep approval
  semantics and load/control causal linkage. Both are now explicit design
  sections and WBS items.

## Decision Log

- 2026-06-12: Treat issue #48 as a product workflow gap rather than a harness
  bug. Rationale: adc-lab is intended to be an Agent companion; safe primitives
  are not enough if the intended workflow still requires fragile shell
  correlation.
- 2026-06-12: Plan the fix as multiple PRs instead of one broad change.
  Rationale: validator semantics, privileged orchestration, summary claims,
  and constraints checking are distinct review surfaces with different risk.
- 2026-06-12: Keep approval explicit in the high-level governor sweep.
  Rationale: a high-level workflow must remove artifact guessing without
  weakening Tier 2 human approval and restore requirements.
- 2026-06-12: Prefer a generated validation artifact over prose-only summary.
  Rationale: downstream Agents and reports need machine-readable validity and
  gaps, not only human text.
- 2026-06-12: Adopt preapproved sweep policy as the only non-dry-run
  high-level sweep authorization path. Rationale: `--approved-by` is an
  Agent-supplied identity string and is not equivalent to human review of
  generated plans; the sweep must not self-approve Tier 2 apply operations.
- 2026-06-12: Require explicit load/control causal evidence for controlled
  load claims. Rationale: timestamp and filename order repeat the issue #48
  failure mode; old unlinked v0.2.1 load artifacts must classify as
  unknown/contaminated for governor-specific claims.
- 2026-06-12: Keep envelope `Status` authoritative and use a payload-local
  validation enum only for per-governor validity. Rationale: `contaminated` and
  `unknown` are validation-specific, but the artifact envelope should not fork
  into another top-level status dialect.
- 2026-06-12: Default sweep exit semantics to non-zero when requested governor
  evidence is non-measured. Rationale: issue #48 is about misuse resistance;
  Agents often key off exit code, so the safe default must fail closed after
  writing validation artifacts.
- 2026-06-12: Ship Phase 0-2 as the first implementation PR and defer the
  sweep command safety invariant seed to the first Phase 3 commit. Rationale:
  the invariant should bind the real `control governor-sweep` surface; adding
  a placeholder test before the command exists would only prove that clap does
  not recognize the future subcommand.
- 2026-06-12: Make `report validate-run` fail closed by default for
  non-measured requested governors while still writing the validation artifact,
  gaps markdown, and audit event first. Rationale: Agents may rely on exit
  status, but reviewers still need durable failure evidence.
- 2026-06-12: Implement sweep approval as a prepare / approve / run workflow.
  Rationale: this preserves a separate human approval step while removing
  filename-order artifact discovery from the normal Agent workflow.
- 2026-06-12: Require an approved sweep policy for the initial dry-run sweep
  implementation too. Rationale: the dry-run path still generates per-governor
  approval artifacts and should exercise the same scope-bound policy contract;
  a looser exploratory mode can be added later without changing real-apply
  safety.
- 2026-06-12: Attempt restore after a successful real apply even when the
  following load step errors. Rationale: restore is safety-critical and must
  not be skipped by a later Tier 1 load failure.
- 2026-06-12: Require `--restore-after-each` for real governor sweep runs.
  Rationale: repeated governor changes without per-step restore can make the
  original operating point ambiguous across the sweep.

## Handoff

- Branch: `codex/issue48-governor-sweep`, stacked on
  `codex/issue48-agent-safe-fullset-plan`.
- Baseline: `27204c1` (`origin/main`, tagged `v0.2.1`).
- Current status: Phase 0-3 implementation is in progress across stacked
  branches. PR #49 contains Phase 0-2. This branch adds Phase 3 governor sweep
  prepare / approve / run on top of that base.
- Untracked local files were present before this plan and were not staged:
  `.DS_Store`, `._.DS_Store`,
  `plans/._20260611-v21-kernel-completion.md`,
  `reports/._20260611-planning-skills-improvement-proposal.md`,
  `reports/._20260611-v2-evidence-kernel-outcome-review.md`,
  `reports/20260611-planning-skills-improvement-proposal.md`, and
  `reports/20260611-v2-evidence-kernel-outcome-review.md`.
- Next steps:
  1. Finish Phase 3 verification with `make verify`.
  2. Open a stacked Phase 3 PR targeting
     `codex/issue48-agent-safe-fullset-plan`.
  3. After PR #49 lands, retarget the Phase 3 PR to `main` and rerun
     `make verify`.
- Read first when resuming:
  - this plan,
  - issue #48,
  - `crates/adc-lab-core/src/control.rs`,
  - `crates/adc-lab/src/commands/control.rs`,
  - `crates/adc-lab-core/src/suitability.rs`,
  - `docs/testing/resource-harness.md`.

## Outcomes & Retrospective

Phase 0-2 implementation snapshot, 2026-06-12:

- Implemented a read-only `run_validation` core module that correlates
  control plan, approval, control result, load, restore, health-check, and
  audit refs without filename-order inference.
- Added per-governor validity states: `measured`, `measured_partial`,
  `insufficient`, `refused`, `contaminated`, `not_applicable`, and `unknown`.
- Added `control_result_ref` and `operating_point_snapshot` to v2 load payloads
  so future sweep-produced loads can prove controlled-governor causality.
- Added `report validate-run`, defaulting to fail-closed for non-measured
  governor evidence after writing `reports/run_validation.v2.json`,
  `reports/GAPS.md`, and a `report.validate_run` audit event.
- Added generated schemas and ledger rows for `report.run_validation` and
  `control.governor_sweep_policy`.
- Implementation economy actuals: the new `run_validation.rs` is larger than
  the initial line estimate because the characterization fixtures live beside
  the validator and cover refused, unlinked, mismatched-link, and measured
  paths. Accepted for this PR because the module remains data-oriented, has no
  new filesystem framework, and is the reusable base for Phase 3/4 instead of
  a CLI-only wrapper.
- Verification so far:
  - `cargo test -p adc-lab-core run_validation -- --nocapture`: pass.
  - `cargo test -p adc-lab --test cli report_validate_run_writes_artifact_gaps_and_fails_closed_for_non_measured -- --nocapture`: pass.
  - `python3 scripts/schema/check-schema-ledger.py --enforce-final`: pass.
  - `cargo clippy --workspace --all-targets -- -D warnings`: pass.
  - `make verify`: pass.
- Deferred by decision:
  - generated constraints self-check characterization remains for Phase 5;
  - Phase 4 summary/operating-contract consumption of validation state remains
    for the next PR.

Phase 3 implementation snapshot, 2026-06-12:

- Added `control governor-sweep prepare`, `approve`, and `run`.
- `prepare` writes a requested `control.governor_sweep_policy` artifact.
- `approve` converts the requested policy into an approved, scope-digested
  policy artifact.
- `run` refuses real apply without an approved policy even if `--approved-by`
  is supplied, validates target/governor/bounds/expiry/digest before creating
  per-governor artifacts, and writes validation/gaps after the sweep.
- Dry-run sweep uses the approved policy to create typed per-governor plan,
  approval, dry-run result, audit, validation artifact, and gaps without
  invoking the privileged helper.
- Verification so far:
  - `cargo test -p adc-lab --test safety_invariants contract_validation_governor_sweep_cannot_self_approve_real_apply -- --nocapture`: pass.
  - `cargo test -p adc-lab --test cli governor_sweep -- --nocapture`: pass.
  - `cargo test -p adc-lab-core run_validation -- --nocapture`: pass.
  - `make schemas-check`: pass.
  - `cargo clippy --workspace --all-targets -- -D warnings`: pass.
  - `make verify`: pass.
