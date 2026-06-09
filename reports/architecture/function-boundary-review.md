# Function Boundary Review: PR11 Workload And Target Capability Profiles

## Scope

Changed functions/helpers:

- `target_capability_profile` in
  `crates/adc-lab-core/src/capability_profile.rs`.
- `validate_workload_profile`, artifact readers, result summarizers, and claim
  builders in `crates/adc-lab-core/src/capability_profile.rs`.
- `command_report_capability_profile` in `crates/adc-lab/src/main.rs`.
- `WorkloadProfile`, `TargetCapabilityProfile`, and related DTOs in
  `crates/adc-lab-core/src/contracts.rs`.

## Semantic Neighbors

| Function / Type | Neighbor classification | Decision |
| --- | --- | --- |
| `capability_cost_model` | parallel architecture-evidence model | keep parallel because cost model classifies architecture options, while capability profile binds a target to a workload |
| `operating_point_coverage` | parallel claim-boundary report | keep separate because operating-point coverage owns factor/level coverage, not workload requirements |
| `pack_run` and `run_manifest` | parallel run-summary report | keep separate because familiarization pack summarizes read-only evidence, not workload-specific capability |
| `collect_artifact_refs` in `report.rs` | same artifact-ref concept, different private module scope | keep small local reader in `capability_profile.rs`; no public utility extraction until more report builders need the same traversal semantics |
| `command_report_operating_point` | parallel CLI report command | keep separate; capability profile has a workload input and a different audit operation |

## Decisions

- Keep workload/profile generation in a new `capability_profile` core module
  instead of extending `report.rs`. The new concept is a target-selection
  foundation layer, not an operating-point or architecture-cost report.
- Keep `capability_cost_model` unchanged. It remains architecture evidence
  classification and does not own workload requirements.
- Keep `selection_ready` as an explicit field on target capability profiles.
  PR11 generator sets it to `false`.
- Keep missing evidence as a profile state instead of a CLI error. Malformed
  JSON artifacts remain validation errors.
- Ledger update not required: no replaced abstraction, intentional duplication,
  or staged adapter remains. The local artifact traversal is narrow and can be
  refactored later if another report reuses it.

## Boundary Decisions

| Boundary | Action | Rationale |
| --- | --- | --- |
| Workload DTOs | keep | workload identity and requirements need schema/golden validation |
| Target capability DTOs | keep | target evidence and blocked claims need schema/golden validation |
| `target_capability_profile` | keep | pure-ish builder with filesystem reads, no target execution side effects |
| Artifact reader helpers | keep local | avoids broad shared helper extraction before semantics settle |
| CLI profile command | keep | artifact write and audit side effects belong at CLI/run boundary |

## Error Behavior

- Invalid workload profile schema version, empty identity, zero duration, or
  empty measurement requirements returns a validation error.
- Missing run artifacts produce an explicit `no_evidence` or
  `exploratory_partial` profile with `selection_ready=false`.
- Malformed run artifacts return validation errors and do not produce a profile.

## Verification

Planned commands:

- `cargo test -p adc-lab-core capability_profile -- --nocapture`
- `cargo test -p adc-lab --test cli report_capability_profile -- --nocapture`
- `make contract`
- `make verify`

Final results are recorded in `reports/quality-gate.md`.
