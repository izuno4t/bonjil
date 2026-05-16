.PHONY: default test regression-test bench corpus-eval review verify fmt lint spell clippy

default: test

test:
	cargo test

regression-test:
	cargo test --test integration
	cargo run --bin bonjil-eval -- tests/fixtures/unit/docx target/eval-report.json
	cat target/eval-report.json
	cargo run --bin bonjil-compare-baseline -- target/eval-report.json tests/thresholds.toml

review:
	cargo test

bench:
	cargo run --bin bonjil-bench -- tests/fixtures/unit/html/basic.html 10

corpus-eval:
	cargo run --bin bonjil-corpus-eval -- --root evaluation/inputs --out evaluation/reports/report.json --output-root evaluation/outputs

fmt:
	cargo fmt

lint:
	markdownlint-cli2 README.md docs/*.md evaluation/*.md evaluation/**/*.md CLAUDE.md AGENTS.local.md tests/fixtures/**/*.md benches/README.md

spell:
	cspell

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

verify: fmt clippy test regression-test lint spell
