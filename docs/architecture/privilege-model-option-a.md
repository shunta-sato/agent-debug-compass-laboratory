# Privilege Model: Option A

MVP privilege model: sudo wrapper helper.

Root-owned install path:

```text
/usr/local/libexec/adc-lab-priv-helper
```

The helper does not accept:

- shell commands
- arbitrary commands
- arbitrary sysfs paths
- arbitrary file writes
- arbitrary script paths

It accepts only typed JSON operations. The MVP allowlist is:

```text
linux.cpufreq.set_governor
```

Tier 2 control requires:

- control plan
- approval artifact generated from the plan
- approval record bound to plan id, canonical plan digest, exact operation, and bounds
- pre-state capture
- restore lease
- verification
- audit event

Control plan `bounds` are authorization and experiment bounds in this MVP. The
helper validates that approval bounds cover the plan before privileged apply.
Runtime duration and thermal-abort enforcement belong to the load or experiment
execution phase; standalone cpufreq apply is not treated as a long-running
timer.

Helper path boundary:

- controller CLI invokes only `/usr/local/libexec/adc-lab-priv-helper`
- public `--helper` override is not exposed in the MVP
- test/dev helper execution uses the helper binary directly, without going through controller `sudo`

MVP target binding:

- `adc-lab control plan --target ssh://...` may create a remote-target plan for review.
- `adc-lab control approve` refuses non-`local-target` plans in this MVP.
- `adc-lab control apply` and `adc-lab restore` refuse any plan or lease whose `target_id` is not `local-target`.
- Remote privileged apply is intentionally deferred until the controller can invoke a target-local helper over an explicit, audited transport.

Failure recovery:

- If apply or verify fails after pre-state capture, helper/core attempts immediate restore.
- The returned `lab.control_result.v1` records `restore_attempted`, `restore_result`, and a restore lease with the resulting restore status.
- When a restore completes successfully through the controller, `adc-lab` records a
  read-only post-restore health-check artifact and audit event. This health
  check is diagnostic evidence only; it does not change the restore result.

The default install keeps normal sudo password prompting. NOPASSWD sudoers is a future lab-machine-only operator decision, not an MVP assumption.
