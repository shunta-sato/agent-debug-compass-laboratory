# ExecPlan: CI/CD Release Binary Foundation

## Purpose / Big Picture

PR11 adds the CI/CD and release-binary foundation required before Pi4/Pi5
measurement runs depend on adc-lab binaries.

The goal is to make `adc-lab`, `adc-lab-target`, and
`adc-lab-priv-helper` available from a single commit/release with checksums,
signed provenance, machine-readable release manifests, and reproducible install
docs. This PR proves build/test/package integrity only; it does not make
resource, NFR, target-selection, or production-readiness claims.

## Scope

In scope:

- GitHub Actions CI workflow that runs the canonical local gate.
- GitHub Actions release workflow for tag/dispatch builds.
- Release tarballs for `linux-aarch64` and `linux-x86_64` asset suffixes,
  built from Rust target triples `aarch64-unknown-linux-gnu` and
  `x86_64-unknown-linux-gnu`.
- `SHA256SUMS` for release assets.
- GitHub artifact attestations for release assets.
- `--version` JSON output for all three binaries.
- `lab.release_manifest.v1` schema and golden fixture.
- Release packaging script usable by the workflow and local smoke tests.
- Install docs for release binaries and Pi5 controller to Pi4 target setup.
- Measurement prompt/docs updated to require release binary and checksum
  verification.

Out of scope:

- Pi4/Pi5 measurement execution.
- Suitability decisions.
- Production/NFR claims.
- New privileged control behavior.
- NOPASSWD sudoers or helper installation by CI.
- macOS, Windows, Jetson, Snapdragon, or Android release targets.

## Constraints / Quality Targets

- CI workflow uses `permissions: contents: read`.
- Release workflow uses the minimum broader permissions needed for release
  assets and provenance: `contents: write`, `id-token: write`, and
  `attestations: write`.
- `make verify` remains the default local gate.
- Release workflow packaging must not imply physical-resource evidence.
- aarch64 Linux is required because Pi4/Pi5 use that release asset.
- Release tarballs include `bin/adc-lab`, `bin/adc-lab-target`,
  `bin/adc-lab-priv-helper`, `README.md`, `LICENSE`,
  `docs/getting-started/pi5-controller-pi4-target.md`, and
  `release-manifest.json`.
- `adc-lab --version`, `adc-lab-target --version`, and
  `adc-lab-priv-helper --version` output JSON with `version`, `git_sha`,
  `target_triple`, and `build_profile`.

## Context & Orientation

Relevant files:

- `Makefile`: canonical local gate.
- `COMMANDS.md`: canonical command map.
- `crates/adc-lab/src/main.rs`: controller CLI entrypoint.
- `crates/adc-lab-target/src/main.rs`: target runner entrypoint.
- `crates/adc-lab-priv-helper/src/main.rs`: privileged helper entrypoint.
- `crates/adc-lab-core/src/contracts.rs`: shared DTOs.
- `docs/getting-started/pi5-controller-pi4-target.md`: current source-build
  install guidance.
- `scripts/resource/run-resource-smoke.sh`: command smoke.

Discovery notes:

- GitHub docs confirm workflow artifacts are for sharing files within or after
  workflow runs, while release assets are the distribution surface.
- GitHub docs describe `GITHUB_TOKEN` as repository-scoped and recommend
  workflow/job permission scoping.
- GitHub artifact attestations provide signed provenance for release assets and
  require OIDC permission.
- `gh` CLI is present locally but not authenticated. Git push and GitHub
  connector PR creation worked for previous PRs.

## Dev Workflow Route

- Risk level: high.
- Why: CI/CD, release artifacts, binary provenance, and install docs affect
  supply-chain trust and future target measurements.
- Triggered branches:
  - `architecture-decision-analysis`: release artifact design and workflow
    permissions are cross-boundary decisions.
  - `function-boundary-governor`: new build-info function, packaging script,
    and CLI pre-parse version boundary.
  - `error-handling`: packaging script, workflow, and version metadata failure
    behavior.
  - `observability`: CI/release signals, release manifests, checksums, and
    attestations.
  - `embedded-nfr-design`: release binaries are used for embedded target runs,
    but this PR must block physical claims.
  - `embedded-nfr-gate`: required because embedded NFR design is triggered.
  - `quality-gate`: final gate.
- Explicitly not triggered:
  - concurrency/thread-safety: no new runtime concurrency.
  - destructive refactor: no replacement migration.
  - bug RCA: no incident under investigation.
  - UI/C++/Android/ROS2/staged lowering/legacy: not applicable.

## Design

### CI Workflow

`.github/workflows/ci.yml` runs on pull requests, pushes to `main`, and manual
dispatch. It uses `permissions: contents: read` and runs `make verify`.

### Release Workflow

`.github/workflows/release.yml` runs on tag pushes matching `v*` and manual
dispatch. It builds:

- `aarch64-unknown-linux-gnu` -> asset suffix `linux-aarch64`.
- `x86_64-unknown-linux-gnu` -> asset suffix `linux-x86_64`.

It uploads workflow artifacts for job-to-job sharing, then publishes release
assets and `SHA256SUMS`. It also emits signed provenance attestations for the
release assets.

### Build Info

`adc-lab-core` owns `BuildInfo` and `build_info(name)`. A build script records:

- git sha from `ADC_LAB_GIT_SHA` or `git rev-parse --short HEAD`.
- target triple from Cargo's build-script `TARGET`.
- build profile from Cargo's build-script `PROFILE`.

Each binary checks for a top-level `--version` argument before Clap parsing and
prints the shared JSON build info.

### Packaging

`scripts/release/package-release.sh` creates the tarball layout and
`release-manifest.json`. The manifest records the release version, git sha,
target triple, and per-binary SHA-256 digests.

### Error Handling

- Missing release binary fails packaging before tarball creation.
- Missing docs/license fails packaging before tarball creation.
- Unknown target in the release workflow fails the job before upload.
- Version metadata fallback is `unknown` only for local builds where git/env
  metadata is unavailable.

### Observability

Release integrity signals:

- CI check result.
- release workflow job logs.
- workflow artifacts.
- release assets.
- `SHA256SUMS`.
- `release-manifest.json` inside tarball.
- GitHub artifact attestations.

### Tests

Test list:

- CLI `--version` JSON works for all three binaries.
- `lab.release_manifest.v1` fixture validates.
- package script creates a tarball with required files and release manifest.
- release manifest records binary checksums.
- CI/release workflow YAML uses expected triggers and permissions.
- `make verify` remains green.

## Milestones

1. Add plan and architecture decision record.
2. Add build-info DTO/build script/version command support.
3. Add release manifest schema, fixture, and packaging script.
4. Add CI/release workflows.
5. Add install docs and measurement prompt docs.
6. Add tests and update quality/NFR artifacts.
7. Run verification and sensitive-data scan.
8. Commit, push, and open draft PR.

## Progress (WBS)

- [x] Sync main and create CI/CD foundation branch.
- [x] Inspect Makefile, binary entrypoints, docs, and current workflow state.
- [x] Route dev workflow and create ExecPlan.
- [x] Add architecture decision record.
- [x] Add build-info/version implementation.
- [x] Add release manifest schema/fixture and packaging script.
- [x] Add CI and release workflows.
- [x] Add install/measurement docs.
- [x] Add tests.
- [x] Update NFR, observability, and boundary reports.
- [x] Update quality report.
- [x] Run verification.
- [x] Run sensitive-data scan.
- [ ] Commit, push, and open draft PR.

## Surprises & Discoveries

- The workload/profile PR was already merged into `origin/main`, so this work
  starts from main and does not mix with the previous branch.
- Local `gh` is installed but not authenticated; GitHub connector PR creation
  remains available.
- Local package smoke initially used an x86_64 asset suffix against an aarch64
  host build. The packaging script now rejects target triple / asset suffix
  mismatches before tarball creation.

## Decision Log

- 2026-06-09: Use workflow artifacts for build handoff and GitHub Release
  assets for user distribution. Rationale: workflow artifacts are run-scoped
  and release assets are the stable binary distribution surface.
- 2026-06-09: Add GitHub artifact attestations in release workflow. Rationale:
  the user asked for signed/checksummed binaries; checksums cover integrity and
  attestations cover signed provenance without introducing a repository-managed
  signing key in this PR.
- 2026-06-09: Use asset suffixes `linux-aarch64` and `linux-x86_64` while
  storing full Rust target triples in `release-manifest.json`. Rationale: the
  user specified those asset names, and the manifest keeps the exact build
  target machine-readable.
- 2026-06-10: Pass manual release tag input through `env:` before shell use.
  Rationale: GitHub expression expansion happens before Bash validation, so
  direct `${{ inputs.tag }}` interpolation inside `run:` scripts can execute
  attacker-controlled shell text before the tag regex runs.

## Validation & Acceptance

Acceptance criteria:

- Pull request CI runs `make verify`.
- Tag push `v*` creates release assets for at least `linux-aarch64-gnu`.
- Release tarball contains all three binaries, README, LICENSE, getting-started
  docs, and `release-manifest.json`.
- Release publishes `SHA256SUMS`.
- All three binaries expose JSON `--version` with version, git sha, target
  triple, and build profile.
- Install docs explain release-binary setup from Pi5 controller to Pi4 target.
- CI permissions remain read-only; release permissions are scoped to release
  and attestation needs.
- Manual release inputs are not directly interpolated into `run:` scripts.
- Release workflow makes no resource/NFR claims.
- `make verify` remains the default local gate.
- Pi4/Pi5 measurement prompt uses release binaries and checksum verification.

## Handoff

- Branch: `codex/pr11-ci-cd-release-binary-foundation`.
- Base: `origin/main` at `c855bd0` after workload/profile merge.
- Current status: implementation and verification complete; uncommitted
  changes exist.
- Verification completed: targeted `--version` test, `make contract`,
  `make build-release`, local package smoke, `git diff --check`,
  `make verify`, and high-confidence sensitive-data scan.
- Next steps: commit, push, and open draft PR.

## Outcomes & Retrospective

PR11 CI/CD release binary foundation is implemented with CI/release workflows,
version JSON output, release manifest contract, checksum packaging, install
docs, measurement prompt updates, and no target/NFR claims.

Security follow-up: release workflow tag input injection was fixed by moving
`workflow_dispatch.inputs.tag` into `RELEASE_TAG_INPUT` env vars and reading
quoted shell variables inside `run:` scripts. Contract validation now parses
workflow run scripts and fails if `${{ inputs.* }}` appears in a shell script.
