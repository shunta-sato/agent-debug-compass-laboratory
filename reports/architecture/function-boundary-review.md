# Function Boundary Review: PR11 CI/CD Release Binary Foundation

## Scope

Changed functions/helpers:

- `build_info` in `crates/adc-lab-core/src/build_info.rs`.
- `print_version_if_requested` in each binary entrypoint.
- `current_git_sha` in `crates/adc-lab-core/build.rs`.
- `scripts/release/package-release.sh` packaging functions:
  `validate_token`, `copy_binary`, and `copy_required_file`.
- `BuildInfo`, `ReleaseManifest`, and `ReleaseBinaryManifest` DTOs in
  `crates/adc-lab-core/src/contracts.rs`.

## Semantic Neighbors

| Function / Type | Neighbor classification | Decision |
| --- | --- | --- |
| Existing CLI `print_json` helpers | same output mechanism | keep local; version output reuses each binary's existing JSON printer |
| `AuditEvent` and run manifests | parallel evidence identity | keep separate; build identity is binary/release metadata, not run audit |
| Existing schema DTOs | same contract pattern | add narrow DTOs with `serde(deny_unknown_fields)` |
| `scripts/install-helper.sh` | parallel install script | keep separate; release packaging is distribution-time, helper install is operator-time |

## Decisions

- Add one shared `build_info(name)` core function. It owns stable JSON fields
  and avoids three binaries drifting in version output shape.
- Keep `print_version_if_requested` small and local in each binary. It is
  entrypoint behavior, and extracting a CLI pre-parser would add indirection
  before more flags share the behavior.
- Keep release packaging in a shell script instead of Rust. The workflow needs
  filesystem packaging, tar, and checksum orchestration; a Rust binary would
  add a new distribution surface before it is justified.
- Keep release manifest DTOs in `contracts.rs` because they are Agent-facing
  contract data and have matching JSON schema/golden fixtures.
- Ledger update not required: no abstraction was replaced, no staged adapter
  remains, and local duplicated pre-parse functions are intentionally tiny.

## Boundary Decisions

| Boundary | Action | Rationale |
| --- | --- | --- |
| `build_info(name)` | keep | single owner for version/git/target/profile JSON fields |
| Binary pre-parse version handler | keep local | avoids broader CLI abstraction while preserving top-level `--version` |
| Build script metadata capture | keep | Cargo build script is the right boundary for target/profile/git env |
| Release packaging shell functions | keep | packaging side effects are script-local and fail before tarball publication |
| Release manifest DTOs | keep | schema-backed contract for release provenance inside tarballs |

## Error Behavior

- If git metadata is unavailable during local builds, `git_sha` becomes
  `unknown`; release workflow sets `ADC_LAB_GIT_SHA` explicitly.
- Package script fails before tarball creation when a required binary or
  release file is missing.
- Package script rejects unexpected metadata token shapes before writing a
  manifest.
- GitHub release workflow validates tag shape before build or publish.

## Verification

Planned and/or required commands:

- `cargo test -p adc-lab --test cli version_commands_emit_build_info_json -- --nocapture`
- `make contract`
- local release package smoke after `make build-release`
- `make verify`

Final command results are recorded in `reports/quality-gate.md`.
