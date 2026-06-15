# adc-lab v0.2.3.1 SSH Full-Set Handoff Fix

## Purpose / Big Picture

Fix the v0.2.3 workflow authority product defect exposed by the target55 SSH
full-set rerun. The goal is not to improve measurements or loosen claims. The
goal is to make the generated source-of-truth chain executable and diagnosable:

```text
workflow.recommendation
  -> workflow.collect_plan
  -> report.run_validation
  -> report.operating_contract
  -> report.suitability
  -> report.constraints
```

## Scope

In scope:

- Make SSH `collect plan` pass the same `--include-run` set to
  `report validate-run` and downstream `report operating-contract`.
- Add regression tests that compare generated argv semantics, not only flag
  presence.
- Add SSH target runner / non-interactive PATH guidance to generated Agent and
  collect-plan instructions.
- Improve target-runner version failure diagnostics so a PATH miss is not
  mistaken for an install failure.
- Record release identity handling for `v0.2.3.1`.

Out of scope:

- Target55 install or repair automation.
- Remote privileged apply/restore over SSH.
- Agent root shells, arbitrary remote commands, or arbitrary sysfs writes.
- Production readiness, Pi4/Pi5 selection, 24h sustained safety, or real
  workload performance claims.

## Problem Frame / RCA

Symptoms:

- target55 had `adc-lab`, `adc-lab-target`, and `adc-lab-priv-helper`
  installed at v0.2.3 / git `87253b5044e9`, but non-interactive SSH PATH did
  not include `~/.local/bin`, so `adc-lab-target` was not found when the
  controller used the default runner name.
- SSH collect-plan generated `report validate-run --include-run <target-local>`
  but generated downstream `report operating-contract --validation` without the
  same `--include-run`.

Five Whys:

1. Why did operating-contract validation risk rejecting good SSH full-set
   evidence? The generated operating-contract step did not use the same run set
   as the generated validation step.
2. Why was that missed? Tests asserted presence of `--validation` /
   `--strict-fullset` and `--include-run` separately, not parity across
   downstream consumers.
3. Why did target runner discovery look like install failure? The default
   runner name depends on the remote non-interactive SSH PATH, while the
   installer default is `~/.local/bin`.
4. Why did diagnostics not guide the operator? The SSH version failure wrapped
   stderr without reporting tried runner, default runner, or suggested
   `ADC_LAB_TARGET_RUNNER`.
5. Root cause: workflow-authority tests were step-local and diagnostics did not
   encode the installer default vs SSH PATH boundary.

## Requirements / Acceptance

- For `target=ssh://target55`, generated `run_validation` and
  `operating_contract` steps use identical `--include-run` values.
- For `target=local`, neither step emits synthetic `--include-run`.
- Generated Agent / collect-plan instructions mention `ADC_LAB_TARGET_RUNNER`,
  `adc-lab-target`, `~/.local/bin`, and non-interactive SSH PATH.
- Target runner version failure diagnostics include tried runner, default
  runner, suggested `/home/<target-user>/.local/bin/adc-lab-target`, and the
  non-interactive SSH PATH boundary.
- No automatic install/repair, root shell, arbitrary remote command, or new
  privileged SSH apply/restore surface is introduced.

## Design

Dev Workflow route:

- Risk route: high. The change touches generated workflow authority, target
  runner failure contracts, and SSH handoff safety.
- Triggered branches: ExecPlan, bug-investigation-and-RCA, error-handling,
  implementation-economy, focused regression tests, full `make verify`, and
  quality gate.
- Error-handling headings used: "Error handling" and "Basic templates" for
  translating SSH runner failure into adc-lab domain language at the boundary.

Implementation economy budget:

- Changed files target: <= 7 tracked files.
- Actual before final verification: 11 tracked files. Accepted because the
  release identity note required installer help/test/docs updates in addition
  to workflow, diagnostics, and regression tests.
- New modules target: 0.
- New helpers target: <= 4 small local helpers for include-run extraction,
  SSH guidance, and diagnostic formatting.
- Production lines target: <= 120; test lines target: <= 140.
- Indirection target: no generic workflow executor and no new runner transport.

Release identity decision:

- Keep Cargo workspace package version unchanged; release identity is carried by
  `ADC_LAB_VERSION`, `--version`, and `release-manifest.json`.
- `v0.2.3.1` is accepted by the existing release tag regex, and
  `0.2.3.1` is accepted by `package-release.sh` token validation.
- No `0.2.3+1` fallback is needed unless GitHub release tooling later rejects
  the tag.

Test strategy:

- Add CLI regression assertions for SSH include-run parity and local absence.
- Add generated-instruction guidance assertions.
- Add a fake SSH failure test for action-oriented target-runner diagnostics.
- Use structural collect-plan assertions instead of a raw JSON golden snapshot
  because artifact IDs, timestamps, and run paths are intentionally dynamic.

## Validation & Acceptance

Required commands:

- `cargo test -p adc-lab --test cli collect_plan -- --nocapture`
- `cargo test -p adc-lab --test cli agent_instructions -- --nocapture`
- `cargo test -p adc-lab --test safety_invariants ssh_runner_missing -- --nocapture`
- `cargo test -p adc-lab-core workflow -- --nocapture`
- `cargo test -p adc-lab-core contract_validation -- --nocapture`
- `cargo test -p adc-lab --test cli report_operating_contract -- --nocapture`
- `make docs-smoke`
- `make schemas-check`
- `make verify`

## Progress (WBS)

- [x] Read v0.2.3.1 GOAL and confirm scope.
- [x] Sync from `origin/main` after PR #61 merge and create branch
      `codex/v0231-ssh-fullset-handoff`.
- [x] Confirm release identity path for `v0.2.3.1`.
- [x] Add regression tests for collect-plan run-set parity and diagnostics.
- [x] Implement include-run parity and SSH runner guidance.
- [x] Improve target-runner diagnostics.
- [x] Run focused and full verification.
- [x] Commit, push, and open PR.

## Design -> WBS Coverage Check

| Deliverable | WBS coverage |
|---|---|
| SSH include-run parity | regression tests + implementation |
| SSH runner/PATH guidance | generated instruction tests + implementation |
| target-runner diagnostic | fake SSH failure test + implementation |
| release identity note | completed in Design |
| RCA/prevention | Problem Frame / RCA + tests |

## Surprises & Discoveries

- 2026-06-15: Release tooling already accepts `v0.2.3.1` as tag and
  `0.2.3.1` as manifest/binary version via `ADC_LAB_VERSION`; Cargo package
  version does not need four-component semver.
- 2026-06-15: Existing docs already documented `ADC_LAB_TARGET_RUNNER`, but
  not the release-installer `~/.local/bin` vs non-interactive SSH PATH failure
  mode. Public docs need that boundary so the generated guidance is reinforced
  outside the artifact.

## Decision Log

- 2026-06-15: Fix generated argv parity in `workflow.rs` rather than relaxing
  operating-contract validation. Rationale: validation must keep rejecting
  foreign run-set artifacts.
- 2026-06-15: Keep runner discovery as guidance/diagnostic, not auto repair.
  Rationale: target install and PATH changes are operator-controlled.
- 2026-06-15: Accept 11 tracked-file touch instead of the initial <=7 target.
  Rationale: release-tag clarity and public SSH PATH guidance require docs,
  installer help, and contract-test updates; no new modules or new transports
  were introduced.
- 2026-06-15: Do not add a raw collect-plan golden fixture in this PR. Rationale:
  the stable contract is argv parity and generated guidance; dynamic IDs,
  timestamps, and absolute temp paths make raw snapshots brittle.

## Handoff

Current branch: `codex/v0231-ssh-fullset-handoff`.

Current PR: #62.

Reviewed implementation commit:
`1a048a34bfef406dce3f29e5fe63d9f8e10a6849`.

Latest PR head at this review checkpoint:
`6d918a4d81511ce3e215aaf7b8ceb87b032ab5f0`.

Status:
Ready for review, draft=false, mergeable=true, CI success on the latest reviewed
head.

Next steps:

1. Merge after final review.
2. Tag/release v0.2.3.1 if release gate remains green.
3. Re-run target55 workflow-authority prompt with `ADC_LAB_TARGET_RUNNER`
   set as needed.

## Outcomes & Retrospective

Focused verification so far:

- `cargo fmt --all -- --check`: pass.
- `cargo test -p adc-lab --test cli collect_plan -- --nocapture`: pass
  (2 passed).
- `cargo test -p adc-lab --test cli agent_instructions -- --nocapture`: pass
  (1 passed).
- `cargo test -p adc-lab --test safety_invariants ssh_runner_missing -- --nocapture`:
  pass (1 passed).
- `cargo test -p adc-lab-core workflow -- --nocapture`: pass.
- `cargo test -p adc-lab-core contract_validation -- --nocapture`: pass.
- `cargo test -p adc-lab --test cli report_operating_contract -- --nocapture`:
  pass (5 passed).
- `make docs-smoke`: pass.
- `make schemas-check`: pass.
- `make verify`: pass.

Prevention added:

- collect-plan tests compare the exact `--include-run` values used by
  validation and operating-contract steps for SSH targets.
- local collect-plan test asserts no synthetic `--include-run` appears.
- fake SSH failure test ensures diagnostics mention tried/default runner,
  suggested `ADC_LAB_TARGET_RUNNER`, installer default, and non-interactive SSH
  PATH.
