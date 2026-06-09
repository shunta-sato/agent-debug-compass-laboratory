# Architecture Decision Analysis: Workload And Target Capability Profiles

## 1. Decision Question

- Deciding: how PR11 should introduce evidence contracts for workload
  requirements and target demonstrated capability.
- Not deciding: Pi4 vs Pi5 comparison, suitability decisions, production
  readiness, sustained benchmark methodology, or new privileged/target-local
  execution behavior.

## 2. Context And Constraints

- Current state: adc-lab can produce read-only familiarization packs,
  tool-qualification reports, bounded non-privileged load artifacts,
  operating-point coverage, and a capability-cost model.
- Constraints:
  - No Agent root shell.
  - No new privileged control or destructive experiment in PR11.
  - Comparison and suitability decisions must remain future PR work.
  - Evidence refs must remain bounded `artifact://lab/runs/...` refs.
  - Hardware-free `make verify` remains the default gate.
- Open uncertainties:
  - Pi5 same-suite evidence is not present in this repository yet.
  - Sustained thermal, wakeup, battery/power, flash/storage, and jitter evidence
    are still missing.
  - Final target-comparison and suitability-decision schemas will arrive later.

## 3. Quality Drivers

| Driver | Scenario | Metric / threshold | Verification |
| --- | --- | --- | --- |
| Contract clarity | Agent asks what workload a target capability claim refers to | every profile has a non-empty `workload_id` linked to a workload profile | schema, fixtures, CLI test |
| Claim safety | Short smoke evidence is reviewed for selection claims | generated profiles keep `selection_ready=false` and block target-selection/production claims | core and CLI tests |
| Reproducibility | Operator reviews a target profile artifact | evidence refs use `artifact://lab/runs/...`; no raw run path in Agent-facing refs | CLI test |
| Maintainability | PR13/PR14 add comparison and suitability later | workload/profile contracts are separate from cost model and comparison | function-boundary review |
| Physical footprint | PR11 report generation runs on the controller only | no target command, helper, load, or daemon added by profile generation | NFR gate and diff review |

## 4. Candidate Options

| Option | Summary | Assumptions |
| --- | --- | --- |
| A | Extend `lab.capability_cost_model.v1` with workload fields | cost evidence and workload requirements share one lifecycle |
| B | Add target comparison and suitability decision now | Pi4/Pi5 evidence is already sufficient for decision-level artifacts |
| C | Add separate workload and target capability profile contracts now | stable workload/capability identities should precede comparison and decisions |

## 5. Risk / Tradeoff Analysis

| Option | Benefits | Risks | Tradeoffs | Sensitivity Points |
| --- | --- | --- | --- | --- |
| A | smallest number of new contracts | mixes architecture cost evidence with target-selection workload requirements; future comparison semantics become harder to isolate | lower short-term schema count, higher coupling | becomes weaker as workload classes diversify |
| B | faster path to user-facing target selection | would imply Pi4/Pi5 conclusions before same-suite evidence exists | high product value but premature claims | depends on Pi5 evidence and sustained target envelope data |
| C | separates workload definition, target evidence, and future decision layers | adds two schemas and a report command before comparison exists | slightly more upfront contract work | good if PR13/PR14 consume these IDs unchanged |

## 6. Decision

- Chosen direction: Option C.
- Rationale: PR11 must create the same measuring stick before drawing
  comparisons. Separate WorkloadProfile and TargetCapabilityProfile contracts
  keep workload requirements, target evidence, and future suitability decisions
  independently reviewable.
- Rejected options:
  - Option A: capability cost model should remain architecture-evidence
    classification, not workload requirement ownership.
  - Option B: comparison/suitability claims would be premature without Pi5
    same-suite evidence and sustained physical-footprint data.

## 7. Verification Tasks

- Tests: add schema fixtures, negative enum tests, core profile generation
  tests, and CLI artifact/audit tests.
- Benchmarks: none in PR11; no new target runtime is added.
- Migration checks: ensure existing report commands and schemas remain
  compatible.
- Monitoring / observability: add `report.target_capability_profile` audit
  event.
- Rollback / fallback: removing the new report command and schemas returns the
  repo to PR10 capability-cost behavior; no target cleanup required.
- Dependency or boundary checks: verify no helper, sudo, systemd, socket,
  target-local loop, or destructive workload is introduced.

## 8. Handoffs

- requirements-engineering: not required; user provided explicit ACs.
- observability: record the profile report audit signal.
- embedded-target-characterization: deferred; Pi5 same-suite evidence remains
  missing.
- embedded-system-familiarization: deferred; PR11 consumes existing evidence
  and examples but does not perform new target familiarization.
- embedded-nfr-design: record that PR11 profiles are experimental/short-smoke
  and block production claims.
- error-handling: malformed workload/run artifact handling uses validation
  errors.
- code-smells-and-antipatterns: not required unless implementation introduces
  broad coupling.
- quality-gate: verify ADR, ExecPlan, NFR report, function-boundary review,
  commands, and sensitive-data scan.
