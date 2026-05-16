# Public Corpus Observations

公開データを `evaluation/inputs/` に置き、bonjilで変換した結果から
fixture化すべき失敗パターンを記録する。

## 2026-05-17 MarkItDown公式テストファイル

入力はMicrosoft MarkItDown公式リポジトリの `packages/markitdown/tests/test_files/`
から取得した。ファイル本体と変換出力はGit管理外に置く。

実行コマンド:

```bash
cargo run --bin bonjil-corpus-eval -- \
  --root evaluation/inputs/markitdown \
  --out evaluation/reports/markitdown-public.json \
  --output-root evaluation/outputs/markitdown-public \
  --limit 4 \
  --per-ext 4 \
  --tools bonjil
```

## 観測した失敗パターン

| Corpus ID | 形式 | 観測 |
| ---- | ---- | ---- |
| PCORPUS-001 | PDF | OCR必須のfallback文だけになった。警告品質fixtureが必要。 |
| PCORPUS-002 | PPTX | run分割で `AutoGen:` が見出しと本文に割れた。 |
| PCORPUS-002 | PPTX | 画像は抽出されたが、caption候補とmedia idの対応が不足。 |
| PCORPUS-003 | XLSX | sheet名、表範囲、数式/表示値、結合セルの説明が不足。 |
| PCORPUS-004 | DOCX | tableとimageは出たが、styleなし見出し、caption/media対応、脚注などの確認が必要。 |

## Fixture化方針

- PPTXのrun分割と視覚順序の問題は、同じPPTX形式の最小再現fixtureへ落とす。
- XLSXの表範囲、結合セル、数式表示値の問題は、同じXLSX形式の最小再現fixtureへ落とす。
- PDFのOCR fallbackは、テキスト抽出不可PDFまたは画像PDFの最小再現fixtureへ落とす。
- DOCXのcaption/media対応は、同じDOCX形式の最小再現fixtureへ落とす。

## 2026-05-17 日本語公式公開文書

日本語文書は、国の機関、独立行政法人、大学が公開している文書を主要評価にする。
教材サイトや個人配布に近い文書も補助コーパスとして残し、失敗パターンの幅を広げる。

実行コマンド:

```bash
cargo run --bin bonjil-corpus-eval -- \
  --root evaluation/inputs/japanese-official \
  --out evaluation/reports/japanese-official.json \
  --output-root evaluation/outputs/japanese-official \
  --limit 10 \
  --per-ext 10 \
  --tools bonjil
```

| Corpus ID | 形式 | 観測 |
| ---- | ---- | ---- |
| PCORPUS-005 | PDF | 厚労省PDFはバイナリ断片が本文へ混入した。 |
| PCORPUS-006 | PDF | 大阪大学PDFもバイナリ断片と日本語抽出問題がある。 |
| PCORPUS-007 | XLSX | sharedStrings未解決に見える数値が出ている。 |
| PCORPUS-008 | XLSX | 実体がHTMLで、入力妥当性確認の対象にする。 |

日本語fixtureでは、少なくとも次を含める。

- 省庁または独立行政法人の帳票型XLSXから導いた結合セル/日本語sharedStrings。
- 大学公開PDFから導いたスライド由来PDFの読順と日本語文字抽出。
- 日本語の全角記号、かな、漢字、英数字混在の見出し・表・注記。
