# Quality Gate Report

Gate decision: submit

Findings: 0

## Checks Run

- `cargo fmt --all --check` - pass.
- `git diff --check` - pass.
- `cargo test -p adc-lab --test cli version_commands_emit_build_info_json -- --nocapture` - pass.
- `cargo test --workspace contract_validation -- --nocapture` - pass.
- `make build-release` - pass.
- Local release package smoke - pass:
  - `scripts/release/package-release.sh ...`
  - `sha256sum -c SHA256SUMS`
  - tarball content inspection for `bin/adc-lab`,
    `bin/adc-lab-target`, `bin/adc-lab-priv-helper`, README, LICENSE,
    getting-started docs, and `release-manifest.json`.
- `make verify` - pass.
- High-confidence sensitive-data scan over changed tracked and untracked files
  - pass. No API keys, passwords, private keys, IP addresses, email addresses,
  personal names, or security-incident indicators were found in PR additions.

## Live Discovery

- GitHub Actions docs: `GITHUB_TOKEN` permissions can be scoped at workflow/job
  level; ordinary CI is kept at `contents: read`, while release publication
  uses the narrower write permissions needed for assets and attestations.
- GitHub workflow artifacts docs: workflow artifacts are used for build
  handoff between jobs, not as the user distribution surface.
- GitHub release assets docs: Release assets are the intended user download
  surface.
- GitHub artifact attestations docs: artifact attestations provide signed
  build provenance for produced release assets.
- Local GitHub CLI status: `gh` is installed but not authenticated locally.
  This does not block adding workflows; release permission can only be fully
  proven by GitHub Actions when the workflow runs.
- Target connection state: no hardware target required. PR11 CI/CD release
  foundation does not contact a target or execute target measurements.

## Triggered Branch Evidence

- ExecPlan - present:
  `plans/20260609-pr11-ci-cd-release-binary-foundation.md`.
- Architecture decision analysis - present:
  `reports/architecture/ci-cd-release-binary-foundation-decision.md`.
- Function boundary review - present:
  `reports/architecture/function-boundary-review.md`.
- Error handling - present:
  `docs/architecture/error-handling.md`.
- Observability - present:
  `docs/architecture/observability-plan.md`.
- Embedded NFR design/gate - present:
  `docs/nfr/adc-lab-target-runtime.md`,
  `requirements/nfr/adc-lab-target-runtime.yaml`,
  `requirements/physical_budgets.yaml`, and
  `reports/resource/nfr-gate-report.md`.
- Hot-path and observer-effect reports - present from prior target-runtime
  gates:
  `reports/resource/hot-path-review.md` and
  `reports/resource/observer-effect-review.md`.

## Exit Criteria Review

- Pull request CI runs `make verify` through `.github/workflows/ci.yml`.
- Release workflow runs `make verify`, builds `aarch64-unknown-linux-gnu` and
  `x86_64-unknown-linux-gnu`, packages tarballs, uploads workflow artifacts,
  publishes GitHub Release assets, emits `SHA256SUMS`, and adds artifact
  attestations.
- CI workflow uses `permissions: contents: read`.
- Release workflow keeps build jobs read-only and gives only the publish job
  `contents: write`, `id-token: write`, and `attestations: write`.
- `adc-lab`, `adc-lab-target`, and `adc-lab-priv-helper` expose top-level
  JSON `--version` output with name, version, git sha, target triple, and build
  profile.
- `lab.release_manifest.v1` has schema and golden fixture coverage.
- Release tarballs contain all three binaries, README, LICENSE, getting-started
  docs, and `release-manifest.json`.
- Install docs require `sha256sum -c SHA256SUMS` before Pi4/Pi5 measurement
  and keep helper install optional.
- Measurement prompt uses release binaries/checksum verification and blocks
  Pi4/Pi5 suitability, battery, sustained thermal, operating-point, and
  production claims.
- `make verify` remains the default local gate.
- PR11 adds no arbitrary shell, no new privileged control surface, no helper
  override, no root daemon, no destructive experiment, and no resource/NFR
  claim.

## Gate Decision

Submit. The change adds CI/CD and release-binary distribution integrity without
turning release artifacts into target measurement evidence.
