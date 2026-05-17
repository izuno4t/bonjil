use crate::AstNode;
use std::path::Path;

pub(crate) fn detect_format(input_name: &str, bytes: &[u8]) -> String {
    let ext = Path::new(input_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !ext.is_empty() {
        return match ext.as_str() {
            "htm" => "html".to_string(),
            other => other.to_string(),
        };
    }
    if bytes.starts_with(b"%PDF") {
        "pdf".to_string()
    } else if bytes.starts_with(b"<") {
        "html".to_string()
    } else {
        "unknown".to_string()
    }
}

pub(crate) fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

pub(crate) fn unsupported_node(format: &str) -> AstNode {
    AstNode::Paragraph(format!("Unsupported input format: {format}"))
}
