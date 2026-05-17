pub mod markdown;

use crate::{AstNode, ConversionOptions, Flavor, OutputFormat, escape_html};

pub(crate) fn render(ast: &[AstNode], options: &ConversionOptions) -> String {
    match options.format {
        OutputFormat::Markdown | OutputFormat::Mdx => markdown::write_markdown(ast, options.flavor),
        OutputFormat::Html => ast_to_html(ast),
    }
}

fn ast_to_html(ast: &[AstNode]) -> String {
    let mut output = String::new();
    for node in ast {
        match node {
            AstNode::Heading { level, text } => {
                output.push_str(&format!("<h{level}>{}</h{level}>\n", escape_html(text)));
            }
            AstNode::Paragraph(text) => output.push_str(&format!("<p>{}</p>\n", escape_html(text))),
            AstNode::Text(text) => output.push_str(&escape_html(text)),
            _ => output.push_str(&markdown::write_markdown(
                std::slice::from_ref(node),
                Flavor::Gfm,
            )),
        }
    }
    output
}
