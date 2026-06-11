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

When a caller supplies `--run-dir`, adc-lab persists the logical run identity
in:

```text
lab/runs/<run_id>/run_context.json
```

Subsequent commands that reuse that directory read the stored identity instead
of deriving a new run id from process state or directory name. Report packing
checks `audit.jsonl` and records a data-quality inconsistency when an audit
event's `run_id` differs from `run_manifest.run_id`.

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
release identity in their run manifest or suitability artifacts, but a release
manifest is not itself target behavior, resource, or NFR evidence.

Run manifests copy the relevant binary identity into the evidence pack:

- `adc_lab_version` and `adc_lab_git_sha`
- `adc_lab_target_version` and `adc_lab_target_git_sha`
- release tag, release asset name, and release asset SHA-256 when available
- per-binary SHA-256 entries for `adc-lab` and `adc-lab-target`

Missing release checksums or target runner version artifacts are not warnings
outside the contract; they are recorded in `data_quality.missing` or
`data_quality.inconsistent` so later suitability artifacts can block
formal comparison.
