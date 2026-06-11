# ExecPlan: adc-lab v2.1 Evidence Kernel Completion

## Purpose / Big Picture

This plan follows the outcome review in
`reports/20260611-v2-evidence-kernel-outcome-review.md`.

The v2 evidence-kernel refactor reached its qualitative goal: common
extensions now flow through typed evidence artifacts, claim catalog entries,
rule rows, generated schemas, and safety-invariant tests. The unfinished work
is narrower than the original quantitative targets:

- Some v1 public report/probe/suitability surfaces remain active.
- Some schemas are still manually maintained even when the Rust DTO is the real
  source of truth.
- `main.rs` still owns too much command implementation.
- Documentation was compressed, but not normalized to the four-document shape
  described by the earlier design.

The goal of v2.1 is to finish those concrete leftovers without redefining the
project into a whole-repository rewrite.

## Scope

In scope:

- Implement `rules/run_report.rs` and make `report.run` the v2 replacement for
  familiarization pack, claim evidence trace, and operating point coverage.
- Finish probe-output cutover so load, pressure, and composite command outputs
  no longer publish v1 result payloads as their primary public contract.
- Retire the v1 suitability/constraints projection where the public CLI can
  consume and produce v2 artifacts instead.
- Split `adc-lab/src/main.rs` into command modules until `main.rs` is parse and
  dispatch oriented.
- Convert remaining active schemas from "handwritten source" to generated
  snapshots, using compatibility checks before safety-critical schemas move.
- Normalize docs into the intended normative set, with redirects or index
  pointers for retained reference material.
- Add budget checks that prevent renewed growth in the largest files.

Out of scope:

- Whole-repository 40-50% LoC reduction. The outcome review shows this is not
  reachable from the evidence-kernel scope.
- Rewriting `control`, `adc-lab-target`, or `adc-lab-priv-helper` semantics.
- Relaxing approval, restore, helper allowlist, SSH quoting, or target runner
  restrictions.
- Claiming Pi4/Pi5 target readiness, production readiness, battery safety, or
  low overhead from the refactor itself.
- Deleting active typed safety plans merely to reduce line count.

## Constraints / Quality Targets

- Keep `adc-lab` a safety-gated experiment laboratory, not a shell wrapper.
- Preserve the North Star: no Agent root shell, no uncontrolled experiment, no
  unapproved hard-to-restore operation, no unqualified tool evidence, no claim
  without audit.
- Keep all existing safety invariant tests green in every phase.
- Use v2 artifacts as the public report/probe/suitability contract only after
  CLI regression tests cover the complete loop.
- Do not remove a v1 schema until either its public producer is deleted or a
  generated snapshot compatibility test proves the replacement schema.
- Treat LoC as a guardrail, not a primary success metric.

Reframed quantitative targets:

- Primary success: the extension-cost invariants from v2 remain mechanically
  checked:
  - pressure-kind extension hand-edited files <= 3,
  - blocked claim addition is one catalog entry plus expectation changes,
  - report behavior changes through rule rows, not bespoke generators,
  - generated schema drift is checked by `make verify`.
- v2.1 LoC expectation: reduce current Rust total from 17,939 to roughly
  16,400-17,000 if Phases 1-3 land and Phases 4-5 add their expected
  structure/schema-test overhead. A 15,500-16,000 landing is a stretch outcome,
  not an acceptance target.
- File budgets after Phase 4:
  - `crates/adc-lab/src/main.rs` <= 800 lines,
  - `crates/adc-lab-core/src/report.rs` <= 900 lines or deleted,
  - no non-exempt Rust source file > 1,500 lines.
- File-budget scope:
  - non-exempt files are hand-authored production Rust files under
    `crates/**/src/**/*.rs`;
  - excluded files are `tests/`, examples, generated files, fixtures, and
    committed snapshots;
  - Phase 0 budget output is informational only;
  - after a budget is enforced, any over-budget production file fails unless
    the same PR records a temporary exemption in this plan with owner, reason,
    and expiry phase.
- `crates/adc-lab-core/src/platform_contract.rs` starts at 1,496 lines, four
  lines below the proposed 1,500-line budget. Until pressure runtime is split,
  any edit that pushes it over budget must either reduce it in the same PR or
  add an explicit temporary exemption.
- Schema target: "maintained-by-hand schemas = 0" means every remaining schema
  in source control is generated or mechanically checked against generated
  output. It does not require deleting active v1 wire contracts before their
  replacements exist.

Phase contribution forecast:

| Phase | Expected LoC effect | Schema effect | Notes |
|---|---:|---:|---|
| Phase 1 report.run | -600 to -900 | -3 if all producers move | Includes report and experiment claim-trace producers. |
| Phase 2 probe cutover | -300 to -500 | -3 to -4 | `load_plan` may become generated rather than deleted. |
| Phase 3 suitability/constraints | -200 to -400 | -2 to -3 | `suitability_policy` may remain as generated input schema. |
| Phase 4 CLI module split | -100 to +100 | 0 | Better boundaries may add small module overhead. |
| Phase 5 generated schemas | +200 to +400 | maintained-by-hand -> 0 | Derives, adapters, and compatibility tests add code. |
| Phase 6 docs | 0 | 0 | Documentation-only. |
| Total forecast | -900 to -1,500 | maintained-by-hand -> 0 | Expected landing: 16,400-17,000 Rust lines. |

## Context & Orientation

Current baseline from `origin/main` after PR #40:

- Commit: `01f9085`.
- Total Rust lines under `crates/`: 17,939.
- Rust test lines under `crates/**/tests`: about 4,044.
- Handwritten top-level schemas: 32.
- Generated v2 schemas: 9.
- Large files:
  - `crates/adc-lab/src/main.rs`: 2,595 lines.
  - `crates/adc-lab-core/src/report.rs`: 1,527 lines.
  - `crates/adc-lab-core/src/platform_contract.rs`: 1,496 lines.
  - `crates/adc-lab-core/src/contracts.rs`: 1,321 lines.
  - `crates/adc-lab-core/src/suitability.rs`: 943 lines.
  - `crates/adc-lab-core/src/control.rs`: 725 lines.

The outcome review classifies the remaining schema surface:

- In-scope execution gaps:
  - `lab.familiarization_pack.v1.schema.json`
  - `lab.claim_evidence_trace.v1.schema.json`
  - `lab.operating_point_coverage.v1.schema.json`
  - `lab.load_plan.v1.schema.json`
  - `lab.load_result.v1.schema.json`
  - `lab.resource_pressure_result.v1.schema.json`
  - `lab.composite_boundary_result.v1.schema.json`
  - `lab.suitability_decision.v1.schema.json`
  - `lab.suitability_policy.v1.schema.json`
  - `lab.design_constraint_pack.v1.schema.json`
- Active but not previously assigned to a v2 migration phase:
  - control, privilege, qualification, toolchain, target inventory,
    experiment, workload, audit, health check, release manifest, and run
    manifest schemas.

Wire contracts currently found in code without a corresponding top-level schema
file:

- `lab.observation_result.v1`
- `lab.run_context.v1`
- `lab.build_info.v1`
- `lab.constraint_check_result.v1`
- `lab.workload_fixture_result.v1`

Phase 0 classification must include both schema files and schema-versioned wire
contracts with no schema file, so "maintained-by-hand = 0" cannot hide
untracked contracts.

Relevant files to read before implementation:

- `reports/20260611-v2-evidence-kernel-outcome-review.md`
- `plans/20260611-v2-evidence-kernel.md`
- `crates/adc-lab/src/main.rs`
- `crates/adc-lab/src/commands/`
- `crates/adc-lab-core/src/report.rs`
- `crates/adc-lab-core/src/rules/`
- `crates/adc-lab-core/src/probe/`
- `crates/adc-lab-core/src/suitability.rs`
- `crates/adc-lab-core/src/control.rs`
- `schemas/` and `schemas/generated/`
- `crates/adc-lab/tests/cli.rs`
- `crates/adc-lab-core/tests/contract_validation.rs`

## Design

### Phase 0: Metric Reframe and Guards

Make the target shift explicit before touching behavior:

- Add a machine-readable or Markdown schema/source classification listing each
  remaining schema file and schema-versioned wire contract as one of:
  - deleted by v2.1,
  - generated snapshot,
  - active v1 wire contract awaiting generated compatibility,
  - intentionally exempt with rationale.
- Include wire contracts with no schema file in the classification:
  `lab.observation_result.v1`, `lab.run_context.v1`, `lab.build_info.v1`,
  `lab.constraint_check_result.v1`, and `lab.workload_fixture_result.v1`.
- Add a file-budget check script and `make file-budgets` command, then wire it
  into `make verify` only after Phase 4 has reduced `main.rs`.
- Record the phase contribution forecast from this plan in a dedicated table,
  then update forecast-vs-actual measurements at each phase exit.
- Define temporary exemption mechanics for production files that exceed file
  budgets before their owning phase has split or deleted them.

This phase prevents the previous failure mode: a design objective not reflected
in WBS or measurable acceptance.

Phase 0 complexity budget:

- Changed files target: 6-7 files.
- New modules/classes target: 0 Rust modules/classes.
- New helper scripts target: 2 dependency-free scripts.
- New indirection layers target: 0.
- Production behavior line budget: 0 Rust behavior lines; Makefile/COMMANDS
  registration only.

Phase 0 post-implementation economy audit:

| New abstraction | Justification | Decision | Evidence |
|---|---|---|---|
| `schemas/schema-ledger.tsv` | Makes schema/source disposition reviewable and machine-checkable. | keep | `make schema-ledger-check` covers 32 schema files and 6 no-schema wire contracts. |
| `scripts/schema/check-schema-ledger.py` | Prevents hidden schema contracts and future false "maintained-by-hand = 0" claims. | keep | Integrated into `make schemas-check`; `--enforce-final` detects the current 32 handwritten schemas. |
| `scripts/ci/check-file-budgets.py` | Provides an informational guard before enforcing file budgets after Phase 4. | keep | `make file-budgets` reports the two known future-budget violations without failing Phase 0. |

### Phase 1: `report.run` Consolidation

Add `crates/adc-lab-core/src/rules/run_report.rs` as the v2 report-run owner.

Responsibilities:

- Load indexed artifacts through `EvidenceStore`.
- Produce `Artifact<RunReportPayload>` with stable claim IDs, data-quality
  notes, evidence refs, and bounded next-evidence guidance.
- Cover the current familiarization pack, claim evidence trace, and operating
  point coverage use cases.
- Cover both current `claim_evidence_trace` producers:
  - report/read-only path in `report.rs::read_only_claim_trace`,
  - experiment path in `experiment.rs` and `main.rs::real_experiment_claim_trace`.
- Keep `run_manifest` separate as an identity/consistency artifact.

Cutover:

- CLI report paths write the v2 `report.run` artifact as primary output.
- v1 familiarization, claim evidence trace, and operating point coverage
  artifacts are retained only until parity tests pass in the same phase.
- Delete the three v1 schemas and DTO paths after parity only if both report and
  experiment trace producers have moved to v2. If the experiment producer is
  explicitly deferred, `lab.claim_evidence_trace.v1.schema.json` must stay
  classified as active/deferred with an owner and later phase.

Acceptance:

- A CLI regression proves read-only familiarization/report-pack workflows emit
  v2 `report.run` and preserve the same or more conservative blocked claims.
- A CLI or core regression proves real experiment claim traces are represented
  by v2 `report.run` or another v2 artifact before the v1 claim-trace schema is
  deleted.
- `report.rs` drops below 900 lines or the plan records a specific remaining
  owner for each retained section.
- Schemas removed: 3.
- Deleted schema references do not remain in `tests/golden`, `Makefile`,
  `COMMANDS.md`, or active CLI expectations.

### Phase 2: Probe Public Cutover

Finish what v2 sidecars started: make v2 probe artifacts the primary public
output contract.

Responsibilities:

- `load cpu` prints/writes `Artifact<LoadPayload>` as the primary result.
- `pressure run` prints/writes `Artifact<PressurePayload>` as the primary
  result.
- `pressure composite` prints/writes `Artifact<CompositePayload>` as the
  primary result.
- Preserve active internal typed plans and runtime safety monitors. Do not
  remove a typed plan if approval/audit/abort behavior depends on it.

Cutover:

- Remove primary v1 result output expectations from CLI tests.
- Delete v1 result schemas whose public producers are gone:
  `load_result`, `resource_pressure_result`, `composite_boundary_result`.
- Decide `load_plan` explicitly:
  - delete if no public producer remains, or
  - convert to generated snapshot if it remains a public safety plan contract.

Acceptance:

- Probe CLI tests assert v2 artifact envelopes and stable kind-specific
  payloads.
- Safety invariant tests for operator abort and remote quoting remain green.
- Schemas removed or generated-converted: 4.
- Deleted schema references do not remain in `tests/golden`, `Makefile`,
  `COMMANDS.md`, or active CLI expectations.

### Phase 3: Suitability and Constraints v2

Retire the remaining v1 suitability/constraints projection where public CLI
behavior can be v2-native.

Responsibilities:

- Add a v2 constraints payload, or explicitly fold constraints into
  `SuitabilityPayload` if that is the smaller stable API.
- `constraints generate` reads v2 suitability artifacts and writes a v2
  constraints artifact as the primary JSON result.
- `constraints check` reads the v2 constraints artifact and scans using catalog
  blocked terms.
- Keep Markdown agent-instruction output, but make its source v2 claim IDs and
  catalog text.
- Treat `SuitabilityPolicy` carefully: if it remains a public input schema,
  convert it to generated snapshot instead of deleting it.

Acceptance:

- The public loop is v2-only:
  `report operating-contract` -> `decide suitability` ->
  `constraints generate` -> `constraints check`.
- `lab.suitability_decision.v1.schema.json` and
  `lab.design_constraint_pack.v1.schema.json` are deleted.
- `lab.suitability_policy.v1.schema.json` is either deleted or generated.
- No v2 artifact uses prose-derived claim IDs.
- Deleted schema references do not remain in `tests/golden`, `Makefile`,
  `COMMANDS.md`, or active CLI expectations.

### Phase 4: CLI Module Split and File Budgets

Move command implementation out of `adc-lab/src/main.rs`.

Target modules:

- `commands/load.rs`
- `commands/pressure.rs`
- `commands/workload.rs`
- `commands/constraints.rs`
- `commands/experiment.rs`
- `commands/control.rs`
- `commands/privilege.rs`
- existing `commands/decide.rs` and `commands/report.rs`

Rules:

- `main.rs` owns CLI structs, parsing, and dispatch only.
- Each command module owns command execution, file IO for that command group,
  and audit event construction for that command group.
- Shared helpers move only when at least two command modules use them.
- If Phase 1, 2, or 3 already changes a command group's public behavior, that
  phase may also move the touched command group into `commands/` when doing so
  reduces duplicate edits and keeps the diff reviewable. Behavior changes and
  structural movement must still be separately described in the PR.

Acceptance:

- `main.rs` <= 800 lines.
- `make file-budgets` exists and is wired into `make verify`.
- No command behavior changes except those already covered by Phases 1-3.

### Phase 5: Generated Schema Source of Truth

Convert remaining active v1 schemas to generated snapshots without changing
wire behavior.

Order:

1. Workload and experiment schemas.
2. Privilege, qualification, toolchain, and target inventory schemas.
3. Audit, health check, release manifest, and run manifest schemas.
4. Control schemas last, after compatibility checks and safety invariant
   coverage are explicitly recorded.

Method:

- Add `JsonSchema` derives or generator adapters for active DTOs.
- Generate snapshots into `schemas/generated` or replace top-level schemas with
  generated files plus a clear marker.
- Add drift checks comparing committed snapshots to generated output.
- For control schemas, first add fixture compatibility tests proving the
  generated schema accepts current valid fixtures and rejects current invalid
  fixtures.

Acceptance:

- Maintained-by-hand schema count is 0.
- Every committed schema is either generated or explicitly generated-checked.
- `make verify` fails on schema drift.

### Phase 6: Documentation Normalization

Separate normative docs from reference/archive docs.

Target normative documents:

- `docs/architecture/safety-model.md`
- `docs/evidence-model.md`
- `docs/rules.md`
- `docs/reference/cli.md`

Tasks:

- Add a docs index that marks normative, reference, and archived documents.
- Move duplicated architecture narrative into the four normative docs or mark it
  as reference-only.
- Update `Makefile docs-smoke` in the same commit as path changes.

Acceptance:

- `docs-smoke` checks the new normative set and passes.
- README links point to the normalized docs.
- No old doc claims v1 report/probe/suitability artifacts as primary outputs.

## Validation & Acceptance

Every phase exits with:

```bash
make verify
```

Additional required commands:

```bash
make schemas
make schemas-check
find crates -name '*.rs' -type f -print0 | xargs -0 wc -l | tail -n 1
find schemas -maxdepth 1 -type f -name '*.schema.json' | wc -l
find schemas/generated -maxdepth 1 -type f -name '*.schema.json' | wc -l
```

Phase-specific gates:

- Phase 1: report-run parity tests and schema deletion evidence.
- Phase 1: `lab.claim_evidence_trace.v1.schema.json` deletion is allowed only
  after both report and experiment claim-trace producers are v2-native or the
  experiment producer is explicitly deferred and classified active.
- Phase 2: probe CLI v2 primary-output tests and safety-invariant tests.
- Phase 3: v2-only suitability/constraints CLI loop.
- Phase 4: `make file-budgets` and `main.rs <= 800`.
- Phase 5: generated-schema compatibility and drift tests.
- Phase 6: docs-smoke and README link review.
- Every phase exit updates the contribution forecast table with actual LoC,
  schema counts, file-budget status, and any target miss disposition.

Quality gate rule:

- Submit is allowed only when all phase acceptance criteria are met and the
  ExecPlan Verification Log records commands, results, and measurements.

## Milestones

1. Phase 0: Reframe metrics and add guardrails so the new plan cannot claim
   unreachable repository-wide reductions.
2. Phase 1: Consolidate run reports into v2 `report.run`.
3. Phase 2: Complete probe primary-output cutover to v2 artifacts.
4. Phase 3: Retire v1 suitability/constraints projection from public CLI.
5. Phase 4: Split CLI command implementation and enforce file budgets.
6. Phase 5: Convert active schemas to generated source of truth.
7. Phase 6: Normalize documentation around the four normative docs.

## Progress (WBS)

- [x] Read outcome review report.
- [x] Read `PLANS.md` and execution-plan reference.
- [x] Create this v2.1 ExecPlan.
- [x] Record current baseline measurements from `origin/main`.
- [x] Phase 0: Add schema-source classification.
- [x] Phase 0: Add file-budget check command, initially informational.
- [x] Phase 0: Add the contribution forecast table as a maintained
      forecast-vs-actual record.
- [ ] Phase 1: Design and implement `rules/run_report.rs`.
- [ ] Phase 1: Cut report CLI paths over to v2 `report.run`.
- [ ] Phase 1: Move or explicitly defer experiment claim-trace output before
      deleting `lab.claim_evidence_trace.v1.schema.json`.
- [ ] Phase 1: Delete or retire familiarization/claim-trace/coverage v1
      schemas after parity.
- [ ] Phase 2: Make load/pressure/composite v2 artifacts primary CLI outputs.
- [ ] Phase 2: Remove or generated-convert probe v1 schemas.
- [ ] Phase 3: Make constraints generation/checking v2-native.
- [ ] Phase 3: Retire v1 suitability/constraint schemas where public producers
      are gone.
- [ ] Phase 4: Split remaining command groups out of `main.rs`.
- [ ] Phase 4: Wire `make file-budgets` into `make verify`.
- [ ] Phase 5: Convert non-control active schemas to generated snapshots.
- [ ] Phase 5: Convert control schemas last after compatibility tests.
- [ ] Phase 6: Normalize docs and update `docs-smoke`.
- [ ] Run final `make verify` and record final measurements.
- [ ] Record final Outcomes against file budgets, schema classification,
      contribution forecast, and any missed target disposition in the Decision
      Log.

## Surprises & Discoveries

- The previous v2 plan's WBS is complete, but its original repository-wide LoC
  and handwritten-schema quantitative targets were structurally unreachable
  from its scoped phases.
- Current branch creation exposed untracked local files:
  `.DS_Store`, `._.DS_Store`,
  `reports/._20260611-v2-evidence-kernel-outcome-review.md`, and
  `reports/20260611-v2-evidence-kernel-outcome-review.md`. Treat these as
  user/local files unless explicitly asked to stage them.
- `commands/` currently contains only `decide.rs`, `report.rs`, and `mod.rs`
  for 92 lines total; most command implementation remains in `main.rs`.
- `platform_contract.rs` is just under the proposed 1,500-line budget but still
  owns active pressure/composite runtime, so it is not an immediate deletion
  target.
- `lab.claim_evidence_trace.v1` has more than one producer: report/read-only
  trace generation and experiment trace generation. Deleting its schema belongs
  behind both producer cutovers, not just report consolidation.
- Several schema-versioned wire contracts do not have top-level schema files.
  Phase 0 classification must include these contracts so "maintained-by-hand"
  cannot be satisfied by ignoring contracts without committed schema files.
- `schema-ledger.tsv` currently records 32 top-level schemas and 6 no-schema
  wire contracts. The sixth no-schema contract is
  `lab.suitability_decision.v1.projected_from_v2`, which is internal and
  exempt rather than a public schema target.
- `make file-budgets` currently reports two informational violations:
  `adc-lab/src/main.rs` at 2,595 lines over the future 800-line budget and
  `adc-lab-core/src/report.rs` at 1,527 lines over the future 900-line budget.

## Decision Log

- 2026-06-11: Do not carry forward the repository-wide 40-50% LoC target.
  Rationale: the outcome review shows the denominator included active
  out-of-scope subsystems and tests, so the target was not reachable by the
  evidence-kernel work.
- 2026-06-11: Redefine schema success as "maintained-by-hand schemas = 0"
  rather than "schema files = 0". Rationale: active wire contracts may remain
  in source control as generated snapshots without duplicating Rust DTO
  maintenance.
- 2026-06-11: Keep control and privileged-helper semantics out of v2.1 behavior
  changes. Rationale: these are safety boundaries; schema generation can be
  added only after compatibility tests prove no wire-format relaxation.
- 2026-06-11: Treat docs normalization as its own phase. Rationale: mixing docs
  movement with report/probe behavior changes obscures review and `docs-smoke`
  risk.
- 2026-06-11: Include experiment claim-trace output in Phase 1 deletion
  criteria. Rationale: `lab.claim_evidence_trace.v1` has report and experiment
  producers; deleting only the report path would repeat the previous
  design/WBS mismatch.
- 2026-06-11: Keep behavior cutover and command-module movement separated by
  default, but allow a phase to move the command group it already touches.
  Rationale: separated phases make behavior review easier; opportunistic moves
  are acceptable when they reduce double touching and keep tests focused.
- 2026-06-11: Extend `make schemas-check` with schema-ledger coverage but keep
  final maintained-by-hand enforcement as an explicit checker mode for Phase 5.
  Rationale: Phase 0 should make the target mechanically measurable without
  failing current development while 32 handwritten schemas intentionally remain.
- 2026-06-11: Keep `make file-budgets` informational until the command split
  phase. Rationale: current over-budget files are known planned targets, and
  failing `make verify` before their owning phases would block unrelated work.

## Verification Log

- `git switch -c codex/adc-labv21-plan origin/main`: created the planning
  branch from merged main.
- `git rev-parse --short HEAD`: `01f9085`.
- `find crates -name '*.rs' -type f -print0 | xargs -0 wc -l | tail -n 1`:
  17,939 total Rust lines.
- `find crates -path '*/tests/*' -name '*.rs' -print0 | xargs -0 wc -l | tail -n 1`:
  4,044 Rust test lines.
- `find schemas -maxdepth 1 -type f -name '*.schema.json' | wc -l`: 32.
- `find schemas/generated -maxdepth 1 -type f -name '*.schema.json' | wc -l`:
  9.
- `wc -l crates/adc-lab/src/main.rs crates/adc-lab-core/src/report.rs crates/adc-lab-core/src/platform_contract.rs crates/adc-lab-core/src/contracts.rs crates/adc-lab-core/src/suitability.rs crates/adc-lab-core/src/control.rs`:
  `main.rs` 2,595; `report.rs` 1,527; `platform_contract.rs` 1,496;
  `contracts.rs` 1,321; `suitability.rs` 943; `control.rs` 725.
- Plan review update: incorporated seven review findings covering the hidden
  experiment claim-trace producer, phase contribution forecast, schema-version
  contracts without schema files, file-budget exemption rules, stale
  golden/Makefile references, final outcome target reconciliation, and the
  behavior-cutover/module-move sequencing decision.
- Phase 0 implementation:
  `schemas/schema-ledger.tsv` added with 32 top-level schema entries and 6
  no-schema wire-contract entries.
- Phase 0 implementation:
  `scripts/schema/check-schema-ledger.py` added and `make schemas-check`
  extended to validate ledger coverage.
- Phase 0 implementation:
  `scripts/ci/check-file-budgets.py` and `make file-budgets` added in
  informational mode.
- Phase 0 verification:
  `make schema-ledger-check` passed with
  `top_level=32 no_schema_wire=6 maintained_by_hand=32`.
- Phase 0 verification:
  `python3 scripts/schema/check-schema-ledger.py --enforce-final` failed as
  expected, listing the 32 remaining maintained-by-hand schemas.
- Phase 0 verification:
  `make file-budgets` passed in informational mode and reported two current
  future-budget violations: `main.rs` and `report.rs`.
- Phase 0 verification:
  `make schemas-check` passed and now includes schema-ledger coverage.
- Phase 0 final gate:
  `make verify` passed. The gate ran workspace build, format check, clippy,
  generated schema drift plus ledger coverage, unit tests, integration tests,
  contract validation, docs smoke, and command smoke.

## Handoff

- Branch: `codex/adc-labv21-plan`.
- Base commit: `01f9085` (`origin/main` after PR #40).
- Current status: Phase 0 guardrail implementation is complete locally with
  `make verify` passing. Behavior implementation has not started.
- Untracked local files exist and were not staged:
  `.DS_Store`, `._.DS_Store`,
  `reports/._20260611-v2-evidence-kernel-outcome-review.md`, and
  `reports/20260611-v2-evidence-kernel-outcome-review.md`.
- Next steps:
  1. Commit and publish the Phase 0 guardrail PR.
  2. Start Phase 1 only after Phase 0 lands.
  3. Keep forecast-vs-actual and schema ledger entries current in each phase.
- Read first when resuming:
  - this plan,
  - `reports/20260611-v2-evidence-kernel-outcome-review.md`,
  - `plans/20260611-v2-evidence-kernel.md`,
  - `crates/adc-lab/src/main.rs`,
  - `crates/adc-lab-core/src/report.rs`.

## Outcomes & Retrospective

Phase 0 outcome:

- Schema classification is now a checked ledger, not prose-only plan text.
- `make schemas-check` validates generated v2 schema drift and schema-ledger
  coverage in one command.
- `make file-budgets` reports current production Rust file budget status in
  informational mode.
- Final maintained-by-hand enforcement is mechanically available through
  `scripts/schema/check-schema-ledger.py --enforce-final`; it currently fails
  as expected because 32 handwritten schemas remain.
