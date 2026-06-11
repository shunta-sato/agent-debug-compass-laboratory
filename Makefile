.PHONY: help build-debug build-release format format-fix lint analysis test-unit test-integration contract schemas schemas-check schema-ledger-check file-budgets docs-smoke command-smoke verify clean

help:
	@printf '%s\n' \
	  'adc-lab canonical commands:' \
	  '  make build-debug       cargo build --workspace' \
	  '  make build-release     cargo build --workspace --release' \
	  '  make format            cargo fmt --all --check' \
	  '  make format-fix        cargo fmt --all' \
	  '  make lint              cargo clippy --workspace --all-targets -- -D warnings' \
	  '  make analysis          cargo test --workspace contract_validation -- --nocapture' \
	  '  make test-unit         cargo test --workspace --lib' \
	  '  make test-integration  cargo test --workspace --tests' \
	  '  make contract          cargo test --workspace contract_validation -- --nocapture' \
	  '  make schemas           cargo run -p adc-lab-core --example generate_schemas -- schemas/generated' \
	  '  make schemas-check     regenerate schemas and fail on generated drift' \
	  '  make schema-ledger-check validate schema classification coverage' \
	  '  make file-budgets      enforce production Rust file budgets' \
	  '  make command-smoke     scripts/resource/run-resource-smoke.sh --host-fallback' \
	  '  make verify            build-debug + format + lint + schemas-check + tests + contract + docs/command smoke'

build-debug:
	cargo build --workspace

build-release:
	cargo build --workspace --release

format:
	cargo fmt --all --check

format-fix:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets -- -D warnings

analysis: contract

test-unit:
	cargo test --workspace --lib

test-integration:
	cargo test --workspace --tests

contract:
	cargo test --workspace contract_validation -- --nocapture

schemas:
	cargo run -p adc-lab-core --example generate_schemas -- schemas/generated

schemas-check:
	tmp_dir=$$(mktemp -d); \
	trap 'rm -rf "$$tmp_dir"' EXIT; \
	cargo run -p adc-lab-core --example generate_schemas -- "$$tmp_dir"; \
	diff -ru schemas/generated "$$tmp_dir"
	python3 scripts/schema/check-schema-ledger.py --enforce-final

schema-ledger-check:
	python3 scripts/schema/check-schema-ledger.py

file-budgets:
	python3 scripts/ci/check-file-budgets.py --enforce

docs-smoke:
	test -f README.md
	test -f docs/README.md
	test -f docs/architecture/privilege-model-option-a.md
	test -f docs/architecture/safety-model.md
	test -f docs/evidence-model.md
	test -f docs/rules.md
	test -f docs/reference/cli.md
	test -f docs/reference/pressure-probes.md
	test -f docs/getting-started/install-release-binaries.md
	test -f reports/resource/nfr-gate-report.md

command-smoke:
	scripts/resource/run-resource-smoke.sh --host-fallback

verify: build-debug format lint schemas-check file-budgets test-unit test-integration contract docs-smoke command-smoke

clean:
	cargo clean
