# Evidence Model

`adc-lab` v2 evidence is written as strict `Artifact<P>` envelopes. The public
envelope uses:

- `schema`: always `lab.artifact.v2`.
- `kind`: a typed evidence or report kind.
- `id`, `run_id`, `target_id`, `status`, `data_quality`, `claims`,
  `time_unix_ms`.
- `payload`: the typed payload for that artifact kind.

Serde representation is intentionally adjacent, not flattened. `kind` stays at
the envelope level and payload fields stay under `payload`, so
`deny_unknown_fields` remains enforceable for generated JSON Schema snapshots.

The v2 store indexes only `lab.artifact.v2` JSON files under explicit run
directories. It refuses symlink paths and does not read or migrate v1 run
artifacts. Fresh v2 demo evidence must be regenerated from probe execution.

Control safety semantics are preserved by payload wrapping. Control payloads
keep the current `ControlResultStatus` vocabulary and refusal structure rather
than mapping `dry_run_ok`, `applied`, `restored`, or `failed` into the generic
evidence `Status`.

Generated schemas live under `schemas/generated/` and are refreshed with:

```sh
make schemas
```
