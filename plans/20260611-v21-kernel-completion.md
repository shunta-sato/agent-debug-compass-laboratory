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
- v2.1 LoC expectation is forecast-driven, not fixed. The actuals table below
  is authoritative when it differs from prose. Final landing is 18,058 Rust
  lines after the Phase 5 control-schema negative-test follow-up; this is a
  guardrail record, not an acceptance target.
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
| Phase 3 suitability/constraints | -100 to +100 | -2 to -3 | Revised after Phase 1/2 showed v2 payload and test cost offsets deleted v1 DTOs. |
| Phase 4 CLI module split | -100 to +100 | 0 | Better boundaries may add small module overhead. |
| Phase 5 generated schemas | +200 to +400 | maintained-by-hand -> 0 | Derives, adapters, and compatibility tests add code. |
| Phase 6 docs | 0, plus test-only follow-up if needed | 0 | Documentation normalization; Phase 5 review added lightweight control negative tests. |
| Total forecast | Actuals plus remaining phase forecast | maintained-by-hand -> 0 | Final landing: 18,058 Rust lines, top-level schemas 0, generated schemas 43. |

Phase contribution actuals:

| Phase | Actual LoC effect | Schema effect | Reforecast |
|---|---:|---:|---|
| Phase 1 report.run | -200 total Rust LoC (`17,939` -> `17,739`) | top-level `32` -> `29`, generated `9` -> `10`, maintained-by-hand `32` -> `29` | Remaining phases now forecast final total around `16,939-17,739` unless later phases delete more code than expected. |
| Phase 2 probe cutover | +117 total Rust LoC (`17,739` -> `17,856`) | top-level `29` -> `25`, generated `10` -> `14`, maintained-by-hand `29` -> `25` | Remaining phases now forecast final total around `17,556-18,156`; v2 public payload and generated v1 wire snapshots offset deleted schemas. |
| Phase 3 suitability/constraints | +68 total Rust LoC (`17,856` -> `17,924`) | top-level `25` -> `22`, generated `14` -> `17`, maintained-by-hand `25` -> `22` | Remaining phases now forecast final total around `18,024-18,424`; LoC is effectively neutral while schema maintenance drops. |
| Phase 4 CLI module split | +84 total Rust LoC (`17,924` -> `18,008`) | no schema change | In forecast. `main.rs` dropped to 605 lines and all production Rust files are under budget. |
| Phase 5 generated schemas | +9 total Rust LoC (`18,008` -> `18,017`) | top-level `22` -> `0`, generated `17` -> `43`, maintained-by-hand `22` -> `0` | Better than forecast. Most DTOs already had `JsonSchema`; generator helper offset small DTO additions. |
| Phase 6 docs normalization | +41 total Rust LoC (`18,017` -> `18,058`) | no schema change | Test-only follow-up for approval/restore schema rejection; production Rust budget remains unchanged. |

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

Phase 1 dev-workflow route:

- Risk: high. The phase changes public CLI report artifacts, schema inventory,
  generated schemas, and core report aggregation behavior.
- Required branches: default implementation lane, `implementation-economy`,
  `design-balance`, and final `quality-gate`.
- Not triggered: concurrency, embedded NFR, performance hot path, UI, and
  destructive-refactor branches. This phase does not alter target-local
  execution, control, restore, helper, SSH, or sampling behavior.
- Verification depth: focused report/experiment CLI regressions, schema ledger
  and schema drift checks, full `make verify`, and phase-exit measurements.

Phase 1 responsibility map:

| Unit | Responsibility sentence | Reason to change | Dependency direction |
|---|---|---|---|
| `rules/run_report.rs` | Own v2 `report.run` payloads, rules, claim IDs, and report-run evaluation from indexed evidence. | v2 report claim semantics change. | Depends on `EvidenceStore`, `rules::engine`, and catalog claim IDs. |
| `report.rs` | Retain shared run evidence summary and run manifest identity/quality helpers while v1 report DTO generators are retired. | Run-manifest identity or legacy report cleanup changes. | Depended on by CLI and `rules/run_report.rs`; avoids depending on CLI. |
| `main.rs` report/experiment paths | Persist v2 `report.run` artifacts and audit events for familiarization, report pack, operating-point, and experiment runs. | CLI routing or public artifact paths change. | Depends on core report/run-report APIs. |
| `schemas/schema-ledger.tsv` | Classify deleted Phase 1 v1 report schemas and keep no-schema contracts visible. | Schema source-of-truth status changes. | Checked by `scripts/schema/check-schema-ledger.py`. |

Phase 1 complexity budget:

- Changed files target: 10-14 files, including tests, schemas, generator, and
  this plan.
- New modules/classes target: 1 Rust module (`rules/run_report.rs`) and small
  payload structs only.
- New helpers/wrappers/adapters target: reuse existing `run_evidence_summary`
  and rule engine; add only persistence helpers needed to replace v1 producers.
- New indirection layers target: 0.
- Rough line budget: production Rust net -300 to -700, tests +100 to +250,
  schema snapshots +1 generated v2 file, top-level schema files -3.

Phase 1 post-implementation economy audit:

| New abstraction | Justification | Decision | Evidence |
|---|---|---|---|
| `rules/run_report.rs` | Centralizes v2 report-run claims and operating-point summary so CLI/report/experiment producers do not keep separate v1 generators. | keep | `make verify`; `report.rs` dropped to 827 lines and CLI tests cover read-only, report-pack, operating-point, and experiment paths. |
| `RunReportPayload` and operating-point payload structs | Replaces three v1 report DTO families with one generated v2 envelope payload while preserving structured operating-point evidence. | keep | `schemas/generated/lab.report.run.v2.schema.json`; CLI assertions inspect `kind=report.run` and stable claim IDs. |
| `deleted` schema-ledger state | Lets the ledger retain explicit retired-schema history while mechanically proving deleted files are absent. | keep | `make schemas-check` reports top-level=29 and maintained_by_hand=29. |

Phase 2 dev-workflow route:

- Risk: high. The phase changes public probe CLI JSON, generated schema
  inventory, schema-ledger semantics, and run-report bounded-load aggregation.
- Required branches: default implementation lane, `implementation-economy`, and
  final `quality-gate`.
- Not triggered: `design-balance` because no new module/class layout is planned;
  concurrency, embedded NFR, performance hot path, UI, and destructive-refactor
  branches remain out of scope. Runtime safety monitors, helper boundaries,
  restore behavior, SSH quoting, and pressure probe mechanics are preserved.
- Verification depth: focused probe CLI regressions, safety-invariant tests,
  schema drift/ledger checks, full `make verify`, and phase-exit measurements.

Phase 2 responsibility map:

| Unit | Responsibility sentence | Reason to change | Dependency direction |
|---|---|---|---|
| `main.rs` load/pressure paths | Persist and print v2 probe artifacts as the public CLI result while retaining v1 runtime DTOs only as internal execution inputs. | Probe public JSON contract changes. | Depends on core probe artifact constructors and `EvidenceStore`. |
| `experiment` execution path in `main.rs` | Store bounded load trial evidence as v2 load artifacts so `lab.load_result.v1` is not a hidden public producer. | Experiment evidence output changes. | Depends on the same v2 load artifact constructor as `load cpu`. |
| `probe/artifacts.rs` | Own v2 probe payload shape and result-id-stable artifact paths. | Public v2 probe envelope changes. | Depends on runtime DTOs, but not on CLI. |
| `report.rs` / `rules/run_report.rs` | Treat v2 load artifacts as bounded-load evidence for run manifests and run reports. | Report aggregation input changes. | Depends on artifact refs, not on CLI paths. |
| `schemas/schema-ledger.tsv` and checker | Classify remaining internal v1 probe wire DTOs as generated snapshots after public producers cut over. | Schema source-of-truth status changes. | Checked by `scripts/schema/check-schema-ledger.py` and `make schemas-check`. |

Phase 2 complexity budget:

- Changed files target: 12-16 files, including CLI tests, generated schemas,
  schema ledger/checker, core probe artifacts, and this plan.
- New modules/classes target: 0 Rust modules/classes.
- New helpers/wrappers/adapters target: at most one local CLI persistence helper
  if repeated v2 artifact writing becomes noisy; prefer direct reuse of
  `EvidenceStore` and existing artifact constructors.
- New indirection layers target: 0.
- Rough line budget: production Rust net -100 to -350, tests +100 to +250,
  generated schema snapshots +4, top-level handwritten schema files -4.

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

- `commands/common.rs`
- `commands/load.rs`
- `commands/pressure.rs`
- `commands/workload.rs`
- `commands/constraints.rs`
- `commands/experiment.rs`
- `commands/control.rs`
- `commands/privilege.rs`
- `commands/familiarize.rs`
- `commands/target.rs`
- `commands/tool.rs`
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

Phase 4 dev workflow route:

- Risk route: high. Rationale: broad CLI implementation refactor across command
  modules, with safety/audit command surfaces preserved.
- Required branches: `design-balance`, `implementation-economy`, ExecPlan
  update, and final `quality-gate`.
- Verification depth: full `make verify`, plus focused `cargo check` and
  `make file-budgets` before the final gate.

Phase 4 responsibility map:

| Unit | Name | Responsibility sentence | Reason to change | Dependency direction |
|---|---|---|---|---|
| module | `main.rs` | Own CLI type definitions, parsing, version handling, and dispatch. | CLI surface changes. | Depends on command modules only for execution entry points. |
| module | `commands/common.rs` | Own shared run/artifact/audit, SSH argument, and persistence helpers used by multiple command groups. | Shared command IO or artifact boundary changes. | Command modules depend on it; it depends on core contracts. |
| module | `commands/control.rs` | Execute control plan/approve/apply/restore and control-specific helper invocation. | Privileged control workflow changes. | Depends on common helpers and control core. |
| module | `commands/load.rs` | Execute `load cpu` and persist v2 load artifacts. | Load command behavior or output changes. | Depends on common helpers and load core. |
| module | `commands/pressure.rs` | Execute pressure and composite probes, including SSH pressure runners. | Pressure command behavior or output changes. | Depends on common helpers and pressure core. |
| module | `commands/workload.rs` | Execute workload runs and bounded workload fixtures. | Workload command behavior changes. | Depends on common helpers and workload core. |
| module | `commands/constraints.rs` | Generate/check v2 constraints artifacts and Markdown. | Constraints public contract changes. | Depends on common helpers and suitability core. |
| module | `commands/experiment.rs` | Execute experiment matrices and trials. | Experiment execution behavior changes. | Depends on common helpers and experiment/load/observe core. |
| module | `commands/familiarize.rs` | Orchestrate read-only familiarization output. | Familiarization workflow changes. | Depends on common persistence helpers. |
| module | `commands/target.rs` | Execute inventory, toolchain discovery, observe, and health-check command wrappers. | Target read-only command behavior changes. | Depends on common helpers and target core. |
| module | `commands/privilege.rs` | Execute privilege provider/doctor/install-plan/uninstall-plan wrappers. | Privilege planning/report behavior changes. | Depends on common helpers and privilege core. |
| module | `commands/tool.rs` | Execute tool qualification and toolchain inventory qualification. | Tool qualification evidence handling changes. | Depends on common helpers and qualification core. |
| module | `commands/report.rs` | Execute report pack/operating-point/operating-contract wrappers. | Report command behavior changes. | Depends on common helpers and report/rules core. |

Phase 4 complexity budget:

- Changed files target: 16-18 files.
- New modules target: 11 command modules plus `common.rs`; no new runtime layer.
- New helper/wrapper target: 0 net new behavior helpers; moved helpers only.
- Production LoC budget: expected +0 to +100 from module headers/imports.
- Test budget: no new behavior tests required; existing CLI/safety tests are
  the regression harness.

Phase 4 post-implementation economy audit:

| New abstraction | Justification | Decision | Evidence |
|---|---|---|---|
| command modules under `commands/` | Separate command execution reasons-to-change from CLI parsing and keep `main.rs` below the budget. | keep | `main.rs` is 605 lines; CLI and safety tests pass. |
| `commands/common.rs` | Centralizes shared run/artifact/audit, SSH, and persistence helpers used by multiple command modules. Single-use helpers were moved back to owning modules during review. | keep | `commands/common.rs` is 362 lines; `make file-budgets` reports 0 violations. |

Phase 4 smells and anti-patterns review:

- Scope: `main.rs` command split, new command modules, and
  `Makefile`/`COMMANDS.md` file-budget enforcement.
- Findings: 0 new or worsened maintainability issues found.
- Boundary check: dependency direction remains CLI dispatch -> command modules
  -> common/core. Core crates do not depend on CLI modules. `common.rs` stays
  below budget and contains shared command concerns rather than command-specific
  behavior.

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

Phase 5 dev workflow route:

- Risk route: high. Rationale: schema contracts and generated drift gates are
  public compatibility boundaries, including safety-critical control contracts.
- Required branches: default implementation lane, `design-balance`,
  `implementation-economy`, ExecPlan update, and final `quality-gate`.
- Not triggered: embedded NFR, observability, performance, concurrency,
  destructive-refactor, and error-handling. This phase changes schema source of
  truth and tests, not target runtime behavior or failure contracts.
- Verification depth: full `make verify`, plus focused schema ledger/drift,
  contract validation, CLI, safety invariant, and file-budget checks.

Phase 5 responsibility map:

| Unit | Name | Responsibility sentence | Reason to change | Dependency direction |
|---|---|---|---|---|
| DTO module | `contracts.rs` | Own active v1 wire DTO shape for generated schemas. | v1 wire contract fields or status vocabulary change. | Schema generator depends on these DTOs; runtime producers already use them. |
| DTO module | `run.rs` | Own `run_context.json` DTO shape and run identity helpers. | Run identity file shape changes. | Schema generator depends on `RunContextArtifact`; run helpers remain runtime owners. |
| DTO module | `observe.rs` | Own observation result DTO shape and signal vocabulary. | Observation result fields or signal vocabulary change. | Schema generator depends on observation DTOs; probe artifacts depend on them. |
| CLI command module | `commands/workload.rs` | Emit typed workload fixture result instead of anonymous JSON. | Workload fixture output shape changes. | Depends on core DTO; does not own schema generation. |
| schema generator | `examples/generate_schemas.rs` | Generate every committed schema snapshot from Rust DTOs. | DTO/schema inventory changes. | Depends on core DTOs; `make schemas-check` verifies drift. |
| schema ledger | `schemas/schema-ledger.tsv` | Classify each v1 contract as deleted or generated snapshot. | Contract source-of-truth status changes. | Checked by `scripts/schema/check-schema-ledger.py`. |

Phase 5 complexity budget:

- Changed files target: 35-45 files including generated snapshots, deleted
  handwritten schemas, tests, ledger, and this plan.
- New modules target: 0.
- New DTO structs target: up to 2 public DTOs for previously anonymous CLI
  outputs; no new runtime layer.
- New helper/wrapper target: one schema-generator helper replacing repeated
  write blocks.
- Production LoC budget: expected +0 to +80 after generator helper savings;
  no `platform_contract.rs` growth or file-budget exemption.
- Test budget: reuse existing golden fixtures and negative tests, adding
  fixtures only for previously no-schema wire contracts.

Phase 5 post-implementation economy audit:

| New abstraction | Justification | Decision | Evidence |
|---|---|---|---|
| `HealthCheck` DTO | Moves health-check wire shape into core so schema generation and CLI output share one contract. | keep | health fixture validates through generated schema; CLI tests pass. |
| `WorkloadFixtureResult` DTO | Replaces anonymous JSON with a typed DTO so the previous no-schema fixture result is generated-checked. | keep | fixture validates through generated schema; CLI tests pass. |
| `write_schema<T>` helper | Removes repeated generator blocks while keeping generation explicit at each call site. | keep | `make schemas-check` passes and generated snapshot count is 43. |

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
- Document public output file names against artifact `kind`, or rename
  examples/defaults where safe, so v2 artifacts no longer look like v1 payloads
  in user-facing docs.
- Update `Makefile docs-smoke` in the same commit as path changes.

Acceptance:

- `docs-smoke` checks the new normative set and passes.
- README links point to the normalized docs.
- No old doc claims v1 report/probe/suitability artifacts as primary outputs.

Phase 6 dev workflow route:

- Risk route: normal. Rationale: docs and schema-validation tests only; no
  runtime command behavior or wire DTO shape changes.
- Required branches: default implementation lane, `implementation-economy`,
  ExecPlan update, and final `quality-gate`.
- Not triggered: design-balance, embedded NFR, observability, performance,
  concurrency, destructive-refactor, and error-handling. This phase clarifies
  existing contracts instead of adding runtime surfaces.
- Verification depth: focused contract validation for the Phase 5 follow-up,
  docs-smoke, stale-reference scans, and full `make verify`.

Phase 6 complexity budget:

- Changed files target: 10-16 files, mostly documentation plus one focused
  contract-validation test file.
- New modules/classes target: 0.
- New helper/wrapper target: 0.
- Production Rust LoC budget: 0; test-only Rust additions are allowed for the
  control schema negative-test follow-up.
- Documentation budget: consolidate wording and add one docs index instead of
  adding a second narrative layer.

Phase 6 post-implementation economy audit:

| New abstraction | Justification | Decision | Evidence |
|---|---|---|---|
| `docs/README.md` documentation index | Makes the normative/reference split explicit without moving historical architecture notes or duplicating contracts. | keep | `docs-smoke` checks the index and the normative docs. |

No new runtime modules, helper wrappers, or production Rust abstractions were
added in Phase 6.

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
- [x] Phase 1: Record high-risk route, responsibility map, and complexity
      budget before implementation.
- [x] Phase 1: Design and implement `rules/run_report.rs`.
- [x] Phase 1: Cut report CLI paths over to v2 `report.run`.
- [x] Phase 1: Move or explicitly defer experiment claim-trace output before
      deleting `lab.claim_evidence_trace.v1.schema.json`.
- [x] Phase 1: Delete or retire familiarization/claim-trace/coverage v1
      schemas after parity.
- [x] Phase 2: Make load/pressure/composite v2 artifacts primary CLI outputs.
- [x] Phase 2: Remove or generated-convert probe v1 schemas.
- [x] Phase 3: Make constraints generation/checking v2-native.
- [x] Phase 3: Retire v1 suitability/constraint schemas where public producers
      are gone.
- [x] Phase 4: Split remaining command groups out of `main.rs`.
- [x] Phase 4: Wire `make file-budgets` into `make verify`.
- [x] Phase 5: Convert non-control active schemas to generated snapshots.
- [x] Phase 5: Convert control schemas last after compatibility tests.
- [x] Phase 6: Normalize docs and update `docs-smoke`.
- [x] Phase 6: Document or rename public output file names whose v2 artifact
      kind no longer matches legacy v1 file names.
- [x] Run final `make verify` and record final measurements.
- [x] Record final Outcomes against file budgets, schema classification,
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
- Phase 1 checkout starts from `origin/main` at `76707e9`, after the Phase 0
  guardrail PR was merged. Local untracked files remain present and must stay
  unstaged unless the user asks otherwise.
- Phase 2 starts from `origin/main` at `229112a`, after the Phase 1 PR was
  merged.
- `lab.load_result.v1`, `lab.resource_pressure_result.v1`, and
  `lab.composite_boundary_result.v1` cannot honestly be ledger `deleted`
  contracts because the SSH target/controller boundary and runtime DTOs still
  deserialize those shapes internally. Their public CLI outputs moved to v2,
  while the remaining v1 wire DTO schemas are generated snapshots.
- The `load cpu` v2 sidecar path was still `load/cpu.v2.json`, which would
  overwrite repeated load results. Phase 2 made load artifact paths result-id
  stable, matching pressure and composite behavior.
- `blocked_claims_for` returns catalog blocked phrases, not claim IDs. Phase 3
  kept v2 suitability/constraints payloads on stable claim IDs and uses catalog
  phrases only for Markdown/check scanning.
- Phase 5 did not need to edit `platform_contract.rs`. The Phase 2 pressure and
  composite v1 wire DTOs were already generated snapshots, so the file-budget
  risk at 1,496/1,500 lines required no temporary exemption.
- `lab.build_info.v1` is an artifact type label used in run manifests, but the
  current `BuildInfo` DTO does not carry a `schema_version` field. Phase 5
  generated and fixture-checked the existing DTO shape rather than changing the
  wire output.

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
- 2026-06-11: Treat `report.rs` run-summary helpers as reusable core
  aggregation during Phase 1 instead of duplicating filesystem scans in
  `rules/run_report.rs`. Rationale: this keeps the v2 report implementation
  small and makes `run_manifest` and `report.run` derive from the same observed
  operation facts.
- 2026-06-11: Accept the Phase 1 LoC forecast miss instead of shrinking v2
  report semantics. Rationale: replacing three v1 report contracts with one v2
  artifact required stable claim IDs, catalog entries, generated schema, and
  report/experiment parity tests; `report.rs` met its file budget and schema
  deletion target, but total Rust LoC only dropped by 200. Reforecast final
  landing to roughly 16,939-17,739 unless later phases over-delete.
- 2026-06-11: Keep `report.run` evaluation summary-based for now instead of
  forcing it into `engine::Rule`. Rationale: run-report claims summarize
  manifest/run-sequence facts as well as store predicates, unlike operating
  contract and suitability rules. Revisit convergence only if Phase 3/6 can
  share predicates without a second bespoke DSL.
- 2026-06-11: Convert Phase 2 v1 probe result/plan schemas to generated
  snapshots rather than marking them deleted. Rationale: public controller CLI
  output now returns v2 artifacts, but v1 DTOs remain internal runtime and SSH
  wire inputs; ledger `deleted` would hide a real remaining contract.
- 2026-06-11: Accept the Phase 2 LoC forecast miss. Rationale: v2 public probe
  payloads, result-id-stable artifact paths, hidden experiment load producer
  cutover, generated v1 wire snapshots, and ledger support for generated-only
  wire contracts added more code than schema/golden deletion removed. Reforecast
  final landing to roughly 17,556-18,156 unless Phase 4 deletes more command
  implementation than currently expected.
- 2026-06-11: Re-baseline the LoC forecast after two same-direction misses and
  before Phase 3 closeout. Rationale: public v2 payloads, generated snapshots,
  and parity tests have a steady cost; the actuals table is authoritative and
  Phase 3 is expected to land near neutral rather than remove hundreds of
  lines.
- 2026-06-11: Make constraints output/checking v2-native instead of preserving
  a compatibility projection. Rationale: public suitability output is already a
  v2 artifact, so generating v1 design packs would keep the public loop split
  and retain prose-derived constraint state.
- 2026-06-11: Keep CLI type definitions in `main.rs` during Phase 4.
  Rationale: `main.rs` is now below the 800-line budget while preserving the
  public Clap surface in one place; command modules own execution, file IO, and
  audit work.
- 2026-06-11: Enforce file budgets through `make verify` after the Phase 4
  split. Rationale: all production Rust files are under configured budgets, so
  renewed file growth should fail CI unless a future PR records a temporary
  exemption.
- 2026-06-11: Defer public output filename/artifact-kind cleanup to Phase 6.
  Rationale: the Phase 3 review identified confusing legacy names for v2
  artifacts, but Phase 4 is a behavior-preserving module split and docs
  normalization is already scoped to Phase 6.
- 2026-06-11: Keep `platform_contract.rs` untouched in Phase 5 and use the
  existing Phase 2 generated snapshots for pressure/composite v1 wire DTOs.
  Rationale: the file has only four lines of budget headroom; no remaining
  Phase 5 schema target required editing it, so no exemption or split was
  justified.
- 2026-06-11: Turn `make schemas-check` into the final schema-source gate by
  passing `--enforce-final`. Rationale: maintained-by-hand schema count is now
  zero, and future drift or handwritten-schema reintroduction should fail
  `make verify`.
- 2026-06-11: Accept the Phase 5 LoC forecast undershoot. Rationale: most
  active DTOs already had `JsonSchema` derives, and replacing repeated
  generator write blocks with one helper offset the small DTO/test additions.
  The final expected Rust LoC after Phase 5 is about 18,017 plus docs-only
  Phase 6 changes.
- 2026-06-11: Add approval-record and restore-lease schema negative tests in
  Phase 6 instead of reopening Phase 5. Rationale: the Phase 5 review accepted
  the generated-schema cutover and identified these as lightweight follow-up
  evidence that generated control schemas did not relax forged-field rejection.
- 2026-06-11: Keep retained architecture documents as reference-only through
  `docs/README.md` instead of moving or deleting them. Rationale: the four
  normative documents are explicit, docs-smoke can verify the map, and avoiding
  path churn keeps historical design context available.
- 2026-06-11: Document public output file names against v2 artifact `kind`
  rather than changing CLI defaults. Rationale: suitability and constraints
  paths are caller-provided, while the stable contract is the envelope kind;
  examples can use `.v2.json` names without a behavior change.

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
- Phase 1 branch setup:
  `git fetch --prune origin` updated `origin/main` to `76707e9`; `git switch
  -c codex/adc-labv21-report-run origin/main` created the Phase 1 branch.
- Phase 1 initial measurements:
  `wc -l crates/adc-lab-core/src/report.rs crates/adc-lab/src/main.rs
  crates/adc-lab-core/src/contracts.rs schemas/schema-ledger.tsv
  crates/adc-lab/tests/cli.rs`: `report.rs` 1,527; `main.rs` 2,595;
  `contracts.rs` 1,321; ledger 39; CLI tests 1,962.
- Phase 1 initial schema counts:
  top-level schemas 32; generated schemas 9.
- Phase 1 focused verification:
  `make schemas-check` passed with
  `top_level=29 no_schema_wire=6 maintained_by_hand=29`.
- Phase 1 focused verification:
  `cargo test -p adc-lab-core run_report -- --nocapture` passed; 2 tests.
- Phase 1 focused verification:
  `cargo test -p adc-lab-core report::tests -- --nocapture` passed; 5 tests.
- Phase 1 focused verification:
  `cargo test -p adc-lab --test cli experiment_real_run_executes_supported_bounded_matrix -- --nocapture`
  passed.
- Phase 1 focused verification:
  CLI focused tests for `familiarize_read_only_writes_manifest_run_report_and_audit`,
  `experiment_real_run_blocks_unsupported_controlled_factor`,
  `report_operating_point_marks_read_only_run_observational_only`,
  `report_operating_point_marks_bounded_matrix_controlled_subset`, and
  `experiment_dry_run_and_report_pack_work` passed.
- Phase 1 lint/contract verification:
  `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, and `cargo test --workspace contract_validation -- --nocapture`
  passed.
- Phase 1 structural scan:
  code-smells/anti-patterns review found no new or worsened maintainability
  issues. Checked changed units `rules/run_report.rs`, `report.rs`,
  `main.rs`, schema ledger/checker, generated schema registration, and CLI
  tests. Dependency direction remains CLI -> core -> evidence/report helpers;
  the new module has one reason to change: v2 report-run claim semantics.
- Phase 1 final gate:
  `make verify` passed. The gate covered build, format, clippy, generated schema
  drift plus ledger, unit tests, integration tests, safety invariants, contract
  validation, docs smoke, and command smoke.
- Phase 1 final measurements:
  total Rust lines `17,739`; Rust test lines `4,006`; top-level schemas `29`;
  generated schemas `10`; `main.rs` `2,470`; `report.rs` `827`;
  `rules/run_report.rs` `727`; `contracts.rs` `1,211`; CLI tests `1,940`.
- Phase 1 file-budget check:
  `make file-budgets` passed in informational mode with one remaining future
  violation, `crates/adc-lab/src/main.rs` at 2,470 lines over the Phase 4
  budget.
- Phase 2 branch setup:
  `git fetch origin --prune` updated `origin/main` to `229112a`; `git switch
  -c codex/adc-labv21-probe-cutover origin/main` created the Phase 2 branch.
- Phase 2 focused verification:
  `make schemas-check` passed with
  `top_level=25 no_schema_wire=10 maintained_by_hand=25`.
- Phase 2 focused verification:
  `cargo test -p adc-lab-core --test probe_artifacts -- --nocapture` passed; 4
  tests.
- Phase 2 focused verification:
  `cargo test -p adc-lab-core report::tests::load_artifact_updates_manifest_summary -- --nocapture`
  passed.
- Phase 2 focused verification:
  CLI focused tests for
  `load_cpu_operator_abort_records_safety_monitor_without_abort_path`,
  `pressure_run_local_writes_typed_artifact_and_audit`,
  `pressure_network_bounded_transfer_records_generated_bytes`,
  `pressure_composite_smoke_does_not_support_coupling_without_measured_effect`,
  and `experiment_real_run_executes_supported_bounded_matrix` passed.
- Phase 2 safety-invariant verification:
  `cargo test -p adc-lab --test safety_invariants -- --nocapture` passed; 7
  tests.
- Phase 2 integration/contract/lint verification:
  `cargo test -p adc-lab --test cli -- --nocapture` passed; 32 tests.
  `cargo test --workspace contract_validation -- --nocapture` passed.
  `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Phase 2 final-enforcement check:
  `python3 scripts/schema/check-schema-ledger.py --enforce-final` failed as
  expected with 25 maintained-by-hand schemas remaining; this remains a Phase 5
  gate.
- Phase 2 measurements:
  total Rust lines `17,856`; Rust test lines `4,032`; top-level schemas `25`;
  generated schemas `14`; `main.rs` `2,491`; `report.rs` `837`;
  `platform_contract.rs` `1,496`; `contracts.rs` `1,211`;
  `probe/artifacts.rs` `414`; CLI tests `1,977`.
- Phase 2 file-budget check:
  `make file-budgets` passed in informational mode with one remaining future
  violation, `crates/adc-lab/src/main.rs` at 2,491 lines over the Phase 4
  budget.
- Phase 2 final gate:
  `make verify` passed. The gate covered workspace build, format check, clippy,
  generated schema drift plus ledger coverage, unit tests, integration tests,
  safety invariants, contract validation, docs smoke, and command smoke.
- Phase 3 branch setup:
  `git fetch origin --prune` updated `origin/main` to `206db6b`; `git switch
  -c codex/adc-labv21-suitability-constraints origin/main` created the Phase 3
  branch.
- Phase 3 focused verification:
  `cargo check --workspace` passed.
- Phase 3 focused verification:
  `make schemas-check` passed with
  `top_level=22 no_schema_wire=9 maintained_by_hand=22`.
- Phase 3 focused verification:
  `cargo test -p adc-lab-core --test rules_engine -- --nocapture` passed; 8
  tests.
- Phase 3 focused verification:
  `cargo test -p adc-lab-core --test contract_validation -- --nocapture`
  passed; 14 tests.
- Phase 3 focused verification:
  `cargo test -p adc-lab --test cli -- --nocapture` passed; 32 tests.
- Phase 3 measurements before final gate:
  total Rust lines `17,924`; top-level schemas `22`; generated schemas `17`;
  `make file-budgets` passed in informational mode with one future violation,
  `crates/adc-lab/src/main.rs` at 2,482 lines over the Phase 4 budget.
- Phase 3 final gate:
  `make verify` passed. The gate covered workspace build, format check, clippy,
  generated schema drift plus ledger coverage, unit tests, integration tests,
  safety invariants, contract validation, docs smoke, and command smoke.
- Phase 4 branch setup:
  `git fetch origin --prune` updated `origin/main` to `8c529fe`; `git switch
  -c codex/adc-labv21-cli-module-split origin/main` created the Phase 4 branch.
- Phase 4 focused verification:
  `cargo check --workspace` passed.
- Phase 4 focused verification:
  `cargo test -p adc-lab --test cli -- --nocapture` passed; 32 tests.
- Phase 4 focused verification:
  `cargo test -p adc-lab --test safety_invariants -- --nocapture` passed; 7
  tests.
- Phase 4 focused verification:
  `make file-budgets` passed with
  `file budgets: enforced checked=50 violations=0`.
- Phase 4 focused verification:
  `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Phase 4 focused verification:
  `make schemas-check` passed with
  `top_level=22 no_schema_wire=9 maintained_by_hand=22`.
- Phase 4 focused verification:
  `cargo test --workspace contract_validation -- --nocapture` passed.
- Phase 4 final gate:
  `make verify` passed. The gate covered workspace build, format check, clippy,
  generated schema drift plus ledger coverage, enforced file budgets, unit
  tests, integration tests, safety invariants, contract validation, docs smoke,
  and command smoke.
- Phase 4 final measurements:
  total Rust lines `18,008`; top-level schemas `22`; generated schemas `17`;
  `main.rs` `605`; `commands/common.rs` `362`; largest command module
  `common.rs`; `make file-budgets` enforced with 0 violations.
- Phase 4 quality gate:
  submit. Acceptance criteria are met: `main.rs <= 800`, `make file-budgets`
  is part of `make verify`, and command behavior is covered by the existing CLI
  and safety regression suites.
- Phase 5 branch setup:
  `git fetch origin --prune` updated `origin/main` to `de23632`; `git switch
  -c codex/adc-labv21-generated-schemas origin/main` created the Phase 5
  branch.
- Phase 5 focused verification:
  `make schemas` passed and generated 43 schema snapshots under
  `schemas/generated`.
- Phase 5 focused verification:
  `make schemas-check` passed with
  `top_level=0 no_schema_wire=31 maintained_by_hand=0`.
- Phase 5 focused verification:
  `cargo test -p adc-lab-core --test contract_validation -- --nocapture`
  passed; 14 tests.
- Phase 5 focused verification:
  `cargo check --workspace` passed.
- Phase 5 focused verification:
  `make file-budgets` passed with
  `file budgets: enforced checked=50 violations=0`.
- Phase 5 focused verification:
  `cargo test -p adc-lab --test safety_invariants -- --nocapture` passed; 7
  tests.
- Phase 5 focused verification:
  `cargo test -p adc-lab --test cli -- --nocapture` passed; 32 tests.
- Phase 5 measurements before final gate:
  total Rust lines `18,017`; top-level schemas `0`; generated schemas `43`;
  `platform_contract.rs` remains `1,496` lines; `contracts.rs` is `1,196`
  lines; file budgets have 0 violations.
- Phase 5 final gate:
  `make verify` passed. The gate covered workspace build, format check, clippy,
  generated schema drift plus final ledger enforcement, enforced file budgets,
  unit tests, integration tests, safety invariants, contract validation, docs
  smoke, and command smoke.
- Phase 5 quality gate:
  submit. Acceptance criteria are met: maintained-by-hand schema count is 0,
  every active schema is generated or generated-checked, and `make verify`
  fails on generated schema drift plus final ledger violations.
- Phase 6 branch setup:
  `git fetch origin --prune` updated `origin/main` to `5a2eb16`; `git switch
  -c codex/adc-labv21-docs-normalization origin/main` created the Phase 6
  branch.
- Phase 6 focused verification:
  `cargo test -p adc-lab-core --test contract_validation -- --nocapture`
  passed; 16 tests. The added approval-record and restore-lease forged-field
  cases both reject invalid fixtures.
- Phase 6 focused verification:
  `make docs-smoke` passed with `docs/README.md`, the normative docs, CLI
  reference, and pressure reference in the checked set.
- Phase 6 stale-reference scan:
  `rg -n "claim_evidence_trace|familiarization_pack|suitability_decision|design_constraint_pack|operating_point_coverage|lab\\.load_result\\.v1|lab\\.resource_pressure_result\\.v1|lab\\.composite_boundary_result\\.v1|reports/(claim_evidence_trace|familiarization_pack|suitability_decision|design_constraint_pack|target_operating_contract\\.json)" README.md docs`
  returned no matches.
- Phase 6 file-budget check:
  `make file-budgets` passed with
  `file budgets: enforced checked=50 violations=0`.
- Phase 6 final measurements:
  total Rust lines `18,058`; top-level schemas `0`; generated schemas `43`;
  maintained-by-hand schemas remain `0`. The Rust increase is test-only from
  the Phase 5 follow-up; production file budgets remain green.
- Phase 6 final gate:
  `make verify` passed. The gate covered workspace build, format check,
  clippy, generated schema drift plus final ledger enforcement
  (`top_level=0 no_schema_wire=31 maintained_by_hand=0`), enforced file
  budgets, unit tests, integration tests, safety invariants, contract
  validation, docs smoke, and command smoke.
- Phase 6 quality gate:
  submit. Acceptance criteria are met: docs-smoke checks the normalized
  normative set, README links to the docs index and v2 examples, and old v1
  report/probe/suitability artifacts are not claimed as primary outputs in
  README/docs.

## Handoff

- Branch: `codex/adc-labv21-docs-normalization`.
- Base commit: `5a2eb16` (`origin/main` after Phase 5 merge).
- Current status: Phase 6 implementation is complete locally with
  `make verify` passing. Commit and PR publication are next.
- Untracked local files exist and were not staged:
  `.DS_Store`, `._.DS_Store`,
  `plans/._20260611-v21-kernel-completion.md`,
  `reports/._20260611-planning-skills-improvement-proposal.md`,
  `reports/._20260611-v2-evidence-kernel-outcome-review.md`,
  `reports/20260611-planning-skills-improvement-proposal.md`, and
  `reports/20260611-v2-evidence-kernel-outcome-review.md`.
- Next steps:
  1. Commit and publish the Phase 6 PR.
  2. After merge, the v2.1 completion plan is satisfied; future work should be
     opened from a new plan or follow-up issue.
- Read first when resuming:
  - this plan,
  - `docs/README.md`,
  - `docs/reference/cli.md`,
  - `reports/20260611-v2-evidence-kernel-outcome-review.md`,
  - `plans/20260611-v2-evidence-kernel.md`,
  - `crates/adc-lab-core/tests/contract_validation.rs`,
  - `schemas/schema-ledger.tsv`.

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

Phase 1 outcome:

- `familiarize read-only`, `report pack`, `report operating-point`, and
  `experiment run` now persist v2 `report.run` artifacts as the report/claim
  surface.
- `lab.familiarization_pack.v1`, `lab.claim_evidence_trace.v1`, and
  `lab.operating_point_coverage.v1` schemas and golden fixtures were deleted
  after report and experiment producers moved.
- `report.rs` is below the 900-line Phase 1 budget; `main.rs` remains a Phase 4
  file-budget target.

Phase 2 outcome:

- `load cpu`, `pressure run`, and `pressure composite` now persist and print v2
  artifact envelopes as their public CLI JSON result.
- Experiment bounded-load trial evidence now stores v2 load artifacts instead
  of `load_result.json`, closing the hidden `lab.load_result.v1` producer.
- `lab.load_plan.v1`, `lab.load_result.v1`,
  `lab.resource_pressure_result.v1`, and
  `lab.composite_boundary_result.v1` top-level handwritten schemas and golden
  fixtures were removed; generated snapshots now cover the remaining internal
  v1 wire DTOs.
- `load` v2 artifacts now include result IDs in their file names so repeated
  load runs do not overwrite previous v2 evidence.

Phase 3 outcome:

- The public suitability/constraints loop is v2-native from
  `report operating-contract` through `decide suitability`,
  `constraints generate`, and `constraints check`.
- `constraints generate` writes `Artifact<ConstraintsPayload>` with stable
  blocked claim IDs; Markdown and check scanning use catalog blocked terms.
- `constraints check` reads `report.constraints` artifacts and prints
  `report.constraints_check` artifacts instead of `lab.constraint_check_result.v1`.
- `lab.suitability_decision.v1` and `lab.design_constraint_pack.v1` schemas and
  golden fixtures were deleted. `lab.suitability_policy.v1` remains an active
  public input schema as a generated snapshot.
- Phase 3 landed at +68 Rust lines while reducing maintained-by-hand schemas
  from 25 to 22. LoC remains a guardrail; schema maintenance is the primary
  completed objective for this phase.

Phase 4 outcome:

- `main.rs` now owns CLI structs, parsing, version handling, and dispatch only;
  command execution moved to focused modules under `crates/adc-lab/src/commands/`.
- File-budget enforcement is active: `make file-budgets` runs with `--enforce`
  and is part of `make verify`.
- `main.rs` dropped from 2,482 lines at the Phase 3 baseline to 605 lines.
  Every production Rust file is under the configured budget.
- Phase 4 landed at +84 Rust lines, inside the -100 to +100 forecast window,
  with no schema-count change.

Phase 5 outcome:

- All active v1 schema contracts are generated snapshots under
  `schemas/generated`; no top-level handwritten `.schema.json` files remain.
- `make schemas-check` now runs the schema ledger with `--enforce-final`, so
  `make verify` fails if maintained-by-hand schema rows return or a
  generated-check target remains ungenerated.
- Previously no-schema wire contracts for observation result, run context,
  build info, and workload fixture result now have generated snapshots and
  golden fixtures.
- Control schemas were converted last and remain covered by fixture validation,
  negative schema tests, CLI tests, and safety invariant tests.
- Phase 5 landed at +9 Rust lines while reducing maintained-by-hand schemas
  from 22 to 0. `platform_contract.rs` stayed at 1,496 lines, so no file-budget
  exemption was needed.

Phase 6 outcome:

- `docs/README.md` now defines the normative documentation set and marks
  architecture notes as reference-only unless they are listed as normative.
- README and CLI/reference docs use v2 example paths for suitability and
  constraints outputs, document public output paths against artifact `kind`,
  and no longer claim retired v1 report/probe/suitability artifacts as primary
  outputs.
- `docs-smoke` checks the docs index, CLI reference, pressure reference, and
  existing normative docs.
- Approval-record and restore-lease schema negative tests now prove generated
  control schemas reject forged shell/restore-command fields.

Final v2.1 outcome:

- The v2.1 completion plan is satisfied through Phase 6.
- Maintained-by-hand schema JSON went from 32 top-level files at Phase 0 to 0;
  generated snapshots now cover 43 contracts.
- Production Rust file budgets are enforced by `make verify` and have 0
  violations. `main.rs` remains 605 lines and `platform_contract.rs` remains
  1,496 lines.
- Total Rust LoC is 18,058 versus the Phase 0 baseline of 17,939. The net
  increase is accepted because LoC is a guardrail, production file budgets are
  enforced, and the primary schema-source objective is complete.
- Final verification: `make verify` passed with schema drift/final ledger
  enforcement, file budgets, unit/integration/safety/contract tests, docs
  smoke, and command smoke.
