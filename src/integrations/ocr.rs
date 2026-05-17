use crate::OcrEngine;
use std::io;
use std::path::Path;
use std::process::Command;

pub trait OcrBackend {
    fn recognize(&self, input: &Path) -> io::Result<String>;
}

pub fn recognize_with(backend: &dyn OcrBackend, input: &Path) -> io::Result<String> {
    backend.recognize(input)
}

pub struct SubprocessOcrBackend {
    pub engine: OcrEngine,
}

impl OcrBackend for SubprocessOcrBackend {
    fn recognize(&self, input: &Path) -> io::Result<String> {
        run_subprocess(&self.engine, input)
    }
}

pub fn command_for_engine(engine: &OcrEngine) -> Option<&str> {
    match engine {
        OcrEngine::NdlOcrLite => Some("ndlocr-lite"),
        OcrEngine::NdlKoten => Some("ndl-koten-ocr"),
        OcrEngine::Tesseract => Some("tesseract"),
        OcrEngine::Surya => Some("surya_ocr"),
        OcrEngine::External(command) => Some(command),
        OcrEngine::Auto | OcrEngine::None => None,
    }
}

pub fn run_subprocess(engine: &OcrEngine, input: &Path) -> io::Result<String> {
    let Some(command) = command_for_engine(engine) else {
        return Ok(String::new());
    };
    let output = Command::new(command).arg(input).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
