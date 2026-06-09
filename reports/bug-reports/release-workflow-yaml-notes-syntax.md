# Bug Report: Release Workflow YAML Notes Syntax

## Symptom

The `v0.1.10` release workflow did not publish binary assets. GitHub reported
an invalid workflow file at `.github/workflows/release.yml` line 167.

## Expected Behavior

The release workflow should parse, run, build binary tarballs, publish
`SHA256SUMS`, and upload release assets.

## Evidence

GitHub Actions showed the release notes text as YAML outside the `run: |`
block. Local inspection confirmed lines inside the multi-line shell string were
not indented as workflow block content.

## Root Cause Analysis

1. Why did the workflow not run? GitHub rejected the workflow YAML.
2. Why was the YAML invalid? Release note lines were parsed as workflow YAML
   rather than shell script content.
3. Why did that happen? The shell variable used a multi-line single-quoted
   string, but the continuation lines lacked YAML block indentation.
4. Why was this not caught locally? Contract tests searched for workflow
   strings but did not parse workflow YAML.
5. Root cause: release notes were embedded in a fragile multi-line shell
   variable and workflow syntax was not validated by tests.

## Fix

- Write release notes with a heredoc inside the `run: |` script.
- Pass notes through `gh release create --notes-file release-notes.md`.
- Add contract validation that parses CI and release workflow YAML with
  `serde_yaml`.

## Verification

- `cargo test --workspace contract_validation -- --nocapture`
- `make verify`

## Prevention

Workflow contract tests now parse `.github/workflows/*.yml`, so future YAML
syntax regressions fail before GitHub Actions.
