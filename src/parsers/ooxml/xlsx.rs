use super::xml::{attr_value, extract_blocks, extract_elements, remove_blocks};
use crate::{AstNode, TableCell, TableRow, decode_entities, strip_tags};

pub fn parse_xlsx_sheet_xml(sheet_xml: &str, shared_strings_xml: &str) -> Vec<AstNode> {
    parse_xlsx_sheet_xml_with_warnings(sheet_xml, shared_strings_xml, &mut Vec::new())
}

pub fn parse_xlsx_sheet_xml_with_warnings(
    sheet_xml: &str,
    shared_strings_xml: &str,
    warnings: &mut Vec<String>,
) -> Vec<AstNode> {
    let shared_strings = extract_blocks(shared_strings_xml, "<si", "</si>")
        .into_iter()
        .map(|item| {
            let without_phonetics = remove_blocks(&item, "<rPh", "</rPh>");
            decode_entities(&strip_tags(&without_phonetics))
                .trim()
                .to_string()
        })
        .collect::<Vec<_>>();
    let merged_cells = parse_merge_cells(sheet_xml);
    let mut rows = extract_blocks(sheet_xml, "<row", "</row>")
        .into_iter()
        .map(|row| TableRow {
            cells: extract_elements(&row, "<c", "</c>")
                .into_iter()
                .map(|cell| {
                    let value = extract_blocks(&cell.body, "<v", "</v>")
                        .first()
                        .map(|value| decode_entities(&strip_tags(value)).trim().to_string())
                        .unwrap_or_default();
                    let inline_text = extract_blocks(&cell.body, "<t", "</t>")
                        .first()
                        .map(|value| decode_entities(&strip_tags(value)).trim().to_string());
                    let text = if cell.opening.contains("t=\"s\"") {
                        value
                            .parse::<usize>()
                            .ok()
                            .and_then(|index| shared_strings.get(index).cloned())
                            .unwrap_or(value)
                    } else if cell.opening.contains("t=\"inlineStr\"") {
                        inline_text.unwrap_or(value)
                    } else {
                        value
                    };
                    if cell.body.contains("<f") {
                        warnings.push(format!(
                            "xlsx formula cell {} emitted cached display value",
                            attr_value(&cell.opening, "r").unwrap_or_else(|| "?".to_string())
                        ));
                    }
                    let reference = attr_value(&cell.opening, "r");
                    let span = reference.as_deref().and_then(|reference| {
                        merged_cells.iter().find(|merge| merge.start == reference)
                    });
                    TableCell {
                        text,
                        rowspan: span.map(|merge| merge.rowspan).unwrap_or(1),
                        colspan: span.map(|merge| merge.colspan).unwrap_or(1),
                        image: None,
                    }
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    trim_empty_table_edges(&mut rows);
    if !merged_cells.is_empty() {
        warnings.push(format!(
            "xlsx mergeCells expanded {} merged range(s)",
            merged_cells.len()
        ));
    }
    vec![AstNode::Table { rows }]
}

fn trim_empty_table_edges(rows: &mut Vec<TableRow>) {
    while rows.first().is_some_and(row_is_empty) {
        rows.remove(0);
    }
    while rows.last().is_some_and(row_is_empty) {
        rows.pop();
    }
    let Some(max_columns) = rows.iter().map(|row| row.cells.len()).max() else {
        return;
    };
    let first_non_empty = (0..max_columns).find(|column| column_has_text(rows, *column));
    let last_non_empty = (0..max_columns)
        .rev()
        .find(|column| column_has_text(rows, *column));
    let (Some(first), Some(last)) = (first_non_empty, last_non_empty) else {
        rows.clear();
        return;
    };
    for row in rows {
        row.cells = row
            .cells
            .iter()
            .skip(first)
            .take(last - first + 1)
            .cloned()
            .collect();
    }
}

fn row_is_empty(row: &TableRow) -> bool {
    row.cells.iter().all(|cell| cell.text.trim().is_empty())
}

fn column_has_text(rows: &[TableRow], column: usize) -> bool {
    rows.iter().any(|row| {
        row.cells
            .get(column)
            .is_some_and(|cell| !cell.text.trim().is_empty())
    })
}

#[derive(Clone, Debug)]
struct MergeRange {
    start: String,
    rowspan: usize,
    colspan: usize,
}

fn parse_merge_cells(sheet_xml: &str) -> Vec<MergeRange> {
    let mut ranges = Vec::new();
    let mut rest = sheet_xml;
    while let Some(start) = rest.find("<mergeCell") {
        let after = &rest[start..];
        let Some(end) = after.find('>') else {
            break;
        };
        let tag = &after[..=end];
        if let Some(reference) = attr_value(tag, "ref")
            && let Some(range) = parse_merge_range(&reference)
        {
            ranges.push(range);
        }
        rest = &after[end + 1..];
    }
    ranges
}

fn parse_merge_range(reference: &str) -> Option<MergeRange> {
    let (start, end) = reference.split_once(':')?;
    let (start_col, start_row) = split_cell_reference(start)?;
    let (end_col, end_row) = split_cell_reference(end)?;
    Some(MergeRange {
        start: start.to_string(),
        rowspan: end_row.saturating_sub(start_row) + 1,
        colspan: end_col.saturating_sub(start_col) + 1,
    })
}

fn split_cell_reference(reference: &str) -> Option<(usize, usize)> {
    let letters = reference
        .chars()
        .take_while(|character| character.is_ascii_alphabetic())
        .collect::<String>();
    let digits = reference
        .chars()
        .skip_while(|character| character.is_ascii_alphabetic())
        .collect::<String>();
    Some((column_number(&letters)?, digits.parse().ok()?))
}

fn column_number(column: &str) -> Option<usize> {
    let mut value = 0usize;
    for character in column.chars() {
        let upper = character.to_ascii_uppercase();
        if !upper.is_ascii_uppercase() {
            return None;
        }
        value = value * 26 + (upper as usize - 'A' as usize + 1);
    }
    Some(value)
}
