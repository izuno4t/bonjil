# PPTX Unit Fixtures

PPTX fixture は、まずOffice Open XML内の slide XML 断片を管理する。
実 `.pptx` パッケージfixtureは、ZIPパッケージ読取実装と合わせて追加する。

期待 Markdown は同名の `*.expected.md` として管理する。

## Fixtures

- `simple-slide`: 見出しと本文だけの最小スライド。
- `meeting-slide`: 会議資料で頻出するタイトル、箇条書き風テキスト、
  注記、次アクションの混在スライド。
