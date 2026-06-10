# Bug Report (RCA): SSH Operator Abort Remote Command Injection

- Title: SSH operator abort file path could be interpreted as remote shell syntax.
- Symptom (actual behavior): The original SSH CPU load path forwarded `--operator-abort-file` as an OpenSSH remote command argument without remote-shell quoting.
- Expected behavior: Every remote command word is shell-quoted before being passed to OpenSSH, so operator-provided paths remain argv data for `adc-lab-target`.
- Severity/Impact: High. A repository user or automation path able to supply an SSH target operator abort path could execute arbitrary commands as the SSH user on the target.
- Environment (versions, platform, config): Controller-side `adc-lab load cpu` and experiment runs targeting `ssh://...`; OpenSSH reconstructs remote command arguments into a command string interpreted by the remote login shell.
- Detection (how it was found): Security review reported that `operator_abort_file` was accepted as CLI input and forwarded through the SSH load path without remote-shell-safe encoding.

## Reproduction

- Steps to reproduce:
  1. Run an SSH CPU load or SSH experiment run with `--operator-abort-file` containing shell metacharacters, for example `x; /usr/bin/id > /tmp/pwned; #`.
  2. Observe that unquoted OpenSSH remote command words are interpreted by `/bin/sh -c` on the target.
  3. Observe that the injected command can execute before or instead of the intended `adc-lab-target` argv handling.
- Minimal repro (if available): The regression tests use a fake `ssh` that logs `$*` and executes `/bin/sh -c "$*"`, matching the relevant OpenSSH remote shell interpretation boundary.
- Frequency: Any vulnerable SSH run where attacker-controlled operator abort path reaches the controller CLI.

## Evidence

- Logs / stack trace / metrics / traces: Existing fixed code routes SSH command words through `remote_shell_quote`; regression tests verify semicolon-bearing abort paths do not create the injected marker file.
- What changed recently (if known): CPU load and experiment commands added operator abort path support for SSH targets.

## Root Cause Analysis (Five Whys)

1. Why #1: The original SSH load path could execute injected shell syntax because OpenSSH remote command arguments are interpreted by the remote shell.
2. Why #2: `std::process::Command` avoided a local shell but did not remove the remote shell boundary created by OpenSSH.
3. Why #3: The operator abort path was treated like a normal process argv value even though it crossed into a remote shell command string.
4. Why #4: Existing validation covered SSH endpoint and runner identity, but not every operator-provided remote command word.
5. Why #5 (root cause): The SSH adapter lacked a single invariant that every remote command word must be shell-quoted before being handed to OpenSSH.

## Fix

- What changed (summary): SSH command construction now quotes remote command words with `remote_shell_quote`, including `--operator-abort-file` and its path value.
- Why this fix addresses the root cause: Shell metacharacters are enclosed inside a single quoted shell word, so the remote shell passes them as argv data to `adc-lab-target`.

## Verification

- Tests run:
  - `cargo test -p adc-lab load_cpu_ssh_operator_abort_file_is_remote_shell_quoted -- --nocapture`
  - `cargo test -p adc-lab experiment_ssh_operator_abort_file_is_remote_shell_quoted -- --nocapture`
  - `make verify`
- Repro re-run result: The fake SSH harness executes `/bin/sh -c "$*"`, and semicolon-bearing abort paths do not create the injection marker.
- Tooling run (if relevant): The tests also assert the fake target receives the exact malicious path in argv logging.

## Prevention

- Prevent: Shared fake SSH regression coverage now exercises both direct `load cpu` and experiment-run SSH operator abort paths.
- Detect: `make verify` runs the CLI tests and catches regressions where the abort path becomes remote shell syntax again.
- Mitigate: SSH runner identity remains allowlisted, and only typed `adc-lab-target` subcommands are exposed.
- Follow-up tasks (with owners / tracking IDs if available): None.
