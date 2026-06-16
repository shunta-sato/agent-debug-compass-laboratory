# adc-lab v0.2.4.2 Evidence Ref Resolution Fix ExecPlan

## Purpose / Big Picture

v0.2.4.1 target55 execution proved the workflow-authority pipeline can execute
and collect a full run set, but its `report.evidence_ref_resolution` artifact was
degraded by invalid refs for the retrieved governor-sweep include-run. The broken
refs had this shape:

```text
artifact://lab/runs/<included-run-id>/included/target-local-governor-sweep/...
```

The included run root was already opened as:

```text
<primary>/included/target-local-governor-sweep
```

so resolving the ref joined the include-run path twice. The goal is to ensure
`EvidenceStore` indexes each opened run only under that run root's logical
identity. A copied include-run under the primary run must not also be indexed as
part of the primary recursive scan.

## Scope

In scope:

- Fix `EvidenceStore` indexing for nested include-run directories.
- Add a regression test reproducing the v0.2.4.1 layout:
  `<primary>/included/target-local-governor-sweep`.
- Record RCA and workflow-contract review.
- Run focused tests and `make verify`.
- Open a PR against `main`.

Out of scope:

- No target55 rerun in this PR.
- No CPU suitability policy change.
- No archive post-processing orchestration change.
- No change to production-readiness claim semantics.
- No privilege doctor UX redesign.
- No relaxation of evidence ref resolution, run validation, or operating-contract
  validation gates.

## Requirements

- R-001: `EvidenceStore::open([primary, included])` must not index v2 artifacts
  from `included` while recursively scanning `primary`.
- R-002: The same included artifact must still be indexed and resolvable when
  the included run root is explicitly opened.
- R-003: Evidence refs generated from included run artifacts must be relative to
  the included run root, not the primary run root.
- R-004: Directory co-presence must not qualify included evidence. An include-run
  must be explicitly opened as an included run root.
- R-005: Safety boundaries stay unchanged: no new target-local primitive, no root
  operation, no claim relaxation.

## Design

`EvidenceStore::open` computes a logical run id for each opened root. During the
recursive scan, `index_json_if_v2` ignores a v2 artifact only when its envelope
`run_id` belongs to a nested directory that has its own `run_context.json`.

This turns nested include-run directories into ordinary files from the primary
root's perspective. They are ignored by the primary scan and become evidence only
when the workflow opens them through `--include-run`.

The filter deliberately does not reject every envelope `run_id` mismatch. Some
legacy-projected sidecars, such as workload demand artifacts, can carry a source
profile run id while still living in the opened run root. The bug class is a
nested run root being indexed by its parent.

Rejected alternative:

- Resolver-side alias stripping. This would hide malformed refs after the fact
  and make directory co-presence look acceptable. The safer boundary is to
  prevent bad artifact refs from being produced by the index.

Deferred review findings:

- Archive checksum audit sequence: likely execution-harness/postprocess ordering,
  not this resolver bug.
- CPU policy at 80%: policy decision, not a tooling defect.
- `production_ready` missing display and composite next-evidence wording: valid
  UX/claim-language follow-ups, but not required to unblock correct ref
  resolution.
- Privilege doctor SSH display: target-local ergonomics follow-up.

## Validation

Focused commands:

```bash
cargo test -p adc-lab-core evidence_store_indexes_nested_include_run_only_under_its_own_root -- --nocapture
cargo test -p adc-lab-core probe_artifacts -- --nocapture
cargo test -p adc-lab --test cli suitability_and_constraints_refs_resolve_across_included_run_set -- --nocapture
make verify
```

Acceptance:

- A-001: Nested include-run v2 artifacts are indexed exactly once when both
  primary and included roots are opened.
- A-002: The indexed ref does not contain
  `/included/target-local-governor-sweep/`.
- A-003: `EvidenceRefResolutionPayload.invalid_refs` is empty for the nested
  include-run regression fixture.
- A-004: Existing included run-set CLI resolution test remains green.
- A-005: `make verify` remains green.

## WBS / Progress

- [x] WBS 0: Inspect target55 v0.2.4.1 review evidence and repo state.
- [x] WBS 1: Identify root cause in `EvidenceStore` recursive indexing.
- [x] WBS 2: Add nested include-run regression test.
- [x] WBS 3: Implement logical-run-id filter in `EvidenceStore` scanning.
- [x] WBS 4: Add RCA and workflow-contract review reports.
- [x] WBS 5: Run focused tests and `make verify`.
- [x] WBS 6: Commit, push, and open PR.

## Decision Log

- 2026-06-17: Fix producer/index side rather than resolver aliasing. Rationale:
  include-run evidence must be explicit in the opened run set; silently
  canonicalizing broken refs would preserve an unsafe directory co-presence
  assumption.
- 2026-06-17: Treat CPU policy, archive checksum sequencing, production-ready
  display, composite wording, and privilege doctor display as follow-ups.
  Rationale: they have separate claim or execution-policy impact and should not
  be bundled into the evidence ref resolver fix.
- 2026-06-17: Do not reject all envelope/root `run_id` mismatches. Rationale:
  focused testing showed existing workload sidecars can legitimately carry
  source profile identity; the resolver bug is specifically nested run-root
  co-presence.

## Verification Log

- 2026-06-17:
  `cargo test -p adc-lab-core evidence_store_indexes_nested_include_run_only_under_its_own_root -- --nocapture`
  passed.
- 2026-06-17:
  `cargo test -p adc-lab-core --test probe_artifacts -- --nocapture` passed.
- 2026-06-17:
  `cargo test -p adc-lab --test cli suitability_and_constraints_refs_resolve_across_included_run_set -- --nocapture`
  passed.
- 2026-06-17: `make verify` passed.

## Handoff

Current PR: #74.

Reviewed implementation commit:
`9b7637c56c43f664debd40fa1e40ade77911ea2a`

Status: draft PR open from `codex/v0242-evidence-ref-resolution`; local
verification passed.

Next steps:

1. Wait for GitHub CI.
2. Address review comments.
3. Mark Ready for review and merge after CI remains green.
