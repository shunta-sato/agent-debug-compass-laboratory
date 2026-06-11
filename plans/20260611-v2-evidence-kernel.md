# ExecPlan: adc-lab v2 Evidence Kernel Refactor

## Purpose / Big Picture

Refactor adc-lab v2 around a unified evidence kernel so the cost of ordinary
extension work becomes near-constant.

The goal is not line-count reduction by itself. The goal is to make common
changes small, typed, reviewed in one place, and hard to turn into unsupported
claims:

- Adding a pressure kind should require at most three hand-edited extension
  files: payload/probe code, one rule-table row, and an optional targeted test.
- Adding a blocked claim should require one claim-catalog entry.
- Adding a report should require one compact rule-set definition instead of a
  new bespoke generator.
- JSON schemas should be generated from Rust types, with handwritten schemas
  removed from source control or treated only as generated snapshots.

The North Star remains unchanged: no Agent root shell, no uncontrolled
experiment, no unapproved hard-to-restore operation, no unqualified tool
evidence, and no claim without audit.

## Scope

In scope:

- Introduce `crates/adc-lab-core/src/evidence/` as the unified evidence kernel:
  envelope, store, and claim catalog.
- Introduce `crates/adc-lab-core/src/rules/` as the rule evaluation layer for
  operating contracts, run reports, and suitability decisions.
- Move run-directory artifact discovery and `artifact://` reference generation
  into a single `EvidenceStore`.
- Replace scattered claim prose with stable claim IDs backed by a catalog.
- Replace handwritten schema maintenance with Rust-type-driven schema
  generation using `schemars` or an equivalent type-first generator.
- Consolidate overlapping report generators into rule sets while keeping
  `run_manifest` as a dedicated consistency artifact.
- Preserve the three-binary boundary:
  - `adc-lab`
  - `adc-lab-target`
  - `adc-lab-priv-helper`
- Preserve privileged control semantics and wrap result payloads only after
  safety invariant tests are frozen.
- Compress docs into a small normative set after v2 behavior is in place.

Out of scope:

- Merging `adc-lab-target` into the host CLI.
- Merging `adc-lab-priv-helper` into any non-privileged binary.
- Adding arbitrary shell execution or a generic remote shell wrapper.
- Relaxing approval, digest-binding, restore, helper allowlist, or SSH
  sanitization rules.
- Reopening Pi4/Pi5 readiness or target-selection claims as part of the
  refactor.
- v2 does not read v1 artifacts. No v1-to-v2 data migration or compatibility
  layer will be built.

## Constraints / Quality Targets

- Keep `adc-lab` as a safety-gated experiment laboratory, not an arbitrary
  shell wrapper.
- Keep privileged operations typed, bounded, approved, audited, and restorable.
- Use the smallest safe change per phase and keep `make verify` green at each
  phase boundary.
- Preserve strict schema posture: no unknown fields, no silent additional
  properties, and no free-form claim prose where stable IDs are required.
- Preserve logical run identity and `artifact://lab/runs/...` references.
- Preserve target-runner and helper allowlists, including
  `ADC_LAB_TARGET_RUNNER` restrictions.
- Treat target-local physical evidence as bounded, audited, and claim-limited.
- `crates/adc-lab-core/tests/safety_invariants.rs` and
  `crates/adc-lab/tests/safety_invariants.rs` must stay green for every phase
  and every commit after Phase 0 creates them.
- Do not add generated agent indexes that reference ignored `.agents/` paths.

Quantitative targets:

- Reduce implementation code by roughly 40-50% after v1 removal.
- Reduce handwritten schema files from the current 40 source files to zero
  maintained-by-hand files.
- Reduce normative documentation to four primary documents:
  `safety-model.md`, `evidence-model.md`, `rules.md`, and `cli.md`.
- Keep the extension scenarios in the Purpose section demonstrably true through
  tests or documented dry-run exercises.

## Context & Orientation

Current repository facts:

- Branch: `codex/adc-labv2`.
- Baseline commit at plan creation: `543edf0`.
- `COMMANDS.md` defines `make verify` as the default final gate.
- `plans/_template_execplan.md` is absent, so this plan follows `PLANS.md`
  required sections directly.
- `crates/adc-lab-core/src/lib.rs` currently exports flat modules including
  `contracts`, `report`, `platform_contract`, `capability_profile`, and
  `suitability`.
- `schemas/` currently contains 40 handwritten `*.schema.json` files.
- `crates/adc-lab-core/Cargo.toml` does not yet depend on `schemars`.
- Large current consolidation targets include:
  - `crates/adc-lab-core/src/contracts.rs` at about 1,690 lines.
  - `crates/adc-lab-core/src/report.rs` at about 2,180 lines.
  - `crates/adc-lab-core/src/platform_contract.rs` at about 3,538 lines.
  - `crates/adc-lab-core/src/capability_profile.rs` at about 623 lines.
  - `crates/adc-lab-core/src/suitability.rs` at about 809 lines.
  - `crates/adc-lab/src/main.rs` at about 2,758 lines.

Key files to read first during implementation:

- `crates/adc-lab-core/src/contracts.rs`: current DTO and schema source model.
- `crates/adc-lab-core/src/report.rs`: current run manifest, pack,
  claim-trace, coverage, and capability-cost generation.
- `crates/adc-lab-core/src/platform_contract.rs`: current operating-contract,
  pressure, coupling, multi-run, and helper discovery logic.
- `crates/adc-lab-core/src/capability_profile.rs`: target capability profile
  that v2 should delete or fold into workload demand and suitability.
- `crates/adc-lab-core/src/suitability.rs`: suitability decision logic that
  should become a rules module.
- `crates/adc-lab-core/src/control.rs`: safety-critical control semantics to
  preserve.
- `crates/adc-lab-core/src/target.rs`: target parsing and runner allowlist.
- `crates/adc-lab/src/main.rs`: CLI dispatch, audit writes, remote quoting, and
  report persistence.
- `crates/adc-lab-target/src/main.rs`: fixed-command non-root target runner.
- `crates/adc-lab-priv-helper/src/main.rs`: typed privileged helper.
- `crates/adc-lab-core/tests/contract_validation.rs`: strict contract and
  workflow regression checks.
- `crates/adc-lab/tests/cli.rs`: CLI integration and safety boundary tests.
- `schemas/` and `tests/golden/`: current v1 schema and fixture surface.

Known duplication and replacement targets:

- `artifact_ref_if_exists` and `read_json_artifact_if_exists` exist in multiple
  generators and should move behind `EvidenceStore`.
- `run_set_manifest_for_runs` is multi-run indexing logic that should become a
  store-open operation over multiple run directories.
- Claim-boundary strings and blocked-claim text appear in reports, docs, and
  tests; v2 should route them through `evidence/catalog.rs`.

## Requirements

- REQ-V2-1: The v2 artifact model shall represent all persisted evidence and
  report outputs through `Artifact<P>` plus kind-specific payloads.
  Acceptance: generated schemas exist for every public artifact payload; tests
  verify strict serde behavior and generated-schema snapshots.

- REQ-V2-2: Claims shall be identified by stable claim IDs backed by one claim
  catalog.
  Acceptance: report generation, constraints check, blocked-claim linting, and
  generated docs read claim text from the catalog; adding a blocked claim
  requires a single catalog entry plus test expectations.

- REQ-V2-3: Run-directory traversal and artifact reference construction shall
  be owned by `EvidenceStore`.
  Acceptance: store tests cover single-run and multi-run indexing, symlink
  refusal, kind indexing, typed artifact load, write audit linkage, and
  malformed JSON handling.

- REQ-V2-4: Report behavior shall be driven by rule sets, not bespoke
  300-800-line generators.
  Acceptance: operating contract and suitability outputs are produced from
  rule tables; a fixture proves adding one rule row changes the expected report
  without adding a new generator.

- REQ-V2-5: Existing safety-critical behavior shall remain semantically
  unchanged while payload types are wrapped or replaced.
  Acceptance: safety invariant tests cover approval mismatch, forged policy
  segment refusal, apply failure restore behavior, restore read-back evidence,
  helper path allowlist, SSH argument sanitization, and runner allowlist.

- REQ-V2-6: v1 source surfaces shall be removed only after v2 parity tests
  pass.
  Acceptance: old handwritten schemas, obsolete report generators, compatibility
  aliases, planning-only adapter docs, and demo v1 artifacts are removed in the
  final phase with `make verify` green.

Parity definition: parity does not mean v2 can read or convert v1 artifacts.
For the same lab operation sequence, newly generated v2 fixtures must produce
claim decisions that are at least as conservative as v1: a v1 supported claim
may remain supported or become more cautious, and a v1 blocked claim must remain
blocked until new v2 evidence supports changing it.

## Dev Workflow Route

This plan is the routing artifact for a future high-risk refactor.

- Risk level: high.
- Why: the implementation crosses public schemas, report semantics,
  run-directory indexing, CLI behavior, target-local evidence handling, docs,
  and safety-critical control payloads.
- Required workflow branches for implementation:
  - `execution-plans`: keep this file current.
  - `destructive-refactor`: v1 report/schema abstractions will be replaced and
    deleted after parity.
  - `function-boundary-governor`: new `evidence`, `rules`, `probe`, `runfs`,
    and CLI module boundaries.
  - `error-handling`: refusal, insufficient evidence, unsafe-blocked, malformed
    artifact, and cutover failure contracts.
  - `observability`: artifacts, audit events, claim traces, and store writes are
    diagnostic signals.
  - `working-with-legacy-code`: snapshot current safety behavior before
    changing data models.
  - `embedded-nfr-design`, `embedded-hot-path-review`,
    `embedded-observer-effect-review`, `embedded-nfr-harness-design`, and
    `embedded-nfr-gate`: target-local probes and measurement overhead remain
    physical-footprint evidence.
  - `code-smells-and-antipatterns`: review the final diff for new coupling or a
    second bespoke DSL.
  - `quality-gate`: final submit gate for each phase and the whole v2 cutover.
- Explicitly not triggered by the plan-writing task:
  - Concurrency/thread-safety: no concurrent implementation is introduced by
    this document.
  - UI/C++/Android/ROS/staged-lowering: not applicable.

These workflow names are advisory routing tags from the agent environment. This
ExecPlan remains self-contained and executable from the repository plus this
file even when those helper playbooks are unavailable.

## Design

### Crate and Module Shape

Keep the three binaries. Refactor the core crate around these internal modules:

```text
crates/
  adc-lab-core/
    src/
      evidence/
        envelope.rs
        store.rs
        catalog.rs
      rules/
        engine.rs
        operating_contract.rs
        suitability.rs
        run_report.rs
      probe/
        observe.rs
        load.rs
        pressure.rs
        composite.rs
        workload.rs
      control/
      runfs/
      transport.rs
  adc-lab/
  adc-lab-target/
  adc-lab-priv-helper/
```

`adc-lab` should move toward command-group modules with `main.rs` reduced to
CLI parsing and dispatch. `adc-lab-target` and `adc-lab-priv-helper` stay thin.

### Evidence Envelope

All persisted v2 evidence should use this conceptual shape:

```rust
pub struct Artifact<P> {
    pub schema: String,
    pub kind: Kind,
    pub id: String,
    pub run_id: String,
    pub target_id: String,
    pub status: Status,
    pub bounds: Option<Bounds>,
    pub factors: Factors,
    pub metrics: Vec<Metric>,
    pub claims: Vec<Claim>,
    pub evidence_refs: Vec<ArtifactRef>,
    pub data_quality: DataQuality,
    pub payload: P,
    pub time_unix_ms: u64,
}
```

Use constructors such as `Artifact::pressure(...)` and
`Artifact::control_result(...)` to enforce kind-specific required fields that a
generic JSON schema cannot express cleanly.

The serialized shape is an ordinary struct with explicit `kind` and nested
`payload`; do not use `#[serde(flatten)]` for payloads. Enum-like status values
with data use an adjacently tagged representation such as
`#[serde(tag = "state", content = "detail")]` so strict
`deny_unknown_fields` schemas remain enforceable.

The unified `Status` should replace the current family of report-specific
status enums where behavior is equivalent:

- `Measured`
- `MeasuredPartial`
- `Insufficient`
- `NotApplicable { reason }`
- `Refused { code, message }`
- `UnsafeBlocked { reason }`

Control artifacts are the explicit exception to status vocabulary
unification. `ControlPayload` must preserve the current `ControlResultStatus`
vocabulary, including dry-run, applied, restored, failed, and refusal
structure, instead of forcing those semantics into the generic `Status` enum.

### Claim Catalog

Claims should carry stable IDs, not copied prose:

```rust
pub struct Claim {
    pub claim_id: ClaimId,
    pub decision: Decision,
    pub evidence_refs: Vec<ArtifactRef>,
    pub next_evidence: Vec<String>,
}
```

`evidence/catalog.rs` owns claim ID, supported wording, blocked wording,
default next evidence, and documentation text. A `Claim` instance may append
context-specific next-evidence items, but it must not duplicate or fork the
catalog wording. `constraints check`, report rendering, and generated
claim-boundary docs must all consume the catalog.

### Schema Generation

Add `schemars` or an equivalent type-first generator to the workspace. Keep
`serde(deny_unknown_fields)` and generate schemas from Rust DTOs. Add a
`make schemas` command and a snapshot check so schema drift is intentional.
Register `make schemas` in `COMMANDS.md` in the same change that introduces the
command.

The final source tree should not require humans to edit 40 schema files by
hand. During staged replacement, generated snapshots may coexist with v1
handwritten schemas until v2 parity is proven.

### EvidenceStore

`EvidenceStore` owns run-directory knowledge:

- one-time traversal of one or more run directories,
- symlink refusal and path normalization,
- `artifact://` reference construction,
- artifact kind indexing,
- typed artifact loading,
- artifact writing,
- audit append on writes where applicable,
- malformed or untrusted artifact diagnostics.

The store should make multi-run reporting an indexing concern rather than a
separate custom traversal path.

### Rules Engine

Use Rust declaration tables with a small predicate vocabulary. Do not start
with an external YAML DSL.

```rust
pub enum Pred {
    Present(Kind),
    PressureEffect(PressureKind),
    CompositeMeasured(Scenario),
    ControlChainComplete(OperationId),
    NetworkBoundedTransfer,
    All(Vec<Pred>),
    Any(Vec<Pred>),
    Not(Box<Pred>),
    Custom(&'static str, fn(&EvidenceStore) -> bool),
}
```

Rules bind claim IDs to predicates, statuses, and evidence-source kinds. Custom
predicates are allowed only with catalog registration and tests. If three
custom predicates share the same shape, promote the shape into the core
predicate vocabulary.

### Report Consolidation

Keep:

- `run_manifest`, as a dedicated identity and consistency artifact.

Consolidate:

- familiarization pack, claim evidence trace, and operating point coverage into
  `report.run`.
- platform mechanism inventory, resource coupling report, target operating
  contract, and multi-run operating contract into `report.operating_contract`.
- suitability decision and design constraint pack behind `rules/suitability.rs`,
  with the constraint pack as a pure projection from the decision.

Static data:

- `boundary_probe_plan` moves to static YAML data because it is an execution
  plan document, not measured evidence or a rule-generated report.

Delete after v2 parity and cutover:

- capability cost model as a standalone report.
- target capability profile as a standalone target-selection surface.
- resource-smoke compatibility alias.
- dry-run-only experiment `not_implemented` trial path.
- planning-only adapter READMEs after `docs/roadmap.md` or another single
  maintained roadmap exists.
- v1 handwritten schemas and v1 demo artifacts.

### CLI Cutover Strategy

Phase 2 and Phase 3 keep v2 output inside core APIs and table-driven tests.
The public CLI continues to emit v1 output shapes during those phases so
`crates/adc-lab/tests/cli.rs` can remain green without dual public contracts.
CLI v2 cutover and `cli.rs` expectation updates happen together in Phase 4. At
that point, v1 generators become parity-test-only code until Phase 5 deletes
them.

### Safety Invariants

`control` semantics are not redesigned in v2. Wrap `ControlResult` in
`Artifact<ControlPayload>` only after golden safety invariant tests are
independent from the old schema layout.

Safety invariant tests must be created before broad refactor work:

- plan digest binding,
- approval mismatch refusal,
- forged policy segment refusal,
- apply failure immediate restore behavior,
- restore lease and restore read-back verification,
- helper allowlist,
- `ADC_LAB_TARGET_RUNNER` allowlist,
- remote shell quoting and endpoint validation,
- no public helper override for restore.

## Validation & Acceptance

Every phase exits with `make verify` passing unless explicitly blocked and
documented with a reproducible command.

Phase-level acceptance:

- Phase 0: safety invariant tests are isolated and passing before v2 replacement
  work.
- Phase 1: `EvidenceStore` honors existing run-directory layout conventions for
  fresh v2 artifacts, refuses symlinks, and generates strict schemas for
  `Artifact<P>` snapshots. It does not implement v1 payload compatibility.
  `make schemas` exists and is registered in `COMMANDS.md`.
- Phase 2: operating contract and suitability report logic exists behind core
  APIs and table-driven tests while the CLI still emits v1 shapes. A one-row
  rule addition changes expected v2 core output in a table-driven test.
- Phase 3: observe, load, pressure, composite, and workload probe paths write
  `Artifact<P>`; a dummy pressure-kind exercise proves the extension goal.
- Phase 4: CLI command groups are split from `main.rs`; target/helper binaries
  use v2 payload types without changing safety semantics; CLI v2 output and
  `cli.rs` expectations cut over together.
- Phase 5: docs and generated schemas are aligned; v1 report/schema/demo
  surfaces are deleted; any `docs-smoke` path changes are made in `Makefile` in
  the same commit as doc deletion; the four extension scenarios are documented
  and mechanically checked where practical.

Extension-cost measurement:

- "Three files" means hand-edited extension files only. Kernel enum
  registration lines and schema snapshots produced by `make schemas` are
  tracked but do not count against the extension limit.
- Phase 3 must include a dummy pressure-kind exercise and record the diff file
  list as evidence that hand-edited files are less than or equal to three.

Required verification commands:

```bash
make schemas
make verify
```

Until `make schemas` exists, the expected substitute is:

```bash
cargo test --workspace contract_validation -- --nocapture
make verify
```

## Milestones

1. Phase 0 - Freeze and guardrails: extract safety invariant tests and make
   them the non-regression baseline for the refactor.
2. Phase 1 - Evidence kernel: add `evidence/`, `runfs/`, schema generation, and
   store indexing while v1 code remains in place.
3. Phase 2 - Rules engine: add `rules/`, replace operating contract and
   suitability internals behind core APIs, and route blocked claims through the
   claim catalog while public CLI output remains v1.
4. Phase 3 - Probe replacement: make observe, load, pressure, composite, and
   workload paths emit `Artifact<P>` directly.
5. Phase 4 - CLI and binary follow-through: split CLI command modules and adapt
   target/helper binaries to v2 payload types.
6. Phase 5 - Delete v1: remove obsolete generators, handwritten schemas,
   aliases, old docs, and v1 demo artifacts after v2 parity is proven.

## Progress (WBS)

- [x] Read the v2 refactoring design request.
- [x] Read repository planning rules in `PLANS.md`.
- [x] Read execution-plan skill reference.
- [x] Confirm current branch, baseline commit, module layout, schema count, and
      command registry.
- [x] Create this ExecPlan.
- [x] Address review findings for CLI cutover timing, no v1 artifact migration,
      measurable extension cost, serde shape, control status preservation,
      boundary probe plan ownership, Makefile/COMMANDS coupling, and WBS
      granularity.
- [x] Phase 0: Extract and freeze safety invariant tests.
- [ ] Phase 1: Implement evidence kernel and schema generation.
- [ ] Phase 1: Add `make schemas` to `Makefile` and register it in
      `COMMANDS.md`.
- [ ] Phase 2: Implement `rules/engine.rs`.
- [ ] Phase 2: Implement claim catalog and connect blocked-claim lookup.
- [ ] Phase 2: Replace operating contract internals behind core APIs while
      keeping CLI v1 output.
- [ ] Phase 2: Replace suitability internals behind core APIs while keeping CLI
      v1 output.
- [ ] Phase 2: Connect `constraints check` to the claim catalog.
- [ ] Phase 3: Replace probe outputs with `Artifact<P>`.
- [ ] Phase 4: Split CLI modules and update target/helper payloads.
- [ ] Phase 4: Cut CLI output and `cli.rs` expectations over to v2 together.
- [ ] Phase 5: Delete v1 surfaces and compress docs.
- [ ] Phase 5: Update `Makefile` `docs-smoke` paths in the same commit as
      normative doc deletion.
- [ ] Run final whole-cutover `make verify`.

## Surprises & Discoveries

- `plans/_template_execplan.md` is referenced by `PLANS.md` but is not present
  in this checkout.
- The checkout already has exactly 40 handwritten schema files under
  `schemas/`, matching the design request's schema-maintenance concern.
- `schemars` is not yet in the workspace dependency table.
- The current design pressure points are concentrated in a small number of
  large files, especially `contracts.rs`, `report.rs`, `platform_contract.rs`,
  and `adc-lab/src/main.rs`.
- Phase 0 cannot use repository-root `tests/safety_invariants.rs` because the
  workspace root is not a Cargo package. The executable safety invariant files
  are crate-local:
  `crates/adc-lab-core/tests/safety_invariants.rs` and
  `crates/adc-lab/tests/safety_invariants.rs`.

## Verification Log

- `git status --short`: clean before this plan file was added.
- `git branch --show-current`: `codex/adc-labv2`.
- `git rev-parse --short HEAD`: `543edf0`.
- `find schemas -maxdepth 1 -type f -name '*.schema.json' | wc -l`: `40`.
- `wc -l` confirmed current large-file orientation targets:
  `contracts.rs` 1,690; `report.rs` 2,180; `platform_contract.rs` 3,538;
  `capability_profile.rs` 623; `suitability.rs` 809; `adc-lab/src/main.rs`
  2,758.
- `find crates -name '*.rs' -type f -print0 | xargs -0 wc -l | tail -n 1`:
  19,311 total Rust implementation lines. This is the denominator for the
  40-50% implementation-code reduction target.
- `make verify`: passed after review updates. The gate ran workspace build,
  `cargo fmt --all --check`, clippy with `-D warnings`, library tests,
  integration tests, contract validation, docs smoke, and command smoke.
- Phase 0 targeted verification:
  `cargo test -p adc-lab-core --test safety_invariants -- --nocapture` passed
  with 19 tests.
- Phase 0 targeted verification:
  `cargo test -p adc-lab --test safety_invariants -- --nocapture` passed with
  7 tests.
- Phase 0 contract gate:
  `cargo test --workspace contract_validation -- --nocapture` passed and now
  includes both safety invariant integration-test files through their
  `contract_validation_*` test names.
- Phase 0 final gate: `make verify` passed after extracting safety invariant
  tests. The gate ran workspace build, `cargo fmt --all --check`, clippy with
  `-D warnings`, library tests, integration tests, contract validation, docs
  smoke, and command smoke.

## Decision Log

- 2026-06-11: Preserve the three-binary architecture. Rationale:
  `adc-lab-target` and `adc-lab-priv-helper` are safety boundaries, not
  accidental implementation detail.
- 2026-06-11: Adopt Rust rule tables over an external YAML DSL. Rationale:
  Rust preserves type checking and compile-time refactor support while still
  making common report extensions small.
- 2026-06-11: Use a claim catalog with stable claim IDs. Rationale: blocked
  claim text is currently copied across reports, docs, and tests; stable IDs
  make "no evidence, no claim" auditable.
- 2026-06-11: Generate schemas from Rust types. Rationale: handwritten Rust DTO
  plus JSON Schema maintenance is duplicate work and a drift risk.
- 2026-06-11: Keep `run_manifest` dedicated. Rationale: run identity, binary
  identity, and data-quality consistency are integrity checks, not ordinary
  evidence rules.
- 2026-06-11: Treat `ControlResult` change as payload wrapping only.
  Rationale: current control semantics are safety-critical and should be
  protected by tests before envelope wrapping.
- 2026-06-11: Discard all v1 run artifacts, including target55 packs, and build
  no v1-to-v2 migration layer. Rationale: explicit owner decision; v2 demo
  evidence is regenerated fresh.
- 2026-06-11: Delete v1 code surfaces only after v2 parity, with no data
  compatibility layer. Rationale: temporary code coexistence is safer than a
  large red-state rewrite across schemas, reports, CLI, and docs, but v1 data
  itself is not migrated.
- 2026-06-11: Implement Phase 0 safety invariants as crate-local integration
  tests rather than repository-root tests. Rationale: the workspace root is not
  a Cargo package, so root `tests/` would not be executed by `make verify`.

## Handoff

- Branch: `codex/adc-labv2`.
- Baseline commit: `543edf0`.
- Current status: Phase 0 implemented and verified; Phase 0 PR publication is
  next.
- Uncommitted changes: this plan file plus Phase 0 test extraction changes.
- Commands run so far:
  - `sed -n ...` on the request attachment, `PLANS.md`, execution-plan
    references, existing plans, `COMMANDS.md`, Cargo manifests, Makefile, and
    relevant module files.
  - `git status --short`.
  - `git branch --show-current`.
  - `git rev-parse --short HEAD`.
  - schema count and line-count orientation commands listed in the verification
    log.
  - `rg` checks for remaining cutover/migration wording.
  - `cargo test -p adc-lab-core --test safety_invariants -- --nocapture`.
  - `cargo test -p adc-lab --test safety_invariants -- --nocapture`.
  - `cargo test --workspace contract_validation -- --nocapture`.
  - `make verify`.
- Next steps:
  1. Commit Phase 0 on `codex/adc-labv2-phase0-safety-invariants`.
  2. Push the branch and open a draft Phase 0 PR.
  3. Start Phase 1 only after the Phase 0 PR exists.
- Read these files first when resuming:
  - `plans/20260611-v2-evidence-kernel.md`
  - `crates/adc-lab-core/src/control.rs`
  - `crates/adc-lab-core/src/platform_contract.rs`
  - `crates/adc-lab-core/src/report.rs`
  - `crates/adc-lab/tests/cli.rs`

## Outcomes & Retrospective

Pending implementation.
