# ExecPlan: Toolchain Qualification v1

## Purpose / Big Picture

PR3 makes toolchain qualification a first-class evidence contract. After PR2 can produce read-only familiarization packs, PR3 decides which discovered tools may be used as evidence sources and which remain blocked until qualification evidence exists.

The goal is to implement:

- builtin read-only tools accepted as evidence sources,
- control/load/external/agent-created tools not accepted by default,
- missing and unqualified tool reports,
- qualification artifacts linked into run manifest / familiarization pack through `artifact://lab/runs/...` refs.

This protects the North Star rule: no unqualified tool becomes evidence.

## Scope

In scope:
- Expand `lab.tool_qualification.v1` with tool kind/category, privilege, source, evidence refs, and qualification reason.
- Generate qualification reports from `lab.toolchain_inventory.v1`.
- Add CLI command for qualifying discovered toolchain inventory.
- Keep agent-created manifest-only tools unqualified.
- Add missing/unqualified tool report fields that can be inspected by agents.
- Include tool qualification refs in run manifest and familiarization pack artifact collections.
- Update observability and docs for qualification events.

Out of scope:
- Running external tools to validate output.
- Dry-run comparison harness for agent-created tools.
- Version/hash capture for external binaries beyond current inventory availability.
- Privileged control qualification.
- Load generation or stress tests.

## Constraints / Quality Targets

- Default `make verify` remains hardware-free.
- No privileged control or load generation is added.
- Tool qualification status must be conservative:
  - builtin read-only observation/probe tools may be `builtin` and `evidence_accepted=true`;
  - control, load, external, missing, or agent-created tools must not be evidence by default.
- Agent-facing evidence refs must remain `artifact://lab/runs/...`.
- Schema remains strict minimal with `required`, `enum`, and `additionalProperties:false`.

## Context & Orientation

Key existing files:
- `crates/adc-lab-core/src/toolchain.rs`: discovers builtin/local/SSH tool availability.
- `crates/adc-lab-core/src/qualification.rs`: manifest-only tool qualification skeleton.
- `crates/adc-lab-core/src/contracts.rs`: DTOs and enums.
- `crates/adc-lab/src/main.rs`: `tool qualify` CLI and run artifact writes.
- `schemas/lab.tool_qualification.v1.schema.json`: current minimal qualification schema.
- `crates/adc-lab/tests/cli.rs`: CLI integration tests.

Existing behavior:
- `toolchain discover` writes `toolchain_inventory.json`.
- `tool qualify --manifest ...` always records agent-created tools as unqualified.
- Read-only familiarization pack collects all artifacts, but does not yet compute qualified evidence source summaries.

## Design

Data model:
- Keep `ToolQualification` as the main report object.
- Add fields:
  - `category`
  - `privilege`
  - `source`
  - `available`
  - `evidence_refs`
  - `reason`
- `source` values: `builtin`, `external`, `agent_created`, `missing`.

Qualification policy:
- Builtin read-only observation/probe/report tools that are available are accepted as evidence.
- Builtin load tools are not evidence until PR5 bounded load evidence exists.
- Observation/control tools requiring privilege remain `needs_control_test` and not evidence.
- External available tools are `external_unqualified`.
- Missing tools are `refused` with `available=false`.
- Manifest-only agent-created tools are `agent_created_unqualified`.

CLI:
- Add `adc-lab tool qualify-inventory --inventory <path> [--run-dir ...]`.
- It writes one report per tool under `tools/<tool_id>.qualification.json`.
- It writes `tools/tool_qualification_summary.json`.
- It appends audit event `tool.qualify_inventory`.

Observability:
- Audit events record qualification summary generation.
- Each report has a status and reason so rejected evidence is diagnosable without reading policy code.

Test strategy:
- Schema fixture validation.
- Unit tests for builtin accepted, control needs test, external unqualified, missing refused, and manifest-only agent-created unqualified.
- CLI test qualifying a discovered local toolchain inventory and asserting builtin read-only accepted while load/control/external remain not accepted.

## Milestones

1. Expand qualification contract and fixtures.
2. Implement qualification policy over `ToolInfo` and `ToolchainInventory`.
3. Add CLI `tool qualify-inventory`.
4. Update report artifact collection/docs/observability.
5. Run verification and quality gate.

## Progress (WBS)

- [x] Create PR3 ExecPlan and route workflow.
- [x] Expand schema, fixture, and DTO.
- [x] Implement inventory qualification policy and summary artifact.
- [x] Add CLI and tests.
- [x] Update docs and quality gate.
- [x] Run full verification.

## Surprises & Discoveries

- `read_only_claim_trace` did not initially mention tool qualification. PR3 now
  treats `tools/tool_qualification_summary.json` as a read-only evidence
  boundary artifact, so the trace and manifest data-quality checks both expose
  missing qualification evidence.

## Decision Log

- 2026-06-09: Treat PR3 as qualification report generation only. No external tool execution, output comparison, version/hash qualification, privileged control qualification, or load qualification is added.
- 2026-06-09: Keep builtin load tools out of evidence until bounded load and safety monitor evidence exists in a later PR.

## Handoff

Branch: `codex/pr3-toolchain-qualification`.

Current status: implementation complete and verified locally.

Next steps:
1. Open review PR.
2. Keep PR4 scoped to local privileged control only after this evidence-source
   qualification boundary is merged.

Expected verification:
- `make contract`
- `cargo test --workspace`
- `make verify`

## Outcomes & Retrospective

- Added conservative toolchain qualification reports and summary artifacts.
- `familiarize read-only` now emits qualification reports before claim trace,
  manifest, and pack generation.
- Built-in read-only tools can become evidence; load, control, external,
  missing, and manifest-only agent-created tools remain rejected or unqualified
  until later PRs add the required evidence.
