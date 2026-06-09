# Concurrency Verification Matrix

## Scope

Concurrent code in MVP is limited to bounded CPU load workers in `crates/adc-lab-core/src/load.rs`.

## Concurrency Plan Summary

- Model: one monitor/controller thread plus N worker threads.
- Shared state: `Arc<AtomicBool>` stop flag only.
- Synchronization: atomic cancellation with `Ordering::Relaxed`; no mutexes, shared buffers, queues, or lock ordering.
- Shutdown: monitor sets stop flag at duration, thermal abort, or operator abort, then joins all workers.
- Error propagation: worker panic maps to `LabError::Validation`; setup failures return before worker spawn.

## Verification Matrix

| Tool / Check | What it catches | How to run | Cost / limits |
| --- | --- | --- | --- |
| Rust type system | data races in safe shared memory | `cargo check --workspace` | cannot prove logical timing properties |
| Unit test | bounded worker completion and join behavior | `cargo test -p adc-lab-core load::tests::contract_validation_cpu_load_is_bounded` | host-only; not target thermal proof |
| Integration test | CLI wiring around bounded load remains buildable | `cargo test --workspace --tests` | does not run long stress |
| Clippy | accidental unsafe or suspicious concurrency patterns | `cargo clippy --workspace --all-targets -- -D warnings` | lint only |
| TSan | C/C++ data races | not applicable | MVP is Rust safe code; no C/C++ thread primitives |

## Risks And Mitigations

- Risk: thermal abort is only as good as target thermal surface availability.
- Risk: operator abort depends on the target-local abort marker being readable
  by the non-root runner.
- Mitigation: result records `max_observed_temp_c` and leaves physical claims experimental-only when unavailable.

- Risk: high worker count can intentionally load the target.
- Mitigation: command requires explicit `--workers` and `--duration`; no default background load.
