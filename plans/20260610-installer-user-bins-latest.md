# ExecPlan: Installer User Binaries And Latest Release

## Purpose / Big Picture

Improve the release installer so target setup installs the full target-local
tooling (`adc-lab`, `adc-lab-target`, and `adc-lab-priv-helper`) and supports a
convenient latest-release flow without hard-coding local SSH aliases like
`target55` in README commands.

## Scope

In scope:

- Install `adc-lab` and `adc-lab-target` to a user bin directory by default.
- Keep privileged helper install fixed at
  `/usr/local/libexec/adc-lab-priv-helper`.
- Add `--latest` as an alternative to `--version vX.Y.Z`.
- Keep pinned `--version` as the reproducible recommended path.
- Update README/install docs to avoid local `ssh target55` assumptions.
- Add static regression tests for installer safety/UX.

Out of scope:

- Compromised-release protection with signatures/attestations.
- Running installer on target55 in this PR.
- Remote privileged apply transport.

## Dev Workflow Route

- Risk level: high.
- Why: release installer and docs affect operator setup for a root-owned helper.
- Triggered branches:
  - `execution-plans`: multi-file release/security workflow change.
  - `dev-workflow`: mandatory for code/test changes.
  - `quality-gate`: mandatory before submit.
- Explicitly skipped:
  - embedded NFR branches: no target runtime/probe behavior is changed.
  - concurrency/UI/C++/destructive-refactor: not applicable.

## Design

- `--latest` downloads `SHA256SUMS` from
  `/releases/latest/download/SHA256SUMS`, parses the matching
  `adc-lab-v*-<asset-triple>.tar.gz` asset name, then downloads that exact asset
  from the latest release.
- `--version vX.Y.Z` keeps the pinned URL path and known asset name.
- User binaries install to `$HOME/.local/bin` by default.
- `--no-user-bins` can skip user binary installation.
- `--user-bin-dir` can override the user install destination.
- README examples use `<target-host>` and `TARGET_HOST`, not `target55`.

## Test List

- `bash -n scripts/install-adc-lab-helper.sh`
- `scripts/install-adc-lab-helper.sh --help`
- `cargo test -p adc-lab-core --test contract_validation`
- `make verify`
- `git diff --check`

## Progress

- [x] Synced latest `main` before editing.
- [x] Created branch `codex/install-user-bins-latest`.
- [x] Implemented installer changes.
- [x] Updated README/docs.
- [x] Updated tests.
- [x] Ran targeted verification:
      `bash -n scripts/install-adc-lab-helper.sh`,
      `scripts/install-adc-lab-helper.sh --help`,
      installer argument refusal checks,
      `cargo test -p adc-lab-core --test contract_validation`.
- [x] Ran final `make verify` successfully.
- [x] Ran `git diff --check`.
- [ ] Open PR.

## Decision Log

- 2026-06-10: Use `--latest` plus parsed `SHA256SUMS` instead of adding
  versionless tarball copies. Rationale: GitHub supports latest-release asset
  links, while existing tarballs remain versioned and auditable.
- 2026-06-10: Install user binaries by default. Rationale: helper-only install
  leaves old `~/.local/bin/adc-lab` in place, which hides new CLI commands such
  as `privilege doctor`.

## Handoff

Target install is not executed in this PR; the new installer behavior applies
to the next release that includes this change.

Verification:

- `bash -n scripts/install-adc-lab-helper.sh`: pass
- `scripts/install-adc-lab-helper.sh --help`: pass
- `scripts/install-adc-lab-helper.sh --version v0.1.15 --latest`: pass
  expected refusal for mutually exclusive source selection
- `scripts/install-adc-lab-helper.sh --latest --user-bin-dir relative/path`:
  pass expected refusal for non-absolute destination
- `cargo test -p adc-lab-core --test contract_validation`: pass
- `make verify`: pass
- `git diff --check`: pass

Quality gate:

- Gate decision: submit
- Findings: 0
- Notes: `--latest` is documented as convenient but less reproducible than a
  pinned version. Compromised-release protection remains explicitly out of
  scope.
