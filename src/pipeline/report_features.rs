use crate::{
    ConversionOptions, LlmBackend, OcrEngine, flavor_name, format_name, llm_destination, ocr_name,
};

pub(crate) fn report_features(options: &ConversionOptions, media: &[String]) -> Vec<String> {
    let mut features = vec![
        format!("format:{}", format_name(options.format)),
        format!("flavor:{}", flavor_name(options.flavor)),
    ];
    if let Some(media_dir) = &options.extract_media {
        features.push("extract_media".to_string());
        features.push(format!("extract_media_dir:{}", media_dir.to_string_lossy()));
    }
    if options.inline_base64_media {
        features.push("inline_base64_media".to_string());
    }
    if options.ocr != OcrEngine::None {
        features.push(format!("ocr:{}", ocr_name(&options.ocr)));
    }
    if options.llm != LlmBackend::None {
        features.push(format!(
            "llm:{}",
            llm_destination(&options.llm).unwrap_or_else(|| "unknown".to_string())
        ));
    }
    if options.restructure {
        features.push("llm:restructure".to_string());
    }
    if let Some(language) = &options.translate {
        features.push(format!("llm:translate:{language}"));
    }
    if !media.is_empty() {
        features.push("media:referenced".to_string());
    }
    features
}
