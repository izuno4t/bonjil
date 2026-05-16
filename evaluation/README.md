# Evaluation

このディレクトリは、`bonjil` の評価に必要なリソースを集約する。

## ディレクトリ構成

| パス | 目的 | Git管理 |
| ---- | ---- | ---- |
| `bin/` | 評価用Cargoバイナリ | 管理対象 |
| `tools.md` | 評価対象ツールの選定 | 管理対象 |
| `methods/evaluation.md` | 評価手法 | 管理対象 |
| `methods/benchmark.md` | ベンチマーク手法 | 管理対象 |
| `tool-runners/` | 比較ツールのDockerfileとラッパー実装 | 管理対象 |
| `inputs/` | 評価入力ファイルの置き場 | 中身は管理外 |
| `outputs/` | 人が確認するMarkdown出力の置き場 | 中身は管理外 |
| `reports/` | JSONレポートや集計結果の置き場 | 中身は管理外 |

`inputs/`、`outputs/`、`reports/` は `.gitkeep` だけを管理し、実ファイルは
Git管理外にする。

## 評価用バイナリ

評価用の実行ファイルは `evaluation/bin/` に置く。Cargo のバイナリ名は
維持しているため、既存の `cargo run --bin ...` コマンドは変わらない。

| バイナリ | 目的 |
| ---- | ---- |
| `bonjil-eval` | fixture と期待Markdownを比較する評価レポート生成 |
| `bonjil-compare-baseline` | 評価レポートをしきい値と比較する回帰検出 |
| `bonjil-bench` | 変換処理の簡易ベンチマーク |
| `bonjil-corpus-eval` | 実ディレクトリの文書を使った既存ツール比較 |

`bonjil-corpus-eval` の比較対象ツールは Docker コンテナとして実行する。
比較ツールのDockerfileとラッパー実装は `tool-runners/` に置く。
既定のDockerイメージは次の通り。

- `pandoc`: `bonjil-eval-pandoc:latest`
- `markitdown`: `bonjil-eval-markitdown:latest`
- `docling`: `bonjil-eval-docling:latest`
- `pymupdf4llm`: `bonjil-eval-pymupdf4llm:latest`
- `mammoth-js`: `bonjil-eval-mammoth-js:latest`

イメージを差し替える場合は、次の環境変数を指定する。

- `BONJIL_EVAL_PANDOC_IMAGE`
- `BONJIL_EVAL_MARKITDOWN_IMAGE`
- `BONJIL_EVAL_DOCLING_IMAGE`
- `BONJIL_EVAL_PYMUPDF4LLM_IMAGE`
- `BONJIL_EVAL_MAMMOTH_JS_IMAGE`

比較ツールランナーは、Markdownとreport JSONを `evaluation/outputs/` 配下に
ファイルとして書き出す。標準出力は短い実行サマリだけに使う。

## 比較ツールイメージのビルド

```bash
docker build -t bonjil-eval-pandoc:latest evaluation/tool-runners/pandoc
docker build -t bonjil-eval-markitdown:latest evaluation/tool-runners/markitdown
docker build -t bonjil-eval-docling:latest evaluation/tool-runners/docling
docker build -t bonjil-eval-pymupdf4llm:latest evaluation/tool-runners/pymupdf4llm
docker build -t bonjil-eval-mammoth-js:latest evaluation/tool-runners/mammoth-js
```

## 実行例

定型コマンドは `make` から実行できる。

```bash
make bench
make corpus-eval
```

```bash
cargo run --bin bonjil-corpus-eval -- \
  --root evaluation/inputs \
  --out evaluation/reports/report.json \
  --output-root evaluation/outputs \
  --limit 30 \
  --per-ext 5 \
  --tools pandoc,markitdown
```

PDFだけを100件評価する場合は次のように実行する。

```bash
cargo run --bin bonjil-corpus-eval -- \
  --root evaluation/inputs \
  --out evaluation/reports/pdf-100-report.json \
  --output-root evaluation/outputs \
  --limit 100 \
  --per-ext 100 \
  --ext pdf \
  --tools docling,pymupdf4llm
```

出力Markdownは `evaluation/outputs/<tool>/` に保存されるため、人が直接確認できる。
