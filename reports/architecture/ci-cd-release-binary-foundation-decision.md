# Architecture Decision Analysis: CI/CD Release Binary Foundation

## 1. Decision Question

- Deciding: how PR11 should build, package, checksum, attest, and publish
  `adc-lab`, `adc-lab-target`, and `adc-lab-priv-helper` binaries for Pi4/Pi5
  measurement handoffs.
- Not deciding: Pi4/Pi5 suitability, target-selection comparison, runtime NFR
  budgets, privileged control behavior, or Option B privilege provider rollout.

## 2. Context And Constraints

- Current state: local `make verify` is the canonical gate; no GitHub Actions
  workflows or binary release path existed in the repository.
- Constraints:
  - Ordinary CI must use `contents: read`.
  - Release publishing may use `contents: write`.
  - Signed provenance needs OIDC/attestation permissions.
  - aarch64 Linux is required for Raspberry Pi 4/5.
  - Release artifacts must not imply target physical-resource evidence.
- Open uncertainties:
  - Actual GitHub Actions/release permissions can only be proven when the
    workflow runs in GitHub.

## 3. Quality Drivers

| Driver | Scenario | Metric / threshold | Verification |
| --- | --- | --- | --- |
| Integrity | Operator downloads a release binary before Pi4/Pi5 measurement | `SHA256SUMS` validates downloaded tarball | release workflow `sha256sum -c SHA256SUMS`; install docs require same command |
| Provenance | Operator needs to know where release assets came from | GitHub artifact attestation exists for release assets | release workflow uses `actions/attest-build-provenance` |
| Least privilege | CI and release jobs use only required token permissions | CI `contents: read`; publish job only has broader release/attestation permissions | workflow review and CI syntax checks |
| Reproducibility | Pi4/Pi5 profiles record binary identity | all three binaries expose JSON `--version`; tarball has `release-manifest.json` | CLI tests and package smoke |
| Scope safety | Release pipeline does not become resource evidence | docs and release notes explicitly block NFR/selection claims | docs review and NFR gate |

## 4. Candidate Options

| Option | Summary | Assumptions |
| --- | --- | --- |
| A | Use workflow artifacts only | Consumers can rely on run-scoped artifacts |
| B | Use Release assets only | Build and publish can happen in one job without handoff |
| C | Use workflow artifacts for job handoff, Release assets for distribution, checksums plus attestations for integrity/provenance | Release job can download build artifacts and publish assets with scoped permissions |

## 5. Risk / Tradeoff Analysis

| Option | Benefits | Risks | Tradeoffs | Sensitivity Points |
| --- | --- | --- | --- | --- |
| A | Simple Actions setup | Run-scoped artifacts expire and are not a stable distribution surface | Weak consumer handoff | Artifact retention policy |
| B | Direct distribution path | Publishing job needs broad responsibility for build and release; harder to isolate permissions | Less job separation | Future multi-target expansion |
| C | Clear handoff/distribution split; least-permission job boundaries; signed provenance | More workflow steps and artifact movement | Slightly more CI complexity for stronger supply-chain evidence | GitHub attestation availability and release permissions |

## 6. Decision

- Chosen direction: Option C.
- Rationale: Pi4/Pi5 measurement needs stable release assets with checksums,
  while CI/CD needs job-level handoff and minimal token permissions. Workflow
  artifacts serve CI handoff; Release assets serve user distribution.
- Rejected options:
  - Option A: not stable enough for measurement handoffs.
  - Option B: conflates build and publish permissions.

## 7. Verification Tasks

- Tests: add CLI `--version` JSON test and release manifest schema fixture.
- Packaging: run local package smoke against host release binaries and inspect
  tarball contents.
- CI: add `.github/workflows/ci.yml` with `make verify`.
- Release: add `.github/workflows/release.yml` with build matrix, tarball
  packaging, `SHA256SUMS`, attestation, and release upload.
- Documentation: install docs require `sha256sum -c SHA256SUMS` and release
  manifest inspection before measurement.
- Claims: NFR gate keeps release artifacts scoped to build/package integrity.

## 8. Handoffs

- observability: release integrity signals are workflow logs, artifacts,
  release assets, `SHA256SUMS`, attestations, and `release-manifest.json`.
- embedded-nfr-design: no new target-local default runtime; release docs block
  physical-resource claims.
- error-handling: packaging and workflow failures must fail before publish when
  binaries/docs/checksums are missing or invalid.
- quality-gate: verify workflow files, docs, schemas, tests, package smoke, and
  no unsupported NFR or target-selection claims.
