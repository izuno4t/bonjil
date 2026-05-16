.PHONY: default test eval review bench compare-baseline ci fmt lint clippy

default: test

test:
	cargo test

eval:
	cargo test --test integration
	cargo run --bin bonjil -- README.md --report target/eval-report.json >/dev/null
	cat target/eval-report.json

review:
	cargo test

bench:
	cargo run --bin bonjil -- tests/fixtures/unit/html/basic.html >/dev/null

compare-baseline:
	cargo test

fmt:
	cargo fmt

lint:
	markdownlint-cli2 README.md docs/*.md CLAUDE.md tests/fixtures/**/*.md benches/README.md

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

ci: fmt clippy test eval bench compare-baseline lint
