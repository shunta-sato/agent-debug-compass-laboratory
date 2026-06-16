# Bug Report: v0.2.4.1 Nested Include-Run Evidence Refs

## Summary

The v0.2.4.1 target55 archive showed
`report.evidence_ref_resolution` with invalid refs for retrieved governor-sweep
artifacts. The run and validation pipeline otherwise executed: the include-run
was retrieved and opened, but some refs were emitted with the include-run path
duplicated.

## Impact

- `report.evidence_ref_resolution` became `insufficient` / degraded.
- Downstream review saw invalid refs even though equivalent valid refs for the
  same included artifacts existed.
- This undermined the handoff chain review, but it did not relax target claims:
  Target55 remained not selection-ready and not production-ready.

## Symptoms

Invalid refs had this shape:

```text
artifact://lab/runs/<included-run-id>/included/target-local-governor-sweep/load/...
artifact://lab/runs/<included-run-id>/included/target-local-governor-sweep/reports/run_validation.v2.json
```

The resolver map already opened `<included-run-id>` at:

```text
<primary>/included/target-local-governor-sweep
```

so resolution attempted:

```text
<primary>/included/target-local-governor-sweep/included/target-local-governor-sweep/...
```

## Root Cause

`EvidenceStore::open` recursively scanned every opened run root. When the primary
root contained a retrieved include-run directory, the primary scan parsed the
included run's v2 artifacts. `index_json_if_v2` used the artifact envelope
`run_id` but calculated the path relative to the primary root. That created an
artifact ref combining the included run id with a primary-root-relative path.

The included run was then scanned again as its own opened run root, producing the
correct ref. The store therefore contained both a bad alias and a good ref for
the same artifact.

## Fix

`EvidenceStore::open` now computes the logical run id for each opened root and
ignores a v2 artifact when its envelope `run_id` belongs to a nested directory
with its own `run_context.json`.

This keeps directory co-presence from creating evidence. Retrieved include-runs
must be explicitly opened through `--include-run` to become resolvable evidence.

A broader "reject every run_id mismatch" rule was tested and rejected because
existing legacy-projected sidecars can carry source profile identity while still
being valid artifacts in the opened run root.

## Prevention

Added a regression test with this exact layout:

```text
<primary>/included/target-local-governor-sweep
```

The test asserts that pressure evidence from the nested include-run is indexed
exactly once, has no duplicated include-run path, and resolves cleanly.

## Follow-ups

- Archive checksum audit ordering should be handled in the run harness or
  postprocess contract.
- CPU policy should be decided explicitly before changing suitability outcomes.
- Production-ready missing display and composite next-evidence wording can be
  refined without changing this resolver boundary.
- Privilege doctor display for SSH targets should be clarified separately.
