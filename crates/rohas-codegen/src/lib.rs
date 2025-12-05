pub mod config;
pub mod error;
pub mod generator;
pub mod python;
pub mod rust;
pub mod templates;
pub mod typescript;

pub use error::{CodegenError, Result};
pub use generator::Generator;

use rohas_parser::Schema;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    TypeScript,
    Python,
    Rust,
}

pub fn generate(schema: &Schema, output_dir: &Path, lang: Language) -> Result<()> {
    let generator = Generator::new(lang);
    generator.generate(schema, output_dir)
}
