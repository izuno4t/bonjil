use crate::{AstNode, TableCell, TableRow, decode_entities, strip_tags};

pub fn parse_html(input: &str, warnings: &mut Vec<String>) -> Vec<AstNode> {
    let mut ast = Vec::new();
    let sanitized = remove_block(input, "script");
    let sanitized = remove_block(&sanitized, "style");
    let mut rest = sanitized.as_str();
    while let Some((tag, start)) = find_next_supported_tag(rest) {
        rest = &rest[start..];
        if tag == "img" {
            let Some((opening_tag, after)) = extract_opening_tag(rest) else {
                break;
            };
            if let Some(path) = attr_string(&opening_tag, "src") {
                let title = attr_string(&opening_tag, "title");
                if title.is_none() {
                    warnings.push(format!("image caption inference failed: {path}"));
                }
                ast.push(AstNode::Image {
                    alt: attr_string(&opening_tag, "alt").unwrap_or_default(),
                    path,
                    title,
                });
            }
            rest = after;
            continue;
        }
        let Some((body, after)) = extract_first_tag(rest, tag) else {
            break;
        };
        match tag {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                ast.push(AstNode::Heading {
                    level: tag[1..].parse().unwrap_or(1),
                    text: decode_entities(&strip_tags(&body)),
                });
            }
            "p" => ast.push(AstNode::Paragraph(decode_entities(&strip_tags(&body)))),
            "ul" | "ol" => {
                let list_items = extract_tag(&body, "li")
                    .into_iter()
                    .map(|item| vec![AstNode::Text(decode_entities(&strip_tags(&item)))])
                    .collect::<Vec<_>>();
                if !list_items.is_empty() {
                    ast.push(AstNode::List {
                        ordered: tag == "ol",
                        items: list_items,
                    });
                }
            }
            "pre" => ast.push(AstNode::CodeBlock {
                language: Some("text".to_string()),
                code: decode_entities(&strip_tags(&body)),
            }),
            "table" => ast.push(AstNode::Table {
                rows: parse_table(&body),
            }),
            _ => {}
        }
        rest = after;
    }
    if ast.is_empty() {
        warnings
            .push("HTML parser found no structural nodes; emitted plain paragraph.".to_string());
        let text = decode_entities(&strip_tags(&sanitized));
        if !text.trim().is_empty() {
            ast.push(AstNode::Paragraph(text));
        }
    }
    ast
}

fn find_next_supported_tag(input: &str) -> Option<(&'static str, usize)> {
    [
        "h1", "h2", "h3", "h4", "h5", "h6", "p", "ul", "ol", "pre", "table", "img",
    ]
    .into_iter()
    .filter_map(|tag| find_tag_start(input, tag).map(|index| (tag, index)))
    .min_by_key(|(_, index)| *index)
}

fn find_tag_start(input: &str, tag: &str) -> Option<usize> {
    let lower = input.to_ascii_lowercase();
    let needle = format!("<{tag}");
    let mut offset = 0;
    while let Some(found) = lower[offset..].find(&needle) {
        let index = offset + found;
        let boundary_index = index + needle.len();
        let boundary = lower.as_bytes().get(boundary_index).copied();
        if matches!(
            boundary,
            Some(b'>') | Some(b' ') | Some(b'\n') | Some(b'\t') | Some(b'/')
        ) {
            return Some(index);
        }
        offset = boundary_index;
    }
    None
}

fn extract_first_tag<'a>(input: &'a str, tag: &str) -> Option<(String, &'a str)> {
    let open_prefix = format!("<{tag}");
    let close = format!("</{tag}>");
    let lower = input.to_ascii_lowercase();
    let start = lower.find(&open_prefix)?;
    let after_open = &input[start..];
    let open_end = after_open.find('>')?;
    let body_start = start + open_end + 1;
    let close_start_rel = input[body_start..].to_ascii_lowercase().find(&close)?;
    let close_start = body_start + close_start_rel;
    let after = &input[close_start + close.len()..];
    Some((input[body_start..close_start].to_string(), after))
}

fn extract_opening_tag(input: &str) -> Option<(String, &str)> {
    let end = input.find('>')?;
    Some((input[..=end].to_string(), &input[end + 1..]))
}

fn parse_table(table: &str) -> Vec<TableRow> {
    extract_tag(table, "tr")
        .into_iter()
        .map(|row| {
            let mut cells = extract_tag(&row, "th");
            cells.extend(extract_tag(&row, "td"));
            TableRow {
                cells: cells
                    .into_iter()
                    .map(|cell| TableCell {
                        text: decode_entities(&strip_tags(&cell)),
                        rowspan: attr_usize(&cell, "rowspan").unwrap_or(1),
                        colspan: attr_usize(&cell, "colspan").unwrap_or(1),
                        image: attr_string(&cell, "src"),
                    })
                    .collect(),
            }
        })
        .collect()
}

fn extract_tag(input: &str, tag: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut rest = input;
    let open_prefix = format!("<{tag}");
    let close = format!("</{tag}>");
    while let Some(start) = rest.to_ascii_lowercase().find(&open_prefix) {
        let after_open = &rest[start..];
        let Some(open_end) = after_open.find('>') else {
            break;
        };
        let body_start = start + open_end + 1;
        let lower_after = rest[body_start..].to_ascii_lowercase();
        let Some(close_start_rel) = lower_after.find(&close) else {
            break;
        };
        let close_start = body_start + close_start_rel;
        result.push(rest[body_start..close_start].to_string());
        rest = &rest[close_start + close.len()..];
    }
    result
}

fn remove_block(input: &str, tag: &str) -> String {
    let mut output = String::new();
    let mut rest = input;
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    loop {
        let lower = rest.to_ascii_lowercase();
        let Some(start) = lower.find(&open) else {
            output.push_str(rest);
            break;
        };
        output.push_str(&rest[..start]);
        let Some(end_rel) = lower[start..].find(&close) else {
            break;
        };
        rest = &rest[start + end_rel + close.len()..];
    }
    output
}

fn attr_usize(input: &str, name: &str) -> Option<usize> {
    attr_string(input, name)?.parse().ok()
}

fn attr_string(input: &str, name: &str) -> Option<String> {
    let pattern = format!("{name}=\"");
    let start = input.find(&pattern)? + pattern.len();
    let end = input[start..].find('"')?;
    Some(input[start..start + end].to_string())
}
