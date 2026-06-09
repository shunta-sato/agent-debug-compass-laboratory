# Function Boundary Review: PR10 Option B Privilege Provider Status

## Scope

Changed functions/helpers:

- `privilege_provider_status` in `crates/adc-lab-core/src/privilege.rs`.
- `option_a_provider` and `option_b_provider` in
  `crates/adc-lab-core/src/privilege.rs`.
- `command_privilege_provider_status` in `crates/adc-lab/src/main.rs`.
- `PrivilegeProviderStatus` and related provider DTOs in
  `crates/adc-lab-core/src/contracts.rs`.

## Semantic Neighbors

| Function / Type | Neighbor classification | Decision |
| --- | --- | --- |
| `build_health_output` | parallel read-only status report | keep parallel because health probes current target surfaces while provider status reports policy posture |
| `validate_priv_helper_path` | Option A helper path policy | keep in control module because it gates privileged execution, not read-only provider reporting |
| `command_health_check` | parallel CLI status command | keep separate; no provider artifact/audit semantics |
| `persist_control_result` | privileged operation audit boundary | keep separate; provider status is Tier 0 and does not produce control results |
| `privilege_provider_status` | new pure core builder | keep in new `privilege` module so future provider transport code does not leak into CLI command dispatch |

## Decisions

- Keep provider posture building in `adc-lab-core`, not inline in the CLI.
- Keep Option A execution validation in `control.rs`; provider status only
  describes policy and does not decide whether a helper path may run.
- Keep Option B as a descriptor with `planned_disabled` availability and empty
  operation allowlist. Do not add a staged socket adapter in PR10.
- The CLI command owns target parsing, run artifact writes, and audit emission.
- Ledger update not required: no replaced abstraction, intentional duplication,
  or staged adapter remains. The new module is a narrow contract/report
  boundary.

## Boundary Decisions

| Boundary | Action | Rationale |
| --- | --- | --- |
| Provider status DTOs | keep | typed contract is required for schema/golden validation |
| `privilege_provider_status` | keep | pure builder, deterministic except timestamp, no filesystem side effects |
| Option A/Option B descriptor helpers | keep | split keeps provider-specific constants readable without generic flags |
| CLI provider status command | keep | artifact/audit side effects belong at CLI boundary with existing run context |

## Verification

Planned commands:

- `cargo test -p adc-lab-core privilege -- --nocapture`
- `cargo test -p adc-lab --test cli privilege_provider -- --nocapture`
- `make contract`
- `make verify`

Final results are recorded in `reports/quality-gate.md`.
