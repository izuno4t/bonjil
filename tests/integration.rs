use bonjil::{
    AstNode, ConversionOptions, Converter, Flavor, OutputFormat, TableCell, TableRow, docx,
    evaluate_heading_recall, evaluate_lint_score, evaluate_structure_fidelity,
    evaluate_table_integrity, evaluate_translation_structure_preserve, markdown, office,
};

#[test]
fn converts_ast_to_commonmark() {
    let ast = vec![
        AstNode::Heading {
            level: 1,
            text: "Title".to_string(),
        },
        AstNode::Paragraph("Hello world".to_string()),
        AstNode::List {
            ordered: false,
            items: vec![vec![AstNode::Text("item".to_string())]],
        },
    ];

    let actual = markdown::write_markdown(&ast, Flavor::CommonMark);

    assert_eq!(actual, "# Title\n\nHello world\n\n- item\n");
}

#[test]
fn converts_simple_html_to_markdown() {
    let converter = Converter::new().with_options(ConversionOptions {
        flavor: Flavor::Gfm,
        format: OutputFormat::Markdown,
        ..ConversionOptions::default()
    });

    let result = converter
        .convert_bytes("sample.html", b"<h1>Title</h1><p>Body</p>")
        .unwrap();

    assert_eq!(result.markdown, "# Title\n\nBody\n");
    assert_eq!(result.report.input_format, "html");
}

#[test]
fn falls_back_to_html_table_for_complex_tables() {
    let ast = vec![AstNode::Table {
        rows: vec![TableRow {
            cells: vec![TableCell {
                text: "A".to_string(),
                rowspan: 2,
                colspan: 1,
                image: None,
            }],
        }],
    }];

    let actual = markdown::write_markdown(&ast, Flavor::Gfm);

    assert!(actual.contains("<table>"));
    assert!(actual.contains("rowspan=\"2\""));
}

#[test]
fn evaluation_functions_return_structured_scores() {
    let expected = vec![AstNode::Heading {
        level: 1,
        text: "Title".to_string(),
    }];
    let actual = expected.clone();
    let markdown = "# Title\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n";

    assert_eq!(evaluate_structure_fidelity(&expected, &actual).score, 1.0);
    assert_eq!(evaluate_heading_recall(&expected, markdown).score, 1.0);
    assert_eq!(evaluate_table_integrity(markdown).score, 1.0);
    assert_eq!(evaluate_lint_score(markdown).errors, 0);
    assert_eq!(
        evaluate_translation_structure_preserve("# A\n\n- x\n", "# B\n\n- y\n").score,
        1.0
    );
}

#[test]
fn reports_unsupported_formats_without_external_calls() {
    let converter = Converter::new();
    let result = converter
        .convert_bytes("slides.pptx", b"not a real pptx")
        .unwrap();

    assert!(
        result
            .report
            .warnings
            .iter()
            .any(|warning| warning.contains("could not read"))
    );
    assert!(result.markdown.contains("Unsupported input format"));
    assert!(!result.report.used_llm);
}

#[test]
fn parses_docx_tables_images_and_captions_from_xml() {
    let xml = r#"
        <w:document><w:body>
          <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Title</w:t></w:r></w:p>
          <w:p><w:r><w:t>Figure 1: System diagram</w:t></w:r></w:p>
          <w:p><w:r><w:drawing><a:blip r:embed="rId5"/></w:drawing></w:r></w:p>
          <w:tbl>
            <w:tr>
              <w:tc><w:tcPr><w:gridSpan w:val="2"/></w:tcPr><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc>
            </w:tr>
          </w:tbl>
        </w:body></w:document>
    "#;
    let rels = r#"<Relationships>
        <Relationship Id="rId5" Target="media/image1.png"/>
    </Relationships>"#;
    let mut warnings = Vec::new();

    let ast = docx::parse_document_xml_with_rels(xml, rels, &mut warnings);
    let rendered = markdown::write_markdown(&ast, Flavor::Gfm);

    assert!(rendered.contains("# Title"));
    assert!(
        rendered
            .contains("![Figure 1: System diagram](media/image1.png \"Figure 1: System diagram\")")
    );
    assert!(rendered.contains("colspan=\"2\""));
}

#[test]
fn parses_xlsx_and_pptx_xml_to_structured_ast() {
    let shared = r#"<sst><si><t>Name</t></si><si><t>Value</t></si></sst>"#;
    let sheet = r#"<worksheet><sheetData><row><c t="s"><v>0</v></c><c><v>42</v></c></row></sheetData></worksheet>"#;
    let slide = r#"<p:sld><p:cSld><p:spTree><a:p><a:r><a:t>Slide title</a:t></a:r></a:p></p:spTree></p:cSld></p:sld>"#;

    let xlsx = office::parse_xlsx_sheet_xml(sheet, shared);
    let pptx = office::parse_pptx_slide_xml(slide);

    assert!(matches!(xlsx.first(), Some(AstNode::Table { .. })));
    assert_eq!(
        markdown::write_markdown(&pptx, Flavor::CommonMark),
        "# Slide title\n"
    );
}
