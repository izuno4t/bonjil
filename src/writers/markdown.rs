use crate::{AstNode, Flavor, TableRow, escape_html};

pub fn write_markdown(ast: &[AstNode], flavor: Flavor) -> String {
    let mut output = String::new();
    let mut heading_state = HeadingState::default();
    let mut nodes = ast.iter().peekable();
    if let Some(AstNode::Paragraph(text)) = nodes.peek()
        && should_promote_first_paragraph(text)
    {
        output.push_str("# ");
        output.push_str(&normalize_inline_text(text));
        output.push_str("\n\n");
        heading_state.h1_written = true;
        nodes.next();
    }
    for node in nodes {
        write_node(node, flavor, &mut output, 0, &mut heading_state);
    }
    while output.ends_with("\n\n") {
        output.pop();
    }
    output
}

#[derive(Default)]
struct HeadingState {
    h1_written: bool,
}

fn write_node(
    node: &AstNode,
    flavor: Flavor,
    output: &mut String,
    depth: usize,
    heading_state: &mut HeadingState,
) {
    match node {
        AstNode::Heading { level, text } => {
            let normalized_level = normalize_heading_level(*level, heading_state);
            output.push_str(&"#".repeat(normalized_level as usize));
            output.push(' ');
            output.push_str(&normalize_inline_text(text));
            output.push_str("\n\n");
        }
        AstNode::Paragraph(text) => {
            write_wrapped_text(&normalize_inline_text(text), output);
            output.push_str("\n\n");
        }
        AstNode::Text(text) => output.push_str(&normalize_inline_text(text)),
        AstNode::List { ordered, items } => {
            for (index, item) in items.iter().enumerate() {
                output.push_str(&"  ".repeat(depth));
                if *ordered {
                    output.push_str(&format!("{}. ", index + 1));
                } else {
                    output.push_str("- ");
                }
                write_inline_nodes(item, flavor, output, heading_state);
                output.push('\n');
            }
            output.push('\n');
        }
        AstNode::Table { rows } => write_table(rows, flavor, output),
        AstNode::Image { alt, path, title } => {
            output.push_str("![");
            output.push_str(alt);
            output.push_str("](");
            output.push_str(path);
            if let Some(title) = title {
                output.push_str(" \"");
                output.push_str(title);
                output.push('"');
            }
            output.push_str(")\n\n");
        }
        AstNode::CodeBlock { language, code } => {
            output.push_str("```");
            output.push_str(language.as_deref().unwrap_or(""));
            output.push('\n');
            output.push_str(code.trim_end());
            output.push_str("\n```\n\n");
        }
        AstNode::Footnote { label, text } => {
            output.push_str("[^");
            output.push_str(label);
            output.push_str("]: ");
            write_wrapped_text(&normalize_inline_text(text), output);
            output.push_str("\n\n");
        }
        AstNode::RawHtml(html) => {
            output.push_str(html.trim());
            output.push('\n');
        }
    }
}

fn normalize_heading_level(level: u8, heading_state: &mut HeadingState) -> u8 {
    let level = level.clamp(1, 6);
    if level == 1 {
        if heading_state.h1_written {
            2
        } else {
            heading_state.h1_written = true;
            1
        }
    } else {
        level
    }
}

fn write_inline_nodes(
    nodes: &[AstNode],
    flavor: Flavor,
    output: &mut String,
    heading_state: &mut HeadingState,
) {
    for node in nodes {
        match node {
            AstNode::Text(text) | AstNode::Paragraph(text) => {
                output.push_str(&normalize_inline_text(text));
            }
            _ => write_node(node, flavor, output, 1, heading_state),
        }
    }
}

fn should_promote_first_paragraph(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= 80
        && !trimmed.starts_with("Unsupported input format:")
        && !trimmed.ends_with('。')
        && !trimmed.ends_with('.')
        && !trimmed.ends_with(',')
        && !trimmed.ends_with('、')
}

fn write_wrapped_text(text: &str, output: &mut String) {
    let mut line = String::new();
    for token in text.split_whitespace() {
        if line.is_empty() {
            push_wrapped_token(token, output, &mut line);
        } else if line.chars().count() + 1 + token.chars().count() <= 80 {
            line.push(' ');
            line.push_str(token);
        } else {
            output.push_str(line.trim_end());
            output.push('\n');
            line.clear();
            push_wrapped_token(token, output, &mut line);
        }
    }
    if !line.is_empty() {
        output.push_str(line.trim_end());
    }
}

fn push_wrapped_token(token: &str, output: &mut String, line: &mut String) {
    if token.chars().count() <= 80 {
        line.push_str(token);
        return;
    }
    for character in token.chars() {
        if line.chars().count() >= 80 {
            output.push_str(line);
            output.push('\n');
            line.clear();
        }
        line.push(character);
    }
}

fn normalize_inline_text(text: &str) -> String {
    text.split_whitespace()
        .map(angle_bracket_bare_link)
        .collect::<Vec<_>>()
        .join(" ")
}

fn angle_bracket_bare_link(token: &str) -> String {
    let (prefix, core, suffix) = split_surrounding_punctuation(token);
    if core.starts_with('<') || core.starts_with('[') || core.starts_with("](") {
        return token.to_string();
    }
    if core.starts_with("http://") || core.starts_with("https://") || looks_like_email(core) {
        format!("{prefix}<{core}>{suffix}")
    } else {
        token.to_string()
    }
}

fn split_surrounding_punctuation(token: &str) -> (&str, &str, &str) {
    let prefix_len = token
        .char_indices()
        .find(|(_, character)| character.is_alphanumeric() || *character == 'h')
        .map(|(index, _)| index)
        .unwrap_or(0);
    let mut suffix_start = token.len();
    for (index, character) in token.char_indices().rev() {
        if character.is_alphanumeric() || matches!(character, '/' | '-') {
            suffix_start = index + character.len_utf8();
            break;
        }
    }
    if prefix_len >= suffix_start {
        return ("", token, "");
    }
    (
        &token[..prefix_len],
        &token[prefix_len..suffix_start],
        &token[suffix_start..],
    )
}

fn looks_like_email(token: &str) -> bool {
    let Some((local, domain)) = token.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && domain
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
}

fn write_table(rows: &[TableRow], flavor: Flavor, output: &mut String) {
    let requires_html = rows.iter().flat_map(|row| &row.cells).any(|cell| {
        cell.rowspan > 1 || cell.colspan > 1 || cell.image.is_some() || cell.text.contains('\n')
    }) || matches!(flavor, Flavor::CommonMark | Flavor::HedgeDoc);
    if requires_html {
        write_html_table(rows, output);
        return;
    }

    if rows.is_empty() {
        return;
    }
    let header = &rows[0];
    output.push('|');
    for cell in &header.cells {
        output.push(' ');
        output.push_str(&cell.text);
        output.push_str(" |");
    }
    output.push('\n');
    output.push('|');
    for _ in &header.cells {
        output.push_str(" --- |");
    }
    output.push('\n');
    for row in rows.iter().skip(1) {
        output.push('|');
        for cell in &row.cells {
            output.push(' ');
            output.push_str(&cell.text);
            output.push_str(" |");
        }
        output.push('\n');
    }
    output.push('\n');
}

fn write_html_table(rows: &[TableRow], output: &mut String) {
    output.push_str("<table>\n");
    for row in rows {
        output.push_str("<tr>");
        for cell in &row.cells {
            output.push_str("<td");
            if cell.rowspan > 1 {
                output.push_str(&format!(" rowspan=\"{}\"", cell.rowspan));
            }
            if cell.colspan > 1 {
                output.push_str(&format!(" colspan=\"{}\"", cell.colspan));
            }
            output.push('>');
            if let Some(path) = &cell.image {
                output.push_str("<img src=\"");
                output.push_str(&escape_html(path));
                output.push_str("\" alt=\"");
                output.push_str(&escape_html(&cell.text));
                output.push_str("\">");
            } else {
                output.push_str(&escape_html(&cell.text));
            }
            output.push_str("</td>");
        }
        output.push_str("</tr>\n");
    }
    output.push_str("</table>\n\n");
}
