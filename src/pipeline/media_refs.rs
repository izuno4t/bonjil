use crate::{AstNode, MediaCandidate};

pub(crate) fn collect_media_paths(ast: &[AstNode]) -> Vec<String> {
    let mut media = Vec::new();
    for node in ast {
        match node {
            AstNode::Image { path, .. } => media.push(path.clone()),
            AstNode::Table { rows } => {
                for row in rows {
                    for cell in &row.cells {
                        if let Some(image) = &cell.image {
                            media.push(image.clone());
                        }
                    }
                }
            }
            AstNode::List { items, .. } => {
                for item in items {
                    media.extend(collect_media_paths(item));
                }
            }
            _ => {}
        }
    }
    media
}

pub(crate) fn collect_media_candidates(ast: &[AstNode]) -> Vec<MediaCandidate> {
    let mut candidates = Vec::new();
    collect_media_candidates_into(ast, &mut candidates);
    candidates
}

fn collect_media_candidates_into(ast: &[AstNode], candidates: &mut Vec<MediaCandidate>) {
    for node in ast {
        match node {
            AstNode::Image { path, title, .. } => {
                let caption = title
                    .as_ref()
                    .filter(|title| !title.trim().is_empty())
                    .cloned();
                candidates.push(MediaCandidate {
                    media_id: path.clone(),
                    path: path.clone(),
                    source: if caption.is_some() {
                        "image-title".to_string()
                    } else {
                        "image-reference".to_string()
                    },
                    confidence: if caption.is_some() { 1.0 } else { 0.0 },
                    caption,
                });
            }
            AstNode::Table { rows } => {
                for row in rows {
                    for cell in &row.cells {
                        if let Some(path) = &cell.image {
                            let caption = (!cell.text.trim().is_empty()).then(|| cell.text.clone());
                            candidates.push(MediaCandidate {
                                media_id: path.clone(),
                                path: path.clone(),
                                source: "table-cell-image".to_string(),
                                confidence: if caption.is_some() { 0.8 } else { 0.0 },
                                caption,
                            });
                        }
                    }
                }
            }
            AstNode::List { items, .. } => {
                for item in items {
                    collect_media_candidates_into(item, candidates);
                }
            }
            _ => {}
        }
    }
}
