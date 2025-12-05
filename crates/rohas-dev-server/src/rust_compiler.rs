use std::path::PathBuf;
use std::process::Command as StdCommand;
use tokio::process::Command;
use std::fs;
use std::io::Read;
use tracing::{error, info, warn};

pub struct RustCompiler {
    project_root: PathBuf,
}

impl RustCompiler {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn project_root(&self) -> &PathBuf {
        &self.project_root
    }

    pub fn is_rust_project(project_root: &PathBuf) -> bool {
        project_root.join("Cargo.toml").exists()
    }

    pub fn compile(&self) -> anyhow::Result<()> {
        info!("Compiling Rust project: {}", self.project_root().display());

        let output = StdCommand::new("cargo")
            .arg("check")
            .arg("--message-format=short")
            .current_dir(self.project_root())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Rust compilation failed:\n{}", stderr);
            return Err(anyhow::anyhow!("Rust compilation failed"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            info!("Compilation output:\n{}", stdout);
        }

        info!("Rust project compiled successfully");
        Ok(())
    }

    pub async fn build_release(&self) -> anyhow::Result<()> {
        use std::fs;

        info!("Building Rust project in release mode: {}", self.project_root().display());

        let package_name = self.get_package_name()?;
        let project_name = package_name.replace('-', "_");

        let dylib_path = self.get_library_path_for_profile("release")?;

        let hash_before = if dylib_path.exists() {
            let hash = Self::compute_file_hash(&dylib_path)?;
            let hash_prefix = u128::from_be_bytes(
                hash[..16].try_into().unwrap_or([0; 16])
            );
            info!("Dylib hash before rebuild: 0x{:032x}", hash_prefix);
            Some(hash)
        } else {
            info!("No existing dylib - this will be a fresh build");
            None
        };

        info!("Cleaning release build artifacts for package: {}", package_name);

        if dylib_path.exists() {
            if let Err(e) = fs::remove_file(&dylib_path) {
                warn!("Failed to delete dylib before clean: {}", e);
            } else {
                info!("Deleted dylib before clean");
            }
        }

        let release_target = self.project_root().join("target").join("release");
        if release_target.exists() {
            if let Ok(entries) = fs::read_dir(&release_target.join("deps")) {
                let package_pattern = format!("lib{}", project_name);
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    if let Some(name_str) = file_name.to_str() {
                        if name_str.starts_with(&package_pattern) {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }

        let target_release = self.project_root().join("target").join("release");
        if target_release.exists() {
            info!("Removing entire target/release directory to force full rebuild");
            if let Err(e) = fs::remove_dir_all(&target_release) {
                warn!("Failed to remove target/release: {}. Continuing anyway.", e);
            } else {
                info!("Removed target/release directory");
            }
        }

        let clean_output = StdCommand::new("cargo")
            .arg("clean")
            .arg("--release")
            .arg("--package")
            .arg(&package_name)
            .current_dir(self.project_root())
            .output();

        match clean_output {
            Ok(output) if output.status.success() => {
                info!("Cleaned release artifacts for package: {}", project_name);
            }
            Ok(output) => {
                warn!("Package-specific clean failed, trying full clean: {}",
                    String::from_utf8_lossy(&output.stderr));
                let fallback_clean = StdCommand::new("cargo")
                    .arg("clean")
                    .arg("--release")
                    .current_dir(self.project_root())
                    .output();
                if let Ok(fb_output) = fallback_clean {
                    if fb_output.status.success() {
                        info!("Cleaned all release artifacts (fallback)");
                    }
                }
            }
            Err(e) => {
                warn!("Failed to clean release artifacts: {}. Continuing anyway.", e);
            }
        }

        if dylib_path.exists() {
            if let Err(e) = fs::remove_file(&dylib_path) {
                warn!("Failed to delete existing dylib before rebuild: {}. Continuing anyway.", e);
            } else {
                info!("Deleted existing dylib to force rebuild");
            }
        }

        for profile in ["debug", "release"] {
            let incremental_dir = self.project_root
                .join("target")
                .join(profile)
                .join("incremental");
            if incremental_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&incremental_dir) {
                    warn!("Failed to remove incremental compilation cache for {}: {}. Continuing anyway.", profile, e);
                } else {
                    info!("Removed incremental compilation cache for {}", profile);
                }
            }
        }

        let fingerprint_dir = self.project_root
            .join("target")
            .join("release")
            .join(".fingerprint");
        if fingerprint_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&fingerprint_dir) {
                warn!("Failed to remove fingerprint directory: {}. Continuing anyway.", e);
            } else {
                info!("Removed fingerprint directory");
            }
        }

        let lib_rs = self.project_root().join("src").join("lib.rs");
        if lib_rs.exists() {
            use std::io::Write;
            if let Ok(mut file) = fs::OpenOptions::new().append(true).open(&lib_rs) {
                let _ = file.write_all(b"\n");
                drop(file);
                info!("Touched lib.rs to force Cargo rebuild");
            }
        }

        let mut build_cmd = Command::new("cargo");
        build_cmd
            .arg("build")
            .arg("--release")
            .arg("--message-format=short")
            .arg("--lib")
            .env("CARGO_INCREMENTAL", "0")
            .current_dir(self.project_root());

        let output = build_cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Rust release build failed:\n{}", stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.trim().is_empty() {
                error!("Build stdout:\n{}", stdout);
            }
            return Err(anyhow::anyhow!("Rust release build failed"));
        }

        if !dylib_path.exists() {
            return Err(anyhow::anyhow!("Dylib was not created at expected path: {}", dylib_path.display()));
        }

        let dylib_hash = Self::compute_file_hash(&dylib_path)?;
        let hash_prefix = u128::from_be_bytes(
            dylib_hash[..16].try_into().unwrap_or([0; 16])
        );
        info!("Dylib hash (first 16 bytes of SHA256): 0x{:032x}", hash_prefix);

        if let Some(prev_hash) = hash_before {
            if dylib_hash == prev_hash {
                warn!("WARNING: Dylib hash did not change after rebuild! This means Cargo did not actually rebuild the code.");
                warn!("The dylib binary is identical to before - hot reload will not work!");
                warn!("Attempting to force rebuild by touching generated handlers.rs...");

                let generated_handlers_rs = self.project_root().join("src").join("generated").join("handlers.rs");
                
                if generated_handlers_rs.exists() {
                    use std::io::Write;
                    if let Ok(mut file) = fs::OpenOptions::new().append(true).open(&generated_handlers_rs) {
                        let _ = file.write_all(b"\n");
                        drop(file);
                        info!("Touched generated handlers.rs to force rebuild");
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }

                let cargo_toml = self.project_root().join("Cargo.toml");
                if cargo_toml.exists() {
                    use std::io::Write;
                    if let Ok(mut file) = fs::OpenOptions::new().append(true).open(&cargo_toml) {
                        let _ = file.write_all(b"\n");
                        drop(file);
                        info!("Touched Cargo.toml to force rebuild");
                    }
                }

                info!("Rebuilding after touching files...");
                let retry_output = StdCommand::new("cargo")
                    .arg("build")
                    .arg("--release")
                    .arg("--message-format=short")
                    .arg("--lib")
                    .env("CARGO_INCREMENTAL", "0")
                    .current_dir(self.project_root())
                    .output()?;

                if !retry_output.status.success() {
                    let stderr = String::from_utf8_lossy(&retry_output.stderr);
                    error!("Rust release build failed on retry:\n{}", stderr);
                    return Err(anyhow::anyhow!("Rust release build failed on retry"));
                }

                let retry_hash = Self::compute_file_hash(&dylib_path)?;
                if retry_hash == prev_hash {
                    warn!("Dylib hash still unchanged after touching files - this may indicate:");
                    warn!("  1. The source code hasn't actually changed");
                    warn!("  2. Cargo is using cached artifacts despite cleaning");
                    warn!("  3. The build system is not detecting file changes");
                    warn!("Proceeding anyway - hot reload may not work, but the dylib will be reloaded");
                } else {
                    info!("Dylib hash changed after touching files - rebuild was successful");
                }
            } else {
                info!("Dylib hash changed - rebuild was successful");
            }
        } else {
            info!("Dylib built fresh (no previous hash to compare)");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined_output = format!("{}\n{}", stdout, stderr);

        let was_compiled = combined_output.contains("Compiling") || combined_output.contains("compiling");

        if was_compiled {
            info!("Rust project rebuilt successfully (release mode) - compilation occurred");
        } else if combined_output.contains("Finished") {
            warn!("Cargo reported 'Finished' but no compilation occurred - forcing rebuild");

            let lib_rs = self.project_root().join("src").join("lib.rs");
            if lib_rs.exists() {
                use std::fs::OpenOptions;
                use std::io::Write;
                if let Ok(mut file) = OpenOptions::new().write(true).append(true).open(&lib_rs) {
                    let _ = file.write_all(b"");
                    drop(file);

                    info!("Forcing rebuild after touching lib.rs...");
                        let retry_output = StdCommand::new("cargo")
                            .arg("build")
                            .arg("--release")
                            .arg("--message-format=short")
                            .arg("--lib")
                            .current_dir(self.project_root())
                            .output()?;

                    if !retry_output.status.success() {
                        let retry_stderr = String::from_utf8_lossy(&retry_output.stderr);
                        error!("Rust release build failed on retry:\n{}", retry_stderr);
                        return Err(anyhow::anyhow!("Rust release build failed on retry"));
                    }

                    let retry_stdout = String::from_utf8_lossy(&retry_output.stdout);
                    if retry_stdout.contains("Compiling") || retry_stdout.contains("compiling") {
                        info!("Rust project rebuilt successfully after forcing rebuild");
                    } else {
                        warn!("Rebuild completed but still no compilation detected - this may indicate cached build");
                    }
                }
            }
        } else {
            warn!("Rust project build completed (output unclear)");
        }
        Ok(())
    }

    fn get_package_name(&self) -> anyhow::Result<String> {
        use std::fs;

        let cargo_toml = self.project_root().join("Cargo.toml");
        let contents = fs::read_to_string(&cargo_toml)?;

        for line in contents.lines() {
            let line = line.trim();
            if line.starts_with("name =") {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line.rfind('"') {
                        if end > start {
                            return Ok(line[start + 1..end].to_string());
                        }
                    }
                }
            }
        }

        let name = self.project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("rust_example")
            .to_string();
        Ok(name)
    }

    pub fn get_library_path(&self) -> anyhow::Result<PathBuf> {
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        self.get_library_path_for_profile(profile)
    }

    pub fn get_library_path_for_profile(&self, profile: &str) -> anyhow::Result<PathBuf> {
        let target_dir = self.project_root().join("target");

        let package_name = self.get_package_name()?;
        let project_name = package_name.replace('-', "_");

        #[cfg(target_os = "macos")]
        let dylib_name = format!("lib{}.dylib", project_name);
        #[cfg(target_os = "linux")]
        let dylib_name = format!("lib{}.so", project_name);
        #[cfg(target_os = "windows")]
        let dylib_name = format!("{}.dll", project_name);
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        let dylib_name = format!("lib{}.so", project_name);

        Ok(target_dir.join(profile).join(&dylib_name))
    }

    fn compute_file_hash(path: &PathBuf) -> anyhow::Result<[u8; 32]> {
        use sha2::{Sha256, Digest};
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        Ok(hasher.finalize().into())
    }
}

