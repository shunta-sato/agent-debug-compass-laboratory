# Bug Report (RCA): v2 suitability loop and coupling parity regressions

## Symptom

`report operating-contract` emits a v2
`Artifact<OperatingContractPayload>`, but `decide suitability
--target-contract` still deserializes the file as a v1
`TargetOperatingContract`. The next public command,
`constraints generate --decision`, still deserializes its input as a v1
`SuitabilityDecision` even though `decide suitability --out` emits a v2
`Artifact<SuitabilityPayload>`.

The operating-contract rule
`operating.memory_storage_coupling_requires_composite` also treats pressure and
composite artifact presence as enough for `Supported`, even when the composite
artifact status is `insufficient`.

## Expected Behavior

The public README loop must work with artifacts produced by the current CLI:

1. `report operating-contract` writes a v2 operating contract.
2. `decide suitability` consumes that v2 contract and writes a v2 suitability
   artifact.
3. `constraints generate` consumes that v2 suitability artifact.

Coupling claims must remain at least as conservative as v1: probe presence alone
cannot support memory/storage coupling unless pressure effect and composite
measurement evidence are present.

## Severity / Impact

Critical for v2 public CLI correctness. The current pipeline breaks
end-to-end, and the coupling rule can make a stronger claim than the evidence
supports.

## Environment

- Repository: `agent-debug-compass-laboratory`
- Branch base: `origin/main` after PR #37
- Date: 2026-06-11
- Gate: `make verify`

## Detection

Human review of PR #37 reported the broken public pipeline and the
over-permissive coupling rule. Local inspection confirmed the command handlers
still read v1 DTOs directly.

## Reproduction

Minimal reproduction:

- Generate `reports/target_operating_contract.v2.json` via
  `adc-lab report operating-contract`.
- Pass that path to `adc-lab decide suitability --target-contract`.
- Pass the resulting v2 suitability artifact to
  `adc-lab constraints generate --decision`.

Before the fix, the first handoff fails at v1 deserialization of the v2
envelope; the second handoff fails for the same reason.

## Evidence

- `crates/adc-lab/src/commands/decide.rs` read
  `TargetOperatingContract` directly.
- `crates/adc-lab/src/main.rs` read `SuitabilityDecision` directly in
  `command_constraints_generate`.
- `crates/adc-lab-core/src/rules/operating_contract.rs` used
  `Pred::Present(Kind::Pressure)` and `Pred::Present(Kind::Composite)` for
  the coupling support decision.

## Root Cause Analysis (Five Whys)

1. Why did the public suitability loop fail?  
   Because v2 CLI outputs were wired to v1-only readers.
2. Why were v1 readers still present?  
   Because Phase 4 changed output shapes but tests still used hand-written v1
   fixtures for the downstream commands.
3. Why did tests miss this?  
   Because there was no end-to-end CLI test that chained only tool-produced v2
   artifacts.
4. Why did coupling become less conservative?  
   Because the initial rule vocabulary only tested artifact presence.
5. Why was presence enough in the rule table?  
   Because the planned measured-effect predicates were documented but not
   implemented before the rule was promoted to public output.

Root cause: the public cutover lacked end-to-end v2 handoff tests and the rule
engine did not implement the planned evidence-status predicates.

## Fix

- Add v2 readers/projections at the CLI boundary:
  - v2 operating contract artifact to a legacy contract view for the existing
    suitability policy evaluator.
  - v2 suitability artifact to a legacy decision view for the existing
    constraint-pack generator.
- Add measured-effect predicates for pressure and composite evidence.
- Preserve repeated v2 pressure/composite sidecars by including result IDs in
  file names.
- Add schema drift detection to `make verify`.
- Record v2 evidence writes as normal `lab.audit_event.v1` entries.

## Verification

Planned:

- Focused CLI tests for the v2 suitability loop.
- Focused rules tests for insufficient composite evidence.
- Focused probe tests for repeated sidecar preservation.
- `make schemas`
- `make verify`

## Prevention

- Prevent: keep E2E CLI tests that chain current public artifacts instead of
  hand-written legacy fixtures only.
- Detect: include generated schema drift checking in `make verify`.
- Mitigate: keep v1 projection helpers narrow and local to the CLI boundary so
  they can be deleted after native v2 suitability policy evaluation exists.
