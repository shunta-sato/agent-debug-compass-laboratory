# Observability Plan

## Live Discovery Evidence

- Instrumentation config/schema paths: none existed before this bootstrap.
- Logging/metrics/tracing library and version/status output: no metrics/tracing dependency in MVP.
- External dependency interface and connection state: local filesystem and fixed `ssh adc-lab-target` command path.
- CI/CD external interface and connection state: GitHub Actions workflow files
  under `.github/workflows/`; Release asset publication uses `GITHUB_TOKEN`
  inside GitHub Actions, not local credentials.
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
| `tool.version` | explain controller or target runner binary identity capture | CLI start to version artifact |
| `tool.qualify_inventory` | explain discovered tool evidence acceptance/rejection | CLI start to qualification summary artifact |
| `control.plan` | explain planned state change | CLI start to plan artifact |
| `control.approve` | explain operator approval artifact generation | CLI start to approval artifact |
| `control.apply` | explain privileged apply result, refusal, or restore-after-failure outcome | CLI start to helper/core result |
| `restore` | explain restore result | CLI start to helper/core result |
| `health-check.restore` | explain read-only post-restore target health | restored result to health artifact |
| `load.cpu` | explain bounded load outcome, abort reason, and safety monitor result | CLI start to load result |
| `experiment.trial` | explain one matrix trial outcome and per-trial evidence | trial start to trial artifact refs |
| `experiment.run` | explain matrix planning/execution summary | CLI start to experiment artifacts |
| `report.operating_point` | explain observed vs controlled operating-point coverage | CLI start to coverage artifact |
| `report.target_operating_contract` | explain rule-set evaluation and blocked claims | CLI start to v2 operating-contract artifact |
| `tool.qualify` | explain whether an agent-created tool can be evidence | CLI start to evidence artifact copy and qualification report |
| `privilege.provider_status` | explain which privilege provider is active or disabled | CLI start to provider status artifact |
| `release.package` | explain binary release identity and package integrity | release workflow build to tarball, checksum, attestation, and Release asset |

## Correlation Identifiers

- Primary: `run_id`.
- Operation-specific: `plan_id`, `approval_id`, `lease_id`, `result_id`, `event_id`, `matrix_id`, `trial_id`, `tool_id`.
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
- Expected action when degraded: inspect trial `failure`, trial artifact refs,
  and `experiment.trial` audit events before rerunning or broadening the matrix.
- Counter-metric: per-trial status prevents a partially blocked or failed matrix
  from becoming a broad supported claim.
- Failure mode / misleading interpretation: completed `cpu_load_workers` trials
  do not prove privileged governor, fixed-frequency, thermal safety, or
  production behavior.
- Artifact path: `artifact://lab/runs/<run_id>/reports/claim_evidence_trace.json`.

- Signal: experiment trial artifact refs.
- Decision supported: which observation/load artifacts justify each completed
  trial.
- Action owner: lab operator or agent reviewing the run.
- Expected action when degraded: reject claims from trials without bounded
  artifact refs or with `status=blocked|failed`.
- Counter-metric: top-level `experiment.run` audit result summarizes completed,
  blocked, and failed trial outcomes.
- Failure mode / misleading interpretation: per-trial artifacts prove only the
  allowlisted non-privileged steps that were executed.
- Artifact path:
  `artifact://lab/runs/<run_id>/experiments/trials/<trial_id>/...`.

- Signal: operating point coverage report.
- Decision supported: whether a run is observational-only, a controlled subset,
  fully controlled, not controllable, or safety-blocked.
- Action owner: lab operator or agent reviewing claims.
- Expected action when degraded: treat blocked points as unsupported and collect
  the listed next evidence before broadening claims.
- Counter-metric: `blocked_points` and `claim_boundaries` prevent observed
  frequency movement from being interpreted as a controlled sweep.
- Failure mode / misleading interpretation: `controlled_subset` for
  `cpu_load_workers` does not prove fixed-frequency or governor behavior.
- Artifact path:
  `artifact://lab/runs/<run_id>/reports/operating_point_coverage.json`.

- Signal: v2 operating contract report.
- Decision supported: whether rule-table predicates support, defer, or block
  named operating claims for the evidence store contents.
- Action owner: lab operator or agent reviewing operating claims.
- Expected action when degraded: inspect rule evaluations, catalog claim IDs,
  and `next_evidence`; collect the listed v2 evidence before repeating the
  report.
- Counter-metric: catalog-backed blocked claims prevent probe existence from
  becoming production or target-selection claims.
- Failure mode / misleading interpretation: a v2 operating contract is not a
  benchmark score or target ranking.
- Artifact path:
  `artifact://lab/runs/<run_id>/reports/target_operating_contract.v2.json`.

- Signal: read-only run manifest.
- Decision supported: whether target familiarization has inventory, toolchain,
  observation, optional bounded-load, audit, release identity, and claim trace
  artifacts consistent with one logical run.
- Action owner: lab operator or agent reviewing the run.
- Expected action when degraded: inspect `operations_summary`,
  `data_quality.missing`, and `data_quality.inconsistent`; rerun missing or
  inconsistent operations before creating suitability or operating-contract
  claims.
- Counter-metric: familiarization pack `pack_status`, claim trace blocked
  entries, and target capability profile `selection_ready=false`.
- Failure mode / misleading interpretation: an exploratory short-smoke pack is
  not controlled operating-point, battery, flash, sustained thermal, or
  production evidence.
- Artifact path: `artifact://lab/runs/<run_id>/run_manifest.json`.

- Signal: run context artifact.
- Decision supported: whether repeated commands using the same `--run-dir`
  share one logical `run_id`.
- Action owner: lab operator or agent reviewing audit completeness.
- Expected action when degraded: reject or regenerate packs whose
  `audit.jsonl` contains events with a different `run_id` than
  `run_manifest.run_id`.
- Counter-metric: `run_manifest.data_quality.inconsistent`.
- Failure mode / misleading interpretation: directory path equality alone does
  not prove run identity; the logical `run_id` must match.
- Artifact path: `artifact://lab/runs/<run_id>/run_context.json`.

- Signal: operation summary.
- Decision supported: which operations actually ran in the evidence pack:
  inventory, toolchain discovery, passive observe, bounded load, privileged
  control, controlled operating point, and sustained thermal.
- Action owner: lab operator or agent preparing target capability profiles.
- Expected action when degraded: keep unsupported claims blocked when an
  operation is `not_run`, and rerun report pack after adding artifacts to a
  run directory.
- Counter-metric: claim trace supported/blocked entries must be generated from
  the same operation summary.
- Failure mode / misleading interpretation: artifact presence without matching
  operation status and audit evidence is not enough for formal comparison.
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

- Signal: agent-created tool qualification report.
- Decision supported: whether a proposed adapter is accepted as a bounded
  evidence source or remains unqualified.
- Action owner: lab operator or agent reviewing adapter output.
- Expected action when degraded: inspect missing checks, collect dry-run/manual
  comparison/output-schema/static-safety-review/version/hash evidence, or keep
  the adapter out of evidence claims.
- Counter-metric: `evidence_accepted=false`, `qualification_scope`, and
  `limitations` prevent manifest-only or unsafe-scope tools from becoming
  evidence.
- Failure mode / misleading interpretation: `qualified` is scoped to the
  declared non-privileged adapter output; it is not permission to run arbitrary
  shell, perform control, or make production physical-footprint claims.
- Artifact path:
  `artifact://lab/runs/<run_id>/tools/<tool_id>.qualification.json`.

- Signal: privilege provider status.
- Decision supported: whether a run used the active Option A provider posture
  and whether Option B was merely planned-disabled.
- Action owner: lab operator or agent reviewing privileged-control readiness.
- Expected action when degraded: keep remote privileged apply and Option B
  claims blocked until a later provider implementation, hardening, audit, and
  physical-footprint evidence exist.
- Counter-metric: `providers[].default_enabled=false` and empty
  `operations_allowed` for Option B prevent planned design from becoming an
  active transport claim.
- Failure mode / misleading interpretation: provider status does not prove the
  helper is installed, root-owned, or safe to use; it reports current adc-lab
  policy posture only.
- Artifact path:
  `artifact://lab/runs/<run_id>/privilege/privilege_provider_status.json`.

- Signal: release binary identity.
- Decision supported: whether a Pi4/Pi5 measurement used a specific adc-lab
  version, git sha, target triple, and build profile.
- Action owner: lab operator or agent preparing target capability evidence.
- Expected action when degraded: reject same-binary comparison claims and rerun
  with a verified GitHub Release asset.
- Counter-metric: `SHA256SUMS`, GitHub artifact attestation, and
  `release-manifest.json` prevent local source builds from being mistaken for
  release binaries.
- Failure mode / misleading interpretation: a release binary proves
  build/package identity only; it does not prove resource overhead, target
  suitability, battery safety, thermal safety, or production readiness.
- Artifact path: GitHub Release asset plus tarball `release-manifest.json`.
