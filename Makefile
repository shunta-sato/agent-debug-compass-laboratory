.PHONY: help build-debug build-release format format-fix lint analysis test-unit test-integration contract docs-smoke command-smoke resource-smoke verify clean

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
	  '  make command-smoke     scripts/resource/run-resource-smoke.sh --host-fallback' \
	  '  make resource-smoke    compatibility alias for command-smoke; does not collect metrics' \
	  '  make verify            build-debug + format + lint + tests + contract + docs/command smoke'

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

docs-smoke:
	test -f README.md
	test -f docs/architecture/privilege-model-option-a.md
	test -f docs/architecture/safety-model.md
	test -f reports/resource/nfr-gate-report.md

command-smoke:
	scripts/resource/run-resource-smoke.sh --host-fallback

resource-smoke: command-smoke

verify: build-debug format lint test-unit test-integration contract docs-smoke command-smoke

clean:
	cargo clean
