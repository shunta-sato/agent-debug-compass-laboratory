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
- `target-characterization-full`: bounded deeper characterization profile. PR 4
  makes the CPU/thermal slice executable: repeated observation, CPU ladder,
  repeatability, sustained bounded load, cooldown, and safety caps. PR 5/6/8
  still own pressure/composite coverage, endpoint-backed network, suitability
  dimension linkage, constraints, and persisted self-check depth.
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
- [x] PR 1: Evidence-ref resolution and operating-contract missing cleanup.
  - [x] Create fresh branch from `origin/main` after PR #63 merge.
  - [x] Inspect EvidenceStore, run validation, operating-contract gate, and
        suitability/constraints evidence-ref flow.
  - [x] Add run-set evidence-ref resolver and resolution report.
  - [x] Add focused resolver / production-ready regression tests.
  - [x] Add PR1 workflow-contract review report.
  - [x] Run focused tests and `make verify`.
- [x] PR 2: Target-local workload demand workflow.
  - [x] Create fresh branch from `origin/main` after PR #64 merge.
  - [x] Change SSH collect-plan workload demand to target-local argv with
        deterministic retrieval path.
  - [x] Ensure suitability consumes the retrieved workload demand path.
  - [x] Keep missing/refused/degraded CPU and memory demand unknown.
  - [x] Add focused workflow, CLI, and suitability regressions.
  - [x] Add PR2 workflow-contract review report.
  - [x] Move workflow unit tests to an integration test to keep `workflow.rs`
        under the enforced file budget.
  - [x] Run full local verification.
  - [x] Commit, push, and open PR.
  - [x] Address PR #65 review by adding explicit workload plan staging and
        deterministic retrieval preparation steps.
  - [x] Run review-fix verification.
  - [x] Push review-fix commit.
- [x] PR 3: Profile split.
  - [x] Create fresh branch from `origin/main` after PR #65 merge.
  - [x] Add explicit smoke and characterization-full profile ids.
  - [x] Add effective profile/depth metadata to workflow recommendation and
        collect-plan artifacts and generated instructions.
  - [x] Require explicit `--profile-depth` for legacy
        `target-operating-contract-fullset` compatibility use.
  - [x] Route `report validate-run`, operating-contract validation, run-report
        summaries, and rules predicates through supported workflow profiles.
  - [x] Add focused CLI regression tests for smoke, legacy-depth rejection, and
        characterization-full planned metadata plus collect-plan fail-closed
        behavior.
  - [x] Update public docs examples to use smoke profile and extend docs-smoke
        to reject legacy fullset examples without `--profile-depth`.
  - [x] Move workflow instruction rendering to `workflow_render.rs` to preserve
        enforced file budgets.
  - [x] Add PR3 workflow-contract review report.
  - [x] Run full verification.
  - [x] Commit, push, and open PR.
- [x] PR 4: Deep CPU / thermal characterization profile.
  - [x] Create fresh branch from `origin/main` after PR #66 merge.
  - [x] Replace characterization-full collect-plan fail-closed behavior with
        CPU/thermal argv steps only.
  - [x] Add 60s and 300s passive observation steps.
  - [x] Add 1/2/4 worker 60s CPU ladder steps with `--abort-temp-c 75`.
  - [x] Add 3 x 4-worker 60s repeatability steps with cooldown observations.
  - [x] Add 4-worker 300s sustained bounded load plus cooldown observation.
  - [x] Preserve the boundary that 300s evidence does not support 24h sustained
        safety and 900s remains optional/approved.
  - [x] Add focused CLI regression coverage for A-020 through A-023.
  - [x] Add PR4 workflow-contract review report.
  - [x] Run full verification.
  - [x] Commit, push, and open PR.
  - [x] Address PR #67 review blocker: preserve repeated/cooldown observation
        v2 artifacts with labeled unique sidecars.
  - [x] Address PR #67 review blocker: align load safety note with actual argv
        by removing the operator-abort claim.
  - [x] Run review-fix verification.
  - [x] Merge PR #67.
- [ ] PR 5: Pressure / composite / endpoint-backed network coverage.
  - [x] Create fresh branch from `origin/main` after PR #67 merge.
  - [x] Inspect existing pressure/composite CLI and operating-contract rules.
  - [x] Add characterization-full pressure coverage map steps.
  - [x] Separate counter-only `network_io` from optional endpoint-backed transfer
        in collect-plan argv.
  - [x] Preserve composite coupling claim boundary in generated step metadata.
  - [x] Add focused CLI regression tests for A-030 through A-033.
  - [x] Add PR5 workflow-contract review report.
  - [x] Run focused tests and full verification.
  - [x] Commit, push, and open PR.
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
  resolution report; included-run operating-contract refs continue to resolve
  after downstream suitability and constraints generation.
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

PR 2 dev-workflow route:

- Risk route: normal for this PR. It changes generated workflow handoff argv
  and suitability claim gating, but does not add a remote shell executor,
  privileged path, schema, or new target-local runtime.
- Definition of Done: A-001 through A-008 have focused tests; SSH collect plans
  run workload demand as target-local `--target local`; suitability consumes
  the retrieved deterministic include path; degraded/refused demand keeps
  CPU/memory unknown and `selection_ready=false`; `make verify` passes.
- Test List: SSH collect-plan workload step uses `execution_location =
  target_local`, argv arrays, `--target local`, `--execution-mode
  target-local`, and no `ssh://` target; workload plan staging is explicit;
  retrieval parent creation and rerun cleanup are explicit; retrieved path is
  deterministic and consumed by suitability; local collect plans do not emit
  synthetic SSH handoff steps; missing CPU/RSS metrics remain unknown; degraded
  workload demand remains unknown.
- Complexity Budget: changed production files <= 2; new modules/classes = 0;
  new helper functions in production = 0; initial new workflow step target <=
  1, revised to <= 5 after PR #65 review required executable staging,
  retrieval-parent, and rerun cleanup steps; production diff target <= 220
  lines; tests may add fixtures/assertions as needed.

PR 2 implementation-economy audit:

| New abstraction | Justification | Decision | Evidence |
|---|---|---|---|
| `prepare_target_local_workload_plan_dir` collect-plan step | Makes target-local input directory creation executable instead of prose-only. | keep | CLI collect-plan test asserts step order, target-local execution location, and exact `mkdir -p` argv. |
| `stage_target_local_workload_plan` collect-plan step | Prevents Agents from inventing scp staging harnesses for workload plans. | keep | CLI collect-plan test asserts exact controller source and target-local destination path. |
| `retrieve_target_local_workload_demand` collect-plan step | Makes the target-local workload handoff path explicit so suitability can consume a deterministic path without filename discovery. | keep | CLI and core collect-plan tests assert the step and exact consumed path. |
| `prepare_target_local_workload_retrieval_parent` and `reset_target_local_workload_retrieval_destination` collect-plan steps | Ensures retrieval parent exists and reruns cannot change `scp -r` destination layout. | keep | CLI collect-plan test asserts ordering, `mkdir -p`, scoped `rm -rf`, and rerun policy text. |
| Additional local variables in `target_operating_contract_collect_plan` | Reuses the existing collect-plan generator and target-local convention instead of adding a new executor layer. | keep | No new module/schema; tests cover SSH and local paths. |
| `crates/adc-lab-core/tests/workflow.rs` integration test target | Keeps workflow contract tests while returning `workflow.rs` below the enforced production file budget. | keep | File budget check reports `workflow.rs` at 1455/1500 after the PR #65 review fix. |

Budget note:
Production changes stay in `workflow.rs` and `suitability.rs`. The PR adds no
new schema or public command; the larger line count is test evidence for the
workflow handoff and conservative suitability semantics. Existing workflow unit
tests moved to an integration test file because `workflow.rs` exceeded the
1500-line production file budget after PR 2. PR #65 review added more workflow
steps, but `workflow.rs` remains under budget at 1455/1500.

### PR 3: Profile Split

Scope:

- Add explicit smoke and characterization-full profile ids.
- Generated recommendation and collect-plan text describe depth.
- Implement the Phase 0 compatibility decision for existing
  `target-operating-contract-fullset`.

Acceptance:

- A-010 through A-012.

PR 3 dev-workflow route:

- Risk: normal/high workflow-contract change. It changes public CLI defaults,
  schema-backed workflow artifacts, and downstream validation consumers.
- Triggered branches: execution-plan update, agent workflow contract review,
  focused characterization tests, schema regeneration, file-budget enforcement,
  full `make verify`.
- Non-triggered branches: no privileged target operation, no new remote executor,
  no UI or performance hot path.

PR 3 implementation-economy audit:

| New abstraction | Justification | Decision | Evidence |
|---|---|---|---|
| `workflow_profile.rs` | Centralizes smoke vs characterization-full profile ids, compatibility resolution, and profile metadata so CLI/report/rules do not drift. | keep | Focused CLI tests assert smoke metadata, characterization-full planned metadata, collect-plan fail-closed behavior, and legacy fullset depth rejection. |
| `workflow_render.rs` | Moves generated instruction rendering out of `workflow.rs` after profile metadata expanded the module past the enforced budget. | keep | `check-file-budgets.py --enforce` reports 0 violations; `workflow.rs` is 1366/1500. |
| `--profile-depth` CLI option | Makes legacy `target-operating-contract-fullset` compatibility explicit instead of silently mapping old fullset to smoke or deep characterization. | keep | `workflow_legacy_fullset_requires_profile_depth` fails closed without depth; legacy warning remains explicit. |

Budget note:

The profile split adds two small core modules rather than growing
`workflow.rs`; `workflow.rs` is 1366 lines after the split, below the 1500-line
budget. Public output schemas changed only for generated workflow artifacts.

PR 3 review-fix note:

- PR #66 intentionally left `target-characterization-full` collect-plan
  generation fail-closed until PR 4 supplied matching deep steps. PR 4 replaces
  that fail-closed behavior for the CPU/thermal slice only.

### PR 4: Deep CPU / Thermal Characterization Profile

Scope:

- CPU ladder, repeatability, sustained bounded load, cooldown, and safety caps.
- Output language distinguishes short seed from sustained bounded evidence.
- Rule/test guard that 300s bounded evidence does not support 24h safety.

Acceptance:

- A-020 through A-023.

PR 4 dev-workflow route:

- Risk: normal Agent-facing workflow-contract change. It changes generated
  collect-plan argv and profile metadata, but does not add a new executor,
  privileged target action, schema field, or claim relaxation.
- Triggered branches: execution-plan update, implementation-economy,
  agent-workflow-contract-review, focused characterization tests, file-budget
  enforcement, full `make verify`.
- Non-triggered branches: no concurrency, no UI, no arbitrary remote command,
  no target-local daemon/hot path.

PR 4 implementation-economy budget:

- Changed files target: 5 production/test/plan/report files plus this plan.
- New modules target: 1 internal core module.
- New helpers target: 3 local helper functions inside that module.
- Public schema/API target: 0 new schema fields; at most 1 optional
  `observe --artifact-label` CLI flag for collect-plan artifact identity.
- Line budget: keep `workflow.rs` under 1500; new module under 300; tests
  focused on A-020 through A-023.

PR 4 implementation-economy audit:

| New abstraction | Justification | Decision | Evidence |
|---|---|---|---|
| `workflow_characterization.rs` | Keeps the long CPU/thermal step list out of `workflow.rs`, preserving the enforced file budget and avoiding a second public workflow layer. | keep | `check-file-budgets.py --enforce` passes with 0 violations; focused CLI tests assert the generated steps. |
| `cpu_thermal_characterization_steps` | Provides one internal owner for PR4 CPU/thermal argv construction so smoke steps remain unchanged and PR5 pressure expansion has a clear boundary. | keep | `collect_plan_characterization_full_emits_cpu_thermal_steps` checks ordering, argv, duration/worker/abort/cooldown notes, and 300s-not-24h claim gate. |
| `write_observation_artifact_v2_with_label` | Preserves repeated/cooldown observation sidecars without teaching filename-order selection; collect-plan steps pass their own labels. | keep | `observation_v2_sidecars_keep_each_artifact_label` verifies distinct resolvable refs and labeled paths. |
| `observe --artifact-label` | Lets generated collect plans bind observation sidecar paths to step ids while keeping existing `observe` calls compatible. | keep | CLI characterization test asserts each observation step passes its step id and expected glob. |

Budget note:

`workflow.rs` remains the workflow authority entrypoint; the new module is
private and only returns `WorkflowCollectPlanStep` values. No generated schema
shape changed. PR #67 review added one optional CLI flag and one sidecar writer
helper to make repeated observation artifacts durable and reviewable.

### PR 5: Pressure / Composite / Endpoint-Backed Network Coverage

Scope:

- Pressure coverage map.
- Endpoint-backed network step separated from counter-only network evidence.
- Observer pressure and composite memory_storage_jitter coverage.

Acceptance:

- A-030 through A-033.

PR 5 dev-workflow route:

- Risk: normal Agent-facing workflow-contract change. It changes generated
  collect-plan argv and adds one optional collect-plan CLI input for existing
  pressure `--network-endpoint` support, but does not add a new executor,
  privileged operation, pressure primitive, or claim relaxation.
- Triggered branches: execution-plan update, implementation-economy,
  agent-workflow-contract-review, focused CLI workflow tests, rules regression
  confirmation, file-budget enforcement, full `make verify`.
- Non-triggered branches: no concurrency, no UI, no arbitrary remote command,
  no target-local daemon/hot path.

PR 5 implementation-economy budget:

- Changed production files target: 5 or fewer, plus tests, docs, plan, and
  workflow-contract review report.
- New modules target: 1 internal core module.
- New helpers target: bounded to the module-local step builders needed to keep
  `workflow.rs` under 1500 lines.
- Public schema/API target: no new artifact schema fields; one optional
  `collect plan --network-endpoint` flag to feed existing `pressure run
  --network-endpoint`.
- Line budget: keep `workflow.rs` under 1500; new module under 300; tests focus
  on A-030 through A-033.

PR 5 implementation-economy audit:

| New abstraction | Justification | Decision | Evidence |
|---|---|---|---|
| `workflow_pressure.rs` | Keeps the pressure/composite coverage map out of `workflow.rs`, preserving the workflow authority file budget and avoiding another public workflow layer. | keep | Focused CLI tests assert characterization-full pressure step ids, argv, expected artifact globs, network separation, and composite claim gate. |
| `pressure_composite_characterization_steps` | Provides one internal owner for pressure/composite argv construction so smoke steps remain unchanged and PR6 suitability linkage has a stable boundary. | keep | `collect_plan_characterization_full_emits_cpu_thermal_steps` now covers pressure map ordering and claim-boundary metadata. |
| `collect plan --network-endpoint` | Lets the workflow authority generate endpoint-backed transfer argv only when an explicit receiver is configured, instead of teaching Agents to hand-write network probe steps. | keep | CLI endpoint regression asserts counter-only and endpoint-backed `network_io` steps remain distinct. |

Budget note:

`pressure run` and `pressure composite` already own the measurement primitives and
artifact schemas. PR5 only routes existing typed commands into
`target-characterization-full` collect plans and keeps network counter-only
evidence separate from endpoint-backed bounded transfer evidence.

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
- 2026-06-15: PR #64 review found that the original A-052 evidence only tested
  suitability/constraints ref resolution against a single-run store. The
  review requires an included-run downstream regression or an explicit scope
  reduction.
- 2026-06-15: PR #64 merged into `origin/main` as
  `76f3e1f66ee80d9ff4af02f9b050d7b2694f27de`; PR 2 branch
  `codex/v024-pr2-target-local-workload` was created from that updated
  `origin/main`.
- 2026-06-15: Suitability memory gating already used available-memory evidence
  but did not require workload RSS. PR 2 tightens this so memory cannot become
  `meet` without process RSS demand from a clean workload profile.
- 2026-06-15: The first PR 2 implementation pushed `workflow.rs` to 1620 lines,
  above the enforced 1500-line file budget. Moving workflow tests to
  `crates/adc-lab-core/tests/workflow.rs` reduced production `workflow.rs` to
  1344 lines without weakening coverage.
- 2026-06-16: PR #65 review found two executable-handoff gaps: the SSH
  workload run plan was only described in prose instead of staged by a collect
  step, and workload retrieval did not create `<primary>/included` or define
  rerun behavior when the destination already existed.

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
- 2026-06-15: Address PR #64 review by adding the included-run downstream
  regression instead of narrowing PR scope. Rationale: PR 1 is the auditability
  foundation, so it should prove operating-contract, suitability, and
  constraints refs resolve through the same primary + included run set.
- 2026-06-15: Keep PR 2 as collect-plan plus suitability-gate work, not a new
  target-local executor. Rationale: the existing collect-plan target-local
  convention and `workload run --target local` are sufficient; `collect run`
  remains deferred.
- 2026-06-15: Model workload retrieval as an `operator_handoff` `scp` argv
  step, not measurement evidence. Rationale: PR 2 needs a deterministic
  retrieved path for suitability without adding a remote shell framework or
  pretending file transfer is target evidence.
- 2026-06-15: Move workflow tests out of `src/workflow.rs` instead of adding a
  file-budget override. Rationale: the tests remain first-class integration
  coverage and the production module stays within the existing budget.
- 2026-06-16: Fix PR #65 with explicit argv steps instead of prose-only
  instructions. Rationale: collect plan is the executable handoff contract, so
  target-local input staging must be represented as `mkdir -p` plus `scp`
  steps before `workload_demand`.
- 2026-06-16: Use controller-side deterministic cleanup for workload retrieval
  reruns. Rationale: `scp -r source dest` changes layout when `dest` already
  exists; deleting only `<primary>/included/target-local-workload-demand`
  before retrieval keeps the consumed suitability path stable without adding an
  rsync dependency.
- 2026-06-16: Implement PR 3 compatibility Option C as fail-closed legacy
  resolution. Rationale: `target-operating-contract-fullset` no longer has a
  single unambiguous depth, so legacy CLI use requires `--profile-depth
  smoke|characterization-full`; new public defaults use
  `target-operating-contract-smoke`.
- 2026-06-16: Keep the workflow id
  `target-operating-contract-fullset.v0.2.3` while splitting effective
  profiles. Rationale: the source-of-truth workflow family remains the same,
  while `goal`, `effective_profile`, and `profile_depth` now carry the
  measurement-depth contract.
- 2026-06-16: Move instruction rendering into `workflow_render.rs` instead of
  adding a file-budget override. Rationale: generated markdown rendering is a
  separate responsibility from collect-plan construction and the move restores
  budget headroom.
- 2026-06-16: Address PR #66 review by making
  `target-characterization-full` collect-plan generation fail closed until PR
  4/5/6 implement matching deep steps. Rationale: Agent-facing profile metadata
  must not claim coverage that the actual generated argv steps cannot produce.
- 2026-06-16: Extend docs-smoke to reject public legacy fullset CLI examples
  without nearby `--profile-depth`. Rationale: public docs must remain
  executable after legacy fullset was made fail-closed.
- 2026-06-16: Make `target-characterization-full` executable for the PR4
  CPU/thermal slice only. Rationale: PR4 can now satisfy A-020 through A-023
  without claiming PR5 pressure/network depth or PR6 suitability linkage.
- 2026-06-16: Put CPU/thermal collect-plan step construction in private
  `workflow_characterization.rs` rather than extending `workflow.rs` inline.
  Rationale: the explicit ladder/repeatability/cooldown sequence is long and
  would erode the 1500-line file budget in the workflow authority module.
- 2026-06-16: Add `observe --artifact-label` and write v2 observation sidecars
  to `observations/<label>.<artifact_id>.v2.json` while keeping the v1
  `observations/observe.json` latest output. Rationale: repeated/cooldown
  observations must survive in the archive as reviewable artifacts, and the
  generated collect plan must not rely on filename order or a single latest
  path.
- 2026-06-16: Do not add `--operator-abort-file` to PR4 CPU/thermal load steps.
  Rationale: deterministic target-local abort paths would require another
  controller/SSH path decision; PR4 already has explicit duration, worker count,
  and thermal abort bounds.
- 2026-06-16: Implement PR5 pressure/composite coverage as generated collect-plan
  argv over existing `pressure run` / `pressure composite` commands, not new
  pressure primitives. Rationale: the missing v0.2.4 behavior is workflow
  authority coverage, while pressure artifacts and conservative rules already
  distinguish measured, insufficient, not_applicable, endpoint-backed network,
  and composite coupling states.
- 2026-06-16: Add optional `collect plan --network-endpoint` only for
  characterization-full endpoint-backed network steps. Rationale: counter-only
  `network_io` must remain executable without a receiver, and bounded transfer
  claims require an explicit endpoint instead of inferred rx/tx deltas.

## Handoff

Base branch:
`origin/main` after PR #67 is merged (`579766000b8d`).

Current implementation branch:
`codex/v024-pr5-pressure-network`.

Status:
PR #67 is merged. PR5 implementation is on a fresh branch from merge commit
`579766000b8d`. The current implementation extends
`target-characterization-full` collect-plan generation from the PR4 CPU/thermal
slice into pressure/composite coverage: latency/jitter, observer pressure,
memory pressure, storage I/O, CPU pressure, thermal pressure, counter-only
network I/O, optional endpoint-backed network transfer when
`--network-endpoint` is supplied, and memory/storage/jitter composite probing.
The generated step metadata keeps counter-only network evidence separate from
bounded transfer evidence and keeps coupling claims blocked unless composite
evidence is measured. Focused tests, schema/docs/file-budget checks, and full
`make verify` passed locally. Draft PR #68 is open.

Reviewed implementation commit:
`4d667f77faf3922aee94802f6163dd65fede3b2b`.

Latest pushed commit:
this ExecPlan-only status update commit on
`codex/v024-pr5-pressure-network`.

Current PR:
https://github.com/shunta-sato/agent-debug-compass-laboratory/pull/68

Next steps:
1. Wait for PR #68 CI and review.
2. Address review comments if any.
3. Merge after review approval and CI success.

Required process:
every implementation PR must include
`reports/workflow-contract-review/<slug>.md` because workflow-contract review
is now part of the development gate.

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
- `cargo test -p adc-lab --test cli suitability_and_constraints_refs_resolve_across_included_run_set -- --nocapture`: pass.
- `cargo test -p adc-lab-core --test probe_artifacts -- --nocapture`: pass.
- `cargo test -p adc-lab-core --test rules_engine operating_contract -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli report_operating_contract -- --nocapture`: pass.
- `make schemas-check`: pass (`schema ledger: ok top_level=0 no_schema_wire=31 maintained_by_hand=0`).
- `make contract`: pass.
- `make docs-smoke`: pass.
- `make verify`: pass.
- PR #64 review-fix verification:
  - `cargo test -p adc-lab --test cli suitability_and_constraints_refs_resolve_across_included_run_set -- --nocapture`: pass.
  - `make docs-smoke`: pass.
  - `make schemas-check`: pass (`schema ledger: ok top_level=0 no_schema_wire=31 maintained_by_hand=0`).
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

PR 2 verification:

- `cargo test -p adc-lab-core suitability_ -- --nocapture`: pass.
- `cargo test -p adc-lab-core collect_plan_steps_are_argv_arrays_and_not_measurement_evidence -- --nocapture`: pass.
- `cargo test -p adc-lab-core --test workflow -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli collect_plan -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli decide_suitability_refused_workload_demand_keeps_selection_not_ready -- --nocapture`: pass.
- `git diff --check`: pass.
- `make docs-smoke`: pass.
- `make schemas-check`: pass (`schema ledger: ok top_level=0 no_schema_wire=31 maintained_by_hand=0`).
- `make verify`: pass (`file budgets: enforced checked=55 violations=0`; CLI 49 tests, safety invariants 9 + 19 tests, and workspace tests green).

PR 2 quality gate:

- Decision: submit.
- Findings: 0.
- Required artifacts present: ExecPlan updated, PR2 workflow-contract review
  report decision `submit`, implementation-economy audit recorded, workflow
  tests moved to an integration test to keep production file budgets green.
- Draft PR opened: #65
  (`https://github.com/shunta-sato/agent-debug-compass-laboratory/pull/65`) at
  initial implementation head `4069641ac34c56e48f132e034d32d229021f036a`.

PR #65 review-fix verification:

- `cargo fmt --all --check`: pass.
- `cargo test -p adc-lab-core --test workflow -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli collect_plan -- --nocapture`: pass.
- `python3 scripts/ci/check-file-budgets.py --enforce`: pass (`workflow.rs`
  1455/1500; `file budgets: enforced checked=55 violations=0`).
- `make docs-smoke`: pass.
- `make schemas-check`: pass (`schema ledger: ok top_level=0 no_schema_wire=31 maintained_by_hand=0`).
- `make verify`: pass.

PR #65 review-fix quality gate:

- Decision: submit.
- Findings: 0.
- Required artifacts present: ExecPlan updated, PR2 workflow-contract review
  report updated with staging/retrieval preparation chain, implementation
  economy audit updated for the added handoff steps, focused regression tests
  and full verification green.

PR 3 verification:

- `cargo test -p adc-lab --test cli workflow_ -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli collect_plan_ -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli validate_run -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli operating_contract -- --nocapture`: pass.
- `make schemas`: pass; generated workflow recommendation / collect-plan
  schemas updated.
- `python3 scripts/ci/check-file-budgets.py --enforce`: pass (`file budgets:
  enforced checked=57 violations=0`; `workflow.rs` 1365/1500).
- `make schemas-check`: pass (`schema ledger: ok top_level=0 no_schema_wire=31
  maintained_by_hand=0`).
- `cargo test -p adc-lab-core -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli -- --nocapture`: pass (51 tests).
- `make docs-smoke`: pass.
- `make verify`: pass.

PR 3 quality gate:

- Decision: submit.
- Findings: 0 from workflow-contract review report
  `reports/workflow-contract-review/v024-pr3-profile-split.md`.
- Required artifacts present: ExecPlan updated, PR3 workflow-contract review
  report added, implementation-economy audit recorded, generated schemas
  updated, focused and full verification green.
- Draft PR opened: #66
  (`https://github.com/shunta-sato/agent-debug-compass-laboratory/pull/66`) at
  implementation head `778e3e33644bc417dc5f05df71c2861db94ddde7`.

PR #66 review-fix verification:

- `make docs-smoke`: pass after public examples were updated and stale legacy
  fullset example guard was added.
- `cargo test -p adc-lab --test cli workflow_ -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli collect_plan_ -- --nocapture`: pass.
- `cargo test -p adc-lab-core --test workflow -- --nocapture`: pass.
- `git diff --check`: pass.
- `make verify`: pass (`file budgets: enforced checked=57 violations=0`; CLI
  52 tests; docs artifact heuristic guard ok).

PR #66 review-fix quality gate:

- Decision: submit.
- Findings addressed: public CLI examples no longer use legacy fullset without
  `--profile-depth`; characterization-full collect-plan generation now fails
  closed until deep steps exist.
- Review-fix commit pushed:
  `605d144a3ef50caa5f8fe9663c0a8be709ef6fb8`.

PR 4 focused verification:

- `cargo test -p adc-lab --test cli characterization_full -- --nocapture`:
  pass (2 tests).
- `cargo test -p adc-lab-core --test workflow -- --nocapture`: pass (3 tests).
- `python3 scripts/ci/check-file-budgets.py --enforce`: pass (`file budgets:
  enforced checked=58 violations=0`).
- `cargo test -p adc-lab --test cli collect_plan_ -- --nocapture`: pass (3
  tests).
- `cargo test -p adc-lab --test cli workflow_ -- --nocapture`: pass (5 tests).
- `make docs-smoke`: pass (`docs artifact heuristic guard: ok`).
- `make verify`: pass (`file budgets: enforced checked=58 violations=0`; CLI
  52 tests; safety invariant tests 9 + 19; docs artifact heuristic guard ok;
  command smoke host fallback ok).

PR 4 quality gate:

- Decision: submit.
- Findings: 0 from workflow-contract review report
  `reports/workflow-contract-review/v024-pr4-deep-cpu-thermal.md`.
- Required artifacts present: ExecPlan updated, PR4 workflow-contract review
  report added, implementation-economy audit recorded, focused regression tests
  and full verification green.
- Draft PR opened: #67
  (`https://github.com/shunta-sato/agent-debug-compass-laboratory/pull/67`) at
  implementation head `8ef4c2bdf3f91b98fe974200a513d6aa2769fb9c`.

PR #67 review-fix verification:

- `cargo test -p adc-lab-core --test probe_artifacts observation_v2_sidecars_keep_each_artifact_label -- --nocapture`:
  pass.
- `cargo test -p adc-lab --test cli observe_artifact_label_preserves_repeated_v2_sidecars -- --nocapture`:
  pass.
- `cargo test -p adc-lab --test cli characterization_full -- --nocapture`:
  pass (2 tests).
- `cargo test -p adc-lab --test cli collect_plan_ -- --nocapture`: pass (3
  tests).
- `cargo test -p adc-lab-core --test workflow -- --nocapture`: pass (3 tests).
- `python3 scripts/ci/check-file-budgets.py --enforce`: pass (`file budgets:
  enforced checked=58 violations=0`).
- `make docs-smoke`: pass (`docs artifact heuristic guard: ok`).
- `make verify`: pass (`file budgets: enforced checked=58 violations=0`; CLI
  53 tests; probe artifact tests 6; docs artifact heuristic guard ok; command
  smoke host fallback ok).

PR #67 review-fix quality gate:

- Decision: submit.
- Findings addressed: repeated/cooldown observations now write labeled unique
  v2 sidecars, collect-plan observation steps include `--artifact-label` and
  expected globs, and load safety notes only claim duration/worker/thermal
  bounds actually present in argv.

PR 5 focused verification:

- `cargo test -p adc-lab --test cli collect_plan_characterization_full -- --nocapture`:
  pass (2 tests).
- `cargo test -p adc-lab-core --test rules_engine -- --nocapture`: pass (14 tests).
- `cargo test -p adc-lab-core --test workflow -- --nocapture`: pass (3 tests).
- `python3 scripts/ci/check-file-budgets.py --enforce`: pass (`file budgets:
  enforced checked=59 violations=0`; `workflow.rs` 1391/1500;
  `workflow_pressure.rs` 273 lines).
- `make docs-smoke`: pass (`docs artifact heuristic guard: ok`).
- `make schemas-check`: pass (`schema ledger: ok top_level=0 no_schema_wire=31
  maintained_by_hand=0`).
- First `make verify`: failed at clippy for useless `Into::into` conversion in
  `workflow_pressure.rs`; fixed by passing the `Vec<String>` directly.
- `cargo fmt --all --check`: pass after the clippy fix.
- `git diff --check`: pass.
- `make verify`: pass (`file budgets: enforced checked=59 violations=0`; CLI
  54 tests; safety invariant tests 9 + 19; schema ledger ok; docs artifact
  heuristic guard ok; command smoke host fallback ok).

PR 5 quality gate:

- Decision: submit.
- Findings: 0 from workflow-contract review report
  `reports/workflow-contract-review/v024-pr5-pressure-network.md`.
- Required artifacts present: ExecPlan updated, PR5 workflow-contract review
  report added, implementation-economy audit recorded, focused regression tests
  and full verification green.
