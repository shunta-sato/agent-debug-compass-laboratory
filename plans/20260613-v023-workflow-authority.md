# adc-lab v0.2.3 Workflow Authority

## Purpose / Big Picture

Execution GOAL:

Make adc-lab the workflow authority for Agent-driven Target / Platform
Operating Contract full-set collection. A controller Agent must not depend on
static version-specific prompts, shell harnesses, filename order, mtimes, or
directory co-presence to create claim-producing evidence.

The v0.2.3 shift is from "safe primitives plus docs" to "adc-lab exposes the
valid workflow, emits a machine-readable plan, and validates whether resulting
claims are measured, insufficient, refused, contaminated, not applicable, or
unknown."

## Scope

In scope:

- `workflow recommend` for `target-operating-contract-fullset`.
- Deterministic Agent instructions generated from the installed workflow
  registry.
- `collect plan` as an argv-array execution handoff contract.
- Multi-run `report validate-run` with workflow/plan/run-set identity and
  version-skew policy.
- `report operating-contract --validation` gating for controlled-governor
  claims.
- `constraints check-candidate` and `constraints self-check`.
- Docs, examples, fixtures, and smoke guards against stale prompt patterns.

Out of scope:

- `collect run`.
- Remote privileged apply/restore over SSH.
- Agent root shells, arbitrary helper paths, arbitrary sysfs writes, or new
  stress tools.
- Pi4/Pi5 selection claims, 24h soak claims, or real application performance
  claims.

## Problem Frame

- Problem owner: adc-lab workflow and evidence contracts, not only the
  controller Agent prompt.
- Current pain / evidence: a v0.2.1 static prompt could be reused against
  v0.2.2, bypassing the claim-producing workflow and relying on fragile
  artifact selection patterns.
- Desired outcome: an Agent asks adc-lab for the valid workflow, executes the
  typed plan, and receives validation that gates downstream claims.
- Solution-first risk: adding another shell wrapper would preserve the stale
  harness failure mode.
- Proceed to implementation: yes.

## Requirements

| ID | Priority | Requirement | Acceptance criteria | Verification method |
|---|---|---|---|---|
| R-001 | Must | adc-lab shall expose a machine-readable workflow recommendation for `target-operating-contract-fullset`. | Recommendation includes workflow id, installed version, must-use steps, forbidden patterns, expected outputs, version policy, and evidence policy. | CLI integration test; generated schema check. |
| R-002 | Must | `workflow.recommendation` shall be a workflow authority artifact, not target measurement evidence. | Recommendation artifact has no capability claims and cannot support operating-contract claims by itself. | CLI/core tests and schema review. |
| R-003 | Must | Agent instructions shall be generated from the installed workflow registry. | Output includes version, workflow id, expected artifacts, forbidden patterns, and fallback prohibition. | Snapshot/fixture test and docs guard. |
| R-004 | Must | `collect plan` shall emit argv-array typed steps. | Every step has the required step contract and no shell fragments. | Schema and CLI tests. |
| R-005 | Must | Multi-run validation shall preserve workflow/plan/run-set identity. | Validation copied from another run set cannot satisfy operating-contract validation. | CLI/core fixture tests. |
| R-006 | Must | Version skew shall block full-set measured claims by default. | `--allow-version-skew` records an override but full-set claims remain insufficient/blocked. | Mixed-version fixture tests. |
| R-007 | Must | Operating-contract controlled-governor claims shall consume matching `report.run_validation`. | Raw primitive artifacts without matching validation are downgraded. | Rule/CLI tests. |
| R-008 | Should | Constraints checking shall expose candidate and self-check commands. | Generated constraints self-check tolerates negative/explanatory blocked claims; candidate content remains strict. | CLI tests. |

## Constraints / Quality Targets

- Preserve the North Star: no Agent root shell, no uncontrolled experiment, no
  unapproved hard-to-restore operation, no unqualified tool evidence, no claim
  without audit.
- Workflow recommendation and collect-plan artifacts are authority / handoff
  artifacts, not target capability measurements.
- File order, timestamp order, directory co-presence, and "latest artifact"
  heuristics are never causal linkage.
- All new public artifacts use the existing v2 `Artifact<P>` envelope and are
  generated-schema checked.
- Every new schema-versioned artifact gets a `schemas/schema-ledger.tsv` row in
  the same PR that introduces it.
- Each PR must pass `make verify` before publication.

## Context & Orientation

Current base:

- `origin/main` contains v0.2.2 after PR #54.
- Existing surfaces: `control governor-sweep`, single-run `report
  validate-run`, operating-contract validation predicates, `constraints check
  --mode`, generated schemas, file budgets, and docs heuristic guard.

Key files:

- `crates/adc-lab/src/main.rs`: CLI definitions and dispatch.
- `crates/adc-lab/src/commands/*`: command implementations.
- `crates/adc-lab-core/src/evidence/envelope.rs`: v2 artifact envelope and
  `Kind`.
- `crates/adc-lab-core/examples/generate_schemas.rs`: generated schema source.
- `schemas/schema-ledger.tsv`: schema contract ledger.
- `crates/adc-lab/tests/cli.rs`: CLI behavior tests.

## Design

### Dev Workflow Route

- Risk route: high, because the full v0.2.3 plan changes public CLI, public
  artifacts, and claim-producing validation semantics.
- Required branches: ExecPlan, implementation-economy, design-balance, focused
  tests, full `make verify`, and quality gate.
- Verification depth: full repository gate for each PR.

### Responsibility Map

| Unit | Responsibility | Reason to change | Dependency direction |
|---|---|---|---|
| `adc_lab_core::workflow` | Own workflow registry DTOs and deterministic payload construction. | Workflow contract or registry changes. | Depends on stable build/evidence types; CLI depends on it. |
| `commands::workflow` | Persist/print workflow recommendations and attach audit only for run-dir writes. | CLI persistence/output policy changes. | Depends on core workflow and common command helpers. |
| `commands::agent` | Render controller-Agent instructions from registry data. | Prompt contract changes. | Depends on core workflow. |
| future `commands::collect` | Persist argv-array collect plans and optional markdown handoff. | Collect handoff contract changes. | Depends on core workflow and CLI command catalog. |

Layout decision: start with one core workflow module and one CLI command module.
Do not create a generic workflow engine in PR1; add only the registry and
payload builder needed by `workflow recommend`.

### Complexity Budget

- PR1 changed files target: <= 8 tracked files.
- New modules target: 2 (`adc_lab_core::workflow`, `commands::workflow`).
- New helper target: <= 4 focused helpers for payload construction/persistence.
- Production lines target: <= 350; test lines target: <= 120.
- Indirection target: no plugin or trait layer; direct registry functions only.

### Source-of-Truth Chain

The intended artifact chain is:

```text
workflow.recommendation
  -> workflow.collect_plan
  -> report.run_validation
  -> report.operating_contract
  -> report.suitability
  -> report.constraints
```

Every downstream artifact that consumes an upstream authority must retain the
upstream artifact ref or digest, workflow id, goal, target id, and target class
when that upstream artifact exists.

### Recommendation Semantics

- `workflow.recommendation` status is `not_applicable` because it is not target
  measurement evidence.
- It contains no capability claims.
- stdout-only recommendation is allowed for offline discovery and does not
  require run audit.
- A recommendation written into a run directory appends an audit event.

## Validation & Acceptance

Per PR:

- `make verify` must pass.
- Focused tests for changed surfaces must pass before the full gate.

Final v0.2.3:

- `make verify`
- `make schemas-check`
- `make docs-smoke`
- `cargo test -p adc-lab-core -- --nocapture`
- `cargo test -p adc-lab --test cli -- --nocapture`

## Progress (WBS)

- [x] Phase 1: Create ExecPlan and record v0.2.3 design decisions.
- [x] Phase 1: Add workflow registry and `workflow.recommendation` DTO.
- [x] Phase 1: Add `workflow recommend` CLI.
- [x] Phase 1: Add generated schema and schema ledger row.
- [x] Phase 1: Add CLI/core tests proving recommendation shape and non-evidence semantics.
- [x] Phase 2: Add `agent instructions` command and deterministic Codex prompt fixture.
- [ ] Phase 3: Extend multi-run `report validate-run` and version-set policy.
- [ ] Phase 4: Add `collect plan` artifact and argv-array step contract.
- [ ] Phase 5: Add operating-contract `--validation` / `--strict-fullset` gate.
- [ ] Phase 6: Add `constraints check-candidate` / `constraints self-check`.
- [ ] Phase 7: Update docs/examples and expand stale-pattern guards.

## Design -> WBS Coverage Check

| Design deliverable | WBS coverage |
|---|---|
| `workflow.recommendation` artifact | Phase 1 |
| `workflow.collect_plan` artifact | Phase 4 |
| `report.run_validation` run-set identity | Phase 3 |
| `report.operating_contract` validation gate | Phase 5 |
| Agent instructions | Phase 2 |
| Constraints split | Phase 6 |
| Docs and stale-pattern guard expansion | Phase 7 |

## Surprises & Discoveries

- 2026-06-13: `plans/_template_execplan.md` is referenced by the skill docs but
  is absent in the repository. This plan follows the established structure of
  `plans/20260612-issue48-agent-safe-fullset.md` instead.

## Decision Log

- 2026-06-13: Treat v0.2.3 as a multi-PR delivery, starting with a small PR1
  that only introduces workflow recommendation. Rationale: `collect plan`
  should not publicly emit commands that require validation semantics not yet
  merged.
- 2026-06-13: Use `Status::NotApplicable` for `workflow.recommendation`.
  Rationale: it is an authority artifact, not target measurement evidence.
- 2026-06-13: Keep stdout-only recommendation audit-free. Rationale: offline
  recommendation should be usable before a run directory exists.
- 2026-06-13: Make workflow/agent surfaces JSON-first for PR1/PR2. Rationale:
  these are Agent-facing machine surfaces; `--json` remains accepted for CLI
  consistency while the commands still produce machine-readable summaries.
- 2026-06-13: Render agent instructions from `workflow.recommendation` without
  copying literal forbidden shell snippets. Rationale: the prompt must prohibit
  fragile artifact selection without teaching the pattern.

## Handoff

Current branch: `codex/v023-agent-instructions`.

PR1 status: merged as PR #55.
PR2 status: implemented locally and verified.

Next steps:

1. Commit and open PR2 against `main`.
2. After PR2 merge, start Phase 3 on a fresh branch from updated `origin/main`.
3. Extend multi-run `report validate-run` and version-set policy.

## Outcomes & Retrospective

PR1 outcomes:

- Added `workflow.recommendation` as a v2 authority artifact.
- Added `adc-lab workflow recommend`.
- Added generated schema and schema-ledger coverage.
- Verified recommendation is not target measurement evidence by status and
  empty claims.

Post-implementation economy audit:

| New abstraction | Justification | Decision | Evidence |
|---|---|---|---|
| `adc_lab_core::workflow` | Keeps workflow registry DTOs out of CLI and prevents prompt strings from becoming ad hoc command logic. | keep | Unit test and generated schema. |
| `commands::workflow` | Owns CLI persistence/audit policy without mixing workflow DTO construction into `main.rs`. | keep | CLI tests for stdout and run-dir write paths. |

Verification:

- `cargo test -p adc-lab-core workflow -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli workflow_recommend -- --nocapture`: pass.
- `make schemas-check`: pass.
- `make verify`: pass.

PR2 outcomes:

- Added `adc-lab agent instructions`.
- Rendered Codex instructions deterministically from `workflow.recommendation`.
- Included installed adc-lab build info, workflow id, expected outputs,
  fallback prohibition, and collect-plan deferred next step.
- Added tests proving generated prompts do not contain fragile artifact
  selection snippets.

PR2 post-implementation economy audit:

| New abstraction | Justification | Decision | Evidence |
|---|---|---|---|
| `render_codex_agent_instructions` | Centralizes prompt rendering in the workflow registry so CLI does not invent a second prompt contract. | keep | Core test and CLI test. |
| `commands::agent` | Keeps agent prompt file output separate from workflow recommendation persistence/audit policy. | keep | CLI test for `agent instructions`. |

PR2 verification:

- `cargo test -p adc-lab-core workflow -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli agent_instructions -- --nocapture`: pass.
- `make verify`: pass.
