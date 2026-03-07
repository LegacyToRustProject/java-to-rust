use crate::compiler::{CompileChecker, CompileResult, format_errors_for_llm};
use anyhow::Result;
use rust_generator::llm::{LlmProvider, LlmRequest};
use rust_generator::prompt::fix_prompt;
use std::path::Path;

pub struct FixLoop {
    llm: Box<dyn LlmProvider>,
    checker: CompileChecker,
    max_iterations: usize,
}

#[derive(Debug)]
pub struct FixResult {
    pub success: bool,
    pub final_code: String,
    pub iterations: usize,
    pub remaining_errors: Vec<String>,
}

impl FixLoop {
    pub fn new(llm: Box<dyn LlmProvider>, max_iterations: usize) -> Self {
        Self {
            llm,
            checker: CompileChecker::new(),
            max_iterations,
        }
    }

    pub async fn run(&self, project_dir: &Path, file_path: &Path) -> Result<FixResult> {
        let mut current_code = std::fs::read_to_string(file_path)?;
        let mut iterations = 0;

        loop {
            iterations += 1;
            tracing::info!("Fix loop iteration {}/{}", iterations, self.max_iterations);

            let (result, raw_output) = self.checker.check_with_full_output(project_dir)?;

            match result {
                CompileResult::Success => {
                    tracing::info!("Compilation successful after {} iterations", iterations);
                    return Ok(FixResult {
                        success: true,
                        final_code: current_code,
                        iterations,
                        remaining_errors: Vec::new(),
                    });
                }
                CompileResult::Errors(errors) => {
                    if iterations >= self.max_iterations {
                        tracing::warn!(
                            "Max iterations ({}) reached with {} remaining errors",
                            self.max_iterations,
                            errors.len()
                        );
                        return Ok(FixResult {
                            success: false,
                            final_code: current_code,
                            iterations,
                            remaining_errors: errors.iter().map(|e| e.to_string()).collect(),
                        });
                    }

                    let error_text = format_errors_for_llm(&errors);
                    tracing::info!(
                        "Found {} errors, requesting fix via {}",
                        errors.len(),
                        self.llm.name()
                    );

                    let request = LlmRequest {
                        system_prompt:
                            "You are a Rust expert. Fix the compilation errors and return corrected code."
                                .to_string(),
                        user_prompt: fix_prompt(&current_code, &error_text),
                        max_tokens: 8192,
                        temperature: 0.0,
                    };

                    let response = self.llm.generate(&request).await?;
                    let fixed_code = extract_rust_code(&response.content);

                    if fixed_code == current_code {
                        tracing::warn!("LLM returned identical code, aborting fix loop");
                        return Ok(FixResult {
                            success: false,
                            final_code: current_code,
                            iterations,
                            remaining_errors: errors.iter().map(|e| e.to_string()).collect(),
                        });
                    }

                    // Write the fixed code
                    std::fs::write(file_path, &fixed_code)?;
                    current_code = fixed_code;

                    // Log raw output for debugging
                    tracing::debug!("Compiler output:\n{}", raw_output);
                }
            }
        }
    }
}

fn extract_rust_code(response: &str) -> String {
    let re = regex::Regex::new(r"```rust\n([\s\S]*?)```").unwrap();
    if let Some(cap) = re.captures(response) {
        cap[1].trim().to_string()
    } else {
        response.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_code() {
        let response = "```rust\nfn main() {}\n```";
        assert_eq!(extract_rust_code(response), "fn main() {}");
    }

    #[test]
    fn test_extract_rust_code_no_block() {
        let response = "fn main() {}";
        assert_eq!(extract_rust_code(response), "fn main() {}");
    }
}
