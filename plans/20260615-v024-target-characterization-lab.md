# adc-lab v0.2.4 Target Characterization Laboratory

## Purpose / Big Picture

Execution GOAL:

Move adc-lab from Agent-safe workflow authority and governor validation into an
Agent-safe target characterization laboratory. v0.2.4 must help an Agent collect
clean target-local workload demand, resource-dimension evidence, and resolvable
evidence refs for a Target / Platform Operating Contract.

The release must preserve the core product boundary:

```text
Target Operating Contract, not benchmark score.
More evidence, not looser claims.
Resolvable refs, not impressive summaries.
```

## Scope

In scope:

- Evidence-ref resolution for opened run sets that include target-local runs.
- Operating-contract production-readiness missing-reason cleanup.
- Target-local workload demand workflow for SSH-controller plans.
- Workflow profile split between smoke and deeper characterization.
- CPU / thermal characterization plan depth for bounded target runs.
- Pressure, composite, observer, and endpoint-backed network coverage planning.
- Suitability dimension linkage for storage, network, and latency where
  policy-backed evidence exists.
- Safer target-local execution ergonomics without adding a remote shell
  framework.
- `constraints self-check --out` or an equivalent persisted
  `report.constraints_check` artifact path.
- `reports/workflow-contract-review/<slug>.md` per implementation PR.

Out of scope:

- Agent root shell.
- Arbitrary remote shell framework.
- Arbitrary helper path or arbitrary sysfs writes.
- Remote privileged apply/restore over SSH.
- Unbounded stress, 24h soak, production readiness, Pi4/Pi5 selection, or real
  application performance claims.
- Converting `unknown`, `degraded`, `insufficient`, `counter-only`, smoke, or
  ingredients-only evidence into stronger claims.

## Problem Frame

- Problem owner: adc-lab workflow authority, evidence store, run reports,
  suitability, and generated handoff products.
- Current pain / evidence: v0.2.3.1 can safely guide target55 through workflow
  authority and measured governor validation, but still leaves clean workload
  demand, storage/network/latency suitability, and included-run evidence refs
  incomplete or hard to audit.
- Desired outcome: target characterization plans collect stronger evidence
  while claim-producing reports keep unknowns blocked and every non-unknown
  claim has resolvable evidence.
- Solution-first risk: making the Agent more confident by prose, filename
  heuristics, or relaxed claims instead of stronger typed evidence.
- Proceed to implementation: yes, after PR #62 / v0.2.3.1 is merged into
  `main`.

## Assumptions / Base

- Base branch: `origin/main` after PR #62 and agent-instructions-playbook PR
  #57 are merged.
- Current implementation branch: `codex/v024-pr1-evidence-ref-resolution`.
- Phase 0 branch was created fresh from updated `origin/main`.
- Release identity: v0.2.4 is a product release after v0.2.3.1; Cargo package
  versions remain decoupled unless release tooling requires otherwise.
- Existing surfaces available from v0.2.3.1:
  - `workflow recommend`
  - `agent instructions`
  - `collect plan`
  - multi-run `report validate-run --include-run`
  - `report operating-contract --validation --strict-fullset`
  - `constraints check-candidate`
  - `constraints self-check`
  - generated schemas and schema ledger
  - docs heuristic guard
- `collect run` remains deferred unless explicitly re-scoped by a later
  decision.
- Required process: every implementation PR must include
  `reports/workflow-contract-review/<slug>.md` because workflow-contract
  review is part of the v0.2.4 development gate.

## Requirements

| ID | Priority | Requirement | Acceptance criteria | Verification method |
|---|---|---|---|---|
| R-001 | Must | When EvidenceStore opens a main run plus included runs, downstream reports shall emit evidence refs that resolve or are explicitly classified. | A-050a, A-050b, A-050c, A-051, and A-052 pass; regression fixture validates operating contract, suitability, and constraints refs. | Core/CLI fixture tests plus workflow-contract review. |
| R-002 | Must | When validation gate evidence is measured, operating-contract production readiness shall not list `matching_report.run_validation` as missing. | A-060 and A-061 pass; `production_ready` remains blocked for production-only evidence gaps. | Core rules tests and CLI operating-contract tests. |
| R-003 | Must | For SSH targets, collect plan shall include a target-local workload demand step that runs `adc-lab workload run --target local` through argv-array target-local execution. | A-001 through A-005 pass; degraded SSH-refused workload demand cannot make CPU/memory meet. | CLI collect-plan tests, workload/suitability tests. |
| R-004 | Must | Generated workflows shall distinguish smoke from deeper characterization. | A-010 through A-012 pass; smoke profile says seed/smoke and cannot be mistaken for characterization-full. | Core workflow tests, CLI tests, docs guard. |
| R-005 | Must | Characterization-full plans shall include bounded CPU/thermal ladder, repeatability, sustained bounded load, and cooldown expectations. | A-020 through A-023 pass; 300s evidence cannot support 24h sustained safety. | Workflow tests, operating-contract rule tests, docs review. |
| R-006 | Must | Characterization-full plans shall include first-class resource pressure and composite coverage while preserving conservative claim boundaries. | A-030 through A-033 pass. | Workflow tests, pressure/composite rule tests. |
| R-007 | Must | Suitability v2 shall evaluate storage/network/latency as non-unknown only when policy-backed matching evidence exists. | A-040 through A-044 pass. | Suitability unit tests and CLI loop tests. |
| R-008 | Must | Target-local execution instructions shall preserve argv semantics and make PATH / runner failures diagnosable. | A-070 through A-073 pass. | CLI instruction tests and safety-invariant diagnostics tests. |
| R-009 | Should | `constraints self-check` shall be able to persist a `report.constraints_check` artifact for handoff review. | A-080 through A-083 pass. | CLI constraints tests and collect-plan tests. |

## Constraints / Quality Targets

- No claim-producing output may rely on filename order, mtimes, or directory
  co-presence as causal linkage.
- All new public artifacts use the v2 `Artifact<P>` envelope and generated
  schema posture unless a Decision Log entry records a justified exception.
- Every schema-versioned artifact must update `schemas/schema-ledger.tsv` in
  the same PR that introduces or changes it.
- Every implementation PR must pass:

```sh
make verify
make schemas-check
make docs-smoke
```

- Every relevant implementation PR must add:

```text
reports/workflow-contract-review/<slug>.md
```

with decision `submit`.

- Any unsupported production, Pi4/Pi5 selection, 24h sustained safety, real
  application performance, or full coupling claim must remain blocked or
  explicitly unknown/insufficient.

## Context & Orientation

Key files to inspect before PR 1:

- `crates/adc-lab-core/src/evidence/store.rs` and
  `crates/adc-lab-core/src/evidence/envelope.rs`: run-set loading, artifact
  indexing, artifact refs.
- `crates/adc-lab-core/src/rules/operating_contract.rs`: operating-contract
  rules, validation gate handling, production readiness missing reasons.
- `crates/adc-lab-core/src/suitability.rs`: suitability decisions,
  constraints generation, constraints check artifact.
- `crates/adc-lab-core/src/workflow.rs`: workflow registry, collect-plan steps,
  generated instructions.
- `crates/adc-lab/src/commands/*`: CLI persistence, audit, output paths.
- `crates/adc-lab/tests/cli.rs`: public command behavior.
- `crates/adc-lab-core/tests/run_validation.rs`,
  `crates/adc-lab-core/tests/rules_engine.rs`, and
  `crates/adc-lab-core/tests/probe_artifacts.rs`: existing characterization
  and report tests.
- `schemas/schema-ledger.tsv`: generated-schema ledger.
- `docs/reference/cli.md`, `README.md`, and `docs/getting-started/*`: public
  Agent/operator guidance.

Existing deferred item from v0.2.3:

- `constraints self-check --out` was explicitly deferred; v0.2.4 PR 8 closes
  that loop.

## Design

### Dev Workflow Route

- Risk route: high for the full release series. v0.2.4 changes workflow
  profiles, target-local handoff, evidence-ref semantics, report claims, and
  suitability decisions.
- Required branches across the series: ExecPlan, requirements-engineering,
  implementation-economy per PR, design-balance when module responsibilities
  change, focused regression tests, `make verify`, workflow-contract review,
  and quality gate.
- Embedded/safety posture: this is target characterization planning and
  evidence collection. Do not add new target-local always-on runtime, unbounded
  stress, or new privileged transport without a new plan and approval.

### Source-of-Truth Chain

The release preserves and extends the v0.2.3 source-of-truth chain:

```text
workflow.recommendation
  -> workflow.collect_plan
  -> report.run_validation
  -> report.operating_contract
  -> report.suitability
  -> report.constraints
  -> report.constraints_check
```

Characterization depth must be visible in the workflow layer before it becomes
visible in downstream claims.

### Profile Model

Introduce explicit workflow depth labels:

- `target-operating-contract-smoke`: setup and workflow correctness profile.
  It includes runner preflight, read-only identity, short seed probes, and
  governor-validation smoke. Its generated text must state that it is not deep
  target characterization.
- `target-characterization-full`: bounded deeper characterization profile. It
  includes read-only identity, repeated observation, CPU/thermal ladder,
  sustained bounded load, pressure/composite coverage, endpoint-backed network
  where configured, target-local workload demand, suitability, constraints, and
  persisted self-check.
- `suitability-focused`: optional later profile for a known workload and known
  evidence set. Defer unless the first two profiles need the distinction.

Compatibility decision moved to Phase 0:

- Whether the existing `target-operating-contract-fullset` remains as a
  compatibility alias for smoke, is deprecated with warning text, or is mapped
  to one of the new profile ids.

Phase 0 compatibility decision must happen before PR 2:

```text
Option A:
  target-operating-contract-fullset remains as compatibility alias to smoke.

Option B:
  target-operating-contract-fullset maps to target-characterization-full.

Option C:
  target-operating-contract-fullset remains accepted but emits deprecation
  warning and requires explicit --profile-depth smoke|characterization-full.
```

Recommended decision: Option C. Rationale: v0.2.3.1 fullset is closer to a
workflow/governor-validation smoke profile than v0.2.4 deep characterization.
Keeping the name while changing depth would make old and new runs difficult to
compare and would let Agents over-read the older profile.

### Included-Run Evidence Refs

Run-set evidence must distinguish:

- physical path used to open an included run under the controller run directory
- artifact URI root originally written by that included run
- logical refs consumed by reports

Evidence-ref categories:

- Resolvable evidence ref: refs such as `artifact://...` that the
  EvidenceStore / run-set resolver is expected to open inside the current
  opened run set.
- Diagnostic / external ref: human-readable paths, stdout/stderr logs,
  operator-provided files, or other handoff refs that are not opened by the
  artifact resolver. These must be explicitly marked external/diagnostic with a
  reason and a handoff-manifest entry.
- Invalid ref: a ref that is neither resolvable through the opened run-set
  resolver nor declared as an external/diagnostic handoff ref.

Run-set resolution map requirement:

```text
run_set_resolution_map:
  - logical_run_id
  - opened_path
  - artifact_uri_root
  - source_role: primary | included
  - included_as
```

The resolution report written for handoff must let a reviewer re-run the
resolver decision without trusting prose in the downstream report.

PR 1 should choose one explicit strategy:

1. Preserve each included run's own artifact URI root and make the opened
   run-set resolver understand multiple roots.
2. Rewrite included-run refs into a canonical opened-run-set namespace and
   record the rewrite map.

Default preference: preserve original artifact roots and add a resolver that
knows every opened run root. Rewriting refs risks obscuring provenance unless
the rewrite map is itself audited.

### Target-Local Workload Demand

For SSH controller workflows, clean workload demand is produced target-local:

```text
execution_location = target_local
command_argv = ["adc-lab", "workload", "run", "--target", "local", ...]
```

The collect plan must also include a controller-side retrieval/include step or
explicit deterministic include path so suitability consumes:

```text
<main_run>/included/target-local-workload-demand/reports/workload_demand_profile.json
```

The exact include path can differ, but it must be stable, typed in the plan,
and used by `decide suitability`. If target-local workload demand is missing or
refused, suitability keeps CPU/memory unknown with next evidence.

Clean target-local workload demand requires:

- workload run status `completed`
- `data_quality.degraded == false`
- `process_cpu_percent_avg` present
- `rss_peak_kb` present
- `system_memory_available_min_kb` present or matching target-run memory
  evidence present
- evidence refs include workload plan and workload result
- run id and target id match the expected target-local run

Degraded target-local workload demand includes:

- `remote_workload_execution_not_supported_in_v1`
- refused command
- bounded execution aborted
- missing process CPU or RSS metrics
- version skew
- target id or run id mismatch

Additional PR 2 acceptance:

- A-006: CPU suitability must not become `meet` unless clean workload demand
  has `process_cpu_percent_avg`.
- A-007: Memory suitability must not become `meet` unless clean workload demand
  has RSS and memory evidence.
- A-008: A degraded target-local workload demand profile is preserved as
  evidence but cannot support `selection_ready`.

Minimal target-local executor contract required in PR 2:

- `execution_location = target_local`
- generated PATH guidance is present
- `command_argv` is rendered safely for SSH execution
- no arbitrary command construction is introduced
- command-not-found diagnostics distinguish PATH missing from install missing

PR 7 may refine executor ergonomics, but PR 2 must contain the minimum contract
needed to make target-local workload demand executable and reviewable.

### Characterization Coverage

`target-characterization-full` should begin with the minimum target55-style
coverage from the GOAL:

- passive observe 60s
- passive observe 300s
- CPU load ladder: 1/2/4 workers for 60s each
- 4-worker repeatability: 3 x 60s with cooldown
- sustained bounded load: 4 workers for 300s
- optional approved 900s profile, disabled by default
- cooldown observation
- default thermal abort: 75C
- pressure kinds: latency_jitter, observer_pressure, memory_pressure,
  storage_io, cpu_pressure, thermal_pressure, network_io counter-only,
  endpoint-backed network when endpoint is configured
- composite: memory_storage_jitter

Coverage must classify evidence as smoke, pressure-induced, boundary probe,
measured partial, insufficient, refused, contaminated, unknown, or
not_applicable as appropriate. The plan should avoid inventing a fourth status
vocabulary if the existing v2 status vocabulary can be extended safely.

Endpoint-backed network semantics:

- Endpoint-backed network is optional unless endpoint configuration is provided.
- If no endpoint is configured:
  - endpoint-backed transfer is `not_applicable` or `evidence_needed`
  - counter-only evidence remains counter-only
  - network suitability remains unknown if required by policy
- If endpoint is configured, evidence must record endpoint address, port,
  direction, bytes, duration, status, sink receipt, and evidence refs.

### Suitability Dimension Linkage

Suitability should consume:

- workload demand profile
- target operating contract
- pressure evidence summary
- endpoint-backed network evidence
- latency jitter evidence
- storage_io evidence

Non-unknown storage/network/latency decisions require concrete evidence refs.
Counter-only network evidence and tempfile/page-cache storage smoke are
explicitly insufficient for boundary meet decisions.

Evidence sufficiency matrix:

| Dimension | Evidence | May support | Must not support |
|---|---|---|---|
| Network | Endpoint-backed bounded transfer with bytes, endpoint, direction, duration, status, sink receipt, and evidence refs | Network decision within the measured endpoint boundary | Broad network safety outside the measured endpoint/direction |
| Network | Counter-only network evidence | Observation context and next-evidence guidance | Network `meet` |
| Storage | Device-visible bounded I/O with path/device/cache semantics known | Storage decision within the measured device/cache boundary | Broad storage device boundary when cache/device semantics are unknown |
| Storage | Tempfile/page-cache smoke | Smoke evidence and next-evidence guidance | Storage device boundary `meet` |
| Latency | `latency_jitter` probe under a defined condition with policy threshold and sample stats | Latency decision within that condition | Generic latency `meet` outside the condition |
| Latency | Generic observation | Context and next-evidence guidance | Latency `meet` |
| Composite | Ingredients-only pressure artifacts | Context and next-evidence guidance | Coupling measured |
| Composite | Phased/composite measured scenario | Coupling claim within scenario boundary | Coupling claims outside the measured scenario |

Potential summary artifact:

- If PR 6 grows beyond focused suitability decision logic, introduce
  `report.resource_evidence_summary.v2` before wiring suitability directly to
  raw pressure/network/storage/latency scans.
- Summary responsibilities: pressure classification, endpoint-backed network
  evidence, storage evidence classification, latency/jitter summary, observer
  pressure summary, composite coverage, evidence refs, and insufficiency
  reasons.

### Target-Local Executor Ergonomics

v0.2.4 should improve execution instructions without adding a remote shell
framework. Acceptable approaches:

- render an adc-lab-generated SSH invocation snippet for target-local argv
  steps
- add collect-plan fields that describe the safe SSH invocation template
- add a helper command that renders target-local steps safely without executing
  arbitrary commands

Any approach must preserve argv semantics, prepend `~/.local/bin` to PATH when
required, and distinguish command-not-found, PATH-missing, permission denied,
helper unavailable, and version skew diagnostics.

### Constraints Self-Check Persistence

Add `--out` to `constraints self-check` unless a stronger collect-run
convention exists by PR 8. The direct CLI option is the preferred v0.2.4 scope
because it is simple and matches current report artifact patterns.

## Validation & Acceptance

Default gates for every implementation PR:

```sh
make verify
make schemas-check
make docs-smoke
```

Focused gates by area:

```sh
cargo test -p adc-lab-core workflow -- --nocapture
cargo test -p adc-lab-core run_validation -- --nocapture
cargo test -p adc-lab-core suitability -- --nocapture
cargo test -p adc-lab-core operating_contract -- --nocapture
cargo test -p adc-lab --test cli collect_plan -- --nocapture
cargo test -p adc-lab --test cli report_validate_run -- --nocapture
cargo test -p adc-lab --test cli report_operating_contract -- --nocapture
cargo test -p adc-lab --test cli constraints_ -- --nocapture
cargo test -p adc-lab --test safety_invariants -- --nocapture
```

Workflow-contract review report template:

```text
reports/workflow-contract-review/<slug>.md
decision: submit | no-submit
profile affected:
artifact chain checked:
argv-array/no-shell check:
claim-boundary check:
evidence-ref resolution check:
docs/prompts stale-pattern check:
verification commands:
known blocked claims:
```

Detailed acceptance criteria:

- A-001: For `target=ssh://target55`, collect plan emits a target-local
  workload demand step.
- A-002: The target-local workload step uses generated argv and the established
  `target_local` execution convention, not arbitrary shell.
- A-003: Workload demand produced target-local is retrieved into a deterministic
  include path.
- A-004: `decide suitability` consumes retrieved workload demand, not a
  degraded SSH-refused profile.
- A-005: If target-local workload demand cannot run, CPU and memory remain
  unknown with next evidence, not meet.
- A-006: CPU suitability does not become `meet` unless clean workload demand has
  `process_cpu_percent_avg`.
- A-007: Memory suitability does not become `meet` unless clean workload demand
  has RSS and memory evidence.
- A-008: Degraded target-local workload demand is preserved but cannot support
  `selection_ready`.
- A-010: Generated recommendation and collect plan describe whether the plan is
  smoke, characterization-full, or suitability-focused.
- A-011: Smoke profile cannot be mistaken for deep target characterization.
- A-012: Full characterization profile includes explicit duration, coverage,
  and safety caps.
- A-020: Characterization-full collect plan emits CPU ladder and sustained
  bounded load steps.
- A-021: Each CPU/thermal step records duration, workers, abort threshold, and
  cooldown expectation.
- A-022: Output distinguishes short seed evidence from sustained bounded
  evidence.
- A-023: 300s evidence cannot support 24h sustained safety.
- A-030: Characterization-full collect plan includes pressure coverage map.
- A-031: Pressure results distinguish smoke, pressure-induced, boundary probe,
  measured_partial, insufficient, and not_applicable states.
- A-032: Network endpoint-backed transfer is separated from counter-only
  evidence.
- A-033: Composite coupling claims remain blocked unless phased/composite
  evidence is measured.
- A-040: Storage/network/latency dimensions are not automatically unknown when
  sufficient target evidence exists.
- A-041: Each non-unknown storage/network/latency decision cites concrete
  evidence refs.
- A-042: Counter-only network evidence cannot become network meet.
- A-043: Tempfile/page-cache-only storage smoke cannot become storage device
  boundary meet.
- A-044: Unknown dimensions still block `selection_ready` when policy marks
  them required.
- A-050a: Every `artifact://` evidence ref in operating-contract, suitability,
  and constraints resolves through the opened run-set resolver.
- A-050b: Non-artifact refs either resolve through a declared resolver type or
  are explicitly marked external/diagnostic with reason.
- A-050c: The handoff archive contains a run-set evidence-ref resolution report.
- A-051: Included-run artifact refs are not prefixed with an include mount path
  if that breaks resolution.
- A-052: Regression fixture includes main run, included target-local governor
  run, operating contract, suitability, constraints, and resolver validation.
- A-060: Measured validation gate removes `matching_report.run_validation` from
  `production_ready` missing reasons.
- A-061: `target.selection.production_ready` remains blocked.
- A-062: Strict-fullset success remains distinct from production readiness.
- A-070: Collect plan provides unambiguous execution instructions for
  target-local steps.
- A-071: Target-local execution preserves argv semantics.
- A-072: Target-local execution prepends `~/.local/bin` to PATH when required.
- A-073: Failure diagnostics distinguish command-not-found, PATH-missing,
  permission denied, helper unavailable, and version skew.
- A-080: `constraints self-check` can persist a `report.constraints_check`
  artifact.
- A-081: Collect plan expects and records the persisted self-check artifact
  path.
- A-082: Generated blocked claims section remains allowed in self-check mode.
- A-083: Candidate content check still fails on unsupported positive claims.

## Test List

- Profile smoke emits only bounded seed steps and explicitly says it is smoke.
- Characterization-full emits CPU ladder, sustained load, pressure, composite,
  endpoint-backed network where configured, observer, and target-local workload
  demand.
- No profile teaches filename-order artifact selection.
- SSH collect plan emits target-local workload demand step.
- Retrieved target-local workload demand is used by suitability.
- Degraded remote workload profile does not produce CPU/memory meet.
- Main run plus included run produces resolvable evidence refs.
- Operating contract, suitability, and constraints refs all resolve.
- Storage/network/latency remain unknown without matching evidence.
- Endpoint-backed network evidence can support a network decision.
- Counter-only network evidence cannot support network meet.
- Page-cache-only storage smoke cannot support storage device boundary meet.
- Validation gate measured removes `matching_report.run_validation` missing
  reason.
- `target.selection.production_ready` remains blocked.
- Strict full-set success does not imply production readiness.
- Generated target-local execution instructions include PATH guidance.
- Command-not-found diagnostic suggests PATH / `ADC_LAB_TARGET_RUNNER` where
  applicable.
- Version skew remains visible and blocks full-set measured claims.
- `constraints self-check --out` persists a `report.constraints_check`
  artifact.

## Progress (WBS)

- [x] Read v0.2.4 GOAL / Design Guidance.
- [x] Create v0.2.4 ExecPlan with requirements, design, validation, WBS, and
      handoff.
- [x] Phase 0: Rebase/start from `origin/main` after PR #62 merge and record
      baseline affected files/tests; decide profile compatibility and evidence
      ref categories.
- [ ] PR 1: Evidence-ref resolution and operating-contract missing cleanup.
  - [x] Create fresh branch from `origin/main` after PR #63 merge.
  - [x] Inspect EvidenceStore, run validation, operating-contract gate, and
        suitability/constraints evidence-ref flow.
  - [x] Add run-set evidence-ref resolver and resolution report.
  - [x] Add focused resolver / production-ready regression tests.
  - [x] Add PR1 workflow-contract review report.
  - [x] Run focused tests and `make verify`.
- [ ] PR 2: Target-local workload demand workflow.
- [ ] PR 3: Profile split.
- [ ] PR 4: Deep CPU / thermal characterization profile.
- [ ] PR 5: Pressure / composite / endpoint-backed network coverage.
- [ ] PR 6: Suitability dimension linkage.
- [ ] PR 7: Target-local executor ergonomics.
- [ ] PR 8: Constraints self-check persistence and docs.
- [ ] Final: Update Outcomes with artifact review criteria and v0.2.4
      target55 rerun readiness.

## PR Phases

### Phase 0: Baseline and Planning Gate

Goal:

- Start from `origin/main` after PR #62 and agent-instructions-playbook PR #57
  merge.
- Confirm current file/test ownership and identify fixture locations.
- Decide profile compatibility for `target-operating-contract-fullset`.
- Confirm evidence-ref categories and run-set resolution-map shape.
- Add v0.2.4 workflow-contract-review report template usage.

Deliverables:

- Updated Handoff in this plan.
- Baseline `make verify`, `make schemas-check`, and `make docs-smoke` result.
- Initial `reports/workflow-contract-review/v024-phase0.md` or equivalent
  template fixture if the review report convention itself needs a seed.
- Decision Log entry choosing profile compatibility option A/B/C. Recommended:
  C, keep `target-operating-contract-fullset` accepted but require explicit
  depth or warning text.

### PR 1: Evidence-Ref Resolution and Operating-Contract Missing Cleanup

Scope:

- Included-run evidence-ref normalization/resolution.
- Run-set resolution map and handoff resolution report.
- Resolver test for main run plus included target-local run.
- Operating-contract production readiness missing-reason cleanup.

Acceptance:

- A-050a through A-050c and A-051 through A-052.
- A-060 through A-062.

Why first:

- Auditability and report correctness should land before adding deeper
  evidence volume.

Split guidance:

- If production-readiness missing cleanup is small and evidence-ref resolution
  touches EvidenceStore internals broadly, split this phase into PR 1a
  (missing reason cleanup) and PR 1b (run-set resolver and resolution report).
- If both remain small, keep them together and record that decision in the PR
  workflow-contract review.

PR 1 dev-workflow route:

- Risk route: normal for this PR. It changes report/evidence behavior and a
  generated schema, but keeps target-local execution and privileged/control
  behavior unchanged.
- Definition of Done: A-050a/A-050b/A-050c/A-051/A-052 and A-060/A-061/A-062
  have focused tests; resolver errors do not relax claim gates; `make verify`
  passes.
- Test List: resolver resolves main/included `artifact://` refs; diagnostic
  non-artifact refs are classified; invalid artifact refs fail the resolution
  report; measured validation removes `matching_report.run_validation` while
  keeping `target.selection.production_ready` blocked; CLI writes the handoff
  resolution report.
- Complexity Budget: changed production files <= 5; new modules = 0; new
  helpers/structs <= 6 inside existing evidence/report modules; schema/golden
  files only as generated contract evidence; production diff target <= 250
  lines.
- Function-boundary plan: keep resolver ownership in `EvidenceStore`; keep CLI
  persistence in `commands/report.rs`; add no generic artifact-discovery
  framework.

PR 1 implementation-economy audit:

| New abstraction | Justification | Decision | Evidence |
|---|---|---|---|
| `EvidenceStore::resolve_evidence_ref` | Centralizes artifact URI root/path validation so downstream reports do not each invent resolver logic. | keep | Core resolver test covers primary/included runs, diagnostic refs, and invalid refs. |
| `EvidenceRefResolutionPayload` | Makes handoff review machine-readable and schema-checked instead of prose-only. | keep | Generated schema and golden fixture added. |
| `evidence_ref_resolution_artifact` in `commands/report.rs` | Keeps CLI write/audit side effects out of the read-only core resolver. | keep | CLI operating-contract test verifies artifact persistence and audit event. |

Budget note:
production diff exceeded the initial 250-line target because the typed payload,
resolver classifications, and CLI persistence are intentionally explicit. File
budget enforcement remains green, no new module was added, and the extra code
prevents a second ad hoc resolver in suitability/constraints.

Function-boundary summary:
recorded in local ignored `.agents/design-ledger/function-boundaries.md`; this
ExecPlan carries the PR-visible summary because the repository intentionally
ignores `.agents/`. Changed functions:
`EvidenceStore::run_set_resolution_map`, `EvidenceStore::resolve_evidence_ref`,
`EvidenceStore::evidence_ref_resolution_payload`,
`evidence_ref_resolution_artifact`, and `operating_contract_evidence_refs`.
Semantic neighbors considered: `artifact_uri_for_run`,
`run_set_identity_for_runs`, `artifact_ref_for_optional_path`, and
`operating_contract_validation_gate`. Decision: keep resolver in core store,
keep report persistence in CLI, no destructive refactor.

### PR 2: Target-Local Workload Demand Workflow

Scope:

- Minimal target-local executor contract needed to run the workload step.
- SSH collect plans emit target-local workload demand step.
- Deterministic include/retrieval path for the workload demand run.
- Suitability consumes clean target-local workload demand when present.
- Missing/refused target-local workload demand keeps CPU/memory unknown.
- Clean/degraded workload demand semantics.

Acceptance:

- A-001 through A-008.

### PR 3: Profile Split

Scope:

- Add explicit smoke and characterization-full profile ids.
- Generated recommendation and collect-plan text describe depth.
- Implement the Phase 0 compatibility decision for existing
  `target-operating-contract-fullset`.

Acceptance:

- A-010 through A-012.

### PR 4: Deep CPU / Thermal Characterization Profile

Scope:

- CPU ladder, repeatability, sustained bounded load, cooldown, and safety caps.
- Output language distinguishes short seed from sustained bounded evidence.
- Rule/test guard that 300s bounded evidence does not support 24h safety.

Acceptance:

- A-020 through A-023.

### PR 5: Pressure / Composite / Endpoint-Backed Network Coverage

Scope:

- Pressure coverage map.
- Endpoint-backed network step separated from counter-only network evidence.
- Observer pressure and composite memory_storage_jitter coverage.

Acceptance:

- A-030 through A-033.

### PR 6: Suitability Dimension Linkage

Scope:

- Resource evidence summary if raw scanning would make suitability too broad.
- Storage/network/latency decisions where matching evidence exists.
- Conservative unknown/insufficient behavior where matching evidence is absent
  or too weak.
- Evidence refs on every non-unknown dimension.
- Evidence sufficiency matrix enforced by tests.

Acceptance:

- A-040 through A-044.

### PR 7: Target-Local Executor Ergonomics

Scope:

- Refine safe target-local execution instructions or renderer after PR 2
  establishes the minimum contract.
- Additional PATH guidance and diagnostics.
- No arbitrary remote command framework.

Acceptance:

- A-070 through A-073.

### PR 8: Constraints Self-Check Persistence and Docs

Scope:

- `constraints self-check --out`.
- Collect-plan expected artifact path for persisted self-check.
- Docs, examples, generated prompt fixtures, and docs-smoke updates.
- Final v0.2.4 release readiness checklist.

Acceptance:

- A-080 through A-083.

## Design -> WBS Coverage Check

| Deliverable | WBS coverage |
|---|---|
| Evidence-ref resolution | PR 1 |
| Production-ready missing reason cleanup | PR 1 |
| Target-local workload demand | PR 2 |
| Minimal target-local executor contract | PR 2 |
| Profile split | PR 3 |
| CPU/thermal characterization | PR 4 |
| Pressure/composite/network coverage | PR 5 |
| Suitability dimension linkage | PR 6 |
| Target-local executor ergonomics refinement | PR 7 |
| Persisted constraints self-check | PR 8 |
| Docs/examples/prompt updates | PR 8 plus each profile PR as needed |
| Workflow-contract review reports | Every implementation PR |

## Release Gate

Before tagging v0.2.4:

- `make verify`
- `make schemas-check`
- `make docs-smoke`
- Final workflow-contract review report with decision `submit`
- Release binary, `SHA256SUMS`, and `release-manifest.json` verification
- target55 smoke profile run
- target55 characterization-full dry or bounded review, if duration permits
- Artifact Review Criteria below recorded against the target55 run or marked
  blocked/deferred with reason

## Artifact Review Criteria

A v0.2.4 target55 artifact is successful only if it can answer:

1. Which profile was run: smoke, characterization-full, or suitability-focused?
2. Which dimensions are measured, insufficient, unknown, refused,
   contaminated, or not_applicable?
3. Which evidence refs support each non-unknown suitability dimension?
4. Are all evidence refs resolvable inside the handoff archive?
5. Was workload demand clean or degraded?
6. Did target-local governor sweep remain measured?
7. Did production readiness remain blocked?
8. Did suitability `selection_ready` remain false when required dimensions are
   unknown or fail?
9. Are pressure, composite, and endpoint-backed network results correctly
   classified?
10. Are `next_evidence_needed` and blocked claims clear enough for the next
    run?

## Surprises & Discoveries

- 2026-06-15: v0.2.4 now treats PR #62 and agent-instructions-playbook PR #57
  as merged base work. Phase 0 starts from updated `origin/main`, not from the
  old PR #62 work branch.
- 2026-06-15: `git fetch --all --prune` advanced `origin/main` to
  `1ebb2b8f287420fa4a527cea3fddcea72ef159d6`, which merges PR #62. Tag
  `v0.2.3.1` also points at that merge commit.
- 2026-06-15: `constraints self-check --out` is already recorded as a deferred
  v0.2.3 follow-up in `plans/20260613-v023-workflow-authority.md`; v0.2.4 PR 8
  should close that explicit deferral.
- 2026-06-15: The GOAL requests `reports/workflow-contract-review/<slug>.md`
  for each relevant PR. This is a new review artifact family and should be
  introduced with a minimal template in the first implementation PR that uses
  it.
- 2026-06-15: PR #63 merged into `origin/main` as
  `74cbd9921fa5b988be5e64fe14ffaac9c60c5627`; PR 1 branch
  `codex/v024-pr1-evidence-ref-resolution` was created from that updated
  `origin/main`.
- 2026-06-15: EvidenceStore currently owns only v2 artifact indexing, but it
  already owns opened run roots and symlink rejection. PR 1 therefore extends
  EvidenceStore with read-only evidence-ref resolution instead of creating a
  separate report resolver module.

## Decision Log

- 2026-06-15: Plan only in this step; do not implement on the historical PR #62
  branch. Rationale: v0.2.4 should start from updated `origin/main` after PR
  #62 and agent-instructions-playbook PR #57 are merged.
- 2026-06-15: Keep `collect run` deferred for v0.2.4 planning. Rationale:
  target-local evidence depth can improve through `collect plan` and explicit
  handoff conventions without adding an executor.
- 2026-06-15: Prefer preserving included runs' own artifact URI roots with a
  multi-root resolver over rewriting refs by default. Rationale: preserving
  provenance is safer than rewriting unless an audited rewrite map is required.
- 2026-06-15: Treat 900s sustained load as optional/approved, not default.
  Rationale: v0.2.4 should deepen characterization without silently increasing
  experiment risk.
- 2026-06-15: Choose profile compatibility Option C for Phase 0. Keep
  `target-operating-contract-fullset` accepted but make the profile depth
  explicit through warning text or an explicit `--profile-depth
  smoke|characterization-full` path before adding target-local workload demand.
  Rationale: v0.2.3.1 fullset behavior is closer to workflow/governor smoke
  than v0.2.4 deep characterization, and silently changing the meaning would
  make old/new runs hard to compare.
- 2026-06-15: Classify evidence refs as resolvable, diagnostic/external, or
  invalid and require a run-set resolution map for included-run work. Rationale:
  PR 1 must be reviewable from a handoff archive, not only from implementation
  claims.
- 2026-06-15: Keep PR 1 as a combined evidence-ref resolution plus
  production-readiness cleanup PR unless implementation exceeds the complexity
  budget. Rationale: existing production-ready gate already has most A-060
  behavior, so the remaining work is small enough to review with the resolver.
- 2026-06-15: Accept a small PR 1 complexity-budget overrun rather than split
  the resolver into another module. Rationale: the added lines are schema-backed
  payload/enum definitions and explicit resolver classifications; file budgets
  remain green and a new module would add more indirection.
- 2026-06-15: Do not force-add `.agents/design-ledger/function-boundaries.md`
  despite function-boundary-governor using that local ledger. Rationale:
  repository instructions and `.gitignore` keep `.agents/` out of source
  control; the PR-visible function-boundary summary is recorded in this
  ExecPlan instead.

## Handoff

Base branch:
`origin/main` after PR #62 and agent-instructions-playbook PR #57 are merged.

Current implementation branch:
`codex/v024-pr1-evidence-ref-resolution`.

Status:
PR #63 is merged. PR 1 branch is open locally from updated `origin/main`.
Implementation is complete locally; resolver/report code, focused tests,
schema generation, workflow-contract review report, and full verification are
green. Draft PR #64 is open and mergeable. This handoff may receive plan-only
status commits, so check the PR UI for latest-head CI before Ready for review.

Next steps:
1. Address review feedback.
2. Mark PR #64 Ready for review after approval.
3. Merge after CI remains green.

Required process:
every implementation PR must include
`reports/workflow-contract-review/<slug>.md` because workflow-contract review
is now part of the development gate.

Suggested next steps:

1. Address review comments on PR #63.
2. Mark PR #63 ready for review when approved.
3. After merge, start PR 1 on evidence-ref resolution and production-readiness
   missing-reason cleanup.

## Outcomes & Retrospective

Phase 0 complete:

- Fresh branch `codex/v024-phase0-characterization` created from
  `origin/main` at `1ebb2b8f287420fa4a527cea3fddcea72ef159d6`.
- PR #62 merge and `v0.2.3.1` tag observed locally.
- Profile compatibility decision recorded as Option C.
- Evidence-ref categories and run-set resolution-map shape recorded.
- Workflow-contract review seed report added at
  `reports/workflow-contract-review/v024-phase0.md` with decision `submit`.
- Draft PR opened: #63.

Plan-drafting verification:

- `git diff --check`: pass.
- `make docs-smoke`: pass.

Phase 0 baseline verification:

- `make docs-smoke`: pass.
- `make schemas-check`: pass (`schema ledger: ok top_level=0 no_schema_wire=31
  maintained_by_hand=0`).
- `make verify`: pass.

PR 1 verification:

- `cargo test -p adc-lab-core evidence_store_resolves_artifact_refs_across_opened_run_set -- --nocapture`: pass.
- `cargo test -p adc-lab-core operating_contract_validation_gate_removes_matching_validation_missing_reason -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli report_operating_contract_accepts_include_run_in_v2_store -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli suitability_loop_consumes_tool_produced_v2_artifacts_end_to_end -- --nocapture`: pass.
- `cargo test -p adc-lab-core --test probe_artifacts -- --nocapture`: pass.
- `cargo test -p adc-lab-core --test rules_engine operating_contract -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli report_operating_contract -- --nocapture`: pass.
- `make schemas-check`: pass (`schema ledger: ok top_level=0 no_schema_wire=31 maintained_by_hand=0`).
- `make contract`: pass.
- `make docs-smoke`: pass.
- `make verify`: pass.

Quality gate:

- Decision: submit.
- Findings: 0.
- Required artifacts present: ExecPlan updated, PR1 workflow-contract review
  report decision `submit`, implementation-economy audit recorded, and
  function-boundary summary recorded in this plan because `.agents/` is ignored.
- Draft PR opened: #64
  (`https://github.com/shunta-sato/agent-debug-compass-laboratory/pull/64`) at
  head `46d3757e05f4af4bd88fc024c4a7f89bba5f6fd1`.
- PR #64 status-only handoff updates are plan-only; latest-head CI should be
  checked in the PR UI before Ready for review.
