# Function Boundary Review: Evidence Pack Consistency & Target Capability Readiness

## Scope

Changed functions/helpers:

- `create_or_open_run` and `run_id_from_run_dir` in
  `crates/adc-lab-core/src/run.rs`.
- `pack_run`, `read_only_claim_trace`, and `run_manifest` in
  `crates/adc-lab-core/src/report.rs`.
- Operation summary, audit-fact, release-identity, and data-quality helpers in
  `crates/adc-lab-core/src/report.rs`.
- `target_runner_build_info` in `crates/adc-lab-core/src/target.rs`.
- CLI persistence helpers `persist_controller_version_if_absent` and
  `persist_target_runner_version_if_absent`.
- `target_capability_profile` gating of formal comparison when manifest
  identity is inconsistent.

## Semantic Neighbors

| Function / Type | Neighbor classification | Decision |
| --- | --- | --- |
| Existing audit append path | correlation source | keep; do not introduce a separate log stream |
| Existing `artifact_uri_for_run` | evidence ref boundary | keep; reuse bounded artifact refs |
| Existing read-only claim helpers | same claim-generation purpose | replace with operation-summary based helpers |
| Existing target capability profile | downstream consumer | keep `selection_ready=false`; add identity inconsistency blocker |
| Tool qualification reports | evidence-source identity | extend summary rather than changing qualification report semantics |

## Decisions

- Replace read-only-only report derivation with one private
  `RunEvidenceSummary`. This avoids manifest, familiarization pack, and claim
  trace drifting when load artifacts are added after read-only operations.
- Persist `run_context.json` for custom `--run-dir` paths. Directory name is
  not a reliable logical run id when users provide arbitrary paths.
- Keep release metadata fields required in `lab.run_manifest.v1`, but use
  `unknown` plus `data_quality.missing` when a local source build lacks release
  asset checksum data. This keeps source-build verification possible without
  making release identity silently complete.
- Keep `target_capability_profile.selection_ready=false` in this phase and add
  blocked claims/next evidence when run manifest identity is inconsistent.

## Boundary Decisions

| Boundary | Action | Rationale |
| --- | --- | --- |
| `RunEvidenceSummary` | add | single owner for artifact-derived operation status |
| `run_id_from_run_dir` | add | shared run identity lookup for reports and capability profiles |
| `read_only_claim_trace` | keep name, change internals | public API remains stable while claims become operation-aware |
| `run_manifest` | change signature | controller `BuildInfo` is required to prevent version drift |
| `ToolQualificationSummary.tools[]` | add | preserves existing lists while making per-tool status machine-readable |
| `target_runner_build_info` | add | fixed command version capture, no arbitrary shell surface |

## Error Behavior

- Artifact collection and malformed JSON still fail commands because evidence
  cannot be trusted.
- Missing optional release identity lowers data quality instead of failing
  source-built local report generation.
- Audit run-id mismatch records `data_quality.inconsistent` and blocks later
  formal comparison.
- Load artifacts without matching load/experiment audit also record
  `data_quality.inconsistent`.

## Verification

Required commands:

- `cargo test --workspace`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `make verify`

Focused tests added:

- Load artifact updates manifest, familiarization pack, and claim trace.
- Load-containing pack cannot be `observational_read_only`.
- Audit run-id mismatch degrades manifest data quality.
- Target capability profile blocks formal comparison when binary identity is
  inconsistent.
