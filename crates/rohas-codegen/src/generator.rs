use crate::error::Result;
use crate::{config, python, typescript, Language};
use rohas_parser::Schema;
use std::fs;
use std::path::Path;
use tracing::{debug, info};

pub struct Generator {
    language: Language,
}

impl Generator {
    pub fn new(language: Language) -> Self {
        Self { language }
    }

    pub fn generate(&self, schema: &Schema, output_dir: &Path) -> Result<()> {
        info!(
            "Generating code for {:?} in {}",
            self.language,
            output_dir.display()
        );

        self.create_directory_structure(output_dir)?;

        self.generate_common_configs(schema, output_dir)?;

        match self.language {
            Language::TypeScript => self.generate_typescript(schema, output_dir)?,
            Language::Python => self.generate_python(schema, output_dir)?,
        }

        info!("Code generation completed successfully");
        Ok(())
    }

    fn create_directory_structure(&self, output_dir: &Path) -> Result<()> {
        let dirs = [
            "generated",
            "generated/models",
            "generated/dto",
            "generated/api",
            "generated/events",
            "generated/cron",
            "handlers",
            "handlers/api",
            "handlers/events",
            "handlers/cron",
        ];

        for dir in &dirs {
            let path = output_dir.join(dir);
            if !path.exists() {
                fs::create_dir_all(&path)?;
                debug!("Created directory: {}", path.display());
            }
        }

        Ok(())
    }

    fn generate_common_configs(&self, schema: &Schema, output_dir: &Path) -> Result<()> {
        info!("Generating common configuration files");
        config::generate_gitignore(schema, output_dir)?;
        config::generate_editorconfig(schema, output_dir)?;
        config::generate_readme(schema, output_dir)?;
        Ok(())
    }

    fn generate_typescript(&self, schema: &Schema, output_dir: &Path) -> Result<()> {
        typescript::generate_models(schema, output_dir)?;
        typescript::generate_dtos(schema, output_dir)?;
        typescript::generate_apis(schema, output_dir)?;
        typescript::generate_events(schema, output_dir)?;
        typescript::generate_crons(schema, output_dir)?;
        typescript::generate_index(schema, output_dir)?;

        info!("Generating TypeScript configuration files");
        config::generate_package_json(schema, output_dir)?;
        config::generate_tsconfig_json(schema, output_dir)?;
        config::generate_rspack_config(schema, output_dir)?;
        config::generate_nvmrc(schema, output_dir)?;
        config::generate_prettierrc(schema, output_dir)?;
        config::generate_prettierignore(schema, output_dir)?;

        Ok(())
    }

    fn generate_python(&self, schema: &Schema, output_dir: &Path) -> Result<()> {
        python::generate_models(schema, output_dir)?;
        python::generate_dtos(schema, output_dir)?;
        python::generate_apis(schema, output_dir)?;
        python::generate_events(schema, output_dir)?;
        python::generate_crons(schema, output_dir)?;
        python::generate_init(schema, output_dir)?;

        info!("Generating Python configuration files");
        config::generate_requirements_txt(schema, output_dir)?;
        config::generate_pyproject_toml(schema, output_dir)?;

        Ok(())
    }
}
