# Function Boundary Review: PR8 Capability Cost Model

## Scope

Changed functions/helpers:

- `capability_cost_model`, `capability_evidence`,
  `capability_model_status`, capability-specific builders, architecture option
  builders, blocked-claim builders, and JSON artifact readers in
  `crates/adc-lab-core/src/report.rs`.
- `command_report_operating_point` in `crates/adc-lab/src/main.rs`.
- `CapabilityCostModel`, `CapabilityEvidence`, `CapabilityCostDimension`,
  `ArchitectureOptionEvidence`, `CapabilityClaimBoundary`, and their enums in
  `crates/adc-lab-core/src/contracts.rs`.

## Semantic Neighbors

| Function / Type | Neighbor classification | Decision |
| --- | --- | --- |
| `capability_cost_model` | report-domain peer of `operating_point_coverage` | keep in `report.rs`; it interprets existing run artifacts only |
| `capability_evidence` | capability packet assembly | keep split from top-level builder so observed/missing capability rules remain readable |
| capability-specific builders | repeated DTO construction with different claim boundaries | keep small helpers instead of a generic map until real platform adapters add evidence |
| `architecture_options` | architecture claim boundary domain | keep separate from capability facts because option decisions are not capability discovery |
| `capability_blocked_claims` | claim-trace neighbor with architecture semantics | keep separate from read-only and operating-point claim traces |
| `read_json_artifact_if_exists` | artifact parsing boundary | use as generic optional parser; malformed JSON is a validation error |
| `read_experiment_run_if_exists` | named domain adapter around generic parser | keep thin wrapper for call-site meaning |
| `command_report_operating_point` | CLI side-effect boundary | keep artifact writes and audit in CLI; core owns model semantics |
| Capability DTOs | contract domain | keep in `contracts.rs`; schema mirrors the public Agent-facing contract |

## Decisions

- No architecture-decision engine is introduced. PR8 records evidence
  sufficiency and blocked options; it does not choose GPU/NPU/DSP/storage
  designs.
- No platform adapter is added. Missing GPU/NPU/DSP/storage/network evidence is
  represented explicitly instead of probing new surfaces.
- No target-local runtime is added. The model reads controller-side run
  artifacts and writes a report artifact.
- Malformed JSON artifacts fail report generation because evidence cannot be
  trusted. Missing optional artifacts become `missing_evidence`, blocked
  claims, or limitations.
- CLI owns persistence and emits `report.capability_cost`; core owns
  architecture evidence semantics.
- Ledger update not required: no long-lived sibling abstraction, staged adapter,
  or intentional duplicate implementation remains.

## Verification

- `cargo test -p adc-lab-core capability_cost -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli report_operating_point -- --nocapture`:
  pass.
- Full workspace verification is recorded in `reports/quality-gate.md`.
