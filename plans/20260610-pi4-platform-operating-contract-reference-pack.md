# ExecPlan: Pi4 Platform Operating Contract Reference Pack v1

## Purpose / Big Picture

Upgrade adc-lab from per-run pressure smoke into a Pi4 Platform Operating
Contract reference system.

The target deliverable is not a benchmark score. The deliverable is a
machine-readable Raspberry Pi 4 Platform Operating Contract that aggregates
relevant evidence across runs, measures memory/cache/storage, network,
latency/jitter, observer, recovery, and composite coupling surfaces, and emits
evidence-bound design rules for AI agents operating on Pi4.

## Scope

In scope:

- Multi-run aggregation:
  - `lab.run_set_manifest.v1`
  - `lab.multi_run_operating_contract.v1`
  - `adc-lab report operating-contract --include-run ...`
  - future `--run-set ...` support when a manifest file is available.
- Pack status and operations summary semantics that distinguish:
  - `observational_read_only`
  - `read_only_plus_pressure_probes`
  - `exploratory_pressure_smoke`
  - `controlled_governor_subset`
  - `controlled_operating_point_subset`
  - `composite_coupling_probe`
  - `platform_operating_contract_candidate`
  - `platform_operating_contract_reference`
- Memory/cache/storage pressure ladder and boundary result semantics.
- Latency/jitter profiles under pressure conditions.
- Network bounded-transfer mode and LAN-confounder metadata.
- Composite boundary probes, initially at least:
  - memory -> storage -> jitter
  - CPU -> thermal -> jitter
  - network -> CPU/jitter
- Target operating contract synthesis that separates generic lab rules,
  evidence-needed rules, and measured-target rules.
- Pi4 reference run and `/mnt/share` artifact zip after implementation and
  operator/target prerequisites are clear.
- Privilege UX:
  - `adc-lab privilege doctor --target local`
  - `adc-lab privilege install-plan --target local`
  - `adc-lab privilege uninstall-plan`
  - non-interactive `sudo -n` readiness classification.

Out of scope:

- Pi5 comparison or target-selection decision.
- Android-specific LMK or Jetson/Snapdragon/Mac adapters.
- Production 24h certification.
- Destructive filesystem fill, brownout, or power-loss testing.
- Agent-entered sudo passwords or arbitrary root shells.

## Constraints / Quality Targets

- Preserve adc-lab as a safety-gated experiment laboratory:
  typed operation, bounded experiment, audit, restore, cleanup, and qualified
  evidence.
- No `unsupported_by_adc_lab` as a final state.
- Do not claim Pi4 reference status until composite evidence exists and the
  generator can explain which rules are measured versus generic/evidence-needed.
- Memory/storage/network pressure must be bounded and cleanup-aware.
- Network results must state topology, endpoint identity, and LAN confounding.
- Privileged operations must be non-interactive from the Agent perspective.
  Password prompt means `operator_setup_required`, not a hanging workflow.
- `make verify` remains the final gate.

## Context & Orientation

Relevant files:

- `crates/adc-lab-core/src/contracts.rs`: DTOs and serialized contracts.
- `crates/adc-lab-core/src/platform_contract.rs`: pressure probes and
  operating-contract report synthesis.
- `crates/adc-lab-core/src/privilege.rs`: current provider status model.
- `crates/adc-lab-core/src/control.rs`: helper allowlist and apply/restore.
- `crates/adc-lab/src/main.rs`: public CLI, reports, pressure, privilege.
- `crates/adc-lab-target/src/main.rs`: target runner pressure commands.
- `schemas/` and `tests/golden/`: JSON schemas and fixtures.
- `docs/targets/target55/system-familiarization.md`: current target context.
- `reports/resource/nfr-gate-report.md`: embedded submit gate evidence.

Current facts:

- `main` is at `cf9c054` after the v0.1.16 installer/release workflow fixes.
- `adc-lab pressure run` already supports CPU, thermal, memory, storage,
  network, latency/jitter, and observer pressure surfaces, but current defaults
  are short bounded smoke-level probes.
- Existing operating-contract report is per-run.
- `adc-lab report operating-contract --include-run ...` can aggregate separate
  pressure, control, composite, and network runs into one Pi4 evidence body.
- `adc-lab pressure composite --scenario memory_storage_jitter` creates
  phase-based composite coupling evidence.
- `adc-lab pressure run --kind network_io --network-endpoint ... --network-bytes ...`
  now records endpoint-backed bounded transfer evidence.
- `privilege doctor`, `install-plan`, and `uninstall-plan` are present.
- Current helper path allowlist is centered on fixed adc-lab helper paths; this
  should be reused, not bypassed.

Unknowns / human confirmation before expanding target execution:

- Whether larger 512MiB/1GiB/2GiB/3GiB memory ladder levels are approved for a
  future target55 run, and the abort/cooldown limits to use.
- Whether concurrent storage+jitter and network+jitter runners should be added
  before promoting Pi4 from candidate to reference.

## Dev Workflow Route

- Risk level: high.
- Why: broad schema/API expansion, target-local pressure and composite
  execution, temporary storage writes, network traffic, latency loops,
  observer-effect claims, and privileged helper readiness.
- Triggered branches:
  - `execution-plans`: required for multi-PR/multi-module work.
  - `requirements-engineering`: user supplied broad GOAL and DoD; acceptance is
    documented here.
  - `embedded-system-familiarization`: target behavior and resource coupling
    define the product value.
  - `embedded-target-characterization`: Pi4 evidence surfaces and baselines are
    part of the final reference pack.
  - `embedded-operating-envelope-discovery`: near-boundary and recovery behavior
    are required.
  - `embedded-nfr-design`: no-claim boundaries and physical pressure budgets are
    needed before target execution.
  - `embedded-hot-path-review`: latency/jitter and observer loops are hot paths.
  - `embedded-observer-effect-review`: observer and artifact writes can perturb
    measurements.
  - `embedded-nfr-harness-design`: target reference runner and artifact pack
    are measurement harness work.
  - `function-boundary-governor`: new report/probe/privilege command surfaces.
  - `error-handling`: pressure and privilege readiness statuses must fail
    safely.
  - `observability`: artifacts/audit are diagnostic signals.
  - `embedded-nfr-gate` and `quality-gate`: final gate.
- Explicitly not triggered:
  - destructive refactor: no replacement migration intended.
  - concurrency/thread-safety: composite phases may spawn workload children, but
    no shared library concurrency design is committed until that slice.
  - UI/C++/Android/ROS2/staged lowering: not applicable.

## Design

### PR Slice 1: Aggregation and Privilege Readiness Foundation

Add contracts and CLI/API needed before running a Pi4 reference pack:

- Run-set manifest records a target-class scoped evidence body:
  - run refs
  - pack status
  - operations summary
  - included surfaces
  - blocked evidence
  - artifact refs
- Multi-run operating contract projects one or more run directories into a
  single contract candidate, preserving evidence source per rule.
- `report operating-contract --include-run` merges pressure/control evidence
  from multiple runs and writes multi-run outputs.
- `privilege doctor` reports helper readiness without prompting for password.
- `privilege install-plan` and `uninstall-plan` emit operator steps only; they
  do not install or remove anything.

### PR Slice 2: Boundary Ladders and Profile Artifacts

- Add memory ladder options with stricter safety caps and explicit requested
  bytes per phase.
- Add storage boundary options for cached read/write, fsync write, and
  read-after-memory-pressure.
- Add `lab.latency_jitter_profile.v1` generated from current condition and
  under-pressure probes.
- Add network bounded-transfer result details with endpoint identity and LAN
  confounder metadata.

### PR Slice 3: Composite Boundary Runner

- Add typed composite probe command and result schema.
- Initial phase runner:
  - baseline
  - pressure-only
  - paired/composite pressure
  - latency loop during composite pressure
  - recovery
- Upgrade resource coupling from `ingredients_only` to `composite_measured` only
  when composite result phases exist.

### PR Slice 4: Pi4 Reference Run

- Run `privilege doctor`.
- Stage or verify target runner.
- Execute approved bounded Pi4 probes on target55.
- Generate target operating contract candidate/reference artifacts.
- Package run artifacts into `/mnt/share`.

## Validation & Acceptance

Test list:

- New schemas validate golden fixtures.
- New schemas reject `unsupported_by_adc_lab`.
- `report operating-contract --include-run` accepts more than one run directory.
- Multi-run contract preserves evidence refs from every included run.
- Pack status reflects pressure, governor control, and composite surfaces.
- `privilege doctor` reports `operator_setup_required` rather than blocking when
  `sudo -n` cannot run the helper.
- `privilege install-plan` and `uninstall-plan` do not execute privileged
  commands.
- Composite coupling remains `ingredients_only` unless composite result
  artifacts exist.
- `make verify` passes.
- Target55 reference pack zip exists under `/mnt/share` only after approved
  target execution.

## Milestones

1. Explore current code and create ExecPlan.
2. Implement run-set and multi-run contract schemas/fixtures.
3. Implement report aggregation CLI for `--include-run`.
4. Implement privilege doctor/install-plan/uninstall-plan.
5. Implement boundary/profile schemas and runner extensions.
6. Implement composite runner and coupling upgrade.
7. Execute Pi4 reference run and package artifacts.
8. Run final verification and publish PR(s).

## Progress (WBS)

- [x] User supplied GOAL and DoD.
- [x] Synced `main` and created branch
      `codex/pi4-operating-contract-reference-pack`.
- [x] Spawned parallel explorers for schema/report, pressure/composite runner,
      and privilege UX.
- [x] Read existing platform contract, pressure, CLI, schemas, and privilege
      surfaces.
- [x] Created this ExecPlan.
- [x] Decided first PR slice boundary: aggregation + privilege readiness
      foundation, without live target pressure expansion.
- [x] Implemented `lab.run_set_manifest.v1` and
      `lab.multi_run_operating_contract.v1`.
- [x] Added `report operating-contract --include-run`.
- [x] Implemented `privilege doctor`, `privilege install-plan`, and
      `privilege uninstall-plan`.
- [x] Added schemas, golden fixtures, and regression tests.
- [x] Ran targeted tests:
      `cargo test -p adc-lab-core --test contract_validation`,
      `cargo test -p adc-lab --test cli`, and `cargo test -p adc-lab-core`.
- [x] Ran final `make verify` successfully.
- [x] Merged aggregation + privilege readiness foundation.
- [x] Confirmed target55 release install and helper readiness with
      `privilege doctor --target local --json` returning `status=ready`,
      helper version `0.1.16`, and `sudo_non_interactive_available=true`.
- [x] Created branch `codex/pi4-reference-pack-live-target55` from updated
      `main`.
- [x] Implemented `lab.composite_boundary_result.v1`.
- [x] Added `adc-lab pressure composite` and
      `adc-lab-target pressure composite`.
- [x] Added `memory_storage_jitter` phased composite runner.
- [x] Upgraded `resource_coupling_report` and multi-run aggregation to treat
      composite result artifacts as `composite_measured`.
- [x] Added endpoint-backed bounded transfer for `network_io`; endpoint-only
      without generated bytes remains insufficient.
- [x] Added regression tests for composite coupling and network bounded
      transfer.
- [x] Staged a current `adc-lab-target` runner on target55 under
      `/home/satoshun/.local/share/adc-lab/runners/20260611-composite/adc-lab-target`.
- [x] Ran target55 primary pressure suite:
      `lab/runs/LAB-RUN-target55-pi4-reference-v1-20260611T0030Z`.
- [x] Ran target55 governor-control evidence:
      `lab/runs/LAB-RUN-target55-governor-control-20260611T0030Z`.
- [x] Ran target55 composite memory/storage/jitter evidence:
      `lab/runs/LAB-RUN-target55-composite-memory-storage-jitter-20260611T0140Z`
      (`coupling_evidence_class=composite_measured`, `status=insufficient`
      because memory pressure effect was not observed).
- [x] Ran target55 endpoint-backed network transfer evidence:
      `lab/runs/LAB-RUN-target55-network-bounded-transfer-20260611T0030Z`.
- [x] Generated multi-run Pi4 operating contract candidate with
      `pack_status=platform_operating_contract_candidate` and
      `contract_status=insufficient`.
- [x] Packaged target55 candidate artifacts:
      `/mnt/share/target55-pi4-reference-pack-v1-candidate-20260611-r2.zip`
      (`sha256=e39efaedc405dc637ce2231518f43d6de84172c1f2e4900442f173ee1274f204`).
- [x] Ran final `make verify`: pass.
- [ ] Update handoff and open PR.

## Surprises & Discoveries

- `plans/_template_execplan.md` is referenced by the skill but absent in this
  repository, so this plan follows `PLANS.md` required sections directly.
- The release/installer and SSH operator-abort security fixes were already on
  `main` before this branch was created.
- Non-interactive SSH does not put `/home/satoshun/.local/bin` on target55's
  `PATH`, so live target commands used absolute runner paths.
- The staged runner is a debug build for this branch. It is isolated under the
  adc-lab runner staging directory and does not replace the installed v0.1.16
  release binaries.
- The 128MiB memory phase did not trigger reclaim/PSI/fault deltas on the 8GiB
  Pi4. The composite artifact records a phased scenario, but chain status stays
  insufficient and memory resident budget claims remain blocked.
- Endpoint-backed network transfer generated exactly 1MiB from target55 to the
  controller endpoint and recorded `network_mode=bounded_transfer`.

## Decision Log

- 2026-06-10: Use multiple PR slices instead of trying to force the entire Pi4
  reference pack into one merge. Rationale: target execution and privileged
  helper readiness have operator prerequisites, while aggregation and privilege
  readiness can be implemented and verified hardware-free first.
- 2026-06-10: Treat `privilege doctor` as required before target control runs.
  Rationale: Agent password prompts are unstable and should classify as
  operator setup required.
- 2026-06-10: Keep multi-run aggregation conservative. Rationale: included
  runs form one evidence body, but they do not prove same-condition resource
  coupling; composite coupling still requires a phased/composite result.
- 2026-06-10: Do not auto-promote to `platform_operating_contract_reference`.
  Rationale: reference status needs repeated Pi4 evidence and at least one
  composite-measured coupling chain, not just schema/API support.
- 2026-06-11: Treat this PR as a candidate-pack slice, not full Pi4 reference
  completion. Rationale: composite and bounded network surfaces are now
  measured, but memory pressure effect and same-condition multi-run guarantees
  remain insufficient.
- 2026-06-11: Keep network bounded-transfer selection claims disabled.
  Rationale: 1MiB LAN transfer proves a bounded endpoint path, not production
  upload cadence, retry/backoff safety, packet loss, or target selection.

## Handoff

- Branch: `codex/pi4-reference-pack-live-target55`.
- Status: implementation, target55 candidate run, artifact package, and
  `make verify` completed.
- Next steps:
  1. Open PR for composite boundary and bounded-transfer candidate pack.
  2. Review candidate pack artifacts and decide memory ladder approval bounds.
- Open risks:
  - larger memory ladder remains operator-approval gated.
  - composite memory/storage/jitter is phase-based, not concurrent.
  - network transfer has LAN topology confounders and no retry/backoff or loss
    scenario.
  - `platform_operating_contract_reference` is not claimed.

## Outcomes & Retrospective

This slice narrows the operating-contract gap by adding the first composite
coupling artifact, endpoint-backed network transfer evidence, and a target55
multi-run candidate pack. It intentionally does not claim Pi4 reference
contract completion because the memory ladder and same-condition coupling
coverage remain open.

Verification:

- `make verify`: pass
- log summary: workspace build, fmt check, clippy with `-D warnings`,
  workspace lib/tests, contract validation tests, README/docs existence checks,
  and host-fallback resource smoke all passed.
