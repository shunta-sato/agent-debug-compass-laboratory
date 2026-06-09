# ExecPlan: Agent-created Adapter Qualification Workflow

## Purpose / Big Picture

PR9 makes the agent-created adapter path concrete without turning `adc-lab`
into an arbitrary tool runner. An Agent may propose and implement an adapter,
but the adapter becomes an evidence source only when bounded manifest data,
dry-run output evidence, manual comparison evidence, version/hash data, output
schema declaration, and safety review evidence are all recorded.

The core rule remains:

```text
No unqualified tool becomes evidence.
```

## Scope

In scope:

- Extend `adc-lab tool qualify --manifest ...` so it can record qualification
  evidence for agent-created observation/probe/report-normalizer adapters.
- Add structured qualification evidence fields:
  - `tool_version`
  - `tool_sha256`
  - `output_schema_ref`
  - `dry_run_ref`
  - `manual_comparison_ref`
  - `static_safety_review_ref`
  - `qualification_scope`
  - `validated_output_bytes`
- Accept evidence only for non-privileged, non-state-writing observation/probe
  style adapters with bounded duration/output.
- Keep control, privileged, state-writing, and load adapters unqualified in
  PR9.
- Add strict schema/fixture coverage and negative tests.
- Emit diagnosable audit results for qualified vs unqualified tool reports.
- Update tool qualification docs, observability, error handling, and NFR/gate
  evidence.

Out of scope:

- Executing agent-created tools.
- Running arbitrary scripts or shell commands.
- Validating full JSON Schema output semantics.
- Qualifying control, restore, privileged, or load tools.
- Installing tools on targets.
- Using qualified adapters in experiment matrix execution.

## Constraints / Quality Targets

- Default `make verify` remains hardware-free.
- PR9 must not add target-local runtime, target probing, or load generation.
- Qualification evidence refs must be bounded logical
  `artifact://lab/runs/...` refs.
- Local paths supplied to CLI are inputs only and must not be serialized into
  Agent-facing fields.
- Missing required evidence keeps the tool `agent_created_unqualified`.
- Malformed or oversized dry-run/comparison evidence is rejected before a
  qualified report is written.
- `qualified` must remain narrower than `builtin`: it is accepted only for the
  declared output scope and limitations.

## Dev Workflow Route

- Selected risk route: high.
- Why: PR9 changes a public evidence contract, CLI inputs, audit outcomes, and
  the conditions under which an agent-created adapter can become evidence.
- Required branches:
  - `execution-plans`: cross-boundary schema/core/CLI/docs/tests work.
  - `function-boundary-governor`: qualification API and helper boundaries
    change.
  - `error-handling`: new file/artifact validation and refusal outcomes.
  - `observability`: `tool.qualify` audit result now distinguishes qualified
    and unqualified outcomes.
  - `embedded-nfr-design`: adapter qualification influences embedded evidence
    claims; keep physical-footprint claims bounded/experimental.
  - `embedded-nfr-gate`: feature-level embedded evidence gate before submit.
- Non-triggered branches:
  - `architecture-decision-analysis`: no architecture option is chosen.
  - `concurrency-core`: no concurrency changes.
  - `embedded-hot-path-review`: no target-local loop is added.
  - `embedded-observer-effect-review`: no target-local observer is added.
  - `embedded-nfr-harness-design`: no measurement harness or target smoke is
    added.

## Requirements / Acceptance

- EARS-AC1: When manifest-only agent-created tools are qualified, the report
  shall remain `agent_created_unqualified` with `evidence_accepted=false`.
- EARS-AC2: When a non-privileged observation/probe manifest includes complete
  evidence refs, version/hash, output schema ref, safety review ref, and bounded
  output bytes, the report shall be `qualified` with `evidence_accepted=true`.
- EARS-AC3: When the manifest writes target state, requires privilege, is a
  control/restore/load adapter, or exceeds output/duration policy, the report
  shall remain unqualified even if evidence refs are supplied.
- EARS-AC4: When dry-run or manual comparison evidence files are missing,
  malformed JSON, or exceed declared output bounds, CLI qualification shall
  fail with a validation error and not claim evidence acceptance.
- EARS-AC5: Tool qualification artifacts shall use artifact refs and shall not
  serialize raw local evidence file paths.
- EARS-AC6: `tool.qualify` audit shall record `qualified` or
  `recorded_unqualified` so audit readers can distinguish outcomes.

## Context & Orientation

Key files:

- `crates/adc-lab-core/src/qualification.rs`: qualification policy and manifest
  DTOs.
- `crates/adc-lab-core/src/contracts.rs`: `ToolQualification` contract.
- `crates/adc-lab/src/main.rs`: CLI command and artifact persistence.
- `schemas/lab.tool_qualification.v1.schema.json`: strict contract schema.
- `tests/golden/lab.tool_qualification.v1.valid.json`: contract fixture.
- `examples/tools/linux_cpufreq_reader.yaml`: agent-created manifest example.
- `docs/architecture/tool-qualification.md`: qualification workflow docs.

Existing behavior:

- `tool qualify --manifest` reads a YAML manifest and always emits
  `agent_created_unqualified`.
- `tool qualify-inventory` qualifies discovered builtin toolchain inventory.
- Qualification reports already participate in read-only packs and audit.

## Design

Data model:

- Keep `ToolManifest` and `ToolQualification` as the main contract families.
- Add optional evidence fields to `ToolQualification` rather than creating a
  second report type. This keeps one report per tool id.
- Add a core `ToolQualificationEvidence` input struct that represents already
  persisted artifact refs and checked evidence size.

Policy:

```text
qualified iff:
  source == agent_created
  writes_target_state == false
  requires_privilege == false
  category in observation/probe/report_normalizer/health_check
  duration_seconds_max <= 30
  output_bytes_max <= 1 MiB
  dry-run evidence exists and is valid JSON
  manual comparison evidence exists and is valid JSON
  static safety review ref exists
  output schema ref exists
  tool version is non-empty
  sha256 is exactly sha256:<64 lowercase hex chars>
```

Control, load, restore, privileged, or state-writing tools remain
`agent_created_unqualified` in PR9. They require future control/load-specific
qualification workflows.

Error handling:

- CLI validates local evidence files before writing the qualification report.
- Missing evidence path, malformed JSON, oversized output, bad sha256, empty
  version, or unsafe output schema ref becomes `LabError::Validation`.
- Optional evidence absent does not error; it produces an unqualified report.

Observability:

- Audit operation remains `tool.qualify`.
- Audit result is `qualified` when `evidence_accepted=true`, otherwise
  `recorded_unqualified`.

## Test Strategy

- Contract fixture validation for the expanded `lab.tool_qualification.v1`.
- Core tests:
  - complete observation adapter evidence becomes `qualified`;
  - manifest-only remains `agent_created_unqualified`;
  - state-writing/control/load/privileged adapters remain unqualified;
  - invalid sha256 or missing refs keep report unqualified.
- CLI tests:
  - complete evidence writes a qualified report with artifact refs and no raw
    local paths;
  - malformed dry-run JSON fails before acceptance;
  - audit result records `qualified`.

## Milestones

1. Expand DTO/schema/golden fixture.
2. Implement qualification evidence input and policy.
3. Wire CLI options and artifact persistence.
4. Add contract/core/CLI tests.
5. Update docs, NFR/gate reports, and function-boundary evidence.
6. Run full verification and sensitive scan.
7. Commit, push, and open draft PR.

## Progress (WBS)

- [x] Sync merged PR8 main and create PR9 branch.
- [x] Explore existing qualification code, schemas, docs, and tests.
- [x] Route dev workflow and create ExecPlan.
- [x] Expand DTO/schema/golden fixture.
- [x] Implement qualification evidence policy and CLI wiring.
- [x] Add contract/core/CLI tests.
- [x] Update docs and NFR/gate evidence.
- [x] Run full verification and sensitive scan.
- [ ] Commit, push, open draft PR.

## Surprises & Discoveries

- `ToolManifest` already has the right minimal shape for PR9. The missing piece
  is artifact-backed evidence, not another manifest type.
- `tool qualify --manifest` currently writes one report per tool id and already
  emits `tool.qualify`, so PR9 can evolve the command without adding a new
  command surface.
- Precomputing artifact refs through `RunContext::artifact_uri` does not work
  before files exist because the helper rejects nonexistent path components.
  PR9 uses internally constructed `artifact://lab/runs/<run_id>/tools/...`
  refs for planned evidence artifacts and then writes those artifacts before the
  qualification report.

## Decision Log

- 2026-06-09: Keep PR9 artifact-backed and non-executing. The CLI validates
  evidence files supplied by the operator/agent; it does not run the adapter.
- 2026-06-09: Restrict `qualified` to non-state-writing, non-privileged
  observation/probe/report-normalizer style adapters.
- 2026-06-09: Keep control/load/restore adapter qualification for later PRs to
  avoid silently bypassing approval, restore, and safety-monitor requirements.

## Handoff

Branch: `codex/pr9-agent-adapter-qualification-workflow`.

Current status: implementation, full verification, and sensitive scan complete;
commit and draft PR creation next.

Run:

```sh
make verify
```

Read first:

- `crates/adc-lab-core/src/qualification.rs`
- `crates/adc-lab-core/src/contracts.rs`
- `crates/adc-lab/src/main.rs`
- `schemas/lab.tool_qualification.v1.schema.json`

## Outcomes & Retrospective

- `adc-lab tool qualify --manifest` now supports artifact-backed qualification
  evidence through `--tool-version`, `--tool-sha256`, `--output-schema`,
  `--dry-run-output`, `--manual-comparison`, and
  `--static-safety-review`.
- Agent-created manifest-only tools remain `agent_created_unqualified`.
- Complete evidence can make only non-state-writing, non-privileged
  observation/probe/report-normalizer/health-check adapters `qualified`.
- Control, restore, privileged, state-writing, and load adapters remain
  unqualified in PR9.
- Report fields use `artifact://lab/runs/...` refs and do not serialize raw
  local input paths.
- Verification passed:
  - `cargo test -p adc-lab-core qualification -- --nocapture`
  - `cargo test -p adc-lab --test cli tool_qualification -- --nocapture`
  - `make contract`
  - `make verify`
  - `git diff --check`
  - high-confidence secret/PII/security scan over PR diff, PR9 ExecPlan, and
    new example tool evidence files
