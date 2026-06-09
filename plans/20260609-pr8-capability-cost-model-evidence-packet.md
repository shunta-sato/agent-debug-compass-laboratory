# ExecPlan: Capability Cost Model and Architecture Evidence Packet

## Purpose / Big Picture

PR8 makes `lab.capability_cost_model.v1` useful as an architecture evidence
packet. Hardware/software capability presence must not automatically become an
architecture recommendation. The model should say what was observed, what cost
dimensions are known or missing, which architecture options are supported or
blocked, and what evidence is needed next.

## Scope

In scope:

- Expand `lab.capability_cost_model.v1` into structured capability, cost
  dimension, architecture option, and claim-boundary sections.
- Generate the model from existing run artifacts:
  - `inventory/target_inventory.json`
  - `toolchain/toolchain_inventory.json`
  - `reports/operating_point_coverage.json`
  - load result artifacts when present
- Include CPU, memory, thermal, cpufreq, load, GPU, NPU, DSP, storage, and
  network capability evidence states.
- Add architecture option evidence for CPU baseline, GPU offload, accelerator
  offload, and storage-heavy designs.
- Keep production physical-footprint, GPU/NPU offload, and fixed-frequency
  architecture claims blocked unless matching evidence exists.
- Add audit signal for capability-cost report generation.
- Add contract/core/CLI tests and update docs/NFR/gate reports.

Out of scope:

- New target probes.
- GPU/NPU/DSP adapter implementation.
- Benchmarking or choosing the best architecture.
- Production budget calibration.
- Privileged control or cpufreq writes.

## Constraints / Quality Targets

- Default `make verify` remains hardware-free.
- The builder reads existing artifacts only; no new target-local runtime.
- Missing optional artifacts become `missing_evidence` or blocked/provisional
  claims, not runtime errors.
- Malformed JSON artifacts fail the report command because evidence cannot be
  trusted.
- Agent-facing refs remain `artifact://lab/runs/...`.
- Cost model output must not imply "GPU present means GPU offload is better" or
  "CPU load completed means production-safe".

## Dev Workflow Route

- Selected risk route: high.
- Why: schema/contract shape, report behavior, architecture claim semantics,
  and embedded physical evidence boundaries all change.
- Required branches:
  - `execution-plans`: cross-boundary schema/core/CLI/docs/tests work.
  - `function-boundary-governor`: new DTOs and model builder helpers change
    function/API boundaries.
  - `error-handling`: missing vs malformed artifacts and unsupported
    architecture claims need explicit outcomes.
  - `observability`: capability-cost report audit and artifact signals.
  - `embedded-nfr-design`: architecture/resource claims need explicit
    no-measurement-no-claim boundaries.
  - `embedded-nfr-gate`: feature-level embedded claim gate before submit.
- Non-triggered branches:
  - `architecture-decision-analysis`: PR8 does not choose among architecture
    options; it records evidence sufficiency.
  - `concurrency-core`: no concurrency changes.
  - `embedded-hot-path-review`: no target-local hot path added; existing
    report updated with this no-op conclusion.
  - `embedded-observer-effect-review`: no target observer added; existing
    report updated with this no-op conclusion.
  - `embedded-nfr-harness-design`: no new measurement harness.

## Requirements / Acceptance

- EARS-AC1: When target inventory exists, the model shall record observed CPU,
  memory, thermal, and cpufreq capabilities with logical evidence refs.
- EARS-AC2: When completed bounded load artifacts exist, the model shall record
  bounded CPU load response as lab evidence only.
- EARS-AC3: When GPU/NPU/DSP/storage/network evidence is absent, the model
  shall record missing evidence and block architecture claims that depend on it.
- EARS-AC4: Architecture option decisions shall be `supported`, `blocked`, or
  `provisional` and include rationale plus next evidence.
- EARS-AC5: Production physical-footprint and "offload is better" claims shall
  remain blocked without target-specific measurements.
- EARS-AC6: `report operating-point` shall emit a `report.capability_cost`
  audit event when it writes the cost model.

## Context & Orientation

Key files:

- `crates/adc-lab-core/src/contracts.rs`: capability cost DTOs.
- `crates/adc-lab-core/src/report.rs`: artifact inspection and report builders.
- `crates/adc-lab/src/main.rs`: `report operating-point` command wiring.
- `schemas/lab.capability_cost_model.v1.schema.json`: strict schema.
- `tests/golden/lab.capability_cost_model.v1.valid.json`: fixture.
- `crates/adc-lab/tests/cli.rs`: hardware-free CLI tests.
- `docs/architecture/claim-boundaries.md`: user-facing semantics.

## Design

The report builder remains read-only:

```text
run artifacts -> capability evidence -> cost dimensions -> architecture option evidence -> blocked/provisional/supported claims
```

Status model:

- `host_fallback_only`: only local/host fallback context is present.
- `target_evidence_partial`: target inventory or target-like run artifacts
  exist, but production cost evidence is incomplete.
- `insufficient_evidence`: no inventory or cost-relevant artifacts exist.
- `cost_model_ready_for_lab_claims`: reserved for a future calibrated model;
  PR8 does not emit it.

Architecture options:

- `cpu_baseline`: supported for bounded lab evidence when CPU inventory exists.
- `gpu_offload`: blocked unless GPU capability and workload cost evidence exist.
- `accelerator_offload`: blocked unless NPU/DSP/GPU evidence exists.
- `storage_heavy_pipeline`: blocked until storage/write/flash evidence exists.

## Test Strategy

- Contract fixture validation for expanded schema.
- Core tests for inventory-derived capabilities and missing accelerator claims.
- CLI tests for capability-cost model after read-only familiarization and after
  bounded matrix execution.
- Negative schema test that legacy string-only `capabilities` shape is rejected.

## Milestones

1. Expand contracts/schema/golden fixture.
2. Implement artifact-driven capability cost builder.
3. Wire audit and tests.
4. Update docs, function-boundary review, and NFR/gate evidence.
5. Run full verification and sensitive scan.
6. Commit, push, and open draft PR.

## Progress (WBS)

- [x] Create PR8 branch from merged PR7 main.
- [x] Route dev workflow and create ExecPlan.
- [x] Expand contracts/schema/golden fixture.
- [x] Implement artifact-driven capability cost builder.
- [x] Add CLI/core tests.
- [x] Update docs and NFR/gate evidence.
- [x] Run full verification and sensitive scan.
- [ ] Commit, push, open draft PR.

## Surprises & Discoveries

- PR7 already writes `capability_cost_model.json` from `report operating-point`,
  but the content is a small target-id heuristic. PR8 can keep the command
  surface and replace the model contents.

## Decision Log

- 2026-06-09: Keep PR8 report-only. It interprets run artifacts and does not
  probe target capabilities directly.
- 2026-06-09: Treat CPU baseline as lab-supported when inventory exists, while
  keeping production and offload architecture claims blocked without measured
  cost evidence.

## Handoff

Branch: `codex/pr8-capability-cost-model-evidence-packet`.

Current status: implementation and verification complete; draft PR creation
next.

Run:

```sh
make verify
```

Read first:

- `crates/adc-lab-core/src/report.rs`
- `crates/adc-lab-core/src/contracts.rs`
- `crates/adc-lab/src/main.rs`
- `schemas/lab.capability_cost_model.v1.schema.json`

## Outcomes & Retrospective

- `lab.capability_cost_model.v1` now contains structured capability evidence,
  cost dimensions, architecture options, blocked claims, limitations, and
  logical evidence refs.
- `report operating-point` writes both operating-point coverage and capability
  cost reports and emits `report.capability_cost` audit.
- Inventory-derived CPU/memory/thermal/cpufreq evidence and bounded-load partial
  lab evidence are represented, while GPU/NPU/DSP/storage/network and
  production claims remain blocked without qualified evidence.
- Verification passed:
  - `cargo test -p adc-lab-core capability_cost -- --nocapture`
  - `cargo test -p adc-lab --test cli report_operating_point -- --nocapture`
  - `make contract`
  - `make verify`
  - `git diff --check`
  - high-confidence secret/PII/security scan over PR diff and PR8 ExecPlan
