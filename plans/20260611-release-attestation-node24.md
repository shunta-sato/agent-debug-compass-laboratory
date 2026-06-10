# ExecPlan: Release Attestation And Node24 Workflow Fix

## Purpose / Big Picture

Fix the GitHub Actions release failure where release asset attestation cannot be
persisted, and remove the reported Node.js 20 action deprecation warning from
CI/release workflows.

## Scope

In scope:

- Add the missing release publish permission required by GitHub artifact
  attestations.
- Move the release attestation step to the current `actions/attest` interface.
- Upgrade JavaScript actions in CI/release workflows to Node.js 24-compatible
  versions.
- Extend static workflow regression tests.

Out of scope:

- Re-running GitHub-hosted workflows from this environment.
- Adding release signature verification to the installer.
- Changing release artifact contents.

## Dev Workflow Route

- Risk level: normal.
- Why: CI/release workflow behavior changes, but scope is limited to workflow
  configuration and static tests.
- Triggered branches:
  - `github:gh-fix-ci`: user reported GitHub Actions failure.
  - `dev-workflow`: mandatory for code/test changes.
  - `execution-plans`: workflow/release fix benefits from explicit handoff.
  - `quality-gate`: mandatory before submit.
- Explicitly skipped:
  - embedded NFR branches: no target-local runtime behavior is changed.
  - concurrency/UI/C++/destructive-refactor: not applicable.

## Context & Orientation

- Release workflow publish job had `contents: write`, `id-token: write`, and
  `attestations: write`, but not `artifact-metadata: write`.
- GitHub's current `actions/attest` documentation requires
  `id-token: write`, `attestations: write`, and `artifact-metadata: write` for
  persisted attestations.
- CI and release workflows used Node.js 20 actions (`actions/checkout@v4`,
  artifact upload/download v4, and attest-build-provenance v2).

## Design

- Use `actions/attest@v4` with the existing `subject-path: dist-assets/*` so the
  release assets remain the attestation subjects.
- Add `artifact-metadata: write` only to the publish job, preserving narrower
  permissions for verify/build jobs.
- Upgrade checkout to `actions/checkout@v5`, upload artifact to
  `actions/upload-artifact@v6`, and download artifact to
  `actions/download-artifact@v7`.

## Test List

- `cargo test -p adc-lab-core --test contract_validation`
- `make verify`
- `git diff --check`

## Progress

- [x] Synced latest `origin/main`.
- [x] Created branch `codex/fix-release-attestation-node24`.
- [x] Updated workflow files.
- [x] Updated static workflow tests.
- [x] Ran targeted workflow validation test.
- [x] Ran final `make verify`.
- [x] Ran `git diff --check`.
- [ ] Open PR.

## Surprises & Discoveries

- `gh` logs were not used because this environment did not have authenticated
  Actions log access; the user-provided failure text and local workflow files
  were sufficient to identify the missing permission and deprecated actions.

## Decision Log

- 2026-06-11: Prefer current official action versions over forcing Node 24 by
  environment variable. Rationale: this removes the warning source and avoids
  carrying a temporary compatibility switch.

## Handoff

Current branch: `codex/fix-release-attestation-node24`.

Targeted verification:

- `cargo test -p adc-lab-core --test contract_validation`: pass
- `git diff --check`: pass

Final verification:

- `make verify`: pass

Next step: push a PR.

## Outcomes & Retrospective

Quality gate:

- Gate decision: submit
- Findings: 0
- Notes: GitHub-hosted workflow rerun is not possible from this local
  environment, but the reported failure maps to a missing publish-job
  permission now covered by static regression tests.
