use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "java-to-rust")]
#[command(about = "AI-powered Java to Rust conversion engine")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a Java project and generate a structure report
    Analyze {
        /// Path to the Java project
        path: PathBuf,

        /// Output format (text, json)
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Convert a Java project to Rust
    Convert {
        /// Path to the Java project
        path: PathBuf,

        /// Output directory for the Rust project
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Conversion profile (generic, spring-boot, java-ee, android)
        #[arg(long, default_value = "generic")]
        profile: String,

        /// Run cargo check verification after conversion
        #[arg(long)]
        verify: bool,

        /// LLM provider (claude)
        #[arg(long, default_value = "claude")]
        llm: String,

        /// Model name
        #[arg(long)]
        model: Option<String>,

        /// Max fix loop iterations
        #[arg(long, default_value = "10")]
        max_fix_iterations: usize,
    },
    /// Convert a single Java file to Rust using pattern-based conversion (no LLM required)
    ConvertFile {
        /// Path to the Java source file (.java)
        file: PathBuf,

        /// Output directory (default: ./output/<ClassName>)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Run cargo check on the generated output
        #[arg(long)]
        verify: bool,

        /// Print conversion summary (skipped methods, etc.)
        #[arg(long)]
        summary: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { path, format } => cmd_analyze(&path, &format),
        Commands::Convert {
            path,
            output,
            profile,
            verify,
            llm: _,
            model,
            max_fix_iterations,
        } => cmd_convert(&path, output, &profile, verify, model, max_fix_iterations).await,
        Commands::ConvertFile {
            file,
            output,
            verify,
            summary,
        } => cmd_convert_file(&file, output, verify, summary),
    }
}

fn cmd_analyze(path: &Path, format: &str) -> Result<()> {
    let project = java_parser::analyze_project(path)?;
    let report = java_parser::generate_report(&project);

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        _ => {
            println!("=== Java Project Analysis ===");
            println!();
            println!("Project: {}", report.project_path.display());
            println!("Java Version: {}", report.java_version);
            println!("Framework: {}", report.framework);
            println!("Build System: {}", report.build_system);
            println!();
            println!("--- Statistics ---");
            println!("Files: {}", report.total_files);
            println!("Classes: {}", report.total_classes);
            println!("Interfaces: {}", report.total_interfaces);
            println!("Enums: {}", report.total_enums);
            println!("Methods: {}", report.total_methods);
            println!();

            if !report.dependencies.is_empty() {
                println!("--- Dependencies ---");
                for dep in &report.dependencies {
                    let version = dep.version.as_deref().unwrap_or("?");
                    println!("  {}:{}:{}", dep.group_id, dep.artifact_id, version);
                }
                println!();
            }

            if !report.annotations_used.is_empty() {
                println!("--- Annotations Used ---");
                let mut anns = report.annotations_used.clone();
                anns.sort();
                for ann in &anns {
                    println!("  @{}", ann);
                }
            }
        }
    }

    Ok(())
}

fn cmd_convert_file(
    file: &Path,
    output: Option<PathBuf>,
    verify: bool,
    summary: bool,
) -> Result<()> {
    use rust_generator::PatternConverter;

    if !file.exists() {
        anyhow::bail!("File not found: {}", file.display());
    }
    if file.extension().and_then(|e| e.to_str()) != Some("java") {
        anyhow::bail!("Expected a .java file, got: {}", file.display());
    }

    println!("=== Pattern-based Java → Rust conversion ===");
    println!("Source: {}", file.display());
    println!();

    // Parse the single file via analyze_project on parent dir, then find the file
    // Alternatively, parse directly
    let java_file = java_parser::analyze_file(file)?;

    let converter = PatternConverter::new();
    let result = converter.convert_file(&java_file);

    println!("Module : {}", result.module_name);
    println!("Converted : {} functions", result.converted_fns.len());
    println!("Skipped   : {} functions", result.skipped.len());

    if summary {
        if !result.converted_fns.is_empty() {
            println!("\n--- Converted ---");
            for f in &result.converted_fns {
                println!("  {} → {}", f.java_name, f.rust_name);
            }
        }
        if !result.skipped.is_empty() {
            println!("\n--- Skipped (non-public/non-static) ---");
            for name in &result.skipped {
                println!("  {}", name);
            }
        }
    }

    // Write output
    let output_dir = output.unwrap_or_else(|| PathBuf::from("output").join(&result.module_name));
    std::fs::create_dir_all(output_dir.join("src"))?;

    // Write Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[workspace]

[dependencies]
anyhow = "1"
"#,
        name = result.module_name
    );
    std::fs::write(output_dir.join("Cargo.toml"), &cargo_toml)?;

    // Write src/lib.rs
    std::fs::write(output_dir.join("src/lib.rs"), &result.rust_source)?;

    println!("\nOutput: {}", output_dir.display());

    if verify {
        println!("\n--- Running cargo check ---");
        let checker = verifier::CompileChecker::new();
        match checker.check(&output_dir)? {
            verifier::CompileResult::Success => {
                println!("cargo check: PASSED ✓");
            }
            verifier::CompileResult::Errors(errors) => {
                println!("cargo check: FAILED ({} errors)", errors.len());
                for e in &errors {
                    println!("  {}:{}:{} {}", e.file, e.line, e.column, e.message);
                }
            }
        }
    }

    Ok(())
}

async fn cmd_convert(
    path: &Path,
    output: Option<PathBuf>,
    _profile: &str,
    verify: bool,
    _model: Option<String>,
    max_fix_iterations: usize,
) -> Result<()> {
    let project = java_parser::analyze_project(path)?;

    println!(
        "Detected: Java {} / {} / {}",
        project.version, project.framework, project.build_system
    );
    println!("Files to convert: {}", project.files.len());
    println!();

    let output_dir = output.unwrap_or_else(|| {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        PathBuf::from(format!("{}-rust", name))
    });

    let llm = rust_generator::ClaudeProvider::from_env()?;
    let generator = rust_generator::Generator::new(Box::new(llm));

    let config = rust_generator::ConversionConfig {
        output_dir: output_dir.clone(),
        profile: _profile.to_string(),
        verify,
    };

    let results = generator.convert_project(&project, &config).await?;

    let success_count = results.iter().filter(|r| r.success).count();
    println!();
    println!("=== Conversion Complete ===");
    println!("Converted: {}/{} files", success_count, results.len());
    println!("Output: {}", output_dir.display());

    if verify {
        println!();
        println!("--- Running verification ---");

        let checker = verifier::CompileChecker::new();
        match checker.check(&output_dir)? {
            verifier::CompileResult::Success => {
                println!("cargo check: PASSED");
            }
            verifier::CompileResult::Errors(errors) => {
                println!("cargo check: FAILED ({} errors)", errors.len());

                println!(
                    "Starting fix loop (max {} iterations)...",
                    max_fix_iterations
                );
                let fix_llm = rust_generator::ClaudeProvider::from_env()?;
                let fix_loop = verifier::FixLoop::new(Box::new(fix_llm), max_fix_iterations);

                // Fix each file with errors
                for result in &results {
                    if result.success {
                        let fix_result = fix_loop.run(&output_dir, &result.output_path).await?;
                        if fix_result.success {
                            println!(
                                "  Fixed: {} ({} iterations)",
                                result.output_path.display(),
                                fix_result.iterations
                            );
                        } else {
                            println!(
                                "  Failed to fix: {} ({} remaining errors)",
                                result.output_path.display(),
                                fix_result.remaining_errors.len()
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
