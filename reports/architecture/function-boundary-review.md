# Function Boundary Review: PR #1 Safety Fixes

## Scope

Changed functions/helpers:

- `apply_control_plan`, `approval_matches`, `restore_lease`, `restore_refused_result`, `failed_result_with_restore_attempt`, `canonical_plan_digest` in `crates/adc-lab-core/src/control.rs`.
- `validate_priv_helper_path`, `validate_restore_lease`, `validate_policy_segment`, `validate_governor_value`, `validate_optional_frequency`, `verify_restored_state`, `restore_failed_result` in `crates/adc-lab-core/src/control.rs`.
- `artifact_uri_for_run`, `RunContext::artifact_uri` in `crates/adc-lab-core/src/run.rs`.
- `collect_artifact_refs`, `collect_files` in `crates/adc-lab-core/src/report.rs`.
- `ssh_runner_program`, `validate_ssh_runner_program`, `is_safe_ssh_endpoint` in `crates/adc-lab-core/src/target.rs`.
- `run_experiment_matrix` in `crates/adc-lab-core/src/experiment.rs`.
- `qualify_tool` in `crates/adc-lab-core/src/qualification.rs`.
- `new_cpu_load_plan` in `crates/adc-lab-core/src/load.rs`.
- `command_control_apply`, `command_restore`, `persist_control_result`, `persist_approval_record`, `safe_artifact_id`, `invoke_helper_apply`, `invoke_helper_restore`, `observe_ssh`, `load_cpu_ssh` in `crates/adc-lab/src/main.rs`.

## Semantic Neighbors

| Function | Neighbor classification | Decision |
| --- | --- | --- |
| `apply_control_plan` / `restore_lease` | same control domain; typed state-changing boundary | keep and extend in place |
| `approval_matches` / `canonical_plan_digest` | same approval policy domain | keep parallel helper; no generic digest util |
| `validate_restore_lease` helpers | restore input validation domain | split by invariant so path/governor/frequency failures stay precise |
| `validate_priv_helper_path` | controller-internal sudo boundary | keep in core so fixed helper path validation stays outside shell invocation |
| `artifact_uri_for_run` / `collect_artifact_refs` | same artifact-ref domain; one owns URI normalization, one owns run traversal | split responsibilities |
| `ssh_runner_program` / `validate_ssh_runner_program` | same target transport domain | split validation from environment lookup |
| `is_safe_ssh_endpoint` | target transport input grammar | keep local to target parsing; no generic URL parser |
| `run_experiment_matrix` | matrix planning and claim trace domain | keep planning-only behavior here; do not fake execution |
| `qualify_tool` | tool qualification contract | keep manifest-only path unqualified until evidence workflow exists |
| `new_cpu_load_plan` | same bounded load policy domain | keep policy in plan constructor |
| CLI apply/restore guards | call-site side-effect boundary before sudo | keep guard at controller boundary plus core/helper guard |

## Decisions

- No merge into generic `util` or `policy` module. The invariants are domain-specific and clearer where they sit.
- No destructive refactor. Existing call sites were migrated coherently without temporary compatibility shims.
- Ledger update not required: no abstraction replacement, intentional duplication, or staged adapter was introduced.
- Public helper override is removed from the controller CLI. Helper path validation remains in core as a guard on the fixed controller-internal helper path without moving `sudo` side effects into core.
- Restore lease validation is split into small invariant checks rather than a broad generic validator because each refusal message is an operator-facing safety boundary.
- Experiment matrix remains a planner until execution is actually wired; claim state is blocked/provisional instead of supported.

## Verification

- `cargo test --workspace`: pass after public helper override removal.
- `make verify`: pass after public helper override removal.
- `make build-release`: pass after public helper override removal.
- `make resource-smoke`: pass after public helper override removal.
- Target command smoke for `ssh://target55`: not rerun for this second-review update; code changes are local/controller contract hardening.
