use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use tracing::{error, info, warn};

pub struct TypeScriptCompiler {
    project_root: PathBuf,
    watch_process: Option<Child>,
}

impl TypeScriptCompiler {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            watch_process: None,
        }
    }

    pub fn compile(&self) -> anyhow::Result<()> {
        info!("Compiling TypeScript to JavaScript using SWC...");

        let status = Command::new("npm")
            .arg("run")
            .arg("compile")
            .current_dir(&self.project_root)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            anyhow::bail!("TypeScript compilation failed");
        }

        info!("✓ TypeScript compilation completed");
        Ok(())
    }

    pub fn start_watch(&mut self) -> anyhow::Result<()> {
        info!("Starting TypeScript watch mode with SWC...");

        self.ensure_swc_installed()?;

        let child = Command::new("npm")
            .arg("run")
            .arg("compile:watch")
            .current_dir(&self.project_root)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        info!("✓ TypeScript watch mode started");
        self.watch_process = Some(child);
        Ok(())
    }

    pub fn stop_watch(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.watch_process.take() {
            info!("Stopping TypeScript watch mode...");
            child.kill()?;
            child.wait()?;
            info!("✓ TypeScript watch mode stopped");
        }
        Ok(())
    }

    fn ensure_swc_installed(&self) -> anyhow::Result<()> {
        let package_json = self.project_root.join("package.json");
        if !package_json.exists() {
            anyhow::bail!("package.json not found in project root");
        }

        let node_modules = self.project_root.join("node_modules");
        if !node_modules.exists() {
            warn!("node_modules not found, installing dependencies...");
            self.install_dependencies()?;
        }

        let swc_cli = node_modules.join("@swc").join("cli");
        if !swc_cli.exists() {
            warn!("@swc/cli not found, installing dependencies...");
            self.install_dependencies()?;
        }

        Ok(())
    }

    fn install_dependencies(&self) -> anyhow::Result<()> {
        info!("Installing npm dependencies...");

        let status = Command::new("npm")
            .arg("install")
            .current_dir(&self.project_root)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            anyhow::bail!("npm install failed");
        }

        info!("✓ npm dependencies installed");
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_output_dir(&self) -> PathBuf {
        self.project_root.join(".rohas")
    }

    #[allow(dead_code)]
    pub fn has_compiled_output(&self) -> bool {
        let output_dir = self.get_output_dir();
        output_dir.exists()
            && output_dir
                .read_dir()
                .map(|mut d| d.next().is_some())
                .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn resolve_compiled_path(&self, source_path: &Path) -> anyhow::Result<PathBuf> {
        let relative_path = if source_path.is_absolute() {
            source_path
                .strip_prefix(&self.project_root)
                .map_err(|_| anyhow::anyhow!("Path is not in project root: {:?}", source_path))?
        } else {
            source_path
        };

        let relative_path = if let Ok(path) = relative_path.strip_prefix("src") {
            path
        } else {
            relative_path
        };

        let mut compiled_path = self.get_output_dir().join(relative_path);
        compiled_path.set_extension("js");

        Ok(compiled_path)
    }
}

impl Drop for TypeScriptCompiler {
    fn drop(&mut self) {
        if let Err(e) = self.stop_watch() {
            error!("Failed to stop TypeScript watch mode: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_compiled_path() {
        let compiler = TypeScriptCompiler::new(PathBuf::from("/project"));

        let source = PathBuf::from("/project/src/handlers/api/Health.ts");
        let compiled = compiler.resolve_compiled_path(&source).unwrap();

        assert_eq!(
            compiled,
            PathBuf::from("/project/.rohas/handlers/api/Health.js")
        );
    }

    #[test]
    fn test_resolve_compiled_path_relative() {
        let compiler = TypeScriptCompiler::new(PathBuf::from("/project"));

        let source = PathBuf::from("src/handlers/api/Health.ts");
        let compiled = compiler.resolve_compiled_path(&source).unwrap();

        assert_eq!(
            compiled,
            PathBuf::from("/project/.rohas/handlers/api/Health.js")
        );
    }
}
