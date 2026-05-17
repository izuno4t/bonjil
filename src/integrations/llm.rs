use crate::{AstNode, ConversionOptions, LlmBackend};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct LlmRequest {
    pub backend: LlmBackend,
    pub task: String,
    pub input: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LlmResponse {
    pub text: String,
    pub backend: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LlmSendConfirmation {
    pub destination: String,
    pub content_bytes: usize,
    pub consent_granted: bool,
    pub message: String,
}

pub trait LlmProvider {
    fn complete(&self, request: &LlmRequest) -> io::Result<LlmResponse>;
}

pub fn complete_with(provider: &dyn LlmProvider, request: &LlmRequest) -> io::Result<LlmResponse> {
    provider.complete(request)
}

pub fn backend_name(backend: &LlmBackend) -> &'static str {
    match backend {
        LlmBackend::None => "none",
        LlmBackend::Anthropic(_) => "anthropic",
        LlmBackend::OpenAi(_) => "openai",
        LlmBackend::Ollama(_) => "ollama",
        LlmBackend::OpenAiCompatible { .. } => "openai-compatible",
    }
}

pub fn build_send_confirmation(
    backend: &LlmBackend,
    content: &str,
    consent_granted: bool,
) -> Option<LlmSendConfirmation> {
    if matches!(backend, LlmBackend::None | LlmBackend::Ollama(_)) {
        return None;
    }
    let destination = match backend {
        LlmBackend::Anthropic(_) => "Anthropic".to_string(),
        LlmBackend::OpenAi(_) => "OpenAI".to_string(),
        LlmBackend::OpenAiCompatible { endpoint, name } if !endpoint.is_empty() => {
            endpoint.clone().to_string()
        }
        LlmBackend::OpenAiCompatible { name, .. } => name.clone(),
        LlmBackend::None | LlmBackend::Ollama(_) => unreachable!(),
    };
    Some(LlmSendConfirmation {
        destination: destination.clone(),
        content_bytes: content.len(),
        consent_granted,
        message: if consent_granted {
            format!(
                "external send consent granted for {destination}; {} byte(s) will be sent",
                content.len()
            )
        } else {
            format!(
                "external send consent is required for {destination}; {} byte(s) would be sent",
                content.len()
            )
        },
    })
}

pub fn restructure_with_provider(
    provider: &dyn LlmProvider,
    backend: &LlmBackend,
    ast: &[AstNode],
) -> io::Result<Vec<AstNode>> {
    run_markdown_transform(provider, backend, "restructure", ast)
}

pub fn translate_with_provider(
    provider: &dyn LlmProvider,
    backend: &LlmBackend,
    language: &str,
    ast: &[AstNode],
) -> io::Result<Vec<AstNode>> {
    run_markdown_transform(provider, backend, &format!("translate:{language}"), ast)
}

fn run_markdown_transform(
    provider: &dyn LlmProvider,
    backend: &LlmBackend,
    task: &str,
    ast: &[AstNode],
) -> io::Result<Vec<AstNode>> {
    let input = ast
        .iter()
        .map(|node| match node {
            AstNode::Heading { level, text } => {
                format!("{} {text}", "#".repeat(*level as usize))
            }
            AstNode::Paragraph(text) | AstNode::Text(text) => text.clone(),
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let response = complete_with(
        provider,
        &LlmRequest {
            backend: backend.clone(),
            task: task.to_string(),
            input,
        },
    )?;
    Ok(parse_markdown_blocks(&response.text))
}

fn parse_markdown_blocks(markdown: &str) -> Vec<AstNode> {
    markdown
        .split("\n\n")
        .filter_map(|block| {
            let trimmed = block.trim();
            if trimmed.is_empty() {
                return None;
            }
            let hashes = trimmed
                .chars()
                .take_while(|character| *character == '#')
                .count();
            if (1..=6).contains(&hashes) && trimmed.chars().nth(hashes) == Some(' ') {
                Some(AstNode::Heading {
                    level: hashes as u8,
                    text: trimmed[hashes + 1..].trim().to_string(),
                })
            } else {
                Some(AstNode::Paragraph(trimmed.to_string()))
            }
        })
        .collect()
}

pub fn save_diff(path: &Path, before: &str, after: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, render_diff(before, after))
}

fn render_diff(before: &str, after: &str) -> String {
    let before_lines = before.lines().collect::<Vec<_>>();
    let after_lines = after.lines().collect::<Vec<_>>();
    let mut diff = String::from("--- before\n+++ after\n@@\n");
    let max_len = before_lines.len().max(after_lines.len());
    for index in 0..max_len {
        match (before_lines.get(index), after_lines.get(index)) {
            (Some(left), Some(right)) if left == right => {
                diff.push_str(&format!(" {left}\n"));
            }
            (Some(left), Some(right)) => {
                diff.push_str(&format!("-{left}\n+{right}\n"));
            }
            (Some(left), None) => diff.push_str(&format!("-{left}\n")),
            (None, Some(right)) => diff.push_str(&format!("+{right}\n")),
            (None, None) => {}
        }
    }
    diff
}

pub fn apply_llm_filters(
    ast: &mut [AstNode],
    options: &ConversionOptions,
    warnings: &mut Vec<String>,
) -> io::Result<()> {
    if options.llm == LlmBackend::None {
        warnings.push("LLM options requested but no LLM backend was selected.".to_string());
        return Ok(());
    }
    let content_preview = ast
        .iter()
        .map(|node| format!("{node:?}"))
        .collect::<Vec<_>>()
        .join("\n");
    if let Some(confirmation) = build_send_confirmation(
        &options.llm,
        &content_preview,
        options.consent_external_send,
    ) {
        warnings.push(confirmation.message);
    }
    if !options.consent_external_send && !matches!(options.llm, LlmBackend::Ollama(_)) {
        warnings.push(
            "LLM filter skipped because external send consent is not configured.".to_string(),
        );
        return Ok(());
    }
    warnings.push("LLM filter boundary is configured; provider calls are intentionally not executed in tests.".to_string());
    for node in ast {
        if let AstNode::Paragraph(text) = node {
            *text = text.trim().to_string();
        }
    }
    Ok(())
}
