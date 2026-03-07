use crate::llm::{LlmProvider, LlmRequest};
use crate::patterns::PatternMapper;
use crate::prompt;
use anyhow::Result;
use java_parser::types::*;
use regex::Regex;
use std::path::{Path, PathBuf};

pub struct ConversionConfig {
    pub output_dir: PathBuf,
    pub profile: String,
    pub verify: bool,
}

pub struct Generator {
    llm: Box<dyn LlmProvider>,
    pattern_mapper: PatternMapper,
}

#[derive(Debug)]
pub struct ConversionResult {
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub rust_code: String,
    pub success: bool,
    pub errors: Vec<String>,
}

impl Generator {
    pub fn new(llm: Box<dyn LlmProvider>) -> Self {
        Self {
            llm,
            pattern_mapper: PatternMapper::new(),
        }
    }

    pub async fn convert_project(
        &self,
        project: &JavaProject,
        config: &ConversionConfig,
    ) -> Result<Vec<ConversionResult>> {
        let mut results = Vec::new();

        // Generate Cargo.toml for the output project
        let cargo_toml = generate_cargo_toml(project);
        let cargo_path = config.output_dir.join("Cargo.toml");
        std::fs::create_dir_all(&config.output_dir)?;
        std::fs::create_dir_all(config.output_dir.join("src"))?;
        std::fs::write(&cargo_path, cargo_toml)?;

        let system = prompt::system_prompt(&project.framework, &project.version);

        for file in &project.files {
            let result = self.convert_file(file, &system, &config.output_dir).await?;
            results.push(result);
        }

        // Generate lib.rs with mod declarations
        let lib_content = generate_lib_rs(&results);
        std::fs::write(config.output_dir.join("src/lib.rs"), lib_content)?;

        Ok(results)
    }

    pub async fn convert_file(
        &self,
        file: &JavaFile,
        system: &str,
        output_dir: &Path,
    ) -> Result<ConversionResult> {
        let mut conversion_prompt = prompt::conversion_prompt(file);

        // Add pattern context
        let pattern_context = self.pattern_mapper.generate_context(&file.imports);
        if !pattern_context.is_empty() {
            conversion_prompt.push_str(&pattern_context);
        }

        tracing::info!(
            "Converting: {} (via {})",
            file.path.display(),
            self.llm.name()
        );

        let request = LlmRequest {
            system_prompt: system.to_string(),
            user_prompt: conversion_prompt,
            max_tokens: 8192,
            temperature: 0.0,
        };
        let response = self.llm.generate(&request).await?;
        let rust_code = extract_rust_code(&response.content);

        // Determine output path
        let module_name = file
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_lowercase();

        let output_path = output_dir.join("src").join(format!("{}.rs", module_name));
        std::fs::write(&output_path, &rust_code)?;

        Ok(ConversionResult {
            source_path: file.path.clone(),
            output_path,
            rust_code,
            success: true,
            errors: Vec::new(),
        })
    }

    pub async fn fix_code(&self, rust_code: &str, error: &str) -> Result<String> {
        let request = LlmRequest {
            system_prompt: "You are a Rust expert. Fix compilation errors in the provided code."
                .to_string(),
            user_prompt: prompt::fix_prompt(rust_code, error),
            max_tokens: 8192,
            temperature: 0.0,
        };
        let response = self.llm.generate(&request).await?;
        Ok(extract_rust_code(&response.content))
    }
}

fn extract_rust_code(response: &str) -> String {
    let re = Regex::new(r"```rust\n([\s\S]*?)```").unwrap();
    if let Some(cap) = re.captures(response) {
        cap[1].trim().to_string()
    } else {
        // If no code block, return the entire response trimmed
        response.trim().to_string()
    }
}

fn generate_cargo_toml(project: &JavaProject) -> String {
    let project_name = project
        .path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("converted-project")
        .to_lowercase()
        .replace(' ', "-");

    let mut deps = String::new();

    // Add framework-specific dependencies
    match project.framework {
        Framework::SpringBoot => {
            deps.push_str("axum = \"0.8\"\n");
            deps.push_str("tokio = { version = \"1\", features = [\"full\"] }\n");
            deps.push_str("serde = { version = \"1\", features = [\"derive\"] }\n");
            deps.push_str("serde_json = \"1\"\n");
            deps.push_str("tower = \"0.5\"\n");
        }
        Framework::JavaEE => {
            deps.push_str("axum = \"0.8\"\n");
            deps.push_str("tokio = { version = \"1\", features = [\"full\"] }\n");
            deps.push_str("serde = { version = \"1\", features = [\"derive\"] }\n");
        }
        _ => {}
    }

    // Check if any file uses collections that need std imports
    deps.push_str("anyhow = \"1\"\n");

    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
{deps}"#,
        name = project_name,
        deps = deps,
    )
}

fn generate_lib_rs(results: &[ConversionResult]) -> String {
    let mut content = String::new();
    for result in results {
        if result.success {
            let module = result
                .output_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            if module != "lib" && module != "main" {
                content.push_str(&format!("pub mod {};\n", module));
            }
        }
    }
    if content.is_empty() {
        content.push_str("// Generated by java-to-rust\n");
    }
    content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_code() {
        let response = r#"Here is the converted code:

```rust
fn hello() {
    println!("Hello!");
}
```

This converts the Java method."#;

        let code = extract_rust_code(response);
        assert!(code.contains("fn hello()"));
        assert!(!code.contains("```"));
    }

    #[test]
    fn test_extract_rust_code_no_block() {
        let response = "fn hello() {}";
        let code = extract_rust_code(response);
        assert_eq!(code, "fn hello() {}");
    }

    #[test]
    fn test_generate_cargo_toml_plain() {
        let project = JavaProject {
            path: PathBuf::from("/home/user/my-project"),
            version: JavaVersion::Java8,
            framework: Framework::Plain,
            build_system: BuildSystem::Maven,
            files: Vec::new(),
            dependencies: Vec::new(),
        };
        let toml = generate_cargo_toml(&project);
        assert!(toml.contains("my-project"));
        assert!(toml.contains("anyhow"));
    }

    #[test]
    fn test_generate_cargo_toml_spring() {
        let project = JavaProject {
            path: PathBuf::from("/home/user/spring-app"),
            version: JavaVersion::Java11,
            framework: Framework::SpringBoot,
            build_system: BuildSystem::Maven,
            files: Vec::new(),
            dependencies: Vec::new(),
        };
        let toml = generate_cargo_toml(&project);
        assert!(toml.contains("axum"));
        assert!(toml.contains("tokio"));
    }
}
