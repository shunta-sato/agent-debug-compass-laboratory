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

Release binary reproducibility uses a separate package-level manifest:

```text
release-manifest.json
SHA256SUMS
```

`release-manifest.json` records the release version, git sha, target triple,
and per-binary SHA-256 digests. Later lab runs may copy or reference that
release identity in their run manifest or target capability profile, but a
release manifest is not itself target behavior, resource, or NFR evidence.
