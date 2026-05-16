# Public Corpus Register

`evaluation/inputs/` に置く公開データと、そこから派生させるfixtureの対応を記録する。
入力ファイル本体と評価出力はGit管理外だが、このregisterはGit管理する。

## 登録ルール

- 公開データを評価に使う前に、このregisterへ出典とライセンスを記録する。
- 変換結果からfixtureを作る場合は、元データ、失敗パターン、fixture名を対応させる。
- fixtureは元データの複製ではなく、同じ問題を同じ文書形式で再現する最小文書にする。
- ライセンスや再配布可否が不明なデータは、実コーパス評価には使ってもfixture化しない。

## Corpus Entries

| ID | Input path | Source URL | License | Format | Pattern | Fixture |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| PCORPUS-001 | `evaluation/inputs/markitdown/test.pdf` | [microsoft/markitdown test.pdf](https://github.com/microsoft/markitdown/blob/main/packages/markitdown/tests/test_files/test.pdf) | MIT | pdf | PDF構造抽出 | 未作成 |
| PCORPUS-002 | `evaluation/inputs/markitdown/test.pptx` | [microsoft/markitdown test.pptx](https://github.com/microsoft/markitdown/blob/main/packages/markitdown/tests/test_files/test.pptx) | MIT | pptx | slide shape/text box | `tests/fixtures/unit/pptx/visual-order-shapes.slide.xml` |
| PCORPUS-003 | `evaluation/inputs/markitdown/test.xlsx` | [microsoft/markitdown test.xlsx](https://github.com/microsoft/markitdown/blob/main/packages/markitdown/tests/test_files/test.xlsx) | MIT | xlsx | worksheet/table | `tests/fixtures/unit/xlsx/merged-header-sheet.worksheet.xml` |
| PCORPUS-004 | `evaluation/inputs/markitdown/test.docx` | [microsoft/markitdown test.docx](https://github.com/microsoft/markitdown/blob/main/packages/markitdown/tests/test_files/test.docx) | MIT | docx | document structure/media | 未作成 |
| PCORPUS-005 | `evaluation/inputs/japanese-official/mhlw-disability.pdf` | [厚生労働省 PDF](https://www.mhlw.go.jp/content/12601000/000520863.pdf) | 政府公開資料、利用条件要確認 | pdf | 日本語/省庁会議資料/表紙/スライド由来PDF | 未作成 |
| PCORPUS-006 | `evaluation/inputs/japanese-official/osaka-orientation.pdf` | [大阪大学 PDF](https://iii.osaka-u.ac.jp/en/wp-content/uploads/sites/2/2026/05/2026%E6%98%A5IJ%E3%82%AA%E3%83%AA%E3%83%86.pdf) | 大学公開資料、利用条件要確認 | pdf | 日本語/大学オリエンテーション/スライド由来PDF | 未作成 |
| PCORPUS-007 | `evaluation/inputs/japanese-official/nlbc-form.xlsx` | [家畜改良センター xlsx index](https://www.nlbc.go.jp/assets/xlsx/) | 政府公開資料、利用条件要確認 | xlsx | 日本語申請書/結合セル/帳票 | 未作成 |
| PCORPUS-008 | `evaluation/inputs/japanese-official/osaka-form.xlsx` | [大阪大学 form0701.xlsx](https://www.osaka-u.ac.jp/ja/campus/alumni/pr/oumail_news/files_OUMail/form0701.xlsx) | 大学公開資料、利用条件要確認 | xlsx | 日本語/大学申請フォーム/帳票 | 未作成 |
| PCORPUS-009 | `evaluation/inputs/japanese/plain_form_ppt.pptx` | [Japanese Teaching Ideas: Plain verbs](https://japaneseteachingideas.weebly.com/plain-verbs.html) | Public download, license要確認 | pptx | 日本語かな/英日混在/slide text box | 補助コーパス |
| PCORPUS-010 | `evaluation/inputs/japanese/vocab_test_verbs.xlsx` | [Japanese Teaching Ideas: Plain verbs](https://japaneseteachingideas.weebly.com/plain-verbs.html) | Public download, license要確認 | xlsx | 日本語かな/結合セル/語彙表 | 補助コーパス |
| PCORPUS-011 | `evaluation/inputs/japanese/plain_dictionary_form_verb_conversion_table.pdf` | [Japanese Teaching Ideas: Plain verbs](https://japaneseteachingideas.weebly.com/plain-verbs.html) | Public download, license要確認 | pdf | 日本語かな/表/PDF読順 | 補助コーパス |
| PCORPUS-012 | `evaluation/inputs/japanese/ipsj-template.docx` | [情報処理学会 原稿テンプレート](https://www.ipsj.or.jp/magazine/sippitsu/magtemp.html) | サイト条件要確認 | docx | 日本語見出し/図表/テンプレート | 補助コーパス |

## Derived Fixture Entries

| Fixture | Source ID | Format | Failure pattern |
| ---- | ---- | ---- | ---- |
| `visual-order-shapes.slide.xml` | PCORPUS-002 | pptx | XML順と視覚順 |
| `merged-header-sheet.worksheet.xml` | PCORPUS-003 | xlsx | 表構造保持 |

Fixture paths:

- `tests/fixtures/unit/pptx/visual-order-shapes.slide.xml`
- `tests/fixtures/unit/xlsx/merged-header-sheet.worksheet.xml`
