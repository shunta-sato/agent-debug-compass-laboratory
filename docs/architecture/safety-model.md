# Safety Model

Risk tiers:

- Tier 0: read-only observation.
- Tier 1: low-risk reversible or non-root bounded activity.
- Tier 2: privileged reversible control with approval and restore.
- Tier 3: hard-to-restore or degradation-inducing experiments.
- Tier 4: normally prohibited operations.

MVP behavior:

- Tier 0 inventory, toolchain, and observation are audited.
- Tier 1 CPU load is bounded by worker count, duration, optional thermal abort,
  and optional operator abort.
- Tier 1 CPU load default policy caps duration at 300s and worker count at available parallelism.
- Tier 1 CPU load records a safety monitor summary in `lab.load_result.v1`,
  including monitor samples, thermal surface availability, operator abort
  observation, and restore-on-abort status.
- Tier 1 CPU load does not mutate target state in this MVP, so
  restore-on-abort status is `not_required`.
- Tier 1 experiment matrix execution is limited to listed-order trials over
  the allowlisted `cpu_load_workers` controlled factor. Each completed trial
  must produce per-trial artifact refs and an `experiment.trial` audit event.
- Tier 1 experiment matrix execution blocks unsupported controlled factors,
  randomized order, trial explosion beyond policy, or failed load/observe
  steps instead of treating the trial as completed evidence.
- Tier 2 cpufreq governor control uses the privileged helper and restore lease, but apply/restore is local-target only in this MVP.
- PR10 privilege provider status is Tier 0 read-only reporting. It records the
  fixed Option A helper as active and the future Option B systemd/Unix-socket
  provider as planned-disabled; it does not enable a daemon, socket, or new
  privileged transport.
- PR11 target capability profile reporting is Tier 0 read-only reporting. It
  reads existing run artifacts for a supplied workload profile and writes a
  controller-side report; it does not execute observe, load, helper apply, SSH,
  or destructive experiments.
- PR11 CI/CD release binary foundation is build/package infrastructure. It
  creates binary identity, checksum, and provenance artifacts only; it is not a
  target operation and cannot support resource/NFR or target-selection claims.
- Tier 2 approval artifacts are generated from a validated control plan and are local-target only in this MVP.
- Tier 2 approvals are bound to plan id, plan digest, exact operation, and bounds.
- Tier 2 control plan bounds are authorization/experiment bounds. The helper
  enforces approval coverage and restore verification; load and future matrix
  execution enforce runtime duration and thermal abort behavior.
- Tier 2 controlled factors such as governor or fixed frequency remain blocked
  in experiment matrices until privileged plan/apply/restore is wired into
  trial execution.
- Operating point coverage reporting is Tier 0. It reads existing artifacts and
  classifies claim boundaries; it does not execute new target operations.
- Target capability profile reporting keeps `selection_ready=false` in PR11.
  Target-selection claims remain blocked until comparison and suitability
  decision contracts are added with matching evidence.
- Coverage statuses are safety-gated: unsupported controlled points become
  `not_controllable`, and degradation-inducing points become `blocked_unsafe`.
- Tier 3 is documentation-only in this MVP.
- Tier 4 is prohibited by normal `adc-lab` approval.
