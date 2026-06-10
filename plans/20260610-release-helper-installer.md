# ExecPlan: Release Helper Installer

## Purpose / Big Picture

Add a release-published installer for the adc-lab privileged helper that reduces
target55 setup to a short operator command while preserving the project safety
model: no Agent root shell, no `curl | sudo sh`, fixed helper path, typed helper
only, checksum verification, and explicit sudoers opt-in.

## Scope

In scope:

- Add a release installer script for `/usr/local/libexec/adc-lab-priv-helper`.
- Publish the installer as a GitHub Release asset.
- Include the installer in release `SHA256SUMS`.
- Update README with the target-local install flow and risk boundaries.
- Update release-install docs when needed.
- Add regression tests that pin the release workflow/script safety properties.

Out of scope:

- Solving compromised GitHub Release / maintainer account risk.
- Adding cosign/minisign/attestation verification to the installer.
- Running target55 install from this PR.
- Broad remote privileged apply transport.

## Dev Workflow Route

- Risk level: high.
- Why: this changes the operator path for installing a root-owned helper and
  touches release assets and supply-chain-sensitive documentation.
- Triggered branches:
  - `execution-plans`: multi-file release/security workflow change.
  - `dev-workflow`: mandatory for code/test/docs changes.
  - `quality-gate`: mandatory before submit.
- Explicitly skipped:
  - embedded NFR skills: no target-local runtime/probe behavior changes.
  - concurrency/UI/C++/destructive-refactor: not applicable.

## Design

Installer constraints:

- Must be run as a normal user, not root.
- Must require `--version vX.Y.Z`.
- Must reject unsupported arch/OS.
- Must not accept arbitrary release URL overrides.
- Must download only the versioned tarball and `SHA256SUMS` from the fixed repo
  release URL.
- Must verify the tarball with `sha256sum -c SHA256SUMS --ignore-missing`.
- Must install only `bin/adc-lab-priv-helper` to the fixed helper path.
- Must create sudoers only with explicit `--install-sudoers --user <current-user>`.
- Must validate sudoers with `visudo -cf` before installation.
- Must verify the helper after install, and run `adc-lab privilege doctor` from
  the release tarball when available.

## Test List

- `bash -n scripts/install-adc-lab-helper.sh`
- workflow YAML parses and includes installer in release assets.
- workflow computes `SHA256SUMS` over installer and tarballs.
- installer text does not include `curl | sudo`.
- installer rejects root execution.
- installer validates version and sudoers user.
- installer uses fixed GitHub release URL and fixed helper destination.
- `make verify`

## Progress

- [x] Synced latest `main` before editing.
- [x] Created branch `codex/release-helper-installer`.
- [x] Implemented installer script.
- [x] Updated release workflow.
- [x] Updated README/docs.
- [x] Added tests.
- [x] Ran targeted verification:
      `bash -n scripts/install-adc-lab-helper.sh`,
      `scripts/install-adc-lab-helper.sh --help`,
      `cargo test -p adc-lab-core --test contract_validation`.
- [x] Ran final `make verify` successfully.
- [x] Ran `git diff --check`.
- [ ] Open PR.

## Decision Log

- 2026-06-10: Provide a convenience one-liner that downloads and runs a
  non-root installer, not `curl | sudo sh`. Rationale: convenience matters for
  target55 setup, but root shell over network input violates adc-lab's safety
  model.
- 2026-06-10: Keep compromised-release protection pending. Rationale:
  checksum verification catches transfer/corruption and mismatched assets, but
  a malicious release can alter both script and `SHA256SUMS`; stronger
  attestation/signature verification is a later slice.

## Handoff

Target install is not executed in this PR; the installer is published as a
release asset and documented for operator execution on target55.

Verification:

- `bash -n scripts/install-adc-lab-helper.sh`: pass
- `scripts/install-adc-lab-helper.sh --help`: pass
- `cargo test -p adc-lab-core --test contract_validation`: pass
- `make verify`: pass
- `git diff --check`: pass

Quality gate:

- Gate decision: submit
- Findings: 0
- Notes: compromised-release protection remains explicitly documented as
  pending; no production or fully supply-chain-secure claim is made.
