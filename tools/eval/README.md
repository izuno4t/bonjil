# Evaluation Tools

このディレクトリは、`bonjil` 本体とは独立した評価・検証用ツールを置く場所です。

## 配置方針

- `tools/eval/bin/bonjil-eval.rs`: fixture と期待Markdownを比較する評価レポート生成
- `tools/eval/bin/bonjil-compare-baseline.rs`: 評価レポートをしきい値と比較する回帰検出
- `tools/eval/bin/bonjil-bench.rs`: 変換処理の簡易ベンチマーク
- `tools/eval/bin/bonjil-corpus-eval.rs`: 実ディレクトリの文書を使った既存ツール比較

Cargo のバイナリ名は維持しているため、既存の実行コマンドは変わりません。
`bonjil-corpus-eval` の比較対象ツールは Docker コンテナとして実行します。

既定のDockerイメージは次の通りです。

- `pandoc`: `pandoc/core:latest`
- `markitdown`: `markitdown:latest`

イメージを差し替える場合は、次の環境変数を指定します。

- `BONJIL_EVAL_PANDOC_IMAGE`
- `BONJIL_EVAL_MARKITDOWN_IMAGE`

`markitdown:latest` は Microsoft MarkItDown のDockerfileからローカルでビルドした
イメージを想定します。

```bash
cargo run --bin bonjil-eval -- tests/fixtures/unit/docx target/eval-report.json
cargo run --bin bonjil-compare-baseline -- target/eval-report.json tests/thresholds.toml
cargo run --bin bonjil-bench -- tests/fixtures/unit/html/basic.html 10
cargo run --bin bonjil-corpus-eval -- --root /path/to/docs --out target/corpus/report.json
cargo run --bin bonjil-corpus-eval -- \
  --root /path/to/docs \
  --out target/corpus/pdf-report.json \
  --limit 100 \
  --per-ext 100 \
  --ext pdf
```
