# Function Boundary Review: PR9 Agent Adapter Qualification

## Scope

Changed functions/helpers:

- `qualify_tool_with_evidence`, `validate_agent_tool_evidence`,
  `agent_adapter_scope_is_qualifiable`, and related policy helpers in
  `crates/adc-lab-core/src/qualification.rs`.
- `command_tool_qualify`, `build_pending_tool_qualification_evidence`,
  evidence readers, and evidence artifact persistence helpers in
  `crates/adc-lab/src/main.rs`.
- `ToolQualification` and `ToolQualificationEvidence` contracts in
  `crates/adc-lab-core/src/contracts.rs` and
  `crates/adc-lab-core/src/qualification.rs`.

## Semantic Neighbors

| Function / Type | Neighbor classification | Decision |
| --- | --- | --- |
| `qualify_tool` | manifest-only compatibility wrapper | keep as wrapper around `qualify_tool_with_evidence` |
| `qualify_tool_with_evidence` | main policy boundary for agent-created tools | split from inventory qualification so agent evidence rules do not leak into builtin policy |
| `validate_agent_tool_evidence` | evidence-field validation boundary | keep pure core validation for refs, sha, bounds, and version |
| `agent_adapter_scope_is_qualifiable` | scope allowlist policy | keep explicit and conservative; no generic adapter framework yet |
| `qualify_tool_info` | discovered toolchain inventory qualification | keep parallel because builtin/external inventory policy differs from agent-created evidence policy |
| `build_pending_tool_qualification_evidence` | CLI boundary for local evidence files | keep in CLI because it reads filesystem inputs and writes run artifacts |
| evidence file readers | filesystem/format validation | keep separate by JSON vs text error contracts |
| `ToolQualificationEvidence` | core input DTO, not an artifact contract | keep in `qualification.rs`; report artifact remains `ToolQualification` |

## Decisions

- No arbitrary adapter execution is added. PR9 validates supplied files and
  records artifact-backed evidence only.
- No merge with `qualify_tool_info`. Inventory qualification and agent-created
  adapter qualification have different sources, side effects, and error
  contracts.
- `ToolQualification` remains the single report artifact. Optional evidence
  fields are explicit nulls for builtin/unqualified cases.
- CLI owns local path handling and artifact copying. Core owns policy decisions
  over logical refs and manifest fields.
- Control/load/privileged/state-writing adapters are deliberately kept
  unqualified in PR9.
- Ledger update not required: no long-lived sibling abstraction, staged adapter,
  or intentional duplicate implementation remains.

## Verification

- `cargo test -p adc-lab-core qualification -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli tool_qualification -- --nocapture`: pass.
- Full workspace verification is recorded in `reports/quality-gate.md`.
