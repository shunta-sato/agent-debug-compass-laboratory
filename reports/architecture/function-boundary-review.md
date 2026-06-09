# Function Boundary Review: PR7 Controlled Operating Point Coverage

## Scope

Changed functions/helpers:

- `operating_point_coverage`, `read_experiment_run_if_exists`,
  `add_completed_trial_points`, `add_blocked_trial_points`,
  `ensure_fixed_frequency_blocked`, `blocked_status_for_factor`,
  `is_safety_blocked_factor`, `next_evidence_for_blocked_factor`,
  `coverage_status`, and `operating_point_claim_boundaries` in
  `crates/adc-lab-core/src/report.rs`.
- `command_report_operating_point` in `crates/adc-lab/src/main.rs`.
- `OperatingPointCoverage`, `OperatingPointCoverageStatus`,
  `OperatingPointEvidenceClass`, `OperatingPointCoveragePoint`,
  `OperatingPointBlockedPoint`, and `OperatingPointClaimBoundary` in
  `crates/adc-lab-core/src/contracts.rs`.

## Semantic Neighbors

| Function / Type | Neighbor classification | Decision |
| --- | --- | --- |
| `operating_point_coverage` | same report domain as `pack_run`, `read_only_claim_trace`, `run_manifest` | keep in `report.rs`; it interprets existing run artifacts |
| `read_experiment_run_if_exists` | parallel to `artifact_ref_if_exists`; different responsibility because it parses trusted schema JSON | keep split to preserve parse-error context |
| `add_completed_trial_points` | same coverage-building domain as blocked-point helpers | split from top-level builder so controlled evidence rules are readable |
| `add_blocked_trial_points` | parallel concept to completed trial points, but opposite outcome semantics | keep parallel because blocked/failure evidence rules will diverge |
| `ensure_fixed_frequency_blocked` | fixed invariant for claim boundary | keep explicit instead of deriving from generic factor names |
| `blocked_status_for_factor` / `is_safety_blocked_factor` | same classification domain | keep small helpers; no generic policy module until more statuses exist |
| `next_evidence_for_blocked_factor` | same blocked-point domain | keep explicit so operator-facing next evidence stays close to reason classification |
| `coverage_status` | top-level status reduction | keep pure helper for testable precedence |
| `operating_point_claim_boundaries` | claim trace neighbor but coverage-specific evidence semantics | keep separate from `read_only_claim_trace` and experiment claim trace |
| `command_report_operating_point` | CLI side-effect boundary | keep artifact writes and audit in CLI, keep classification in core |
| Coverage DTOs | contract domain | keep in `contracts.rs`; no separate module until contract family grows |

## Decisions

- No merge into a generic evidence or policy utility. Coverage classification,
  blocked-point reasons, and claim boundaries are domain-specific.
- No replacement or staged migration. The previous skeleton shape is replaced
  in one coherent schema/runtime update with tests.
- `operating_point_coverage` remains read/report-only. It does not execute
  target commands, control operations, or loads.
- CLI owns artifact persistence and `report.operating_point` audit; core owns
  artifact interpretation.
- Ledger update not required: no long-lived sibling abstraction, staged adapter,
  or intentional duplicate implementation remains.

## Verification

- `cargo test -p adc-lab-core operating_point -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli report_operating_point -- --nocapture`:
  pass.
- Full workspace verification is recorded in `reports/quality-gate.md`.
