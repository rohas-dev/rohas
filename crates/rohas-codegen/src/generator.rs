use crate::error::Result;
use crate::{config, python, rust, typescript, Language};
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

        if let Some(parent) = output_dir.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(output_dir)?;

        self.create_directory_structure(output_dir)?;

        self.generate_common_configs(schema, output_dir)?;

        match self.language {
            Language::TypeScript => self.generate_typescript(schema, output_dir)?,
            Language::Python => self.generate_python(schema, output_dir)?,
            Language::Rust => self.generate_rust(schema, output_dir)?,
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
            "generated/websockets",
            "handlers",
            "handlers/api",
            "handlers/events",
            "handlers/cron",
            "handlers/websockets",
            "middlewares",
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
        use tracing::error;
        info!("Generating common configuration files");
        info!("Output directory: {}", output_dir.display());
        
        info!("Generating .gitignore...");
        config::generate_gitignore(schema, output_dir)
            .map_err(|e| {
                error!("Failed to generate .gitignore: {}", e);
                crate::error::CodegenError::GenerationFailed(format!(
                    "Failed to generate .gitignore: {}",
                    e
                ))
            })?;
        info!("Generated .gitignore successfully");
        
        info!("Generating .editorconfig...");
        config::generate_editorconfig(schema, output_dir)
            .map_err(|e| {
                error!("Failed to generate .editorconfig: {}", e);
                crate::error::CodegenError::GenerationFailed(format!(
                    "Failed to generate .editorconfig: {}",
                    e
                ))
            })?;
        info!("Generated .editorconfig successfully");
        
        info!("Generating README.md...");
        config::generate_readme(schema, output_dir)
            .map_err(|e| {
                error!("Failed to generate README.md: {}", e);
                crate::error::CodegenError::GenerationFailed(format!(
                    "Failed to generate README.md: {}",
                    e
                ))
            })?;
        info!("Generated README.md successfully");
        
        Ok(())
    }

    fn generate_typescript(&self, schema: &Schema, output_dir: &Path) -> Result<()> {
        typescript::generate_state(output_dir)?;
        typescript::generate_models(schema, output_dir)?;
        typescript::generate_dtos(schema, output_dir)?;
        typescript::generate_apis(schema, output_dir)?;
        typescript::generate_events(schema, output_dir)?;
        typescript::generate_crons(schema, output_dir)?;
        typescript::generate_websockets(schema, output_dir)?;
        typescript::generate_middlewares(schema, output_dir)?;
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
        python::generate_state(output_dir)?;
        python::generate_models(schema, output_dir)?;
        python::generate_dtos(schema, output_dir)?;
        python::generate_apis(schema, output_dir)?;
        python::generate_events(schema, output_dir)?;
        python::generate_crons(schema, output_dir)?;
        python::generate_websockets(schema, output_dir)?;
        python::generate_middlewares(schema, output_dir)?;
        python::generate_init(schema, output_dir)?;

        info!("Generating Python configuration files");
        config::generate_requirements_txt(schema, output_dir)?;
        config::generate_pyproject_toml(schema, output_dir)?;

        Ok(())
    }

    fn generate_rust(&self, schema: &Schema, output_dir: &Path) -> Result<()> {
        use tracing::error;
        
        info!("Generating Rust code...");
        info!("Generating state...");
        rust::generate_state(output_dir)?;
        info!("Generating models...");
        rust::generate_models(schema, output_dir)?;
        info!("Generating DTOs...");
        rust::generate_dtos(schema, output_dir)?;
        info!("Generating APIs...");
        rust::generate_apis(schema, output_dir)?;
        info!("Generating events...");
        rust::generate_events(schema, output_dir)?;
        info!("Generating crons...");
        rust::generate_crons(schema, output_dir)?;
        info!("Generating websockets...");
        rust::generate_websockets(schema, output_dir)
            .map_err(|e| {
                error!("Failed to generate websockets: {}", e);
                crate::error::CodegenError::GenerationFailed(format!(
                    "Failed to generate websockets: {}",
                    e
                ))
            })?;
        info!("Generating middlewares...");
        rust::generate_middlewares(schema, output_dir)?;
        info!("Generating lib.rs...");
        rust::generate_lib_rs(schema, output_dir)?;

        info!("Generating Rust configuration files");
        config::generate_cargo_toml(schema, output_dir)?;
        
        if rust::is_in_rohas_workspace(output_dir) {
            rust::generate_dev_scripts(output_dir)?;
        }

        Ok(())
    }
}
