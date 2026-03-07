use anyhow::Result;
use regex::Regex;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CompileError {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub suggestion: Option<String>,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.file, self.line, self.column, self.message
        )
    }
}

#[derive(Debug)]
pub enum CompileResult {
    Success,
    Errors(Vec<CompileError>),
}

pub struct CompileChecker;

impl CompileChecker {
    pub fn new() -> Self {
        Self
    }

    pub fn check(&self, project_dir: &Path) -> Result<CompileResult> {
        let output = Command::new("cargo")
            .arg("check")
            .arg("--message-format=short")
            .current_dir(project_dir)
            .output()?;

        if output.status.success() {
            return Ok(CompileResult::Success);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let errors = parse_compiler_errors(&stderr);

        if errors.is_empty() {
            // If we couldn't parse errors but compilation failed, return raw error
            Ok(CompileResult::Errors(vec![CompileError {
                file: "unknown".to_string(),
                line: 0,
                column: 0,
                message: stderr.to_string(),
                suggestion: None,
            }]))
        } else {
            Ok(CompileResult::Errors(errors))
        }
    }

    pub fn check_with_full_output(&self, project_dir: &Path) -> Result<(CompileResult, String)> {
        let output = Command::new("cargo")
            .arg("check")
            .current_dir(project_dir)
            .output()?;

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok((CompileResult::Success, stderr))
        } else {
            let errors = parse_compiler_errors(&stderr);
            if errors.is_empty() {
                Ok((
                    CompileResult::Errors(vec![CompileError {
                        file: "unknown".to_string(),
                        line: 0,
                        column: 0,
                        message: stderr.clone(),
                        suggestion: None,
                    }]),
                    stderr,
                ))
            } else {
                Ok((CompileResult::Errors(errors), stderr))
            }
        }
    }
}

impl Default for CompileChecker {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_compiler_errors(stderr: &str) -> Vec<CompileError> {
    let error_re = Regex::new(r"error(?:\[E\d+\])?: (.+)\n\s*--> ([^:]+):(\d+):(\d+)").unwrap();
    let suggestion_re = Regex::new(r"help: (.+)").unwrap();

    let mut errors = Vec::new();

    for cap in error_re.captures_iter(stderr) {
        let message = cap[1].to_string();
        let file = cap[2].to_string();
        let line: usize = cap[3].parse().unwrap_or(0);
        let column: usize = cap[4].parse().unwrap_or(0);

        // Look for a suggestion near this error
        let error_end = cap.get(0).unwrap().end();
        let remaining = &stderr[error_end..];
        let suggestion = suggestion_re.captures(remaining).map(|s| s[1].to_string());

        errors.push(CompileError {
            file,
            line,
            column,
            message,
            suggestion,
        });
    }

    errors
}

pub fn format_errors_for_llm(errors: &[CompileError]) -> String {
    let mut output = String::new();
    for (i, error) in errors.iter().enumerate() {
        output.push_str(&format!(
            "Error {}: {} (at {}:{}:{})\n",
            i + 1,
            error.message,
            error.file,
            error.line,
            error.column
        ));
        if let Some(ref suggestion) = error.suggestion {
            output.push_str(&format!("  Suggestion: {}\n", suggestion));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_compiler_errors() {
        let stderr = r#"error[E0308]: mismatched types
 --> src/main.rs:5:20
  |
5 |     let x: i32 = "hello";
  |            ---   ^^^^^^^ expected `i32`, found `&str`
  |            |
  |            expected due to this
  |
help: try converting the string to an integer
"#;

        let errors = parse_compiler_errors(stderr);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].file, "src/main.rs");
        assert_eq!(errors[0].line, 5);
        assert!(errors[0].message.contains("mismatched types"));
        assert!(errors[0].suggestion.is_some());
    }

    #[test]
    fn test_format_errors_for_llm() {
        let errors = vec![CompileError {
            file: "src/lib.rs".to_string(),
            line: 10,
            column: 5,
            message: "cannot find value `x`".to_string(),
            suggestion: Some("did you mean `y`?".to_string()),
        }];
        let formatted = format_errors_for_llm(&errors);
        assert!(formatted.contains("cannot find value"));
        assert!(formatted.contains("did you mean"));
    }
}
