# ExecPlan: Platform Operating Contract Discovery

## Purpose / Big Picture

Upgrade adc-lab from a bounded experiment runner into a Platform Operating
Contract Discovery laboratory for Raspberry Pi 4 and Raspberry Pi 5.

The output is not a benchmark score. The output is a machine-readable Target
Operating Contract that states what software patterns are allowed by evidence,
which patterns are burst-only, which conditions require degraded mode, and
which claims are blocked until more target evidence exists.

## Scope

In scope:

- New JSON contracts and golden fixtures:
  - `lab.platform_mechanism_inventory.v1`
  - `lab.boundary_probe_plan.v1`
  - `lab.resource_pressure_result.v1`
  - `lab.resource_coupling_report.v1`
  - `lab.target_operating_contract.v1`
- A typed, bounded pressure probe surface covering CPU, thermal, memory,
  storage, network, latency/jitter, and observer pressure.
- Report generation that projects probe artifacts into resource coupling and
  target operating contract artifacts.
- Pi4/Pi5 API/schema support, with target55 Pi4 execution first.
- Target55 run evidence collected through `ssh target55`.
- Documentation, embedded NFR evidence, observer-effect evidence, and final
  `make verify`.

Out of scope:

- Arbitrary shell execution as a public product surface.
- Privileged root shell or uncontrolled hard-to-restore target changes.
- Perfect long-duration 5/15/30 minute soak proof in the first implementation.
  Existing default load policy has a 300s ceiling; longer claims remain
  evidence-needed, not `unsupported_by_adc_lab`.
- Pi5 live execution in this turn unless a reachable Pi5 target appears.
- Android-specific LMK or other OS-specific degradation as core behavior.

## Constraints / Quality Targets

- Preserve the North Star: no Agent root shell, no uncontrolled experiment, no
  unapproved hard-to-restore operation, no unqualified tool evidence, no claim
  without audit.
- `unsupported_by_adc_lab` is not an allowed final status. Every required
  operating-contract area must end as `measured`, `measured_partial`,
  `not_controllable`, `unsafe_to_run_with_reason`, or
  `not_applicable_with_reason`, or `insufficient` with structured reason and
  next evidence. `insufficient` must not be a silent replacement for
  `unsupported_by_adc_lab`.
- All probes must be typed, bounded, audited, and restorable/cleanup-aware.
- Target-local writes must be temporary, bounded, and cleaned up.
- New SSH runner execution must avoid overwriting the existing target55 runner
  when possible. If a staged runner is needed, use a fixed adc-lab-target path
  under an allowlisted adc-lab-owned directory.
- Hardware-free `make verify` must remain green.

## Context & Orientation

Relevant files:

- `crates/adc-lab-core/src/contracts.rs`: serialized DTOs.
- `crates/adc-lab-core/src/report.rs`: existing report generators.
- `crates/adc-lab-core/src/observe.rs`: passive CPU/freq/thermal/memory
  samples.
- `crates/adc-lab-core/src/load.rs`: bounded CPU load and thermal monitor.
- `crates/adc-lab-core/src/target.rs`: target parsing and SSH runner allowlist.
- `crates/adc-lab/src/main.rs`: public CLI, run-dir artifact writes, audit.
- `crates/adc-lab-target/src/main.rs`: fixed-command target runner.
- `schemas/` and `tests/golden/`: strict minimal schema validation.
- `COMMANDS.md`: canonical command registry; final gate is `make verify`.

Discovery notes:

- The repository is a Rust workspace.
- `plans/_template_execplan.md` is absent; this plan follows `PLANS.md`
  required sections directly.
- Existing target55 is Raspberry Pi 4 Model B Rev 1.5 on Debian 13, aarch64.
- Existing target55 runner is `/home/satoshun/.local/bin/adc-lab-target` version
  `0.1.11`.
- Current experiment execution only runs `cpu_load_workers`; other controlled
  factors are blocked as not supported by that runner.
- Existing CPU load duration policy caps load at 300 seconds.

## Requirements

- REQ-POC-1: When adc-lab discovers a target platform, it shall write a
  platform mechanism inventory that separates visible platform mechanisms,
  platform mechanism control, pressure-probe availability, and current evidence
  status.
  Acceptance: schema fixture validates; target55 generated artifact contains
  compute, thermal, memory, storage, network, scheduler/latency, and observer
  mechanisms without `unsupported_by_adc_lab`.

- REQ-POC-2: When adc-lab plans boundary discovery, it shall write probe plans
  for CPU governor, fixed frequency, thermal, memory pressure, page
  cache/storage coupling, network I/O, latency/jitter, observer effect, and
  recovery boundaries.
  Acceptance: schema fixture validates; each probe has controlled factors,
  observed covariates, uncontrolled confounders, safety/abort condition,
  restore/cleanup, supported claim, and blocked claim fields.

- REQ-POC-3: When adc-lab runs pressure probes, it shall record evidence class,
  pressure intensity, pressure-effect basis, condition metadata, and
  cross-resource side effects, not only per-resource scores.
  Acceptance: target55 run writes `lab.resource_pressure_result.v1` artifacts
  for CPU, thermal, memory, storage, network, latency/jitter, and observer
  pressure with side-effect fields and a non-unsupported status.

- REQ-POC-4: When adc-lab reports an operating contract, it shall derive rules
  from evidence refs and classify remaining gaps as evidence-needed,
  unsafe/not-controllable/not-applicable, or measured partial.
  Acceptance: target55 run writes resource coupling and target operating
  contract reports, with allowed, burst-only, degraded-mode, forbidden, and
  blocked-claim rule categories. Rules must state whether they are generic lab
  rules, measured target-evidence rules, or evidence-needed rules.

- REQ-POC-5: Probe execution shall remain bounded and cleanup-aware.
  Acceptance: tests cover memory/storage/network/latency status decisions and
  cleanup notes; target55 artifact records abort and cleanup fields.

## Dev Workflow Route

- Risk level: high.
- Why: public schema/API expansion plus target-local resource pressure probes,
  sampling, temporary storage writes, network attempts, latency/jitter
  measurement, observer-effect claims, and target55 live execution.
- Triggered branches:
  - requirements-engineering: broad product goal with acceptance criteria.
  - embedded-system-familiarization: Pi4/Pi5 mechanism/envelope discovery.
  - embedded-nfr-design: physical NFR and no-claim boundaries.
  - embedded-hot-path-review: target-local loops and sampling.
  - embedded-observer-effect-review: observation and artifact writing can
    perturb workload.
  - embedded-nfr-harness-design: target smoke commands and measurement harness.
  - function-boundary-governor: new DTOs, probe module, report generators, CLI
    commands, and target runner commands.
  - error-handling: pressure statuses and cleanup/failure contracts.
  - observability: artifacts and audit events are diagnostic signals.
  - embedded-nfr-gate: required before final submit.
  - quality-gate: final gate.
- Explicitly not triggered:
  - bug RCA: no incident/regression is under investigation.
  - architecture-decision-analysis: no competing cross-boundary architecture
    options are being evaluated; the change follows existing CLI/artifact
    patterns.
  - destructive-refactor: no flawed abstraction replacement.
  - concurrency/thread-safety: no shared mutable concurrent API is introduced;
    existing CPU load threading remains unchanged.
  - UI/C++/Android/ROS2/staged lowering: not applicable.

## Design

### Contracts

Add the requested contract DTOs and schemas. The schemas avoid
`unsupported_by_adc_lab` entirely and use explicit statuses:

- `measured`
- `measured_partial`
- `not_controllable`
- `unsafe_to_run_with_reason`
- `not_applicable_with_reason`
- `insufficient`

`insufficient` is allowed only for whole contract/report status when evidence
is present but not enough to make a reference contract claim; individual
mechanisms and probes still use explicit non-unsupported classifications.

### Probe Surface

Add `adc-lab pressure run` and `adc-lab-target pressure run`.

The target runner implements bounded local probes:

- `cpu_pressure`: reuses bounded CPU load evidence shape where possible.
- `thermal_pressure`: classifies thermal visibility and uses bounded CPU load
  observations instead of unbounded heat stress.
- `memory_pressure`: bounded anonymous memory allocation with page touching,
  meminfo/vmstat/PSI before and after, and cleanup by process exit.
- `storage_io`: bounded tempfile write/read in a caller-provided or system temp
  directory, diskstats before/after, latency summary, and cleanup verification.
- `network_io`: interface counters and local/default endpoint latency attempt;
  if no bounded endpoint is configured, classify as `not_applicable_with_reason`
  with visible network counters.
- `latency_jitter`: monotonic timing loop with p50/p95/p99/max under the current
  condition.
- `observer_pressure`: compares a bounded baseline loop against observation and
  artifact-write overhead.

The first implementation focuses on bounded smoke evidence rather than long
soak proof.
Pressure result status is intentionally stricter than command success:
memory allocation without reclaim/PSI/fault deltas is allocation smoke and may
be `insufficient`; endpoint-less network probing is counter-only and not a
network boundary measurement.

### Reports

Add `adc-lab report operating-contract` to generate:

- `reports/platform_mechanism_inventory.json`
- `reports/boundary_probe_plan.json`
- `reports/resource_coupling_report.json`
- `reports/target_operating_contract.json`

The report generator reads existing run artifacts. Separate pressure artifacts
are treated as evidence ingredients, not measured coupling. Coupling chains are
`ingredients_only` until a composite or phased scenario records baseline,
pressure-only, paired-pressure, and recovery phases. Missing Pi5 live evidence
does not produce a fake Pi5 contract; it remains a schema/API-supported target
class until executed.

### Target55 Execution

Because the local host and target55 are both `aarch64`, the locally built
`adc-lab-target` can run on target55. To avoid overwriting the installed
v0.1.11 runner, stage the new runner at an adc-lab-owned fixed path and set
`ADC_LAB_TARGET_RUNNER` to that path for this run.

## Validation & Acceptance

Test list:

- Schema fixtures validate for all new contracts.
- New schemas reject `unsupported_by_adc_lab`.
- New schemas reject legacy `control_status`, missing `evidence_class`,
  missing `coupling_evidence_class`, and missing operating-contract
  `rule_source`.
- Core report generator emits all required platform contract artifacts from
  pressure artifacts.
- Pressure result helpers classify network-without-endpoint as
  `not_applicable_with_reason`, not unsupported.
- CLI pressure smoke writes a resource pressure result artifact and audit event.
- CLI report operating-contract writes all contract reports and audit events.
- `make verify` passes.
- Target55 staged run produces all required contract artifacts.

## Milestones

1. Explore current CLI/schema/report/test surfaces and create this ExecPlan.
2. Add DTOs, schemas, golden fixtures, and status taxonomy.
3. Add pressure probe implementation and target-runner command.
4. Add host CLI pressure command and operating-contract report command.
5. Add tests and update docs/NFR/review artifacts.
6. Build/stage target runner, execute target55 Pi4 probes, generate reports.
7. Run `make verify`, quality gate, and update handoff/outcomes.

## Progress (WBS)

- [x] Confirm user scope and target execution permission.
- [x] Read command registry, existing schemas, DTOs, CLI, target runner, report,
      observe, load, experiment, and tests.
- [x] Start Subagent exploration for schema/report, target/probe, and verify
      surfaces.
- [x] Create ExecPlan.
- [x] Confirm verification chain: `make verify` expands to build, format,
      clippy, unit tests, integration tests, contract validation, docs smoke,
      and command smoke.
- [x] Add contract DTOs, schemas, and golden fixtures.
- [x] Add pressure probe core module.
- [x] Add CLI and target-runner pressure commands.
- [x] Add operating-contract report generation.
- [x] Add tests and docs/resource evidence updates.
- [x] Run local verification.
- [x] Stage and execute target55 Pi4 probes.
- [x] Generate target55 operating contract artifacts.
- [x] Run final `make verify`.
- [x] Run quality gate and update handoff/outcomes.
- [x] Address review request: split platform control from pressure-probe
      availability.
- [x] Address review request: add pressure `evidence_class`, `intensity`,
      `pressure_effect`, network mode, and condition metadata.
- [x] Address review request: classify coupling report chains as
      `ingredients_only` and `insufficient` until composite evidence exists.
- [x] Address review request: add operating rule `rule_source` and
      `derivation`.
- [x] Rerun target55 with the review-fix runner and attach artifact zip.
- [x] Rerun final `make verify` after review-fix changes.
- [x] Push updated PR branch.

## Surprises & Discoveries

- `plans/_template_execplan.md` is referenced but absent.
- Existing report pack can under-classify nested control artifacts, so this
  feature should read typed pressure/report artifacts directly.
- Existing runner allowlist permits `adc-lab-target`, system paths, and
  `~/.local/bin/adc-lab-target`, but not versioned adc-lab-owned staging paths.
- The current remote target55 runner is v0.1.11 and does not include the new
  pressure commands.
- `make verify` remains hardware-free by default. Target smoke is separate and
  target-dependent.
- `cargo check --workspace` passed after DTO, pressure core, CLI, target-runner,
  schema, and fixture additions.
- `cargo test --workspace contract_validation -- --nocapture` passed after new
  schemas and unsupported-status rejection tests.
- Target-independent CLI tests for `pressure run` and `report
  operating-contract` passed individually.
- Full CLI integration test (`cargo test -p adc-lab --test cli -- --nocapture`)
  passed with 27 tests.
- Staged target runner at
  `/home/satoshun/.local/share/adc-lab/runners/20260610-platform-contract/adc-lab-target`
  on target55 without overwriting `/home/satoshun/.local/bin/adc-lab-target`
  v0.1.11.
- Target55 live run wrote `lab.resource_pressure_result.v1` artifacts for CPU
  pressure (1/2/4 workers), thermal pressure, memory pressure, storage I/O,
  network I/O, latency/jitter, and observer pressure.
- Target55 live run generated `platform_mechanism_inventory.json`,
  `boundary_probe_plan.json`, `resource_coupling_report.json`, and
  `target_operating_contract.json`; the target contract status is
  `measured_partial` with no `unsupported_by_adc_lab` occurrences.
- Added `docs/targets/target55/system-familiarization.md` with artifact status,
  freshness/revisit conditions, blocked claims, and handoff statuses.
- Final `make verify` passed.
- Quality gate report updated at `reports/quality-gate.md` with
  `Gate decision: submit` and 0 findings.
- Review on 2026-06-10 requested stricter evidence semantics: probe existence
  is not mechanism control; individual pressure artifacts are not coupling
  measurement; target contract rules must distinguish generic, evidence-needed,
  and measured target evidence.
- Schema tests now explicitly reject legacy `control_status`, missing pressure
  evidence class, missing coupling evidence class, and missing rule source.
- Review-fix target55 run generated
  `lab/runs/LAB-RUN-target55-platform-contract-review-20260610`; target
  contract status is `insufficient`, resource coupling report status is
  `insufficient`, and coupling chains are `ingredients_only`.
- Review artifact zip created at
  `/mnt/share/target55-platform-contract-review-20260610.zip` with SHA-256
  `e3099c7fabbfb0481840e3200247d7096fc39b91a8f8d310b37e6d112c32ef30`.
- Review-fix final `make verify` passed after clippy caught and fixed one
  local builder `too_many_arguments` issue.
- Review-fix commit `29728f6` was pushed to PR #15 and the PR body/comment were
  updated with artifact zip and conservative target55 status summary.

## Decision Log

- 2026-06-10: Add a new pressure/operating-contract layer instead of
  overloading the existing experiment matrix. Rationale: the current matrix only
  safely executes CPU worker ladders, while platform operating contract
  discovery needs typed pressure results and cross-resource coupling reports.
- 2026-06-10: Allow fixed adc-lab-owned staged target runner paths rather than
  overwriting the existing target55 runner. Rationale: target execution needs
  the new command surface, but replacing the installed runner is avoidable.
- 2026-06-10: Keep `lab.target_operating_contract.v1` status `insufficient`
  for target55 smoke when pressure effects or composite coupling are not
  measured. Rationale: this avoids making a smoke suite look like a discovered
  Pi4 reference operating contract.
- 2026-06-10: Treat network without endpoint as `counter_only` and
  `not_applicable_with_reason`, not a network I/O boundary measurement.
  Rationale: interface counter visibility and traffic/retry/latency behavior
  are distinct evidence classes.

## Handoff

Current state:

- Uncommitted review-fix changes tighten Platform Operating Contract evidence
  semantics.
- Review-fix target55 Pi4 evidence exists under
  `lab/runs/LAB-RUN-target55-platform-contract-review-20260610` and is ignored
  by git.
- Staged target55 runner exists at
  `/home/satoshun/.local/share/adc-lab/runners/20260610-platform-contract-review/adc-lab-target`.
- Review-fix final verification passed with `make verify`.

Commands run so far:

- `cargo check --workspace`
- `cargo test --workspace contract_validation -- --nocapture`
- `cargo test -p adc-lab --test cli -- --nocapture`
- `cargo build -p adc-lab-target --release`
- target55 pressure suite via `ADC_LAB_TARGET_RUNNER=/home/satoshun/.local/share/adc-lab/runners/20260610-platform-contract/adc-lab-target`
- `make verify`
- Review-fix local checks:
  - `cargo test -p adc-lab-core contract_validation -- --nocapture`
  - `cargo test -p adc-lab --test cli pressure -- --nocapture`
- Review-fix target55 suite via
  `ADC_LAB_TARGET_RUNNER=/home/satoshun/.local/share/adc-lab/runners/20260610-platform-contract-review/adc-lab-target`

Known risks:

- Long soak claims are limited by existing 300s CPU-load policy; longer
  sustained claims must remain evidence-needed, not claimed.
- Pi5 reference execution is still pending.
- Battery/power, wakeup, flash-wear, controlled governor/fixed-frequency, and
  long soak evidence remain blocked/follow-up areas.
- Composite coupling runner is still pending; current coupling report is
  ingredients-only by design.

## Outcomes & Retrospective

- adc-lab now has first-class Platform Operating Contract contracts, bounded
  pressure probes, SSH target-runner wiring, report generation, tests, and docs.
- target55 can produce Pi4 operating-contract artifacts, but the contract
  remains insufficient until pressure effects, network transfer evidence, and
  composite resource coupling are measured.
- No final artifact uses `unsupported_by_adc_lab`; unsupported implementation
  gaps are represented as explicit evidence-needed follow-ups or measured
  partial boundaries.
