default:
    cargo test

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

ci: test eval bench compare-baseline
