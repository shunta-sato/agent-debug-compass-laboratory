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

Phase 1 responsibility map:

- `evidence/envelope.rs`: v2 artifact envelope, status vocabulary, claim
  instance DTOs, strict serde/schema shape.
- `evidence/catalog.rs`: stable claim IDs and default wording/next-evidence
  text. Claim instances may add context-specific next evidence but do not fork
  catalog wording.
- `evidence/store.rs`: run-directory indexing, v2 artifact load/write,
  `artifact://` reference construction, symlink refusal, malformed JSON
  diagnostics, and write audit events.
- `runfs/mod.rs`: bounded run-directory-relative path construction.
- `examples/generate_schemas.rs`: generated schema snapshots from Rust DTOs.

Phase 1 intentionally adds one kernel abstraction instead of wiring reports or
probes through it immediately. Public CLI/report behavior remains v1 until
Phase 4 cutover.

Phase 1 complexity budget and audit:

- New public abstraction budget: `Artifact<P>`, `EvidenceStore`, and
  `artifact_path`.
- New support DTO budget: claim catalog entries, artifact metadata, strict
  envelope status/data-quality DTOs, and schema generator example.
- Post-implementation decision: keep these abstractions because they each own
  one repeated concern from the plan; do not add report/probe adapters until
  Phases 2-3 prove the call sites.

Phase 2 responsibility map:

- `rules/engine.rs`: predicate evaluation, table-driven rule evaluation,
  evidence-ref collection, and claim instance projection from catalog defaults.
- `rules/operating_contract.rs`: v2 operating-contract artifact payload and
  rule table behind core API only.
- `rules/suitability.rs`: v2 suitability artifact payload and conservative
  selection-readiness rule table behind core API only.
- `evidence/catalog.rs`: claim IDs, blocked scan terms, and default next
  evidence lookup for both v1 constraints and v2 rules.
- `suitability.rs`: legacy v1 public DTO generation remains in place, but its
  initial blocked claim terms now come from the claim catalog.

Phase 2 complexity budget and audit:

- New public abstraction budget: `Pred`, `Rule`, `RuleEvaluation`, plus two
  v2 report payload structs.
- Post-implementation decision: keep this as Rust tables, not YAML, and keep
  CLI/report cutover out of Phase 2. The v1 generators remain parallel until
  Phase 4/5 parity cleanup.

Phase 3 responsibility map:

- `probe/artifacts.rs`: core-only adapters that normalize existing v1 observe,
  load, pressure, composite, and workload payloads into v2 `Artifact<P>`
  records and write them through `EvidenceStore`.
- `examples/generate_schemas.rs`: v2 schema snapshots for observation, load,
  pressure, composite, and workload artifacts.
- `tests/probe_artifacts.rs`: verifies all five probe kinds are indexed by the
  store and records the dummy pressure-kind extension exercise.

Phase 3 dummy pressure-kind exercise evidence:

- Hand-edited extension files: `crates/adc-lab-core/src/probe/artifacts.rs`,
  `crates/adc-lab-core/src/rules/operating_contract.rs`, and
  `crates/adc-lab-core/tests/probe_artifacts.rs`.
- Count: 3 hand-edited files. Kernel enum registration and `make schemas`
  snapshots are counted as generated/mechanical evidence, not extension files.
- The exercise is encoded in
  `dummy_pressure_kind_extension_exercise_stays_within_three_hand_edited_files`.

Phase 4 responsibility map:

- `adc-lab/src/commands/decide.rs`: `decide suitability` now writes a v2
  `Artifact<SuitabilityPayload>` as the primary `--out` artifact and keeps the
  v1 decision as a `.v1.json` parity sidecar.
- `adc-lab/src/commands/report.rs`: `report operating-contract` now returns
  and writes `reports/target_operating_contract.v2.json` while preserving v1
  report artifacts for parity and Phase 5 cleanup.
- `adc-lab/src/main.rs`: dispatches cutover command groups through
  `commands/`, and probe commands write v2 sidecar artifacts through
  `EvidenceStore` after their existing v1 writes.
- `rules/suitability.rs`: projects legacy suitability decisions into the v2
  artifact envelope during the transition.

Phase 4 complexity budget and audit:

- New CLI module budget: two command modules, `decide` and `report`.
- New compatibility budget: one `.v1.json` sidecar path helper and one
  legacy-to-v2 suitability projection helper.
- Post-implementation decision: keep target/helper wire output unchanged in
  this phase; the host CLI wraps parsed outputs into v2 artifacts so safety
  semantics and SSH/helper contracts remain unchanged.

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
- [x] Phase 1: Implement evidence kernel and schema generation.
- [x] Phase 1: Add `make schemas` to `Makefile` and register it in
      `COMMANDS.md`.
- [x] Phase 2: Implement `rules/engine.rs`.
- [x] Phase 2: Implement claim catalog and connect blocked-claim lookup.
- [x] Phase 2: Replace operating contract internals behind core APIs while
      keeping CLI v1 output.
- [x] Phase 2: Replace suitability internals behind core APIs while keeping CLI
      v1 output.
- [x] Phase 2: Connect `constraints check` to the claim catalog.
- [x] Phase 3: Replace probe outputs with `Artifact<P>`.
- [x] Phase 4: Split CLI modules and update target/helper payloads.
- [x] Phase 4: Cut CLI output and `cli.rs` expectations over to v2 together.
- [x] Phase 5: Delete v1 surfaces and compress docs.
- [x] Phase 5: Update `Makefile` `docs-smoke` paths in the same commit as
      normative doc deletion.
- [x] Run final whole-cutover `make verify`.
- [x] Review follow-up: Restore v2 suitability E2E flow and conservative
      coupling semantics.
  - [x] Add CLI regression coverage for `report operating-contract` ->
        `decide suitability` -> `constraints generate` using only
        tool-produced v2 artifacts.
  - [x] Add rules regression coverage so insufficient composite evidence does
        not support memory/storage coupling.
  - [x] Add probe regression coverage so repeated v2 pressure/composite
        sidecars are not overwritten.
  - [x] Add generated schema drift detection to the default `make verify`
        gate.
  - [x] Update audit behavior, docs, RCA artifact, outcomes, and final
        verification evidence.
- [x] Final cleanup follow-up: delete remaining dead v1 public report surfaces,
      expand v2 operating-contract semantic coverage, and remeasure final
      LoC/schema counts.
  - [x] Delete public v1 platform-mechanism inventory, boundary-probe plan,
        resource-coupling report, target-operating-contract generator,
        run-set manifest, and multi-run operating-contract generator surfaces.
  - [x] Delete the corresponding dead DTOs while preserving active pressure
        runtime DTOs and the `TargetOperatingContract` projection used by the
        suitability policy engine.
  - [x] Add v2 predicates for long load duration, endpoint-backed bounded
        network transfer, and observation sample count.
  - [x] Expand v2 operating-contract rules for thermal soak, storage writes,
        network bounded transfer, pressure jitter, and observer cadence.
  - [x] Regenerate schemas and record final Rust LoC plus handwritten/generated
        schema counts.
- [x] Native suitability follow-up: remove the remaining v1 operating-contract
      projection from the public `decide suitability` path.
  - [x] Make `decide suitability` read only `Artifact<OperatingContractPayload>`
        for `--target-contract`.
  - [x] Build `Artifact<SuitabilityPayload>` directly from the existing numeric
        suitability policy evaluator.
  - [x] Keep v2 suitability `payload.blocked_claims` as stable catalog claim
        IDs, not prose-derived legacy IDs.
  - [x] Delete `TargetOperatingContract` projection helpers and now-unused DTOs.
  - [x] Update CLI/core regressions to use tool-produced v2 artifacts.
  - [x] Run final verification and record final LoC/schema counts.

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
- Phase 1 schema generation uses `schemars` 0.8 because it fits the existing
  serde DTO stack and does not force external schema DSL maintenance.
- `ArtifactHeader` is intentionally not `deny_unknown_fields`: it is an
  internal partial reader used to index a full strict artifact without parsing
  the payload.
- The generated `schemars::schema_for!` root stores envelope
  `additionalProperties: false` at the root object, while nested properties
  are under `properties`.
- Phase 2 keeps the public CLI on v1 shapes. The new operating-contract and
  suitability artifacts are exposed only through core API tests and generated
  v2 schemas.
- Making `Kind` `Copy` simplified rule-table evidence kind slices; existing
  store code was updated to avoid clone-on-copy clippy failures.
- Phase 3 wraps existing v1 probe outputs into compact v2 payloads instead of
  deriving schemas for every nested v1 DTO. Rationale: Phase 4 owns the public
  CLI cutover, and Phase 5 deletes obsolete v1 surfaces after parity.
- Phase 4 changed public CLI output for `decide suitability` and
  `report operating-contract` to v2 artifacts. Probe commands now write v2
  sidecars, but still print their v1 result payloads until Phase 5 cleanup.
- Phase 5 could not delete `platform_contract.rs` as a whole because that file
  still owns the active pressure probe runtime (`run_resource_pressure`,
  `run_composite_boundary`, `PressureProbeOptions`) used by the CLI. Phase 5
  deleted the public v1 report/schema/demo surfaces and left the pressure
  runtime for a later file-boundary split.
- Review follow-up discovery: after PR #37 merged the phase stack into `main`,
  `decide suitability` still read `TargetOperatingContract` directly, so the
  v2 `target_operating_contract.v2.json` artifact emitted by
  `report operating-contract` failed before policy evaluation.
- Review follow-up discovery: `constraints generate` still read
  `SuitabilityDecision` directly, so the v2 `Artifact<SuitabilityPayload>`
  emitted by `decide suitability` could not feed the public README loop.
- Review follow-up discovery:
  `operating.memory_storage_coupling_requires_composite` used only artifact
  presence, allowing insufficient composite evidence to support a coupling
  claim.
- Final cleanup discovery: `TargetOperatingContract` and its rule DTOs are not
  dead yet because `decide suitability` still projects the v2 operating
  contract into the legacy numeric suitability policy evaluator. The run-dir
  v1 target-contract generator is dead and was deleted; the projection DTOs
  remain.
- Final cleanup discovery: deleting the remaining v1 report generators removes
  about two thousand lines from `platform_contract.rs`, but the whole-repo Rust
  LoC target of 40-50% reduction remains unmet because active probe, control,
  suitability, manifest, and report-pack code still exist.
- Native suitability follow-up discovery: after the final cleanup follow-up,
  the last `TargetOperatingContract` use was a compatibility projection inside
  `decide suitability`. The documented public input was already
  `target_operating_contract.v2.json`, so the remaining v1 DTO surface could be
  removed by making the numeric suitability evaluator return
  `Artifact<SuitabilityPayload>` directly.

Review follow-up route summary:

- Risk level: high. Public CLI artifacts, claim decisions, schema verification,
  and audit behavior change, but target/helper/control safety semantics are not
  touched.
- Definition of Done:
  - `decide suitability` accepts the v2 operating contract emitted by
    `report operating-contract`.
  - `constraints generate` accepts the v2 suitability artifact emitted by
    `decide suitability`.
  - memory/storage coupling is not `Supported` unless measured pressure effect
    and measured composite scenario evidence are both present.
  - repeated pressure/composite probes preserve v2 sidecars instead of
    overwriting prior results.
  - `make verify` detects generated schema drift.
- Test List:
  - CLI E2E suitability loop with v2 artifacts only.
  - CLI constraints generation from v2 suitability artifact.
  - rules engine insufficient composite regression.
  - probe v2 sidecar uniqueness regression.
  - schema drift check through `make verify`.
- Responsibility map:
  - `rules/engine.rs`: owns generic predicate evaluation over indexed
    artifacts and typed payload loading.
  - `rules/operating_contract.rs`: owns operating contract rule catalog and
    conservative report status aggregation.
  - `rules/suitability.rs`: owns v2 suitability artifact projection and
    v2-to-v1 constraint-pack view.
  - `probe/artifacts.rs`: owns v1 probe result to v2 sidecar naming and payload
    normalization.
  - `adc-lab` command layer: owns public CLI compatibility, audit events, and
    file IO.
- Complexity budget:
  - New modules/classes: 0.
  - New helper functions: up to 8, all local to existing modules.
  - New dependencies: 0.
  - Production line budget: about 180 lines.
  - Test/docs/plan line budget: about 260 lines.
- Branch evidence:
  - Bug RCA artifact: `reports/bug-reports/20260611-v2-review-followup.md`.
  - Architecture option analysis: not triggered; no competing architecture
    options are being selected.
  - Concurrency/embedded NFR/UI: not triggered; no runtime loop, control
    surface, background execution, or UI changes.

Final cleanup follow-up route summary:

- Risk level: high. The change deletes public core exports and changes
  operating-contract claim semantics, but leaves target/helper/control safety
  semantics untouched.
- Definition of Done:
  - code/tests no longer reference the dead v1 public report generators or DTOs
    named in review.
  - v2 operating-contract rule output covers coupling, thermal, storage,
    network, jitter, observer, and production-readiness claim boundaries.
  - network bounded-transfer support requires endpoint-backed payload fields,
    not a pressure artifact's existence alone.
  - generated schemas are updated and drift check passes.
  - final Rust LoC and schema counts are recorded in Outcomes.
- Test List:
  - rules engine expanded coverage and conservative blocked-claim regression.
  - rules engine endpoint-backed network bounded-transfer regression.
  - probe artifact payload/schema regression.
  - CLI v2 suitability loop smoke.
  - full `make verify`.
- Responsibility map:
  - `rules/engine.rs`: owns typed semantic predicates over v2 artifacts.
  - `rules/operating_contract.rs`: owns operating-contract claim coverage and
    conservative decisions.
  - `probe/artifacts.rs`: owns normalization from active v1 pressure runtime
    DTOs into compact v2 pressure payloads.
  - `contracts.rs`: retains active runtime DTOs and deletes dead v1 report
    DTOs.
  - `platform_contract.rs`: owns only bounded pressure/composite runtime after
    this cleanup.
- Complexity budget:
  - New modules/classes: 0.
  - New helper functions: 4 small predicate/payload helpers.
  - New dependencies: 0.
  - Production line budget: net deletion, with about 170 new semantic lines and
    about 2,100 deleted v1 generator/helper lines.
  - Test/docs/plan line budget: about 240 lines.
- Post-implementation economy audit:
  - Kept new predicate helpers because they remove rule-table pressure to use
    `Custom` closures for common evidence semantics.
  - Deleted dead v1 report generators and DTOs instead of adding adapters.
  - Kept `TargetOperatingContract` DTOs only for the existing suitability
    policy projection; removing them requires a separate suitability-engine
    replacement.

Native suitability follow-up route summary:

- Risk level: high. The change removes public core DTO exports and tightens the
  `decide suitability --target-contract` input contract, but it does not touch
  target/helper/control safety semantics.
- Definition of Done:
  - `decide suitability` consumes the v2 operating-contract artifact emitted by
    `report operating-contract` without projecting it into v1 DTOs.
  - v2 suitability artifacts use stable claim IDs in `payload.blocked_claims`;
    legacy prose remains only in the v1 design-constraint-pack projection.
  - dead `TargetOperatingContract` DTOs and projection helpers have no remaining
    code/test references.
  - focused CLI/core tests and `make verify` pass.
- Test List:
  - core unknown-required-dimension regression over a v2 operating-contract
    artifact.
  - CLI `decide suitability` with a tool-produced v2 operating-contract
    artifact.
  - CLI `report operating-contract` -> `decide suitability` ->
    `constraints generate` loop.
  - rules-engine catalog/projection regression.
  - full `make verify`.
- Responsibility map:
  - `suitability.rs`: owns numeric policy evaluation and direct v2 suitability
    artifact construction.
  - `rules/suitability.rs`: owns the v2 payload shape plus v2-to-v1
    constraint-pack projection.
  - `rules/operating_contract.rs`: owns only v2 operating-contract rule output,
    not v1 DTO projection.
  - `adc-lab/src/commands/decide.rs`: owns CLI input validation, output write,
    and audit event for suitability decisions.
- Complexity budget:
  - New modules/classes: 0.
  - New helper functions: 4 local helpers for suitability policy evaluations,
    next-evidence derivation, and rule ID suffixes.
  - New dependencies: 0.
  - Production line budget: net deletion, with no new public compatibility
    layer.
  - Test/docs/plan line budget: about 130 lines.
- Post-implementation economy audit:
  - Deleted the v2-to-v1-to-v2 projection instead of adding another adapter.
  - Kept the existing numeric policy evaluator because it still owns useful
    CPU/thermal/memory threshold semantics.
  - Left the v1 `DesignConstraintPack` projection in place because
    `constraints generate` still publishes that schema.

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
- Phase 0 PR: draft PR #30 opened at
  `https://github.com/shunta-sato/agent-debug-compass-laboratory/pull/30`.
- Phase 1 schema generation: `make schemas` passed and regenerated
  `schemas/generated/lab.artifact.v2.schema.json` plus
  `schemas/generated/lab.claim_catalog_entry.v2.schema.json`.
- Phase 1 focused verification:
  `cargo test -p adc-lab-core --test evidence_kernel -- --nocapture` passed
  with 7 tests.
- Phase 1 contract gate:
  `cargo test --workspace contract_validation -- --nocapture` passed and kept
  the Phase 0 safety invariant integration tests green.
- Phase 1 final gate: `make verify` passed after adding the evidence kernel,
  schema generation, and command registry entry. The gate ran workspace build,
  `cargo fmt --all --check`, clippy with `-D warnings`, library tests,
  integration tests, contract validation, docs smoke, and command smoke.
- Phase 1 PR: draft PR #31 opened at
  `https://github.com/shunta-sato/agent-debug-compass-laboratory/pull/31`.
- Phase 2 schema generation: `make schemas` passed and regenerated catalog
  schema plus new `lab.report.operating_contract.v2.schema.json` and
  `lab.report.suitability.v2.schema.json` snapshots.
- Phase 2 focused verification:
  `cargo test -p adc-lab-core --test rules_engine -- --nocapture` passed with
  5 tests.
- Phase 2 kernel regression:
  `cargo test -p adc-lab-core --test evidence_kernel -- --nocapture` passed
  with 7 tests.
- Phase 2 contract gate:
  `cargo test --workspace contract_validation -- --nocapture` passed and kept
  Phase 0 safety invariant tests green.
- Phase 2 final gate: `make verify` passed after adding the rules engine,
  catalog-backed blocked claim terms, v2 report schemas, and focused tests.
- Phase 2 PR: draft PR #32 opened at
  `https://github.com/shunta-sato/agent-debug-compass-laboratory/pull/32`.
- Phase 3 schema generation: `make schemas` passed and added
  `lab.observation.v2.schema.json`, `lab.load.v2.schema.json`,
  `lab.pressure.v2.schema.json`, `lab.composite.v2.schema.json`, and
  `lab.workload.v2.schema.json`.
- Phase 3 focused verification:
  `cargo test -p adc-lab-core --test probe_artifacts -- --nocapture` passed
  with 2 tests.
- Phase 3 regression verification:
  `cargo test -p adc-lab-core --test evidence_kernel -- --nocapture` passed
  with 7 tests, and `cargo test -p adc-lab-core --test rules_engine -- --nocapture`
  passed with 5 tests.
- Phase 3 contract gate:
  `cargo test --workspace contract_validation -- --nocapture` passed and kept
  Phase 0 safety invariant tests green.
- Phase 3 final gate: `make verify` passed after adding v2 probe artifact
  adapters, generated schemas, and the dummy pressure-kind exercise.
- Phase 3 PR: draft PR #33 opened at
  `https://github.com/shunta-sato/agent-debug-compass-laboratory/pull/33`.
- Phase 4 schema generation: `make schemas` passed with no schema command
  failure after the CLI cutover changes.
- Phase 4 focused CLI verification:
  `cargo test -p adc-lab --test cli decide_suitability_reads_run_evidence_and_keeps_thermal_target_conditioned -- --nocapture`
  passed.
- Phase 4 focused CLI verification:
  `cargo test -p adc-lab --test cli report_operating_contract_writes_contract_artifacts -- --nocapture`
  passed.
- Phase 4 full CLI verification:
  `cargo test -p adc-lab --test cli -- --nocapture` passed with 32 tests.
- Phase 4 contract gate:
  `cargo test --workspace contract_validation -- --nocapture` passed and kept
  Phase 0 safety invariant tests green.
- Phase 4 final gate: `make verify` passed after CLI cutover, command module
  split, v2 probe sidecar writes, and updated `cli.rs` expectations.
- Phase 4 PR: draft PR #34 opened at
  `https://github.com/shunta-sato/agent-debug-compass-laboratory/pull/34`.
- Phase 5 schema generation: `make schemas` passed and regenerated the v2
  generated schema set after deleting obsolete v1 report schemas.
- Phase 5 schema count: `find schemas -maxdepth 1 -type f -name '*.schema.json' | wc -l`
  reports 32 handwritten v1 schemas, down from the 40 baseline; generated v2
  schemas remain 9.
- Phase 5 focused CLI verification:
  `cargo test -p adc-lab --test cli report_operating_contract_writes_contract_artifacts -- --nocapture`
  passed.
- Phase 5 focused CLI verification:
  `cargo test -p adc-lab --test cli report_operating_contract_accepts_include_run_in_v2_store -- --nocapture`
  passed.
- Phase 5 focused CLI verification:
  `cargo test -p adc-lab --test cli pressure_composite_promotes_coupling_evidence_class -- --nocapture`
  passed after adding the v2 rule-required pressure artifact to the test setup.
  Review follow-up later replaced this expectation with
  `pressure_composite_smoke_does_not_support_coupling_without_measured_effect`.
- Phase 5 full CLI verification:
  `cargo test -p adc-lab --test cli -- --nocapture` passed with 31 tests.
- Phase 5 contract gate:
  `cargo test --workspace contract_validation -- --nocapture` passed with
  Phase 0 safety invariant tests still green.
- Phase 5 final gate: `make verify` passed after deleting the capability
  profile module, capability cost model generator, public v1 report sidecars,
  v1 report schemas/goldens, v1 demo packs, resource-smoke alias, and stale
  docs.
- Phase 5 PR: draft PR #36 opened at
  `https://github.com/shunta-sato/agent-debug-compass-laboratory/pull/36`.
- Review follow-up baseline: `git rev-parse --short origin/main` reported
  `03529e3`, the PR #37 rollup merge commit.
- Review follow-up red tests:
  `cargo test -p adc-lab-core --test rules_engine operating_contract_coupling_requires_measured_effect_not_just_presence -- --nocapture`
  failed because presence-only coupling matched insufficient composite
  evidence.
- Review follow-up red tests:
  `cargo test -p adc-lab-core --test probe_artifacts pressure_and_composite_v2_sidecars_keep_each_result_id -- --nocapture`
  failed because same-kind/scenario v2 sidecars overwrote prior results.
- Review follow-up red tests:
  `cargo test -p adc-lab --test cli suitability_loop_consumes_tool_produced_v2_artifacts_end_to_end -- --nocapture`
  failed because `decide suitability` tried to deserialize a v2 operating
  contract as v1.
- Review follow-up focused verification:
  `cargo test -p adc-lab-core --test rules_engine -- --nocapture` passed with
  6 tests.
- Review follow-up focused verification:
  `cargo test -p adc-lab-core --test probe_artifacts -- --nocapture` passed
  with 3 tests.
- Review follow-up focused verification:
  `cargo test -p adc-lab-core --test evidence_kernel -- --nocapture` passed
  with 7 tests.
- Review follow-up focused verification:
  `cargo test -p adc-lab --test cli -- --nocapture` passed with 32 tests.
- Review follow-up schema verification: `make schemas` passed and regenerated
  `schemas/generated/lab.report.suitability.v2.schema.json`; `make
  schemas-check` passed using a temporary schema output directory.
- Review follow-up final gate: `make verify` passed. The gate now includes
  build, format, clippy, generated schema drift detection, library tests,
  integration tests, contract validation, docs smoke, and command smoke.
- Final cleanup baseline: `git rev-parse --short origin/main` reported
  `8e7036a`, the merged review-follow-up main commit.
- Final cleanup focused verification:
  `cargo check -p adc-lab-core` passed after deleting dead v1 report surfaces.
- Final cleanup focused verification:
  `cargo test -p adc-lab-core --test rules_engine -- --nocapture` passed with
  8 tests.
- Final cleanup focused verification:
  `cargo test -p adc-lab-core --test probe_artifacts -- --nocapture` passed
  with 3 tests.
- Final cleanup focused verification:
  `cargo test -p adc-lab-core --test safety_invariants -- --nocapture` passed
  with 19 tests.
- Final cleanup focused verification:
  `cargo test -p adc-lab --test cli suitability_loop_consumes_tool_produced_v2_artifacts_end_to_end -- --nocapture`
  passed.
- Final cleanup focused verification: `cargo check --workspace` passed.
- Final cleanup schema verification: `make schemas` passed and regenerated
  `schemas/generated/lab.pressure.v2.schema.json`; `make schemas-check`
  passed using a temporary schema output directory.
- Final cleanup measurement:
  `find crates -name '*.rs' -type f -print0 | xargs -0 wc -l | tail -n 1`
  reported 18,038 total Rust lines.
- Final cleanup measurement:
  `find schemas -maxdepth 1 -type f -name '*.schema.json' | wc -l` reported
  32 handwritten schema files.
- Final cleanup measurement:
  `find schemas/generated -maxdepth 1 -type f -name '*.schema.json' | wc -l`
  reported 9 generated schema files.
- Final cleanup large-file measurement:
  `wc -l crates/adc-lab-core/src/contracts.rs crates/adc-lab-core/src/platform_contract.rs crates/adc-lab-core/src/rules/operating_contract.rs crates/adc-lab-core/src/rules/engine.rs crates/adc-lab-core/src/probe/artifacts.rs crates/adc-lab/src/main.rs`
  reported `contracts.rs` 1,393; `platform_contract.rs` 1,496;
  `operating_contract.rs` 246; `engine.rs` 200; `artifacts.rs` 378;
  `main.rs` 2,595.
- Final cleanup final gate: `make verify` passed. The gate ran workspace
  build, format check, clippy with `-D warnings`, generated schema drift check,
  library tests, integration tests, contract validation, docs smoke, and
  command smoke.
- Native suitability follow-up baseline:
  `git log --oneline -5` showed `35cb532`, the merged final-cleanup follow-up
  main commit, as the current branch base.
- Native suitability follow-up stale-surface scan:
  `rg "TargetOperatingContract|OperatingContractRule|OperatingRuleCategory|ContractConfidence|OperatingRuleSource|OperatingBoundary|legacy_contract_from_v2_artifact|suitability_artifact_from_legacy_decision_v2|claim_id_for_blocked_claim|legacy\\.suitability|projected-v2-contract|target_operating_contract\\.json" crates/adc-lab-core/src crates/adc-lab/src crates/adc-lab-core/tests crates/adc-lab/tests schemas tests/golden -n`
  returned no matches after the cleanup.
- Native suitability follow-up focused verification:
  `cargo check -p adc-lab-core` passed.
- Native suitability follow-up focused verification:
  `cargo test -p adc-lab-core suitability_policy_unknown_cannot_become_meet -- --nocapture`
  passed.
- Native suitability follow-up focused verification:
  `cargo test -p adc-lab --test cli decide_suitability_writes_v2_without_legacy_sidecar -- --nocapture`
  passed.
- Native suitability follow-up focused verification:
  `cargo test -p adc-lab --test cli suitability_loop_consumes_tool_produced_v2_artifacts_end_to_end -- --nocapture`
  passed.
- Native suitability follow-up focused verification:
  `cargo check --workspace` passed.
- Native suitability follow-up focused verification:
  `cargo fmt --all -- --check` passed.
- Native suitability follow-up focused verification:
  `cargo test -p adc-lab-core --test rules_engine -- --nocapture` passed with
  8 tests.
- Native suitability follow-up clippy fix:
  the first `make verify` run failed on
  `clippy::too_many_arguments` for `decide_suitability_artifact_v2`; the fix
  introduced `SuitabilityArtifactContext` and reran focused checks.
- Native suitability follow-up verification:
  `cargo clippy --workspace --all-targets -- -D warnings` passed after the
  context-struct fix.
- Native suitability follow-up schema verification:
  `make schemas` passed and regenerated the current generated schema snapshots.
- Native suitability follow-up schema verification:
  `make schemas-check` passed using a temporary schema output directory.
- Native suitability follow-up final gate: `make verify` passed. The gate ran
  workspace build, format check, clippy with `-D warnings`, generated schema
  drift check, library tests, integration tests, contract validation, docs
  smoke, and command smoke.
- Native suitability follow-up measurement:
  `find crates -name '*.rs' -type f -print0 | xargs -0 wc -l | tail -n 1`
  reported 17,939 total Rust lines.
- Native suitability follow-up measurement:
  `find schemas -maxdepth 1 -type f -name '*.schema.json' | wc -l` reported
  32 handwritten schema files.
- Native suitability follow-up measurement:
  `find schemas/generated -maxdepth 1 -type f -name '*.schema.json' | wc -l`
  reported 9 generated schema files.
- Native suitability follow-up large-file measurement:
  `wc -l crates/adc-lab-core/src/contracts.rs crates/adc-lab-core/src/suitability.rs crates/adc-lab-core/src/rules/operating_contract.rs crates/adc-lab-core/src/rules/suitability.rs crates/adc-lab/src/commands/decide.rs crates/adc-lab/src/main.rs`
  reported `contracts.rs` 1,321; `suitability.rs` 943;
  `operating_contract.rs` 190; `rules/suitability.rs` 222;
  `commands/decide.rs` 54; `main.rs` 2,595.

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
- 2026-06-11: Implement Phase 1 as a side-by-side v2 evidence kernel with no
  CLI/report cutover. Rationale: this proves the envelope, store, schema, and
  command surfaces while keeping existing public output and safety tests green.
- 2026-06-11: Use `LabError::Validation` for evidence-store trust-boundary
  refusals such as symlinks, malformed JSON artifacts, and path escapes.
  Rationale: these are repository/run-directory contract violations, not claim
  decisions.
- 2026-06-11: Keep Phase 2 v2 operating-contract and suitability outputs
  core-only. Rationale: this satisfies the table-driven rule acceptance
  criteria without creating a dual public CLI contract before Phase 4.
- 2026-06-11: Store blocked claim scan terms in the claim catalog while
  preserving v1 `DesignConstraintPack` string shape. Rationale: constraints
  can move to one source of truth without changing the CLI schema yet.
- 2026-06-11: Keep Phase 3 probe v2 writes behind core APIs instead of changing
  CLI persistence paths. Rationale: Phase 2-3 must keep CLI v1 shapes green,
  with public cutover deferred to Phase 4.
- 2026-06-11: Use compact v2 probe payloads as adapter outputs rather than
  schema-deriving every nested v1 DTO. Rationale: this keeps the v2 artifact
  contract strict and small while v1 payloads are still temporary.
- 2026-06-11: Keep target/helper wire payloads unchanged in Phase 4 and wrap
  them into v2 artifacts in the host CLI. Rationale: changing remote/helper
  stdout at the same time as public CLI cutover would risk safety-boundary
  regressions; Phase 0 safety invariant tests stay authoritative.
- 2026-06-11: Keep v1 artifacts as explicit parity sidecars after public v2
  cutover. Rationale: Phase 5 owns deletion after green parity evidence.
- 2026-06-11: Delete public v1 report/schema/demo surfaces in Phase 5 while
  leaving the pressure runtime in `platform_contract.rs` until it can be split
  without changing probe behavior. Rationale: `platform_contract.rs` still owns
  active bounded pressure execution, while the CLI no longer emits the deleted
  v1 report artifacts.
- 2026-06-11: Review follow-up keeps the legacy suitability policy evaluator as
  the numeric policy engine but adds v2 artifact readers/projections at the CLI
  boundary. Rationale: this restores the public v2 E2E loop without rebuilding
  target-run numeric suitability semantics in the same PR.
- 2026-06-11: Promote measured-effect checks into the core predicate
  vocabulary. Rationale: v2 parity requires claim decisions to inspect artifact
  status and effect evidence, not mere file existence.
- 2026-06-11: Write `evidence.write` as normal `lab.audit_event.v1` entries.
  Rationale: one audit stream should keep a single schema for existing
  consumers, and v2 evidence writes still need audit coverage.
- 2026-06-11: Delete the remaining dead v1 public report generators and DTOs
  from `platform_contract.rs`/`contracts.rs`, but keep `TargetOperatingContract`
  projection DTOs. Rationale: the public CLI now emits v2 artifacts, while the
  legacy suitability policy evaluator still needs a typed projection until the
  suitability engine is replaced.
- 2026-06-11: Expand operating-contract coverage through typed predicates
  rather than `Custom` closures. Rationale: thermal, network, observer, and
  jitter semantics are common enough to be first-class rule vocabulary, and
  artifact presence alone must not support claim boundaries.
- 2026-06-11: Make public suitability decisions consume v2 operating-contract
  artifacts directly and delete the `TargetOperatingContract` projection DTOs.
  Rationale: the documented CLI input is already
  `target_operating_contract.v2.json`; the old v2-to-v1-to-v2 path preserved a
  dead public surface and allowed prose-derived claim IDs to leak into v2.

## Handoff

- Branch: `codex/adc-labv2-suitability-native`.
- Baseline commit: `35cb532`.
- Current status: Phase 0-5, review follow-up, and final cleanup follow-up were
  merged into `main`. Native suitability follow-up is implemented locally from
  `origin/main` with `make verify` passing.
- Uncommitted changes: native suitability follow-up pending commit/PR.
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
  - `make schemas`.
  - `cargo test -p adc-lab-core --test evidence_kernel -- --nocapture`.
  - `cargo test -p adc-lab-core --test rules_engine -- --nocapture`.
  - `cargo test -p adc-lab-core --test probe_artifacts -- --nocapture`.
  - `cargo test -p adc-lab --test cli -- --nocapture`.
  - `make verify`.
- Review follow-up commands:
  - `git fetch --prune origin`.
  - `git switch -c codex/adc-labv2-review-fixes origin/main`.
  - focused red/green tests listed in the verification log.
  - `make schemas`.
  - `make schemas-check`.
  - `cargo fmt --all`.
  - `make verify`.
- Final cleanup commands:
  - `git switch -c codex/adc-labv2-final-cleanup origin/main`.
  - `cargo check -p adc-lab-core`.
  - `cargo test -p adc-lab-core --test rules_engine -- --nocapture`.
  - `cargo test -p adc-lab-core --test probe_artifacts -- --nocapture`.
  - `cargo test -p adc-lab-core --test safety_invariants -- --nocapture`.
  - `cargo test -p adc-lab --test cli suitability_loop_consumes_tool_produced_v2_artifacts_end_to_end -- --nocapture`.
  - `cargo check --workspace`.
  - `cargo fmt --all`.
  - `make schemas`.
  - `make schemas-check`.
  - `make verify`.
  - final LoC/schema measurement commands listed in the verification log.
- Native suitability follow-up commands:
  - `git switch -c codex/adc-labv2-suitability-native origin/main`.
  - stale-surface `rg` checks listed in the verification log.
  - `cargo check -p adc-lab-core`.
  - `cargo test -p adc-lab-core suitability_policy_unknown_cannot_become_meet -- --nocapture`.
  - `cargo test -p adc-lab --test cli decide_suitability_writes_v2_without_legacy_sidecar -- --nocapture`.
  - `cargo test -p adc-lab --test cli suitability_loop_consumes_tool_produced_v2_artifacts_end_to_end -- --nocapture`.
  - `cargo check --workspace`.
  - `cargo fmt --all -- --check`.
  - `cargo test -p adc-lab-core --test rules_engine -- --nocapture`.
  - `cargo clippy --workspace --all-targets -- -D warnings`.
  - `make schemas`.
  - `make schemas-check`.
  - `make verify`.
  - final LoC/schema measurement commands listed in the verification log.
- Next steps:
  1. Commit and push `codex/adc-labv2-suitability-native`.
  2. Open a PR targeting `main`.
  3. Watch CI for `make verify`.
- Read these files first when resuming:
  - `plans/20260611-v2-evidence-kernel.md`
  - `crates/adc-lab-core/src/control.rs`
  - `crates/adc-lab-core/src/platform_contract.rs`
  - `crates/adc-lab-core/src/report.rs`
  - `crates/adc-lab-core/src/evidence/`
  - `crates/adc-lab-core/src/probe/`
  - `crates/adc-lab-core/src/rules/`
  - `crates/adc-lab/src/commands/`
  - `crates/adc-lab/tests/cli.rs`

## Outcomes & Retrospective

Phase 0 through Phase 5 are implemented, locally verified, pushed, and
published as stacked PRs, then rolled into `main` via PR #37.

Review follow-up outcome:

- The public v2 suitability loop now accepts the artifacts emitted by the CLI:
  v2 operating contract -> v2 suitability artifact -> design constraint pack.
- The coupling claim is conservative again: pressure/composite artifact
  presence is insufficient without observed pressure effect and measured
  composite evidence.
- Repeated pressure/composite probes retain distinct v2 sidecars by result ID.
- `make verify` now detects generated schema drift through `schemas-check`.
- EvidenceStore writes `evidence.write` as normal `lab.audit_event.v1`, and
  `decide.suitability` / `constraints.generate` add audit events for
  claim-producing outputs.

Final cleanup follow-up outcome:

- The remaining dead v1 public report surfaces named in review are removed
  from code/tests: platform-mechanism inventory, boundary-probe plan,
  resource-coupling report, v1 target-operating-contract run-dir generator,
  run-set manifest, and multi-run operating-contract.
- `platform_contract.rs` is now reduced to active bounded pressure/composite
  runtime plus tests: 1,496 lines, down from the plan baseline of about 3,538.
- v2 operating-contract coverage now evaluates seven rule rows: coupling,
  sustained thermal soak, bounded storage writes, bounded network transfer,
  pressure-specific jitter, observer cadence, and production readiness.
- `PressurePayload` now carries optional endpoint-backed network transfer
  fields so `NetworkBoundedTransfer` can inspect payload semantics.
- Final measured Rust implementation total is 18,038 lines versus the original
  19,311-line denominator, a 1,273-line reduction. The planned 40-50% overall
  code-reduction target remains unmet.
- Final schema counts are 32 handwritten v1 schema files and 9 generated v2
  schema files. The original "0 handwritten schemas" quantitative target
  remains unmet because active v1 control, run manifest, workload, suitability,
  and report-pack surfaces still publish v1 schemas.

Native suitability follow-up outcome:

- `decide suitability` now consumes only v2
  `Artifact<OperatingContractPayload>` input for `--target-contract` and builds
  `Artifact<SuitabilityPayload>` directly from the numeric suitability policy
  evaluator.
- The old v2 operating-contract -> `TargetOperatingContract` ->
  v2 suitability projection is removed, along with `TargetOperatingContract`,
  `OperatingContractRule`, `OperatingBoundary`, and related enum DTOs.
- v2 suitability `payload.blocked_claims` now stays in stable catalog claim IDs;
  legacy prose rendering remains confined to the current
  `DesignConstraintPack` projection.
- Final measured Rust implementation total is 17,939 lines versus the original
  19,311-line denominator, a 1,372-line reduction. The planned 40-50% overall
  code-reduction target remains unmet.
- Final schema counts remain 32 handwritten v1 schema files and 9 generated v2
  schema files.
