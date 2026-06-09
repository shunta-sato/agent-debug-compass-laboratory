# Observability Plan

## Live Discovery Evidence

- Instrumentation config/schema paths: none existed before this bootstrap.
- Logging/metrics/tracing library and version/status output: no metrics/tracing dependency in MVP.
- External dependency interface and connection state: local filesystem and fixed `ssh adc-lab-target` command path.
- Existing dashboard/query/log artifact paths: none.

## Operations To Observe

| Operation | Purpose | Boundary |
| --- | --- | --- |
| `inventory` | explain target fingerprint collection | CLI start to artifact write |
| `toolchain.discover` | explain evidence-source availability | CLI start to artifact write |
| `observe` | explain bounded read-only sampling | CLI start to observation artifact |
| `familiarize.read_only` | explain read-only evidence pack generation | CLI start to manifest and pack artifacts |
| `report.claim_trace` | explain claim-boundary artifact generation | CLI start to claim trace artifact |
| `run_manifest.write` | explain run manifest generation | CLI start to manifest artifact |
| `report.pack` | explain familiarization pack generation | CLI start to pack artifact |
| `tool.qualify_inventory` | explain discovered tool evidence acceptance/rejection | CLI start to qualification summary artifact |
| `control.plan` | explain planned state change | CLI start to plan artifact |
| `control.approve` | explain operator approval artifact generation | CLI start to approval artifact |
| `control.apply` | explain privileged apply result, refusal, or restore-after-failure outcome | CLI start to helper/core result |
| `restore` | explain restore result | CLI start to helper/core result |
| `health-check.restore` | explain read-only post-restore target health | restored result to health artifact |
| `load.cpu` | explain bounded load outcome, abort reason, and safety monitor result | CLI start to load result |
| `experiment.run` | explain matrix trial planning/execution | CLI start to experiment artifacts |
| `tool.qualify` | explain whether a tool can be evidence | CLI start to qualification report |

## Correlation Identifiers

- Primary: `run_id`.
- Operation-specific: `plan_id`, `approval_id`, `lease_id`, `result_id`, `event_id`, `matrix_id`, `tool_id`.
- Propagation: identifiers are serialized into JSON artifacts and audit events.

## Logs

MVP uses structured audit JSONL rather than process logs:

- start: represented by plan or run artifact creation where relevant.
- outcome: audit event with `result`.
- failure: CLI error or structured `control_result.status=refused|failed`.

Required fields are present in `lab.audit_event.v1`: operation, identifiers, result, risk tier, policy version, and timestamp. Failure category for privileged refusal is in `lab.control_result.v1.refusal.reason_code`. State-changing apply failures also record `restore_attempted` and `restore_result` in `lab.control_result.v1`.

Tier 2 control audit records the approval artifact ref when an approval is
submitted. The controller copies the approval into the run directory and stores
only the bounded logical ref (`artifact://lab/runs/<run_id>/approvals/...`) in
the audit event.

Controller-generated approval artifacts emit `control.approve`. Successful
controller restores emit a follow-up `health-check.restore` audit event after
the restore result is persisted.

## Metrics And Traces

No metrics or tracing backend is added in MVP to avoid implying production observability or target-local overhead. Run artifacts provide reproducible evidence for lab use.

## Measurement Purpose And Feedback

- Signal: audit event.
- Decision supported: whether an artifact is valid evidence.
- Action owner: operator or agent reviewing the lab run.
- Expected action when degraded: reject audit-less evidence and rerun the bounded operation.
- Counter-metric: restore status and claim decision prevent "successful command" from becoming unsupported claim.
- Failure mode / misleading interpretation: audit proves what was attempted, not that a physical NFR was met.
- Artifact path: `lab/runs/<run_id>/audit.jsonl`.

- Signal: control result restore attempt.
- Decision supported: whether a failed apply left target state requiring operator action.
- Action owner: lab operator.
- Expected action when degraded: inspect failed restore result, run explicit restore or recovery procedure, and block operating-point claims.
- Counter-metric: restore lease status.
- Failure mode / misleading interpretation: restore success after failed apply does not make the apply successful.
- Artifact path: `artifact://lab/runs/<run_id>/plans/<result_id>.result.json`.

- Signal: post-restore health check.
- Decision supported: whether basic read-only inventory and toolchain discovery still work after a successful restore.
- Action owner: lab operator.
- Expected action when degraded: inspect the restore result and target manually before accepting further control experiments.
- Counter-metric: restore `ControlResult.status`; health-check degradation does not rewrite the restore result.
- Failure mode / misleading interpretation: health check `ok` is not proof of thermal, battery, load, or production readiness.
- Artifact path: `artifact://lab/runs/<run_id>/health/restore_health_check.json`.

- Signal: CPU load safety monitor result.
- Decision supported: whether a Tier 1 CPU load completed or stopped due to an
  explicit safety boundary.
- Action owner: lab operator or agent reviewing the run.
- Expected action when degraded: inspect `abort_reason`, lower duration/worker
  count, improve cooling, or rerun with a clearer operator abort procedure.
- Counter-metric: `safety_monitor.thermal_surface_available` and
  `safety_monitor.operator_abort_observed`; missing thermal surface blocks
  thermal safety claims.
- Failure mode / misleading interpretation: a completed bounded load is not a
  sustained thermal, battery, flash, latency, or production readiness claim.
- Artifact path: `artifact://lab/runs/<run_id>/loads/<load_id>.result.json`.

- Signal: experiment claim decision.
- Decision supported: whether matrix artifacts can support a target behavior claim.
- Action owner: lab operator or agent reviewing the run.
- Expected action when degraded: treat non-dry `not_implemented` trials as blocked evidence and wire audited execution before using claims.
- Counter-metric: audit result for `experiment.run`.
- Failure mode / misleading interpretation: planned matrix factors are not observed behavior.
- Artifact path: `artifact://lab/runs/<run_id>/reports/claim_evidence_trace.json`.

- Signal: read-only run manifest.
- Decision supported: whether read-only target familiarization has inventory, toolchain, observation, audit, and claim trace artifacts.
- Action owner: lab operator or agent reviewing the run.
- Expected action when degraded: inspect `data_quality.missing`, rerun missing read-only operations, and keep control/load/production claims blocked.
- Counter-metric: familiarization pack `blocked_claims` and claim trace blocked entries.
- Failure mode / misleading interpretation: a complete read-only pack is still not controlled operating-point, load, battery, flash, or production evidence.
- Artifact path: `artifact://lab/runs/<run_id>/run_manifest.json`.

- Signal: tool qualification summary.
- Decision supported: which discovered tools may be used as evidence sources in the current run.
- Action owner: lab operator or agent reviewing evidence.
- Expected action when degraded: reject claims that depend on unqualified,
  missing, privileged, external, or non-allowlisted load tools and run the
  later qualification/control workflow.
- Counter-metric: `evidence_rejected_tool_ids` and `missing_tool_ids`.
- Failure mode / misleading interpretation: `builtin` means accepted by the
  current allowlist. For `adc-lab-builtin-cpu-load`, it supports bounded load
  result evidence only; it does not prove production overhead or thermal safety.
- Artifact path: `artifact://lab/runs/<run_id>/tools/tool_qualification_summary.json`.
