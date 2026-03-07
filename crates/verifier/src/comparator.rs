use anyhow::Result;
use std::path::Path;
use std::process::Command;

#[derive(Debug)]
pub struct ComparisonResult {
    pub matches: bool,
    pub java_output: String,
    pub rust_output: String,
    pub diff: Option<String>,
}

pub struct OutputComparator {
    java_binary: String,
}

impl OutputComparator {
    pub fn new(java_binary: Option<String>) -> Self {
        Self {
            java_binary: java_binary.unwrap_or_else(|| "java".to_string()),
        }
    }

    /// Compare the output of running a Java class vs a Rust binary
    pub fn compare(
        &self,
        java_source: &Path,
        rust_binary: &Path,
        args: &[String],
    ) -> Result<ComparisonResult> {
        let java_output = self.run_java(java_source, args)?;
        let rust_output = self.run_rust(rust_binary, args)?;

        let matches = java_output.trim() == rust_output.trim();
        let diff = if matches {
            None
        } else {
            Some(generate_diff(&java_output, &rust_output))
        };

        Ok(ComparisonResult {
            matches,
            java_output,
            rust_output,
            diff,
        })
    }

    fn run_java(&self, source: &Path, args: &[String]) -> Result<String> {
        // Compile first
        let compile_output = Command::new("javac").arg(source).output()?;

        if !compile_output.status.success() {
            let stderr = String::from_utf8_lossy(&compile_output.stderr);
            anyhow::bail!("Java compilation failed: {}", stderr);
        }

        // Run
        let class_name = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Main");

        let parent = source.parent().unwrap_or(Path::new("."));

        let output = Command::new(&self.java_binary)
            .arg("-cp")
            .arg(parent)
            .arg(class_name)
            .args(args)
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn run_rust(&self, binary: &Path, args: &[String]) -> Result<String> {
        let output = Command::new(binary).args(args).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Rust execution failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl Default for OutputComparator {
    fn default() -> Self {
        Self::new(None)
    }
}

fn generate_diff(java_output: &str, rust_output: &str) -> String {
    let java_lines: Vec<&str> = java_output.lines().collect();
    let rust_lines: Vec<&str> = rust_output.lines().collect();

    let mut diff = String::new();
    let max_lines = java_lines.len().max(rust_lines.len());

    for i in 0..max_lines {
        let java_line = java_lines.get(i).unwrap_or(&"<missing>");
        let rust_line = rust_lines.get(i).unwrap_or(&"<missing>");

        if java_line != rust_line {
            diff.push_str(&format!("Line {}:\n", i + 1));
            diff.push_str(&format!("  Java: {}\n", java_line));
            diff.push_str(&format!("  Rust: {}\n", rust_line));
        }
    }

    diff
}

pub fn format_comparison_for_llm(result: &ComparisonResult) -> String {
    if result.matches {
        return "Output matches perfectly.".to_string();
    }

    let mut output = String::from("Output mismatch detected:\n\n");
    output.push_str(&format!(
        "Java output:\n```\n{}\n```\n\n",
        result.java_output
    ));
    output.push_str(&format!(
        "Rust output:\n```\n{}\n```\n\n",
        result.rust_output
    ));

    if let Some(ref diff) = result.diff {
        output.push_str(&format!("Differences:\n{}\n", diff));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_diff_matching() {
        let diff = generate_diff("Hello\nWorld", "Hello\nWorld");
        assert!(diff.is_empty());
    }

    #[test]
    fn test_generate_diff_mismatch() {
        let diff = generate_diff("Hello\nWorld", "Hello\nRust");
        assert!(diff.contains("Line 2:"));
        assert!(diff.contains("World"));
        assert!(diff.contains("Rust"));
    }

    #[test]
    fn test_format_comparison_match() {
        let result = ComparisonResult {
            matches: true,
            java_output: "Hello".to_string(),
            rust_output: "Hello".to_string(),
            diff: None,
        };
        assert_eq!(
            format_comparison_for_llm(&result),
            "Output matches perfectly."
        );
    }

    #[test]
    fn test_format_comparison_mismatch() {
        let result = ComparisonResult {
            matches: false,
            java_output: "Hello".to_string(),
            rust_output: "Hi".to_string(),
            diff: Some("Line 1: Hello vs Hi".to_string()),
        };
        let formatted = format_comparison_for_llm(&result);
        assert!(formatted.contains("mismatch"));
        assert!(formatted.contains("Hello"));
    }
}
