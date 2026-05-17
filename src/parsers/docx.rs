use crate::{AstNode, TableCell, TableRow, decode_entities, strip_tags};

pub fn parse_document_xml(xml: &str, warnings: &mut Vec<String>) -> Vec<AstNode> {
    parse_document_xml_with_rels(xml, "", warnings)
}

pub fn parse_document_xml_with_rels(
    xml: &str,
    rels_xml: &str,
    warnings: &mut Vec<String>,
) -> Vec<AstNode> {
    parse_document_xml_with_rels_and_notes(xml, rels_xml, "", "", warnings)
}

pub fn parse_document_xml_with_rels_and_notes(
    xml: &str,
    rels_xml: &str,
    footnotes_xml: &str,
    comments_xml: &str,
    warnings: &mut Vec<String>,
) -> Vec<AstNode> {
    let mut ast = Vec::new();
    let mut pending_caption: Option<String> = None;
    let body_without_tables = remove_blocks(xml, "<w:tbl", "</w:tbl>");
    for paragraph in extract_blocks(&body_without_tables, "<w:p", "</w:p>") {
        let style = extract_style(&paragraph);
        let text = extract_text_with_relationships(&paragraph, rels_xml);
        if paragraph.contains("<w:drawing") {
            let target = extract_embed_id(&paragraph)
                .and_then(|id| relationship_target(rels_xml, &id))
                .unwrap_or_else(|| "media/unknown-image.png".to_string());
            let alt = pending_caption
                .clone()
                .unwrap_or_else(|| "image".to_string());
            ast.push(AstNode::Image {
                alt,
                path: target,
                title: pending_caption.take(),
            });
            continue;
        }
        if text.trim().is_empty() {
            continue;
        }
        if let Some(level) = heading_level(style.as_deref()) {
            ast.push(AstNode::Heading { level, text });
        } else if paragraph.contains("<w:numPr") {
            ast.push(AstNode::List {
                ordered: false,
                items: vec![vec![AstNode::Text(text)]],
            });
        } else {
            if is_caption(&text) {
                pending_caption = Some(text.clone());
            }
            if let Some(style) = style.as_deref()
                && !is_known_paragraph_style(style)
            {
                warnings.push(format!("unmapped docx paragraph style: {style}"));
            }
            ast.push(AstNode::Paragraph(text));
        }
        for id in extract_reference_ids(&paragraph, "w:footnoteReference") {
            if let Some(text) = note_text_by_id(footnotes_xml, "w:footnote", &id) {
                ast.push(AstNode::Footnote { label: id, text });
            }
        }
        for id in extract_reference_ids(&paragraph, "w:commentReference") {
            if let Some(text) = note_text_by_id(comments_xml, "w:comment", &id) {
                ast.push(AstNode::Footnote {
                    label: format!("comment:{id}"),
                    text,
                });
            }
        }
    }
    for table in extract_blocks(xml, "<w:tbl", "</w:tbl>") {
        ast.push(AstNode::Table {
            rows: parse_table(&table),
        });
    }
    if ast.is_empty() {
        warnings.push("DOCX document.xml contained no supported paragraphs.".to_string());
    }
    ast
}

fn extract_blocks(input: &str, open: &str, close: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut rest = input;
    while let Some(start) = rest.find(open) {
        let after = &rest[start..];
        let Some(open_end) = after.find('>') else {
            break;
        };
        let body_start = start + open_end + 1;
        let Some(end_rel) = rest[body_start..].find(close) else {
            break;
        };
        let end = body_start + end_rel;
        result.push(rest[body_start..end].to_string());
        rest = &rest[end + close.len()..];
    }
    result
}

fn extract_text(paragraph: &str) -> String {
    extract_blocks(paragraph, "<w:t", "</w:t>")
        .into_iter()
        .map(|part| decode_entities(&strip_tags(&part)))
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string()
}

fn extract_text_with_relationships(paragraph: &str, rels_xml: &str) -> String {
    let mut text = extract_text(paragraph);
    for id in extract_reference_ids(paragraph, "w:hyperlink")
        .into_iter()
        .chain(extract_attr_values_for_tag(
            paragraph,
            "w:hyperlink",
            "r:id",
        ))
    {
        let Some(target) = relationship_target(rels_xml, &id) else {
            continue;
        };
        if !text.contains(&target) {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push('<');
            text.push_str(&target);
            text.push('>');
        }
    }
    text
}

fn extract_style(paragraph: &str) -> Option<String> {
    let marker = "w:pStyle";
    let start = paragraph.find(marker)?;
    let rest = &paragraph[start..];
    let attr = "w:val=\"";
    let value_start = rest.find(attr)? + attr.len();
    let value_end = rest[value_start..].find('"')?;
    Some(rest[value_start..value_start + value_end].to_string())
}

fn heading_level(style: Option<&str>) -> Option<u8> {
    let style = style?.to_ascii_lowercase();
    for level in 1..=6 {
        if style.contains(&format!("heading{level}")) || style.contains(&format!("見出し{level}"))
        {
            return Some(level);
        }
    }
    None
}

fn parse_table(table: &str) -> Vec<TableRow> {
    extract_blocks(table, "<w:tr", "</w:tr>")
        .into_iter()
        .map(|row| TableRow {
            cells: extract_blocks(&row, "<w:tc", "</w:tc>")
                .into_iter()
                .map(|cell| TableCell {
                    text: extract_text(&cell),
                    rowspan: if cell.contains("<w:vMerge") { 2 } else { 1 },
                    colspan: extract_grid_span(&cell).unwrap_or(1),
                    image: extract_embed_id(&cell).map(|id| format!("media/{id}.png")),
                })
                .collect(),
        })
        .collect()
}

fn extract_grid_span(cell: &str) -> Option<usize> {
    let marker = "w:gridSpan";
    let start = cell.find(marker)?;
    let rest = &cell[start..];
    let attr = "w:val=\"";
    let value_start = rest.find(attr)? + attr.len();
    let value_end = rest[value_start..].find('"')?;
    rest[value_start..value_start + value_end].parse().ok()
}

fn extract_embed_id(input: &str) -> Option<String> {
    let attr = "r:embed=\"";
    let value_start = input.find(attr)? + attr.len();
    let value_end = input[value_start..].find('"')?;
    Some(input[value_start..value_start + value_end].to_string())
}

fn relationship_target(rels_xml: &str, id: &str) -> Option<String> {
    let marker = format!("Id=\"{id}\"");
    let start = rels_xml.find(&marker)?;
    let rest = &rels_xml[start..];
    let attr = "Target=\"";
    let value_start = rest.find(attr)? + attr.len();
    let value_end = rest[value_start..].find('"')?;
    Some(rest[value_start..value_start + value_end].to_string())
}

fn extract_reference_ids(paragraph: &str, tag: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = paragraph;
    let marker = format!("<{tag} ");
    while let Some(start) = rest.find(&marker) {
        let after = &rest[start..];
        let Some(end) = after.find('>') else {
            break;
        };
        if let Some(id) = attr_value(&after[..=end], "w:id") {
            ids.push(id);
        }
        rest = &after[end + 1..];
    }
    ids
}

fn note_text_by_id(notes_xml: &str, tag: &str, id: &str) -> Option<String> {
    let marker = format!("<{tag} ");
    let close = format!("</{tag}>");
    let mut rest = notes_xml;
    while let Some(start) = rest.find(&marker) {
        let after = &rest[start..];
        let Some(open_end) = after.find('>') else {
            break;
        };
        let opening = &after[..=open_end];
        let body_start = start + open_end + 1;
        let Some(end_rel) = rest[body_start..].find(&close) else {
            break;
        };
        let end = body_start + end_rel;
        if attr_value(opening, "w:id").as_deref() == Some(id) {
            let text = extract_text(&rest[body_start..end]);
            return (!text.trim().is_empty()).then_some(text);
        }
        rest = &rest[end + close.len()..];
    }
    None
}

fn extract_attr_values_for_tag(input: &str, tag: &str, attr: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = input;
    let marker = format!("<{tag}");
    while let Some(start) = rest.find(&marker) {
        let after = &rest[start..];
        let Some(end) = after.find('>') else {
            break;
        };
        if let Some(value) = attr_value(&after[..=end], attr) {
            values.push(value);
        }
        rest = &after[end + 1..];
    }
    values
}

fn attr_value(input: &str, name: &str) -> Option<String> {
    let pattern = format!("{name}=\"");
    let value_start = input.find(&pattern)? + pattern.len();
    let value_end = input[value_start..].find('"')?;
    Some(input[value_start..value_start + value_end].to_string())
}

fn remove_blocks(input: &str, open: &str, close: &str) -> String {
    let mut output = String::new();
    let mut rest = input;
    while let Some(start) = rest.find(open) {
        output.push_str(&rest[..start]);
        let Some(end_rel) = rest[start..].find(close) else {
            return output;
        };
        rest = &rest[start + end_rel + close.len()..];
    }
    output.push_str(rest);
    output
}

fn is_caption(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    lower.starts_with("figure ")
        || lower.starts_with("fig. ")
        || lower.starts_with("図 ")
        || lower.starts_with("図表")
}

fn is_known_paragraph_style(style: &str) -> bool {
    heading_level(Some(style)).is_some() || style.eq_ignore_ascii_case("caption")
}
