use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub enum AstNode {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    List {
        ordered: bool,
        items: Vec<Vec<AstNode>>,
    },
    Text(String),
    Table {
        rows: Vec<TableRow>,
    },
    Image {
        alt: String,
        path: String,
        title: Option<String>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Footnote {
        label: String,
        text: String,
    },
    RawHtml(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableCell {
    pub text: String,
    pub rowspan: usize,
    pub colspan: usize,
    pub image: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Flavor {
    CommonMark,
    Gfm,
    Markdownlint,
    HedgeDoc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Markdown,
    Mdx,
    Html,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OcrEngine {
    None,
    Auto,
    NdlOcrLite,
    NdlKoten,
    Tesseract,
    Surya,
    External(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LlmBackend {
    None,
    Anthropic(String),
    OpenAi(String),
    Ollama(String),
    OpenAiCompatible { name: String, endpoint: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConversionOptions {
    pub flavor: Flavor,
    pub format: OutputFormat,
    pub extract_media: Option<PathBuf>,
    pub inline_base64_media: bool,
    pub ocr: OcrEngine,
    pub llm: LlmBackend,
    pub restructure: bool,
    pub translate: Option<String>,
    pub report_path: Option<PathBuf>,
    pub strict: bool,
    pub config_path: Option<PathBuf>,
    pub consent_external_send: bool,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            flavor: Flavor::CommonMark,
            format: OutputFormat::Markdown,
            extract_media: None,
            inline_base64_media: false,
            ocr: OcrEngine::None,
            llm: LlmBackend::None,
            restructure: false,
            translate: None,
            report_path: None,
            strict: false,
            config_path: None,
            consent_external_send: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConversionResult {
    pub ast: Vec<AstNode>,
    pub markdown: String,
    pub report: ConversionReport,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConversionReport {
    pub input_path: String,
    pub input_format: String,
    pub output_format: String,
    pub flavor: String,
    pub warnings: Vec<String>,
    pub metadata: Vec<(String, String)>,
    pub elapsed_ms: u128,
    pub used_ocr: bool,
    pub ocr_engine: Option<String>,
    pub used_llm: bool,
    pub llm_destination: Option<String>,
    pub media: Vec<String>,
}

impl ConversionReport {
    pub fn to_json(&self) -> String {
        let warnings = json_array(&self.warnings);
        let metadata = self
            .metadata
            .iter()
            .map(|(key, value)| {
                format!(
                    "{{\"key\":\"{}\",\"value\":\"{}\"}}",
                    escape_json(key),
                    escape_json(value)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let media = json_array(&self.media);
        format!(
            concat!(
                "{{",
                "\"input_path\":\"{}\",",
                "\"input_format\":\"{}\",",
                "\"output_format\":\"{}\",",
                "\"flavor\":\"{}\",",
                "\"warnings\":{},",
                "\"metadata\":[{}],",
                "\"elapsed_ms\":{},",
                "\"used_ocr\":{},",
                "\"ocr_engine\":{},",
                "\"used_llm\":{},",
                "\"llm_destination\":{},",
                "\"media\":{}",
                "}}\n"
            ),
            escape_json(&self.input_path),
            escape_json(&self.input_format),
            escape_json(&self.output_format),
            escape_json(&self.flavor),
            warnings,
            metadata,
            self.elapsed_ms,
            self.used_ocr,
            json_option(self.ocr_engine.as_deref()),
            self.used_llm,
            json_option(self.llm_destination.as_deref()),
            media
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricScore {
    pub name: String,
    pub score: f64,
    pub errors: usize,
    pub warnings: Vec<String>,
}

pub struct Converter {
    options: ConversionOptions,
}

impl Converter {
    pub fn new() -> Self {
        Self {
            options: ConversionOptions::default(),
        }
    }

    pub fn with_options(mut self, options: ConversionOptions) -> Self {
        self.options = options;
        self
    }

    pub fn convert_file<P: AsRef<Path>>(&self, input: P) -> io::Result<ConversionResult> {
        let path = input.as_ref();
        let bytes = fs::read(path)?;
        if extension(path) == "docx" {
            return self.convert_docx_file(path);
        }
        self.convert_bytes(&path.to_string_lossy(), &bytes)
    }

    pub fn convert_bytes(&self, input_name: &str, bytes: &[u8]) -> io::Result<ConversionResult> {
        let started = Instant::now();
        let mut warnings = Vec::new();
        let input_format = detect_format(input_name, bytes);
        let mut metadata = vec![("bytes".to_string(), bytes.len().to_string())];
        let mut media = Vec::new();

        if let Some(media_dir) = &self.options.extract_media {
            fs::create_dir_all(media_dir)?;
            media.push(media_dir.to_string_lossy().to_string());
        }

        let mut ast = match input_format.as_str() {
            "html" => html::parse_html(
                std::str::from_utf8(bytes).unwrap_or_default(),
                &mut warnings,
            ),
            "markdown" => vec![AstNode::RawHtml(String::from_utf8_lossy(bytes).to_string())],
            "pdf" => pdf::parse_pdf(bytes, &mut warnings),
            "docx" => {
                warnings.push(
                    "DOCX byte conversion cannot unzip in-memory input; use convert_file for DOCX."
                        .to_string(),
                );
                vec![unsupported_node("DOCX in-memory input")]
            }
            "pptx" | "xlsx" => {
                warnings.push(format!(
                    "could not read {} package from in-memory bytes",
                    input_format
                ));
                vec![unsupported_node(&input_format)]
            }
            _ => {
                warnings.push(format!("unsupported input format: {input_format}"));
                vec![unsupported_node(&input_format)]
            }
        };

        if self.options.ocr != OcrEngine::None {
            warnings.push(format!(
                "OCR engine selected: {}",
                ocr_name(&self.options.ocr)
            ));
        }

        if self.options.restructure || self.options.translate.is_some() {
            llm::apply_llm_filters(&mut ast, &self.options, &mut warnings)?;
        }

        metadata.push(("nodes".to_string(), ast.len().to_string()));
        let rendered = render(&ast, &self.options);
        let report = ConversionReport {
            input_path: input_name.to_string(),
            input_format,
            output_format: format_name(self.options.format).to_string(),
            flavor: flavor_name(self.options.flavor).to_string(),
            warnings,
            metadata,
            elapsed_ms: started.elapsed().as_millis(),
            used_ocr: self.options.ocr != OcrEngine::None,
            ocr_engine: (self.options.ocr != OcrEngine::None)
                .then(|| ocr_name(&self.options.ocr).to_string()),
            used_llm: self.options.llm != LlmBackend::None,
            llm_destination: llm_destination(&self.options.llm),
            media,
        };
        Ok(ConversionResult {
            ast,
            markdown: rendered,
            report,
        })
    }

    fn convert_docx_file(&self, path: &Path) -> io::Result<ConversionResult> {
        let started = Instant::now();
        let mut warnings = Vec::new();
        let output = Command::new("unzip")
            .arg("-p")
            .arg(path)
            .arg("word/document.xml")
            .output();
        let ast = match output {
            Ok(output) if output.status.success() => {
                let xml = String::from_utf8_lossy(&output.stdout);
                docx::parse_document_xml(&xml, &mut warnings)
            }
            Ok(output) => {
                warnings.push(format!(
                    "failed to extract DOCX document.xml: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
                vec![unsupported_node("docx")]
            }
            Err(error) => {
                warnings.push(format!("failed to run unzip for DOCX: {error}"));
                vec![unsupported_node("docx")]
            }
        };
        let rendered = render(&ast, &self.options);
        Ok(ConversionResult {
            ast,
            markdown: rendered,
            report: ConversionReport {
                input_path: path.to_string_lossy().to_string(),
                input_format: "docx".to_string(),
                output_format: format_name(self.options.format).to_string(),
                flavor: flavor_name(self.options.flavor).to_string(),
                warnings,
                metadata: vec![("parser".to_string(), "unzip+document.xml".to_string())],
                elapsed_ms: started.elapsed().as_millis(),
                used_ocr: false,
                ocr_engine: None,
                used_llm: self.options.llm != LlmBackend::None,
                llm_destination: llm_destination(&self.options.llm),
                media: Vec::new(),
            },
        })
    }
}

impl Default for Converter {
    fn default() -> Self {
        Self::new()
    }
}

pub mod markdown {
    use super::{AstNode, Flavor, TableRow, escape_html};

    pub fn write_markdown(ast: &[AstNode], flavor: Flavor) -> String {
        let mut output = String::new();
        for node in ast {
            write_node(node, flavor, &mut output, 0);
        }
        while output.ends_with("\n\n") {
            output.pop();
        }
        output
    }

    fn write_node(node: &AstNode, flavor: Flavor, output: &mut String, depth: usize) {
        match node {
            AstNode::Heading { level, text } => {
                output.push_str(&"#".repeat((*level).clamp(1, 6) as usize));
                output.push(' ');
                output.push_str(text.trim());
                output.push_str("\n\n");
            }
            AstNode::Paragraph(text) => {
                output.push_str(text.trim());
                output.push_str("\n\n");
            }
            AstNode::Text(text) => output.push_str(text),
            AstNode::List { ordered, items } => {
                for (index, item) in items.iter().enumerate() {
                    output.push_str(&"  ".repeat(depth));
                    if *ordered {
                        output.push_str(&format!("{}. ", index + 1));
                    } else {
                        output.push_str("- ");
                    }
                    write_inline_nodes(item, flavor, output);
                    output.push('\n');
                }
                output.push('\n');
            }
            AstNode::Table { rows } => write_table(rows, output),
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
                output.push_str(text.trim());
                output.push_str("\n\n");
            }
            AstNode::RawHtml(html) => {
                output.push_str(html.trim());
                output.push('\n');
            }
        }
    }

    fn write_inline_nodes(nodes: &[AstNode], flavor: Flavor, output: &mut String) {
        for node in nodes {
            match node {
                AstNode::Text(text) | AstNode::Paragraph(text) => output.push_str(text.trim()),
                _ => write_node(node, flavor, output, 1),
            }
        }
    }

    fn write_table(rows: &[TableRow], output: &mut String) {
        if rows.iter().flat_map(|row| &row.cells).any(|cell| {
            cell.rowspan > 1 || cell.colspan > 1 || cell.image.is_some() || cell.text.contains('\n')
        }) {
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
}

pub mod html {
    use super::{AstNode, TableCell, TableRow, decode_entities, strip_tags};

    pub fn parse_html(input: &str, warnings: &mut Vec<String>) -> Vec<AstNode> {
        let mut ast = Vec::new();
        let sanitized = remove_block(input, "script");
        let sanitized = remove_block(&sanitized, "style");
        for level in 1..=6 {
            for text in extract_tag(&sanitized, &format!("h{level}")) {
                ast.push(AstNode::Heading {
                    level,
                    text: decode_entities(&strip_tags(&text)),
                });
            }
        }
        for text in extract_tag(&sanitized, "p") {
            ast.push(AstNode::Paragraph(decode_entities(&strip_tags(&text))));
        }
        let list_items = extract_tag(&sanitized, "li")
            .into_iter()
            .map(|item| vec![AstNode::Text(decode_entities(&strip_tags(&item)))])
            .collect::<Vec<_>>();
        if !list_items.is_empty() {
            ast.push(AstNode::List {
                ordered: sanitized.contains("<ol"),
                items: list_items,
            });
        }
        for code in extract_tag(&sanitized, "pre") {
            ast.push(AstNode::CodeBlock {
                language: None,
                code: decode_entities(&strip_tags(&code)),
            });
        }
        for table in extract_tag(&sanitized, "table") {
            let rows = extract_tag(&table, "tr")
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
                .collect::<Vec<_>>();
            ast.push(AstNode::Table { rows });
        }
        if ast.is_empty() {
            warnings.push(
                "HTML parser found no structural nodes; emitted plain paragraph.".to_string(),
            );
            let text = decode_entities(&strip_tags(&sanitized));
            if !text.trim().is_empty() {
                ast.push(AstNode::Paragraph(text));
            }
        }
        ast
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
}

pub mod docx {
    use super::{AstNode, TableCell, TableRow, decode_entities, strip_tags};

    pub fn parse_document_xml(xml: &str, warnings: &mut Vec<String>) -> Vec<AstNode> {
        parse_document_xml_with_rels(xml, "", warnings)
    }

    pub fn parse_document_xml_with_rels(
        xml: &str,
        rels_xml: &str,
        warnings: &mut Vec<String>,
    ) -> Vec<AstNode> {
        let mut ast = Vec::new();
        let mut pending_caption: Option<String> = None;
        for paragraph in extract_blocks(xml, "<w:p", "</w:p>") {
            let style = extract_style(&paragraph);
            let text = extract_text(&paragraph);
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
            } else if paragraph.contains("<w:numPr>") {
                ast.push(AstNode::List {
                    ordered: false,
                    items: vec![vec![AstNode::Text(text)]],
                });
            } else {
                if is_caption(&text) {
                    pending_caption = Some(text.clone());
                }
                ast.push(AstNode::Paragraph(text));
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
            if style.contains(&format!("heading{level}"))
                || style.contains(&format!("見出し{level}"))
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

    fn is_caption(text: &str) -> bool {
        let lower = text.trim().to_ascii_lowercase();
        lower.starts_with("figure ")
            || lower.starts_with("fig. ")
            || lower.starts_with("図 ")
            || lower.starts_with("図表")
    }
}

pub mod office {
    use super::{AstNode, TableCell, TableRow, decode_entities, strip_tags};

    pub fn parse_xlsx_sheet_xml(sheet_xml: &str, shared_strings_xml: &str) -> Vec<AstNode> {
        let shared_strings = extract_blocks(shared_strings_xml, "<si", "</si>")
            .into_iter()
            .map(|item| decode_entities(&strip_tags(&item)))
            .collect::<Vec<_>>();
        let rows = extract_blocks(sheet_xml, "<row", "</row>")
            .into_iter()
            .map(|row| TableRow {
                cells: extract_blocks(&row, "<c", "</c>")
                    .into_iter()
                    .map(|cell| {
                        let value = extract_blocks(&cell, "<v", "</v>")
                            .first()
                            .map(|value| decode_entities(&strip_tags(value)))
                            .unwrap_or_default();
                        let text = if cell.contains("t=\"s\"") {
                            value
                                .parse::<usize>()
                                .ok()
                                .and_then(|index| shared_strings.get(index).cloned())
                                .unwrap_or(value)
                        } else {
                            value
                        };
                        TableCell {
                            text,
                            rowspan: 1,
                            colspan: 1,
                            image: None,
                        }
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        vec![AstNode::Table { rows }]
    }

    pub fn parse_pptx_slide_xml(slide_xml: &str) -> Vec<AstNode> {
        let texts = extract_blocks(slide_xml, "<a:t", "</a:t>")
            .into_iter()
            .map(|text| decode_entities(&strip_tags(&text)))
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
}

pub mod pdf {
    use super::AstNode;

    pub fn parse_pdf(bytes: &[u8], warnings: &mut Vec<String>) -> Vec<AstNode> {
        warnings.push("PDF parser currently extracts readable text heuristically; layout inference is limited.".to_string());
        let lossy = String::from_utf8_lossy(bytes);
        let text = lossy
            .lines()
            .filter(|line| line.chars().any(|ch| ch.is_alphanumeric()))
            .take(200)
            .collect::<Vec<_>>()
            .join("\n");
        if text.trim().is_empty() {
            warnings.push("PDF text extraction produced no text; OCR may be required.".to_string());
            vec![AstNode::Paragraph(
                "PDF content requires OCR or a full PDF backend.".to_string(),
            )]
        } else {
            infer_headings(&text)
        }
    }

    pub fn infer_headings(text: &str) -> Vec<AstNode> {
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let trimmed = line.trim();
                if trimmed.len() < 80
                    && trimmed.chars().any(|ch| ch.is_uppercase())
                    && !trimmed.ends_with('.')
                {
                    AstNode::Heading {
                        level: 2,
                        text: trimmed.to_string(),
                    }
                } else {
                    AstNode::Paragraph(trimmed.to_string())
                }
            })
            .collect()
    }
}

pub mod ocr {
    use super::OcrEngine;
    use std::io;
    use std::path::Path;
    use std::process::Command;

    pub fn run_subprocess(engine: &OcrEngine, input: &Path) -> io::Result<String> {
        let command = match engine {
            OcrEngine::NdlOcrLite => "ndlocr-lite",
            OcrEngine::NdlKoten => "ndl-koten-ocr",
            OcrEngine::Tesseract => "tesseract",
            OcrEngine::Surya => "surya_ocr",
            OcrEngine::External(command) => command,
            OcrEngine::Auto | OcrEngine::None => {
                return Ok(String::new());
            }
        };
        let output = Command::new(command).arg(input).output()?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

pub mod llm {
    use super::{AstNode, ConversionOptions, LlmBackend};
    use std::io;

    pub fn apply_llm_filters(
        ast: &mut [AstNode],
        options: &ConversionOptions,
        warnings: &mut Vec<String>,
    ) -> io::Result<()> {
        if options.llm == LlmBackend::None {
            warnings.push("LLM options requested but no LLM backend was selected.".to_string());
            return Ok(());
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
}

pub fn evaluate_structure_fidelity(expected: &[AstNode], actual: &[AstNode]) -> MetricScore {
    let total = expected.len().max(1);
    let matched = expected
        .iter()
        .zip(actual)
        .filter(|(left, right)| node_kind(left) == node_kind(right))
        .count();
    MetricScore {
        name: "structure_fidelity".to_string(),
        score: matched as f64 / total as f64,
        errors: total.saturating_sub(matched),
        warnings: Vec::new(),
    }
}

pub fn evaluate_heading_recall(expected: &[AstNode], markdown: &str) -> MetricScore {
    let expected_headings = expected
        .iter()
        .filter_map(|node| match node {
            AstNode::Heading { text, .. } => Some(text),
            _ => None,
        })
        .collect::<Vec<_>>();
    let found = expected_headings
        .iter()
        .filter(|heading| {
            markdown
                .lines()
                .any(|line| line.trim_start_matches('#').trim() == heading.trim())
        })
        .count();
    let total = expected_headings.len().max(1);
    MetricScore {
        name: "heading_recall".to_string(),
        score: found as f64 / total as f64,
        errors: total.saturating_sub(found),
        warnings: Vec::new(),
    }
}

pub fn evaluate_table_integrity(markdown: &str) -> MetricScore {
    let has_pipe_table = markdown.lines().any(|line| line.trim().starts_with('|'))
        && markdown.lines().any(|line| line.contains("---"));
    let has_html_table = markdown.contains("<table>") && markdown.contains("</table>");
    let score = if has_pipe_table || has_html_table {
        1.0
    } else {
        0.0
    };
    MetricScore {
        name: "table_integrity".to_string(),
        score,
        errors: usize::from(score < 1.0),
        warnings: Vec::new(),
    }
}

pub fn evaluate_lint_score(markdown: &str) -> MetricScore {
    let mut errors = 0;
    let mut warnings = Vec::new();
    if !markdown.ends_with('\n') {
        errors += 1;
        warnings.push("Markdown must end with a trailing newline.".to_string());
    }
    if markdown.lines().any(|line| line.ends_with(' ')) {
        errors += 1;
        warnings.push("Markdown contains trailing spaces.".to_string());
    }
    MetricScore {
        name: "lint_score".to_string(),
        score: if errors == 0 { 1.0 } else { 0.0 },
        errors,
        warnings,
    }
}

pub fn evaluate_ocr_cer(expected: &str, actual: &str) -> MetricScore {
    let distance = levenshtein(expected, actual);
    let total = expected.chars().count().max(1);
    MetricScore {
        name: "ocr_cer".to_string(),
        score: 1.0 - (distance as f64 / total as f64).min(1.0),
        errors: distance,
        warnings: Vec::new(),
    }
}

pub fn evaluate_translation_structure_preserve(before: &str, after: &str) -> MetricScore {
    let before_markers = structure_markers(before);
    let after_markers = structure_markers(after);
    let total = before_markers.len().max(1);
    let matched = before_markers
        .iter()
        .zip(after_markers.iter())
        .filter(|(left, right)| left == right)
        .count();
    MetricScore {
        name: "translation_structure_preserve".to_string(),
        score: matched as f64 / total as f64,
        errors: total.saturating_sub(matched),
        warnings: Vec::new(),
    }
}

pub fn load_config(path: &Path) -> io::Result<ConversionOptions> {
    let text = fs::read_to_string(path)?;
    let mut options = ConversionOptions::default();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        match key.trim() {
            "flavor" => options.flavor = parse_flavor(value).unwrap_or(options.flavor),
            "format" => options.format = parse_format(value).unwrap_or(options.format),
            "strict" => options.strict = value == "true",
            "consent_external_send" => options.consent_external_send = value == "true",
            _ => {}
        }
    }
    Ok(options)
}

pub fn parse_flavor(value: &str) -> Option<Flavor> {
    match value {
        "commonmark" | "CommonMark" => Some(Flavor::CommonMark),
        "gfm" | "GFM" => Some(Flavor::Gfm),
        "markdownlint" => Some(Flavor::Markdownlint),
        "hedgedoc" | "hackmd" => Some(Flavor::HedgeDoc),
        _ => None,
    }
}

pub fn parse_format(value: &str) -> Option<OutputFormat> {
    match value {
        "md" | "markdown" => Some(OutputFormat::Markdown),
        "mdx" => Some(OutputFormat::Mdx),
        "html" => Some(OutputFormat::Html),
        _ => None,
    }
}

pub fn parse_ocr(value: &str) -> OcrEngine {
    match value {
        "none" => OcrEngine::None,
        "auto" => OcrEngine::Auto,
        "ndlocr-lite" => OcrEngine::NdlOcrLite,
        "ndl-koten" => OcrEngine::NdlKoten,
        "tesseract" => OcrEngine::Tesseract,
        "surya" => OcrEngine::Surya,
        other => OcrEngine::External(other.to_string()),
    }
}

pub fn parse_llm(value: &str) -> LlmBackend {
    if value == "none" {
        LlmBackend::None
    } else if let Some(model) = value.strip_prefix("ollama:") {
        LlmBackend::Ollama(model.to_string())
    } else if value.starts_with("gpt-") {
        LlmBackend::OpenAi(value.to_string())
    } else if value.starts_with("claude-") {
        LlmBackend::Anthropic(value.to_string())
    } else {
        LlmBackend::OpenAiCompatible {
            name: value.to_string(),
            endpoint: String::new(),
        }
    }
}

fn render(ast: &[AstNode], options: &ConversionOptions) -> String {
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

fn detect_format(input_name: &str, bytes: &[u8]) -> String {
    let ext = Path::new(input_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !ext.is_empty() {
        return match ext.as_str() {
            "htm" => "html".to_string(),
            "md" => "markdown".to_string(),
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

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn unsupported_node(format: &str) -> AstNode {
    AstNode::Paragraph(format!("Unsupported input format: {format}"))
}

fn node_kind(node: &AstNode) -> &'static str {
    match node {
        AstNode::Heading { .. } => "heading",
        AstNode::Paragraph(_) => "paragraph",
        AstNode::List { .. } => "list",
        AstNode::Text(_) => "text",
        AstNode::Table { .. } => "table",
        AstNode::Image { .. } => "image",
        AstNode::CodeBlock { .. } => "code",
        AstNode::Footnote { .. } => "footnote",
        AstNode::RawHtml(_) => "raw_html",
    }
}

fn flavor_name(flavor: Flavor) -> &'static str {
    match flavor {
        Flavor::CommonMark => "commonmark",
        Flavor::Gfm => "gfm",
        Flavor::Markdownlint => "markdownlint",
        Flavor::HedgeDoc => "hedgedoc",
    }
}

fn format_name(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Markdown => "markdown",
        OutputFormat::Mdx => "mdx",
        OutputFormat::Html => "html",
    }
}

fn ocr_name(engine: &OcrEngine) -> &str {
    match engine {
        OcrEngine::None => "none",
        OcrEngine::Auto => "auto",
        OcrEngine::NdlOcrLite => "ndlocr-lite",
        OcrEngine::NdlKoten => "ndl-koten",
        OcrEngine::Tesseract => "tesseract",
        OcrEngine::Surya => "surya",
        OcrEngine::External(command) => command,
    }
}

fn llm_destination(llm: &LlmBackend) -> Option<String> {
    match llm {
        LlmBackend::None => None,
        LlmBackend::Anthropic(_) => Some("Anthropic".to_string()),
        LlmBackend::OpenAi(_) => Some("OpenAI".to_string()),
        LlmBackend::Ollama(_) => Some("local".to_string()),
        LlmBackend::OpenAiCompatible { endpoint, .. } if !endpoint.is_empty() => {
            Some(endpoint.clone())
        }
        LlmBackend::OpenAiCompatible { name, .. } => Some(name.clone()),
    }
}

fn json_array(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("\"{}\"", escape_json(value)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn json_option(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", escape_json(value)))
        .unwrap_or_else(|| "null".to_string())
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn strip_tags(input: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
}

fn decode_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn structure_markers(markdown: &str) -> Vec<String> {
    markdown
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                Some("#".repeat(trimmed.chars().take_while(|ch| *ch == '#').count()))
            } else if trimmed.starts_with("- ") {
                Some("-".to_string())
            } else if trimmed.starts_with('|') {
                Some("|".to_string())
            } else {
                None
            }
        })
        .collect()
}

fn levenshtein(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut costs = (0..=right_chars.len()).collect::<Vec<_>>();
    for (i, left_char) in left.chars().enumerate() {
        let mut last = i;
        costs[0] = i + 1;
        for (j, right_char) in right_chars.iter().enumerate() {
            let old = costs[j + 1];
            costs[j + 1] = if left_char == *right_char {
                last
            } else {
                1 + last.min(old).min(costs[j])
            };
            last = old;
        }
    }
    costs[right_chars.len()]
}
