# XLSX Unit Fixtures

XLSX fixture は、まずOffice Open XML内の worksheet XML と shared strings XML
断片を管理する。
実 `.xlsx` パッケージfixtureは、ZIPパッケージ読取実装と合わせて追加する。

期待 Markdown は同名の `*.expected.md` として管理する。

## Fixtures

- `simple-sheet`: 見出し行と1データ行だけの最小表。
- `budget-sheet`: 部門別予算表で頻出する文字列、数値、空セルの混在表。
- `shared-string-sheet`: shared strings を使う最小表。
