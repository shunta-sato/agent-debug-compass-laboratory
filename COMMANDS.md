# COMMANDS.md — canonical build/test/lint commands

## Initialization verification
- verified by agent: yes (2026-06-08)
- verification command: make verify
- note: initialized for this repository after `make verify` succeeded; adc-lab workspace command wrapper re-verified on 2026-06-08.

## Build
- build (debug): make build-debug
- build (release): make build-release

## Format / Lint / Static analysis
- format: make format
- lint: make lint
- static analysis: make analysis
- contract validation: make contract
- schema generation: make schemas
- generated schema drift check: make schemas-check
- schema classification coverage check: make schema-ledger-check
- Rust file budget report: make file-budgets

## Tests
- unit tests: make test-unit
- integration/e2e tests: make test-integration
- command smoke: make command-smoke

## Notes
- If a command differs between local and CI, document both.
- If a command is intentionally unavailable, explain the alternative.
- `make command-smoke` verifies command wiring only. It does not collect resource metrics and does not support resource/NFR claims by itself.
- GitHub CI runs `make verify`, including generated schema drift detection.
- `make schemas-check` also validates that top-level schemas and schema-versioned wire contracts are covered by `schemas/schema-ledger.tsv`.
- `make file-budgets` is informational until the v2.1 command split phase makes the configured budgets enforceable.
- GitHub release workflow runs `make verify`, builds release binaries, packages tarballs, publishes `SHA256SUMS`, and does not support resource/NFR claims by itself.
- If `make verify` fails during initialization, keep the verification placeholder and document the failure plus next steps in `INIT_REPORT.md` (or append to this file).
