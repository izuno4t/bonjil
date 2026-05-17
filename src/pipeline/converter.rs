use crate::pipeline::input_detection::extension;
use crate::*;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::time::Instant;
use zip::ZipArchive;

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
        match extension(path).as_str() {
            "docx" | "pptx" | "xlsx" => {
                return self.convert_ooxml_file(path, extension(path));
            }
            _ => {}
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

        let mut pdf_report = None;
        let mut ast = match input_format.as_str() {
            "html" => html::parse_html(
                std::str::from_utf8(bytes).unwrap_or_default(),
                &mut warnings,
            ),
            "markdown" => vec![AstNode::RawHtml(String::from_utf8_lossy(bytes).to_string())],
            "pdf" => {
                let result = pdf::parse_pdf_with_embedded_backend(bytes, &mut warnings);
                metadata.push(("pdf_backend".to_string(), result.backend.clone()));
                metadata.push((
                    "pdf_extraction_failed".to_string(),
                    result.extraction_failed.to_string(),
                ));
                metadata.push((
                    "pdf_ocr_required".to_string(),
                    result.ocr_required.to_string(),
                ));
                let ast = result.ast.clone();
                pdf_report = Some(result);
                ast
            }
            "docx" => {
                warnings.push(
                    "DOCX byte conversion cannot unzip in-memory input; use convert_file for DOCX."
                        .to_string(),
                );
                vec![unsupported_node("DOCX in-memory input")]
            }
            "pptx" => {
                let text = std::str::from_utf8(bytes).unwrap_or_default();
                if text.contains("<p:sld") {
                    ooxml::parse_pptx_slide_xml(text)
                } else {
                    warnings.push(format!(
                        "could not read {} package from in-memory bytes",
                        input_format
                    ));
                    vec![unsupported_node(&input_format)]
                }
            }
            "xlsx" => {
                let text = std::str::from_utf8(bytes).unwrap_or_default();
                if text.contains("<worksheet") {
                    ooxml::parse_xlsx_sheet_xml(text, "")
                } else {
                    warnings.push(format!(
                        "could not read {} package from in-memory bytes",
                        input_format
                    ));
                    vec![unsupported_node(&input_format)]
                }
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

        media.extend(collect_media_paths(&ast));
        media.sort();
        media.dedup();
        let media_candidates = collect_media_candidates(&ast);
        metadata.push(("nodes".to_string(), ast.len().to_string()));
        let mut features = report_features(&self.options, &media);
        if let Some(result) = &pdf_report {
            features.push(format!("pdf_backend:{}", result.backend));
            if result.ocr_required {
                features.push("pdf:ocr_required".to_string());
            }
            if result.extraction_failed {
                features.push("pdf:extraction_failed".to_string());
            }
        }
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
            media_candidates,
            features,
        };
        Ok(ConversionResult {
            ast,
            markdown: rendered,
            report,
        })
    }

    fn convert_ooxml_file(
        &self,
        path: &Path,
        input_format: String,
    ) -> io::Result<ConversionResult> {
        let started = Instant::now();
        let mut warnings = Vec::new();
        let mut metadata = vec![("parser".to_string(), "zip+ooxml-package".to_string())];
        let ast = match input_format.as_str() {
            "docx" => match unzip_part(path, "word/document.xml") {
                Ok(xml) => {
                    metadata.push(("part".to_string(), "word/document.xml".to_string()));
                    let rels = unzip_part(path, "word/_rels/document.xml.rels").unwrap_or_default();
                    if !rels.is_empty() {
                        metadata.push((
                            "relationships".to_string(),
                            "word/_rels/document.xml.rels".to_string(),
                        ));
                    }
                    let footnotes = unzip_part(path, "word/footnotes.xml").unwrap_or_default();
                    let comments = unzip_part(path, "word/comments.xml").unwrap_or_default();
                    if !footnotes.is_empty() {
                        metadata.push(("part".to_string(), "word/footnotes.xml".to_string()));
                    }
                    if !comments.is_empty() {
                        metadata.push(("part".to_string(), "word/comments.xml".to_string()));
                    }
                    docx::parse_document_xml_with_rels_and_notes(
                        &xml,
                        &rels,
                        &footnotes,
                        &comments,
                        &mut warnings,
                    )
                }
                Err(error) => {
                    warnings.push(format!("failed to extract DOCX document.xml: {error}"));
                    vec![unsupported_node("docx")]
                }
            },
            "pptx" => {
                let slides = read_numbered_parts(path, "ppt/slides/slide", ".xml", 200);
                if slides.is_empty() {
                    warnings.push("failed to extract PPTX slide parts from package.".to_string());
                    vec![unsupported_node("pptx")]
                } else {
                    metadata.push(("slides".to_string(), slides.len().to_string()));
                    let rels =
                        read_numbered_parts(path, "ppt/slides/_rels/slide", ".xml.rels", 200)
                            .into_iter()
                            .collect::<Vec<_>>();
                    if !rels.is_empty() {
                        metadata.push(("slide_relationships".to_string(), rels.len().to_string()));
                    }
                    let mut ast = Vec::new();
                    for (index, slide) in slides.iter().enumerate() {
                        let slide_rels = rels.get(index).map(String::as_str).unwrap_or_default();
                        ast.extend(ooxml::parse_pptx_slide_xml_with_rels(
                            slide,
                            slide_rels,
                            &mut warnings,
                        ));
                    }
                    ast
                }
            }
            "xlsx" => {
                let shared_strings = unzip_part(path, "xl/sharedStrings.xml").unwrap_or_default();
                if !shared_strings.is_empty() {
                    metadata.push(("part".to_string(), "xl/sharedStrings.xml".to_string()));
                }
                let sheets = read_numbered_parts(path, "xl/worksheets/sheet", ".xml", 200);
                if sheets.is_empty() {
                    warnings
                        .push("failed to extract XLSX worksheet parts from package.".to_string());
                    vec![unsupported_node("xlsx")]
                } else {
                    metadata.push(("worksheets".to_string(), sheets.len().to_string()));
                    let mut ast = Vec::new();
                    let multiple_sheets = sheets.len() > 1;
                    for (index, sheet) in sheets.iter().enumerate() {
                        if multiple_sheets {
                            ast.push(AstNode::Heading {
                                level: 1,
                                text: format!("Sheet {}", index + 1),
                            });
                        }
                        ast.extend(ooxml::parse_xlsx_sheet_xml_with_warnings(
                            sheet,
                            &shared_strings,
                            &mut warnings,
                        ));
                    }
                    ast
                }
            }
            _ => vec![unsupported_node(&input_format)],
        };
        let rendered = render(&ast, &self.options);
        let mut media = collect_media_paths(&ast);
        media.sort();
        media.dedup();
        let media_candidates = collect_media_candidates(&ast);
        let features = report_features(&self.options, &media);
        Ok(ConversionResult {
            ast,
            markdown: rendered,
            report: ConversionReport {
                input_path: path.to_string_lossy().to_string(),
                input_format,
                output_format: format_name(self.options.format).to_string(),
                flavor: flavor_name(self.options.flavor).to_string(),
                warnings,
                metadata,
                elapsed_ms: started.elapsed().as_millis(),
                used_ocr: false,
                ocr_engine: None,
                used_llm: self.options.llm != LlmBackend::None,
                llm_destination: llm_destination(&self.options.llm),
                media,
                media_candidates,
                features,
            },
        })
    }
}

fn unzip_part(path: &Path, part: &str) -> io::Result<String> {
    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file).map_err(zip_error)?;
    let mut file = archive.by_name(part).map_err(zip_error)?;
    let mut output = String::new();
    file.read_to_string(&mut output)?;
    Ok(output)
}

fn zip_error(error: zip::result::ZipError) -> io::Error {
    io::Error::other(error.to_string())
}

fn read_numbered_parts(path: &Path, prefix: &str, suffix: &str, max: usize) -> Vec<String> {
    (1..=max)
        .filter_map(|index| unzip_part(path, &format!("{prefix}{index}{suffix}")).ok())
        .filter(|content| !content.trim().is_empty())
        .collect()
}

impl Default for Converter {
    fn default() -> Self {
        Self::new()
    }
}

pub fn convert_bytes(
    input_name: &str,
    bytes: &[u8],
    options: ConversionOptions,
) -> io::Result<ConversionResult> {
    Converter::new()
        .with_options(options)
        .convert_bytes(input_name, bytes)
}

pub fn convert_reader<R: Read>(
    input_name: &str,
    mut reader: R,
    options: ConversionOptions,
) -> io::Result<ConversionResult> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    convert_bytes(input_name, &bytes, options)
}
