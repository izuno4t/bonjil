use std::io;
use std::path::{Path, PathBuf};

pub trait VectorRasterizer {
    fn rasterize(&self, input: &Path, output: &Path) -> io::Result<PathBuf>;
}

pub fn is_vector_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| matches!(extension.to_ascii_lowercase().as_str(), "wmf" | "emf"))
        .unwrap_or(false)
}

pub fn rasterize_vector_image(
    rasterizer: &dyn VectorRasterizer,
    input: &Path,
    output_dir: &Path,
) -> io::Result<PathBuf> {
    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("image");
    let output = output_dir.join(format!("{stem}.png"));
    rasterizer.rasterize(input, &output)
}
