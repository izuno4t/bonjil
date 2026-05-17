use super::xml::{
    attr_value_after, extract_blocks, extract_elements, parse_i64_attr_after, relationship_target,
};
use crate::{AstNode, TableCell, TableRow, decode_entities, strip_tags};

pub fn parse_pptx_slide_xml(slide_xml: &str) -> Vec<AstNode> {
    parse_pptx_slide_xml_with_rels(slide_xml, "", &mut Vec::new())
}

pub fn parse_pptx_slide_xml_with_rels(
    slide_xml: &str,
    rels_xml: &str,
    warnings: &mut Vec<String>,
) -> Vec<AstNode> {
    let mut items = Vec::new();
    for shape in extract_elements(slide_xml, "<p:sp", "</p:sp>") {
        let x = parse_i64_attr_after(&shape.body, "<a:off", "x").unwrap_or(0);
        let y = parse_i64_attr_after(&shape.body, "<a:off", "y").unwrap_or(0);
        if let Some(table) = parse_drawing_table(&shape.body) {
            items.push(PositionedNode {
                x,
                y,
                node: AstNode::Table { rows: table },
                source: "table",
            });
            continue;
        }
        let paragraphs = extract_text_paragraphs(&shape.body);
        let texts = paragraphs
            .iter()
            .map(|paragraph| paragraph.text.trim().to_string())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>();
        if texts.is_empty() {
            continue;
        }
        let placeholder = attr_value_after(&shape.body, "<p:ph", "type");
        let mut nodes = if matches!(placeholder.as_deref(), Some("title" | "ctrTitle")) {
            let mut nodes = vec![AstNode::Heading {
                level: 1,
                text: texts[0].clone(),
            }];
            nodes.extend(texts.iter().skip(1).cloned().map(AstNode::Paragraph));
            nodes
        } else {
            nodes_from_pptx_body_paragraphs(&paragraphs, warnings)
        };
        for node in nodes.drain(..) {
            items.push(PositionedNode {
                x,
                y,
                node,
                source: "shape",
            });
        }
    }
    for picture in extract_elements(slide_xml, "<p:pic", "</p:pic>") {
        let x = parse_i64_attr_after(&picture.body, "<a:off", "x").unwrap_or(0);
        let y = parse_i64_attr_after(&picture.body, "<a:off", "y").unwrap_or(0);
        let embed = attr_value_after(&picture.body, "<a:blip", "r:embed");
        let path = embed
            .as_deref()
            .and_then(|id| relationship_target(rels_xml, id))
            .unwrap_or_else(|| {
                embed
                    .map(|id| format!("media/{id}.png"))
                    .unwrap_or_else(|| "media/unknown-image.png".to_string())
            });
        items.push(PositionedNode {
            x,
            y,
            node: AstNode::Image {
                alt: "slide image".to_string(),
                path,
                title: None,
            },
            source: "picture",
        });
    }
    if !items.is_empty()
        && items
            .iter()
            .all(|item| item.x == 0 && item.y == 0 && item.source == "shape")
        && !slide_xml.contains("<p:ph")
    {
        return parse_pptx_legacy_text_order(slide_xml);
    }
    if items.is_empty() {
        return parse_pptx_legacy_text_order(slide_xml);
    }
    if items.iter().any(|item| item.x != 0 || item.y != 0) {
        warnings.push("pptx visual order inferred from shape coordinates".to_string());
        items.sort_by_key(|item| (item.y, item.x));
    }
    if looks_like_pseudo_table(&items) {
        warnings
            .push("pptx shape grid looks like a pseudo table; kept as ordered text".to_string());
    }
    items.into_iter().map(|item| item.node).collect()
}

#[derive(Clone, Debug)]
struct PositionedNode {
    x: i64,
    y: i64,
    node: AstNode,
    source: &'static str,
}

fn parse_pptx_legacy_text_order(slide_xml: &str) -> Vec<AstNode> {
    let texts = extract_blocks(slide_xml, "<a:t", "</a:t>")
        .into_iter()
        .map(|text| decode_entities(&strip_tags(&text)).trim().to_string())
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>();
    texts
        .into_iter()
        .enumerate()
        .map(|(index, text)| {
            if index == 0 {
                AstNode::Heading { level: 1, text }
            } else {
                AstNode::Paragraph(text)
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
struct TextParagraph {
    text: String,
    list_kind: Option<ListKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ListKind {
    Ordered,
    Unordered,
}

fn extract_text_paragraphs(shape_body: &str) -> Vec<TextParagraph> {
    let paragraphs = extract_blocks(shape_body, "<a:p", "</a:p>");
    if paragraphs.is_empty() {
        return extract_blocks(shape_body, "<a:t", "</a:t>")
            .into_iter()
            .map(|text| TextParagraph {
                text: decode_entities(&strip_tags(&text)),
                list_kind: None,
            })
            .collect();
    }
    paragraphs
        .into_iter()
        .map(|paragraph| {
            let text = extract_blocks(&paragraph, "<a:t", "</a:t>")
                .into_iter()
                .map(|text| decode_entities(&strip_tags(&text)))
                .collect::<Vec<_>>()
                .join("");
            TextParagraph {
                text,
                list_kind: pptx_list_kind(&paragraph),
            }
        })
        .collect()
}

fn nodes_from_pptx_body_paragraphs(
    paragraphs: &[TextParagraph],
    warnings: &mut Vec<String>,
) -> Vec<AstNode> {
    let mut nodes = Vec::new();
    let mut pending_items = Vec::new();
    let mut pending_kind = None;

    for paragraph in paragraphs {
        let text = paragraph.text.trim();
        if text.is_empty() {
            continue;
        }
        if let Some(kind) = paragraph.list_kind {
            if pending_kind.is_some_and(|current| current != kind) {
                flush_pptx_list(&mut pending_items, pending_kind, &mut nodes, warnings);
            }
            pending_kind = Some(kind);
            pending_items.push(vec![AstNode::Text(text.to_string())]);
        } else {
            flush_pptx_list(&mut pending_items, pending_kind, &mut nodes, warnings);
            nodes.push(AstNode::Paragraph(text.to_string()));
        }
    }

    flush_pptx_list(&mut pending_items, pending_kind, &mut nodes, warnings);
    if nodes.is_empty() {
        let text = paragraphs
            .iter()
            .map(|paragraph| paragraph.text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if !text.is_empty() {
            nodes.push(AstNode::Paragraph(text));
        }
    }
    nodes
}

fn flush_pptx_list(
    pending_items: &mut Vec<Vec<AstNode>>,
    pending_kind: Option<ListKind>,
    nodes: &mut Vec<AstNode>,
    warnings: &mut Vec<String>,
) {
    if pending_items.is_empty() {
        return;
    }
    if pending_items.len() == 1 {
        let text = pending_items
            .pop()
            .and_then(|mut item| item.pop())
            .and_then(|node| match node {
                AstNode::Text(text) => Some(text),
                _ => None,
            })
            .unwrap_or_default();
        nodes.push(AstNode::Paragraph(text));
        return;
    }
    warnings.push("pptx list structure restored from paragraph properties".to_string());
    nodes.push(AstNode::List {
        ordered: pending_kind == Some(ListKind::Ordered),
        items: std::mem::take(pending_items),
    });
}

fn pptx_list_kind(paragraph: &str) -> Option<ListKind> {
    if paragraph.contains("<a:buAutoNum") {
        Some(ListKind::Ordered)
    } else if paragraph.contains("<a:buChar") || paragraph.contains("<a:buBlip") {
        Some(ListKind::Unordered)
    } else {
        None
    }
}

fn parse_drawing_table(shape_body: &str) -> Option<Vec<TableRow>> {
    let table = extract_blocks(shape_body, "<a:tbl", "</a:tbl>")
        .into_iter()
        .next()?;
    let rows = extract_blocks(&table, "<a:tr", "</a:tr>")
        .into_iter()
        .map(|row| TableRow {
            cells: extract_blocks(&row, "<a:tc", "</a:tc>")
                .into_iter()
                .map(|cell| TableCell {
                    text: extract_blocks(&cell, "<a:t", "</a:t>")
                        .into_iter()
                        .map(|text| decode_entities(&strip_tags(&text)))
                        .collect::<Vec<_>>()
                        .join(""),
                    rowspan: 1,
                    colspan: 1,
                    image: None,
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    Some(rows)
}

fn looks_like_pseudo_table(items: &[PositionedNode]) -> bool {
    let shape_items = items
        .iter()
        .filter(|item| item.source == "shape" && matches!(item.node, AstNode::Paragraph(_)))
        .collect::<Vec<_>>();
    if shape_items.len() < 4 {
        return false;
    }
    let mut xs = shape_items.iter().map(|item| item.x).collect::<Vec<_>>();
    let mut ys = shape_items.iter().map(|item| item.y).collect::<Vec<_>>();
    xs.sort();
    xs.dedup();
    ys.sort();
    ys.dedup();
    xs.len() >= 2 && ys.len() >= 2
}
