# Quality Gate Report

Gate decision: submit

Findings: 0

## Checks Run

- `cargo fmt --all --check` - pass.
- `cargo clippy --workspace --all-targets -- -D warnings` - pass.
- `cargo test --workspace` - pass.
- `make verify` - pass.
- `git diff --check` - pass.

## Live Discovery

- Repo command surface: `COMMANDS.md` still defines `make verify` as the
  canonical local gate.
- Report generation paths inspected:
  `crates/adc-lab-core/src/report.rs`, `crates/adc-lab/src/main.rs`,
  `crates/adc-lab-core/src/run.rs`, `crates/adc-lab-core/src/target.rs`, and
  `crates/adc-lab-core/src/capability_profile.rs`.
- Contract schemas inspected and updated:
  `schemas/lab.run_manifest.v1.schema.json`,
  `schemas/lab.familiarization_pack.v1.schema.json`, and
  `schemas/lab.tool_qualification_summary.v1.schema.json`.
- External/target state: no hardware target was contacted. This PR changes
  evidence-pack consistency and comparison readiness only.

## Triggered Branch Evidence

- ExecPlan - present:
  `plans/20260609-evidence-pack-consistency-readiness.md`.
- Function boundary review - present:
  `reports/architecture/evidence-pack-consistency-function-boundary-review.md`.
- Observability - present:
  `docs/architecture/observability-plan.md`.
- Error handling - present:
  `docs/architecture/error-handling.md`.
- Audit/reproducibility documentation - present:
  `docs/architecture/audit-and-reproducibility.md`.

## Exit Criteria Review

- `lab.run_manifest.v1` now records controller/target binary identity,
  release asset identity fields, binary SHA map, operation summary, operation
  audit refs, and `data_quality.missing|inconsistent|notes`.
- `run_context.json` persists logical run id for custom `--run-dir`, so
  repeated operations in the same directory share one `run_id`.
- Report packing detects audit run-id mismatch and records it as
  `data_quality.inconsistent`.
- Manifest, familiarization pack, and claim trace are generated from the same
  artifact-derived operation summary.
- Load-containing packs cannot report `observational_read_only`; bounded load
  artifacts generate only short-smoke load claims and keep sustained,
  production, and all-operating-point claims blocked.
- Tool qualification summary exposes per-tool status and evidence acceptance;
  familiarization pack includes `tool_qualification_summary_ref`.
- Target capability profiles keep `selection_ready=false` and add a formal
  comparison blocker when run manifest binary identity is inconsistent.
- No arbitrary shell, new privileged control surface, destructive experiment,
  Pi4/Pi5 measurement, suitability decision, or production/NFR claim was added.

## Gate Decision

Submit. The change makes report packs version-correct, run-id-consistent,
operation-aware, claim-traced, and tool-qualified enough for later target
capability profile generation, without expanding measurement or control scope.
