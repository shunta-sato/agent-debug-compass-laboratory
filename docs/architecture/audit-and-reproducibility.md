# Audit And Reproducibility

Every evidence-producing controller operation appends an audit event to:

```text
lab/runs/<run_id>/audit.jsonl
```

Audit correlation identifiers:

- `run_id`
- `plan_id`
- `approval_id`
- `lease_id`
- `result_id`
- `event_id`

Audit-less outputs are not valid evidence. Report packing preserves bounded logical artifact references of the form:

```text
artifact://lab/runs/<run_id>/<relative_path>
```

Raw local filesystem paths, paths outside the run directory, and symlink-traversal paths are not valid claim evidence refs. Claims can then be traced back to run context, target fingerprint, tool inventory, factor levels, restore status, and claim decisions without leaking controller-local paths into the evidence contract.
