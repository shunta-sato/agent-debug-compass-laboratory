# Bug Report (RCA): Release Workflow Input Injection

- Title: Workflow dispatch tag input allowed shell injection before validation.
- Symptom (actual behavior): `.github/workflows/release.yml` interpolated `${{ inputs.tag }}` directly inside Bash `run` blocks as `tag="${{ inputs.tag }}"`.
- Expected behavior: Manual workflow inputs are passed through `env:` and referenced as quoted shell variables, so shell parsing never evaluates the raw GitHub expression expansion as script text.
- Severity/Impact: High. The publish job has `contents: write`, `id-token: write`, and `attestations: write`; command execution there could tamper with release assets, checksums, attestations, or published binaries.
- Environment (versions, platform, config): GitHub Actions release workflow with `workflow_dispatch.inputs.tag` and publish permissions.
- Detection (how it was found): Security review reported direct interpolation of the manual `tag` input in release metadata steps.

## Reproduction

- Steps to reproduce:
  1. Inspect `.github/workflows/release.yml`.
  2. Observe `workflow_dispatch.inputs.tag`.
  3. Observe `tag="${{ inputs.tag }}"` inside both release metadata `run` scripts.
  4. Note that GitHub expression expansion occurs before Bash validation.
- Minimal repro (if available): A tag input containing quote and shell metacharacters can break out of the assignment before the regex check runs.
- Frequency: Any manual run with attacker-controlled `tag` input by a user allowed to run the workflow.

## Evidence

- Logs / stack trace / metrics / traces: Static workflow evidence showed two direct `tag="${{ inputs.tag }}"` assignments in shell `run` blocks.
- What changed recently (if known): The release workflow introduced manual tag input handling for release metadata.

## Root Cause Analysis (Five Whys)

1. Why #1: The release job could evaluate untrusted tag input as shell text because the input was directly interpolated into a Bash script.
2. Why #2: GitHub expressions are expanded before the shell starts, so the regex validation did not protect the assignment itself.
3. Why #3: The workflow mixed trusted workflow expression interpolation and untrusted manual input handling in the same shell string.
4. Why #4: Existing workflow tests checked release behavior and YAML parsing, but did not assert that `workflow_dispatch` inputs are excluded from `run` scripts.
5. Why #5 (root cause): The release workflow lacked a guardrail for the safe GitHub Actions pattern: pass untrusted workflow inputs through `env:` and consume them as quoted shell variables.

## Fix

- What changed (summary): Both release metadata steps now pass `${{ inputs.tag }}` through `env: RELEASE_TAG_INPUT` and read `tag="$RELEASE_TAG_INPUT"`. The publish step also reads the sanitized release output through `env: RELEASE_TAG`.
- Why this fix addresses the root cause: The untrusted input no longer becomes part of the shell script source; Bash receives it as environment data after the script is parsed.

## Verification

- Tests run:
  - `cargo test -p adc-lab-core contract_validation_release_workflow_publishes_checksummed_assets_with_scoped_permissions -- --nocapture`
  - `make verify`
- Repro re-run result: Contract validation now fails if `${{ inputs.* }}` appears inside any parsed workflow `run` script.
- Tooling run (if relevant): Workflow YAML parsing remains covered by the existing schema/contract validation test.

## Prevention

- Prevent: Contract test asserts workflow dispatch inputs are not directly interpolated into `run` scripts.
- Detect: `make verify` runs the contract validation test before release workflow changes can be submitted.
- Mitigate: Publish job keeps scoped permissions and release tag regex validation after env handoff.
- Follow-up tasks (with owners / tracking IDs if available): None; this report documents the prevention test added in the same fix.
