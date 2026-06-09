# ExecPlan: Evidence Pack Consistency & Target Capability Readiness

## Purpose / Big Picture

Make adc-lab report packs internally consistent across run manifest,
familiarization pack, claim evidence trace, audit, operation summary, release
identity, and tool qualification summary. The outcome is not a new Pi4/Pi5
measurement or comparison result. The outcome is a safer evidence pack that can
later be transformed into target capability profiles without hidden drift.

## Scope

In scope:

- Extend `lab.run_manifest.v1` with controller/target binary identity, release
  asset identity, binary checksums, operation summary, and richer data quality.
- Derive manifest, familiarization pack, and claim trace from one operation
  summary source.
- Detect run-id mismatch between `audit.jsonl` and `run_manifest.run_id`.
- Prevent load-containing packs from claiming `observational_read_only`.
- Generate bounded load short-smoke supported claims when load result artifacts
  exist.
- Ensure familiarization pack references tool qualification summary.
- Add contract/unit tests for artifact/manifest consistency.

Out of scope:

- New Pi4/Pi5 measurements.
- Pi4 vs Pi5 comparison or suitability decision.
- New privileged control surface, cpufreq/governor changes, sustained thermal,
  all-core, wakeup, latency, or battery measurements.
- Release workflow changes beyond using existing build identity in manifests.

## Constraints / Quality Targets

- No arbitrary shell, no new privileged operation, no destructive experiment.
- Existing `artifact://lab/runs/...` evidence ref semantics remain bounded and
  raw filesystem paths do not enter Agent-facing fields.
- Report generation must be conservative: inconsistencies degrade evidence
  quality instead of being silently hidden.
- `make verify` remains the default local gate.

## Context & Orientation

Key paths:

- `crates/adc-lab-core/src/report.rs`: report pack, run manifest, claim trace,
  operating-point and capability report builders.
- `crates/adc-lab-core/src/contracts.rs`: contract DTOs.
- `crates/adc-lab/src/main.rs`: CLI persistence functions and audit emission.
- `schemas/lab.run_manifest.v1.schema.json`
- `schemas/lab.familiarization_pack.v1.schema.json`
- `schemas/lab.claim_evidence_trace.v1.schema.json`
- `tests/golden/*.valid.json`
- `crates/adc-lab-core/tests/contract_validation.rs`

Discovered facts:

- `pack_run` currently infers `observational_read_only` from inventory,
  toolchain, and observation only.
- `read_only_claim_trace` does not include bounded load completion claims.
- `read_only_data_quality_missing` always records `no load or stress experiment
  was run`, even if `loads/*.result.json` exists.
- `persist_run_manifest` uses `env!("CARGO_PKG_VERSION")`, which can differ
  from release-injected `ADC_LAB_VERSION` used by `--version`.
- Existing audit events use `RunContext.run_id`, but report pack does not
  detect pre-existing audit lines with a different `run_id`.
- `--run-dir` paths in tests and operator workflows may not be named
  `LAB-RUN-*`; deriving run id from the directory name is not sufficient.

## Dev Workflow Route

Risk level: high-normal.

Why: this changes Agent-facing evidence contracts and report generation
semantics, but does not introduce privileged control, concurrency, or target
runtime measurement behavior.

Triggered branches:

- `dev-workflow`: mandatory for code/test changes.
- `execution-plans`: cross-boundary contract/report change.
- `function-boundary-governor`: new helper boundaries and shared operation
  summary source.
- `observability`: report pack/audit/run-id consistency must be diagnosable.
- `error-handling`: inconsistent evidence becomes explicit data-quality state.
- `quality-gate`: final submit decision.

Skipped branches:

- Embedded NFR skills: no new target-local runtime, polling, measurement
  cadence, budget, or production-resource claim.
- Architecture decision analysis: no competing architecture options.
- Concurrency/thread-safety: no concurrency change.
- Bug RCA: no live incident reproduction required, though regression tests will
  capture the reported inconsistency class.

## Design

Introduce an operation summary in `report.rs` derived from artifacts and audit:

- inventory
- toolchain discovery
- passive observe
- bounded load
- privileged control
- controlled operating point
- sustained thermal

Use that same summary to build:

- `RunManifest.operations_summary`
- `RunManifest.data_quality`
- `FamiliarizationPack.pack_status`
- `FamiliarizationPack.supported_claims`
- `FamiliarizationPack.blocked_claims`
- `ClaimEvidenceTrace.claims`

Release and binary identity:

- Use `build_info("adc-lab")` for controller version/git/target/profile.
- Persist `tools/adc-lab-target.version.json` when target operations run.
- Read target version from that artifact when generating the manifest.
- Read optional release metadata from environment and/or release manifest when
  available; otherwise record fields as `unknown` and add data-quality missing
  entries.
- Compute local controller binary checksum from `current_exe` when available.

Error handling:

- Artifact discovery failures remain errors.
- Evidence inconsistencies that do not make artifact refs unsafe are recorded in
  `data_quality.inconsistent`.
- Audit run-id mismatch is recorded as inconsistent and should make downstream
  capability comparison refuse formal comparison.

Observability:

- Primary signal is `run_manifest.data_quality`.
- Correlation identifier is `run_id`.
- Degraded evidence is machine-readable through `operations_summary`,
  `data_quality.missing`, `data_quality.inconsistent`, and audit refs.

Function boundary decisions:

- Replace separate read-only missing/claims helpers with operation-summary based
  helpers.
- Keep `pack_run`, `read_only_claim_trace`, and `run_manifest` public entry
  points for CLI compatibility.
- Add small private helpers for operation discovery, identity discovery, and
  claim generation instead of broad refactoring.

## Validation & Acceptance

Commands:

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `make verify`

Focused tests:

- load artifact prevents `observational_read_only`.
- load artifact generates supported short-smoke claim.
- manifest no longer records "no load was run" when load result exists.
- audit `run_id` mismatch is detected.
- schema fixtures validate with new required fields.
- claim evidence refs remain `artifact://lab/runs/...`.

## Progress (WBS)

- [x] Explore existing report/schema/audit generation paths.
- [x] Create ExecPlan and route work.
- [x] Update contracts and schemas.
- [x] Implement operation summary and identity discovery.
- [x] Update CLI persistence to capture target version and use release build
  identity.
- [x] Add tests and update golden fixtures.
- [x] Update docs/reports.
- [x] Run verification.
- [x] Final quality gate and PR handoff.

## Surprises & Discoveries

- `plans/_template_execplan.md` is referenced by the skill docs but does not
  exist in this repository; this plan was created directly with the required
  sections from `PLANS.md`.
- PR #13 was merged into `origin/main` as `a3a6102`; this work branches from
  that merged state.
- Custom `--run-dir` was a bigger consistency issue than only report packing:
  without a persisted context, separate CLI invocations could use different
  logical run ids in one directory.

## Decision Log

- 2026-06-09: Use one derived operation summary as the source for manifest,
  pack, and claim trace. This prevents read-only and load evidence paths from
  diverging.
- 2026-06-09: Represent unavailable release metadata as present `unknown`
  fields plus data-quality missing entries, rather than omitting required
  fields. This keeps the schema stable for source-built local verification.
- 2026-06-09: Persist `run_context.json` for every run directory. This makes
  `--run-dir` a stable logical run boundary across CLI invocations.

## Handoff

Current branch: `codex/evidence-pack-consistency-readiness`.

Current status: implementation and verification complete.

Next steps:

1. Commit and push branch.
2. Open PR against `main`.
3. After merge, rerun the Pi4/Pi5 measurement flow from release binaries.

## Outcomes & Retrospective

Implemented evidence pack consistency fixes:

- run manifests include version/git/release/binary identity and operation
  summaries.
- custom run directories persist `run_context.json`, preventing logical run-id
  drift across commands.
- manifest, familiarization pack, and claim trace are generated from one
  operation summary.
- bounded load artifacts create short-smoke supported claims while sustained
  and production claims remain blocked.
- target capability profiles block formal comparison when manifest binary
  identity is inconsistent.

Verification passed:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `make verify`
- `git diff --check`
