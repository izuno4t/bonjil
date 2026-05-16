# Fixtures

評価データセットは `unit`、`integration`、`regression` の3層で管理する。

- `unit`: 1ファイル1機能を確認する最小入力
- `integration`: 実文書に近い中規模入力
- `regression`: 過去に壊れたケースの最小再現

`*.expected.md` は人間レビュー対象であり、自動更新しない。
