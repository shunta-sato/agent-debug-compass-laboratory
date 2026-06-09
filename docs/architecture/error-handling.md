# Error Handling

Relevant workflow headings: "13. Error handling" and "13.1 Basic templates".

## Boundary Translation

| Boundary | Low-level failure | Domain result |
| --- | --- | --- |
| Filesystem read/write | `std::io::Error` | `LabError::IoWithPath` with path context |
| JSON/YAML parsing | serde error | `LabError::Json` or `LabError::Yaml` |
| Target string parsing | unsupported scheme, empty endpoint, or SSH endpoint option/shell injection shape | `LabError::InvalidTarget` |
| Duration parsing | invalid duration string | `LabError::InvalidDuration` |
| SSH fixed command | non-zero exit | `LabError::Command` / CLI error with stderr |
| Control approval generation | invalid plan, non-local target, empty approver id, invalid operation summary | CLI error; no approval artifact is created |
| Control policy before state change | unsupported operation, non-local privileged target, missing approval, approval mismatch, invalid governor | structured `ControlResult.status=refused` |
| Control apply/verify after pre-state capture | partial write, verify failure, backend error | structured `ControlResult.status=failed` with `restore_attempted` and `restore_result` |
| Helper path before sudo | controller-internal helper path differs from fixed MVP helper | `LabError::Policy` before invoking `sudo` |
| Restore lease validation | forged policy segment, invalid governor, non-numeric frequency, unsupported operation | structured `ControlResult.status=refused` |
| Restore | backend restore failure or read-back verification failure | structured `ControlResult.status=failed` with restore attempt details |
| CPU load policy | zero duration, duration above 300s, zero workers, or workers above available parallelism | `LabError::Validation` or `LabError::Policy` before load starts |
| CPU load safety monitor | operator abort marker observed or thermal threshold reached | structured `LoadResult.status=aborted` with `abort_reason` |
| CPU load worker execution | worker thread panic or unreadable operator abort marker | CLI error; load result is not trusted as evidence |
| Experiment matrix execution | non-dry execution requested before runner wiring exists | trial `status=not_implemented` and blocked/provisional claims |

## Caller Contract

- CLI commands return non-zero with context for ordinary setup/read failures.
- Privileged control operations return structured JSON refusal where the helper can parse the plan.
- Missing optional target surfaces are represented as unavailable inventory/toolchain/observation fields when safe.
- Unknown fields in control JSON are rejected by `serde(deny_unknown_fields)` and schema `additionalProperties:false`.

## Recovery Rules

- Tier 2 control without approval is refused, not applied.
- Tier 2 approval artifacts are generated from an existing control plan; operation, bounds, target, and digest are not free-form CLI inputs.
- Tier 2 approval must match plan id, canonical plan digest, exact operation, target, risk, restore requirement, and bounds.
- Privileged apply/restore refuses non-`local-target` plans in this MVP; remote privileged apply requires a future target-local helper transport.
- The controller CLI has no public `--helper` override; privileged apply/restore use the fixed MVP helper path.
- Dry-run validates policy and approval shape without writing target state.
- Restore dry-run returns structured `dry_run_ok` without writing target state.
- Restore leases are treated as untrusted input and validated before target writes.
- Restore success requires read-back verification.
- Post-restore health check is diagnostic evidence only and does not rewrite a restore `ControlResult`.
- If apply or verify fails after state capture, restore is attempted immediately and the original operation remains failed even if restore succeeds.
- Tier 1 CPU load is refused before execution when duration or worker bounds
  exceed MVP policy.
- Operator abort stops CPU load as `LoadResult.status=aborted`; the abort file
  path is runtime input only and is not serialized into load artifacts.
- CPU load restore-on-abort status is `not_required` in PR5 because the load
  command does not mutate target state.
- Non-dry experiment matrix output cannot become supported evidence until execution is implemented.
- If target physical evidence is unavailable, reports mark claims provisional or blocked.
