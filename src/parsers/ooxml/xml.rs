pub(super) fn extract_blocks(input: &str, open: &str, close: &str) -> Vec<String> {
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

#[derive(Clone, Debug)]
pub(super) struct Element {
    pub(super) opening: String,
    pub(super) body: String,
}

pub(super) fn extract_elements(input: &str, open: &str, close: &str) -> Vec<Element> {
    let mut result = Vec::new();
    let mut rest = input;
    while let Some(start) = rest.find(open) {
        let after = &rest[start..];
        let Some(open_end) = after.find('>') else {
            break;
        };
        let opening = after[..=open_end].to_string();
        let body_start = start + open_end + 1;
        let Some(end_rel) = rest[body_start..].find(close) else {
            break;
        };
        let end = body_start + end_rel;
        result.push(Element {
            opening,
            body: rest[body_start..end].to_string(),
        });
        rest = &rest[end + close.len()..];
    }
    result
}

pub(super) fn remove_blocks(input: &str, open: &str, close: &str) -> String {
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

pub(super) fn parse_i64_attr_after(input: &str, marker: &str, name: &str) -> Option<i64> {
    attr_value_after(input, marker, name)?.parse().ok()
}

pub(super) fn attr_value_after(input: &str, marker: &str, name: &str) -> Option<String> {
    let start = input.find(marker)?;
    let rest = &input[start..];
    let end = rest.find('>')?;
    attr_value(&rest[..=end], name)
}

pub(super) fn attr_value(input: &str, name: &str) -> Option<String> {
    let pattern = format!("{name}=\"");
    let start = input.find(&pattern)? + pattern.len();
    let end = input[start..].find('"')?;
    Some(input[start..start + end].to_string())
}

pub(super) fn relationship_target(rels_xml: &str, id: &str) -> Option<String> {
    let marker = format!("Id=\"{id}\"");
    let start = rels_xml.find(&marker)?;
    attr_value(&rels_xml[start..], "Target")
}
