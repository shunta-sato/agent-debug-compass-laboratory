# Function Boundary Review: PR6 Real Experiment Matrix Runner

## Scope

Changed functions/helpers:

- `apply_control_plan`, `approval_matches`, `restore_lease`, `restore_refused_result`, `failed_result_with_restore_attempt`, `canonical_plan_digest` in `crates/adc-lab-core/src/control.rs`.
- `validate_priv_helper_path`, `validate_restore_lease`, `validate_policy_segment`, `validate_governor_value`, `validate_optional_frequency`, `verify_restored_state`, `restore_failed_result` in `crates/adc-lab-core/src/control.rs`.
- `artifact_uri_for_run`, `RunContext::artifact_uri` in `crates/adc-lab-core/src/run.rs`.
- `collect_artifact_refs`, `collect_files` in `crates/adc-lab-core/src/report.rs`.
- `ssh_runner_program`, `validate_ssh_runner_program`, `is_safe_ssh_endpoint` in `crates/adc-lab-core/src/target.rs`.
- `run_experiment_matrix` in `crates/adc-lab-core/src/experiment.rs`.
- `expand_factors`, `validate_experiment_matrix_bounds` in `crates/adc-lab-core/src/experiment.rs`.
- `qualify_tool` in `crates/adc-lab-core/src/qualification.rs`.
- `new_cpu_load_plan` in `crates/adc-lab-core/src/load.rs`.
- `command_control_apply`, `command_restore`, `persist_control_result`, `persist_approval_record`, `safe_artifact_id`, `invoke_helper_apply`, `invoke_helper_restore`, `observe_ssh`, `load_cpu_ssh` in `crates/adc-lab/src/main.rs`.
- `execute_experiment_matrix`, `execute_supported_experiment_trial`, `blocked_trial_reason`, `real_experiment_claim_trace`, `experiment_run_result` in `crates/adc-lab/src/main.rs`.

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
| `run_experiment_matrix` | matrix planning and claim trace domain | keep dry-run and unsupported non-dry planning semantics in core |
| `expand_factors` / `validate_experiment_matrix_bounds` | shared matrix policy domain | keep in core so CLI and contract tests use the same bounds |
| `execute_experiment_matrix` | controller-side real execution boundary | keep in CLI until supported trial execution outgrows the controller command surface |
| `execute_supported_experiment_trial` | per-trial bounded load/observe orchestration | split from matrix expansion so completion depends on real artifacts and audit |
| `blocked_trial_reason` | trial policy classification | keep explicit instead of folding unsupported factors into execution errors |
| `real_experiment_claim_trace` | claim boundary for executed matrix subset | keep separate from dry-run trace so supported claims require completed trial artifacts |
| `qualify_tool` | tool qualification contract | keep manifest-only path unqualified until evidence workflow exists |
| `new_cpu_load_plan` | same bounded load policy domain | keep policy in plan constructor |
| CLI apply/restore guards | call-site side-effect boundary before sudo | keep guard at controller boundary plus core/helper guard |

## Decisions

- No merge into generic `util` or `policy` module. The invariants are domain-specific and clearer where they sit.
- No destructive refactor. Existing call sites were migrated coherently without temporary compatibility shims.
- Ledger update not required: no abstraction replacement, intentional duplication, or staged adapter was introduced.
- Public helper override is removed from the controller CLI. Helper path validation remains in core as a guard on the fixed controller-internal helper path without moving `sudo` side effects into core.
- Restore lease validation is split into small invariant checks rather than a broad generic validator because each refusal message is an operator-facing safety boundary.
- Experiment matrix core remains a planner. Real execution lives in the CLI
  orchestration boundary and supports only listed-order `cpu_load_workers`
  trials with bounded load and passive observe artifacts.
- Unsupported controlled factors, randomized order, and failed trial steps
  become blocked or failed trial outcomes. They do not become supported claims.

## Verification

- `cargo test -p adc-lab --test cli experiment_real_run -- --nocapture`: pass
  after adding the real matrix runner subset tests.
- Full workspace verification is recorded in `reports/quality-gate.md`.
