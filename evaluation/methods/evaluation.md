# Evaluation

この文書は、実文書コーパスでbonjilと既存ツールを比較する手順を定義する。
形式ごとの比較対象ツールは [../tools.md](../tools.md) に定義する。

## 目的

「変換できた」だけでは品質を判断しない。次の観点を分けて記録する。

- 変換成功率
- Markdown構造量
- 見出し、表、画像、コードブロック、リストの保持
- 処理時間
- 失敗理由
- 人間レビューが必要な優位性判定

## 実行方法

```bash
cargo run --bin bonjil-corpus-eval -- \
  --root evaluation/inputs \
  --out evaluation/reports/report.json \
  --output-root evaluation/outputs \
  --limit 30 \
  --per-ext 5 \
  --tools pandoc,markitdown
```

PDFだけを100件評価する場合は、次のように実行する。

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

出力Markdownは `evaluation/outputs/<tool>/` に保存される。

## 比較対象

- `bonjil`: このリポジトリの変換器
- `pandoc`: Dockerイメージ `bonjil-eval-pandoc:latest` で実行
- `markitdown`: Dockerイメージ `bonjil-eval-markitdown:latest` で実行
- `docling`: Dockerイメージ `bonjil-eval-docling:latest` で実行
- `pymupdf4llm`: Dockerイメージ `bonjil-eval-pymupdf4llm:latest` で実行
- `mammoth-js`: Dockerイメージ `bonjil-eval-mammoth-js:latest` で実行

Dockerが未導入、イメージが未作成、または変換に失敗したツールは、report JSONに
`missing` または `error` として記録する。

## 優位性の扱い

自動スコアだけでは「既存ツールより優れている」と断定しない。
report JSONの `superiority_claim` は、人間レビューまたはground truthがない限り
`not_proven_without_human_review_or_ground_truth` とする。

優れていると言えるのは、同じ入力に対して以下を確認した場合に限る。

- 既存ツールより構造保持が高い
- 既存ツールが失敗した入力でbonjilが有用なMarkdownを出す
- 表、画像、キャプション、コードブロックの破損が少ない
- warning/reportにより失敗原因を追跡できる
