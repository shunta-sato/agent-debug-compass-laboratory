# Workflow Contract Review: v0.2.4.2 Evidence Ref Resolution

## Scope

Review target: nested include-run evidence refs in v0.2.4.1 target55 workflow
authority runs.

Changed contract: `EvidenceStore` indexing behavior for v2 artifacts under
opened run roots.

## Source-of-Truth Chain

The v0.2.4 chain remains unchanged:

```text
workflow.recommendation
  -> workflow.collect_plan
  -> report.run_validation
  -> report.operating_contract / report.evidence_ref_resolution
  -> report.suitability
  -> report.constraints
  -> report.constraints_check
```

This change only fixes how the store indexes opened roots for
`report.evidence_ref_resolution` and rule evidence refs.

## Producer / Consumer Consistency

| Item | Producer | Consumer | Result |
| --- | --- | --- | --- |
| Primary run artifacts | primary run root | `EvidenceStore` primary scan | unchanged |
| Retrieved governor sweep | collect-plan retrieval step | explicit `--include-run` root | fixed |
| Nested include-run files during primary scan | directory co-presence | none | ignored |

The important boundary is that retrieval into `<primary>/included/...` does not
itself make the contents primary-run evidence. The contents become evidence only
when opened as an include-run root.

## Generated Argv Replay

No generated argv changes are made. Existing collect plans still pass the same
retrieved path to `report validate-run --include-run` and
`report operating-contract --include-run`. The resolver now treats that opened
root as the sole owner for the included run's v2 artifact refs.

## Claim Boundary

This change does not:

- turn `workflow.recommendation` or `workflow.collect_plan` into measurement
  evidence
- infer controlled-governor claims from raw primitive artifacts
- relax run validation or operating-contract validation
- change Target55 suitability or production readiness decisions

Invalid refs that truly point outside the opened run set remain invalid.

## Verdict

Submit after focused tests and `make verify` pass. The fix tightens the
workflow-authority evidence boundary by removing a directory co-presence alias
instead of accepting it.
