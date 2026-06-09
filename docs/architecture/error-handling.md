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
| Experiment matrix execution | unsupported controlled factor, randomized order, failed load, failed observation, or safety-aborted load | trial `status=blocked` or `status=failed`; only successful supported trials become `completed` |
| Operating point coverage report | missing observation/experiment artifacts, blocked trial, failed trial, unsupported factor, or unsafe factor | structured coverage with `observational_only`, `controlled_subset`, `not_controllable`, or `blocked_unsafe`; malformed JSON artifacts are CLI errors |
| Capability cost model report | missing optional inventory/toolchain/coverage/load artifacts, malformed JSON, or absent accelerator/storage/network evidence | missing optional artifacts become `missing_evidence`/blocked claims; malformed JSON artifacts are CLI errors because evidence cannot be trusted |
| Target capability profile report | missing optional run/load/observation artifacts, malformed workload profile, or malformed run artifact | missing optional artifacts become `no_evidence`/`exploratory_partial` with `selection_ready=false`; malformed workload or evidence JSON is a CLI validation error |
| Top-level version command | missing build environment metadata in local/dev builds | JSON still emits required fields; `git_sha` may be `unknown` outside release workflow |
| Release packaging | missing binary, missing required docs/license, invalid metadata token, or checksum command failure | script exits before tarball publication |
| Release workflow metadata | malformed tag or missing build artifact | workflow job fails before release asset upload |
| Release asset verification | checksum mismatch | `sha256sum -c SHA256SUMS` fails; measurement prompt must stop |
| Agent-created adapter qualification | missing evidence flag, unreadable file, malformed JSON, oversized output, invalid sha256, unsafe adapter scope | CLI validation error before evidence acceptance, or `agent_created_unqualified` report when scope is outside PR9 allowlist |
| Privilege provider status | invalid target, artifact write failure, or audit write failure | CLI error with context; planned-disabled Option B is report data, not a failure |

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
- Non-dry experiment matrix output can support claims only for completed trials
  that executed the PR6 allowlist: listed order and optional bounded CPU load
  through `cpu_load_workers`.
- Unsupported controlled factors such as `governor` and fixed frequency are
  recorded as `blocked`, not `completed`.
- Failed or safety-aborted trial steps remain diagnostic evidence and cannot
  support behavior claims.
- Operating point coverage is generated from existing run artifacts. Missing
  optional artifacts lower the coverage status or add blocked points; malformed
  artifacts fail the command because evidence cannot be trusted.
- Observed frequency variation is always kept separate from fixed-frequency
  control coverage.
- Capability cost model generation is generated from existing run artifacts.
  Missing optional evidence blocks or limits architecture claims; malformed
  artifacts fail the command.
- Capability presence is not an architecture recommendation. Offload and
  production physical-footprint claims require qualified, target-specific cost
  evidence.
- Target capability profile generation reads existing run artifacts for a
  supplied workload profile. Missing artifacts become explicit evidence gaps;
  malformed artifacts fail the command because target-selection profiles must
  not be built from corrupted evidence.
- PR11 target capability profiles keep `selection_ready=false`; Pi4/Pi5
  comparison and suitability claims remain blocked even when short-smoke
  artifacts exist.
- PR11 CI/CD release artifacts record binary identity only. `--version`,
  `release-manifest.json`, `SHA256SUMS`, and GitHub attestations are
  build/package integrity evidence, not target physical-resource evidence.
- Same-binary Pi4/Pi5 measurement prompts must stop on checksum failure before
  running target commands.
- Agent-created adapter qualification does not execute adapter commands.
  Provided evidence files are validated and copied into run artifacts. Reports
  store artifact refs, not raw local input paths.
- Complete evidence can qualify only non-state-writing, non-privileged
  observation/probe/report-normalizer/health-check adapters in PR9. Control,
  restore, privileged, state-writing, and load adapters remain unqualified.
- Privilege provider status does not contact or start a provider. Option B
  planned-disabled is represented as structured report state rather than a
  recoverable runtime failure.
- If target physical evidence is unavailable, reports mark claims provisional or blocked.
