#[derive(Clone, Debug, PartialEq)]
pub enum AstNode {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    List {
        ordered: bool,
        items: Vec<Vec<AstNode>>,
    },
    Text(String),
    Table {
        rows: Vec<TableRow>,
    },
    Image {
        alt: String,
        path: String,
        title: Option<String>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Footnote {
        label: String,
        text: String,
    },
    RawHtml(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableCell {
    pub text: String,
    pub rowspan: usize,
    pub colspan: usize,
    pub image: Option<String>,
}
