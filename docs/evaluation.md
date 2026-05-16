# Evaluation

この文書は、実文書コーパスでbonjilと既存ツールを比較する手順を定義する。

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
  --root /Users/izuno/マイドライブ/docs/outdated \
  --out target/corpus/report.json \
  --limit 30 \
  --per-ext 5
```

出力Markdownは `target/corpus/<tool>/` に保存される。

## 比較対象

- `bonjil`: このリポジトリの変換器
- `pandoc`: PATHに存在する場合のみ実行
- `markitdown`: PATHに存在する場合のみ実行

未導入または変換失敗したツールは、report JSONに `missing` または `error`
として記録する。

## 優位性の扱い

自動スコアだけでは「既存ツールより優れている」と断定しない。
report JSONの `superiority_claim` は、人間レビューまたはground truthがない限り
`not_proven_without_human_review_or_ground_truth` とする。

優れていると言えるのは、同じ入力に対して以下を確認した場合に限る。

- 既存ツールより構造保持が高い
- 既存ツールが失敗した入力でbonjilが有用なMarkdownを出す
- 表、画像、キャプション、コードブロックの破損が少ない
- warning/reportにより失敗原因を追跡できる
