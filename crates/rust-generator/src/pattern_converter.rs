/// Pattern-based Java → Rust converter that operates without an LLM.
///
/// Converts static utility methods deterministically using type mapping rules.
/// Suitable for offline use, CI pipelines, and generating compilation stubs
/// before LLM refinement.
use java_parser::types::{JavaFile, JavaMethod, Modifier};

use crate::patterns::PatternMapper;

pub struct PatternConverter {
    mapper: PatternMapper,
}

/// A single converted Rust function.
#[derive(Debug)]
pub struct ConvertedFn {
    pub java_name: String,
    pub rust_name: String,
    pub code: String,
}

/// Result of converting a single Java file via pattern matching.
#[derive(Debug)]
pub struct PatternConversionResult {
    pub module_name: String,
    pub converted_fns: Vec<ConvertedFn>,
    pub skipped: Vec<String>,
    pub rust_source: String,
}

impl PatternConverter {
    pub fn new() -> Self {
        Self {
            mapper: PatternMapper::new(),
        }
    }

    /// Convert a parsed Java file to a Rust module using pattern rules.
    /// Only public static methods are converted; others are listed as skipped.
    pub fn convert_file(&self, file: &JavaFile) -> PatternConversionResult {
        let module_name = file
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_lowercase();

        let mut converted_fns = Vec::new();
        let mut skipped = Vec::new();
        // Track how many times each base rust_name has been used (for overload resolution).
        let mut name_counter: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for class in &file.classes {
            for method in &class.methods {
                if method.is_constructor {
                    continue;
                }
                let is_public = method.modifiers.contains(&Modifier::Public);
                let is_static = method.modifiers.contains(&Modifier::Static);

                if is_public && is_static {
                    if let Some(mut converted) = self.convert_static_method(method) {
                        // Resolve Java method overloading with a counter suffix.
                        let base_name = converted.rust_name.clone();
                        let count = name_counter.entry(base_name.clone()).or_insert(0);
                        if *count > 0 {
                            let suffixed = format!("{}_{}", base_name, count);
                            converted.code = converted.code.replacen(
                                &format!("pub fn {}(", base_name),
                                &format!("pub fn {}(", suffixed),
                                1,
                            );
                            converted.rust_name = suffixed;
                        }
                        *count += 1;
                        converted_fns.push(converted);
                    } else {
                        skipped.push(method.name.clone());
                    }
                } else {
                    skipped.push(method.name.clone());
                }
            }
        }

        let rust_source = self.generate_module(&module_name, &converted_fns, file);

        PatternConversionResult {
            module_name,
            converted_fns,
            skipped,
            rust_source,
        }
    }

    fn convert_static_method(&self, method: &JavaMethod) -> Option<ConvertedFn> {
        let rust_name = to_snake_case(&method.name);

        // Convert return type
        let ret = method.return_type.as_deref().unwrap_or("void");
        let (rust_ret, uses_result) = self.convert_return_type(ret, !method.throws.is_empty());

        // Convert parameters - collect (rust_name, rust_type) pairs
        let param_pairs: Vec<(String, String)> = method
            .params
            .iter()
            .map(|p| {
                let rust_type = self.convert_param_type(&p.type_name);
                (to_snake_case(&p.name), rust_type)
            })
            .collect();

        let params_str = param_pairs
            .iter()
            .map(|(n, t)| format!("{}: {}", n, t))
            .collect::<Vec<_>>()
            .join(", ");

        let sig = if rust_ret == "()" {
            format!("pub fn {}({})", rust_name, params_str)
        } else {
            format!("pub fn {}({}) -> {}", rust_name, params_str, rust_ret)
        };

        // Generate body using actual parameter names
        let body = self.generate_body(method, &param_pairs, uses_result);

        let code = format!("{} {{\n    {}\n}}", sig, body);

        Some(ConvertedFn {
            java_name: method.name.clone(),
            rust_name,
            code,
        })
    }

    fn convert_return_type(&self, java_type: &str, throws: bool) -> (String, bool) {
        let inner = self.map_type_str(java_type);
        if throws {
            (format!("Result<{}, anyhow::Error>", inner), true)
        } else {
            (inner, false)
        }
    }

    fn convert_param_type(&self, java_type: &str) -> String {
        // CharSequence, String → Option<&str> (most common for utility methods)
        if java_type == "CharSequence" || java_type == "String" {
            return "Option<&str>".to_string();
        }
        // Varargs → &[&str]
        if java_type == "String..." || java_type == "CharSequence..." {
            return "&[&str]".to_string();
        }
        // int, long, etc.
        if let Some(mapped) = self.mapper.map_type(java_type) {
            return mapped.to_string();
        }
        // Generic fallback
        sanitize_type(java_type)
    }

    fn map_type_str(&self, java_type: &str) -> String {
        // Strip leading generic type parameter declarations like "<T extends X> T"
        let java_type = if java_type.starts_with('<') {
            // e.g. "<T extends CharSequence> T" → take part after closing ">"
            if let Some(end) = java_type.find('>') {
                java_type[end + 1..].trim()
            } else {
                java_type
            }
        } else {
            java_type
        };

        match java_type {
            "void" => "()".to_string(),
            "String" => "Option<String>".to_string(),
            "boolean" | "Boolean" => "bool".to_string(),
            "int" | "Integer" => "i32".to_string(),
            "long" | "Long" => "i64".to_string(),
            "double" | "Double" => "f64".to_string(),
            "float" | "Float" => "f32".to_string(),
            "char" | "Character" => "char".to_string(),
            "byte" | "Byte" => "u8".to_string(),
            "short" | "Short" => "i16".to_string(),
            _ => {
                if let Some(mapped) = self.mapper.map_type(java_type) {
                    mapped.to_string()
                } else {
                    sanitize_type(java_type)
                }
            }
        }
    }

    /// Generate a Rust function body based on known method name patterns.
    /// Uses actual parameter names from the method definition.
    fn generate_body(
        &self,
        method: &JavaMethod,
        params: &[(String, String)],
        uses_result: bool,
    ) -> String {
        let name = method.name.as_str();

        // Helper: get nth parameter name
        let p = |n: usize| params.get(n).map(|(name, _)| name.as_str()).unwrap_or("_");

        // First param type check (CharSequence/String)
        let first_is_str = method
            .params
            .first()
            .map(|p| p.type_name == "CharSequence" || p.type_name == "String")
            .unwrap_or(false);

        let body = match (name, first_is_str) {
            ("isEmpty", true) => {
                format!("{}.map(|s| s.is_empty()).unwrap_or(true)", p(0))
            }
            ("isNotEmpty", true) => {
                format!("{}.map(|s| !s.is_empty()).unwrap_or(false)", p(0))
            }
            ("isBlank", true) => {
                format!("{}.map(|s| s.trim().is_empty()).unwrap_or(true)", p(0))
            }
            ("isNotBlank", true) => {
                format!("{}.map(|s| !s.trim().is_empty()).unwrap_or(false)", p(0))
            }
            ("length", true) => {
                format!("{}.map(|s| s.len() as i32).unwrap_or(0)", p(0))
            }
            ("strip", true) if params.len() == 1 => {
                format!("{}.map(|s| s.trim().to_string())", p(0))
            }
            ("stripStart", true) if params.len() == 1 => {
                format!("{}.map(|s| s.trim_start().to_string())", p(0))
            }
            ("stripEnd", true) if params.len() == 1 => {
                format!("{}.map(|s| s.trim_end().to_string())", p(0))
            }
            ("trim", true) if params.len() == 1 => {
                format!("{}.map(|s| s.trim().to_string())", p(0))
            }
            ("reverse", true) => {
                format!("{}.map(|s| s.chars().rev().collect::<String>())", p(0))
            }
            ("capitalize", true) => {
                format!(
                    r#"{p}.map(|s| {{
        let mut c = s.chars();
        match c.next() {{
            None => String::new(),
            Some(first) => first.to_uppercase().to_string() + c.as_str(),
        }}
    }})"#,
                    p = p(0)
                )
            }
            ("upperCase", true) | ("toUpperCase", true) => {
                format!("{}.map(|s| s.to_uppercase())", p(0))
            }
            ("lowerCase", true) | ("toLowerCase", true) => {
                format!("{}.map(|s| s.to_lowercase())", p(0))
            }
            ("deleteWhitespace", true) => {
                format!(
                    "{}.map(|s| s.chars().filter(|c| !c.is_whitespace()).collect::<String>())",
                    p(0)
                )
            }
            _ => format!("todo!(\"{name}\")", name = name),
        };

        if uses_result {
            format!("Ok({{ {} }})", body)
        } else {
            body
        }
    }

    fn generate_module(&self, _module_name: &str, fns: &[ConvertedFn], file: &JavaFile) -> String {
        let mut out = String::new();

        // Header comment
        out.push_str("// Generated by java-to-rust PatternConverter\n");
        out.push_str("// Source: ");
        out.push_str(&file.path.display().to_string());
        out.push('\n');
        if let Some(pkg) = &file.package {
            out.push_str(&format!("// Package: {}\n", pkg));
        }
        out.push('\n');

        // Import context from pattern mapper
        let import_hints = self.mapper.generate_context(&file.imports);
        if !import_hints.is_empty() {
            for line in import_hints.lines() {
                if !line.is_empty() {
                    out.push_str(&format!("// {}\n", line));
                }
            }
            out.push('\n');
        }

        // Allow dead_code for stubs
        out.push_str("#![allow(dead_code, unused_variables)]\n\n");

        // Functions
        for f in fns {
            out.push_str(&format!("// Java: {}\n", f.java_name));
            out.push_str(&f.code);
            out.push_str("\n\n");
        }

        out
    }
}

impl Default for PatternConverter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert Java camelCase to Rust snake_case.
pub fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    let mut prev_upper = false;
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && !prev_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_upper = true;
        } else {
            prev_upper = false;
            result.push(ch);
        }
    }
    result
}

/// Sanitize a Java type name into a plausible Rust type.
fn sanitize_type(java_type: &str) -> String {
    // Basic primitive/common type mappings (before generic/array handling)
    match java_type {
        "String" => return "String".to_string(),
        "Object" => return "Box<dyn std::any::Any>".to_string(),
        "int" | "Integer" => return "i32".to_string(),
        "long" | "Long" => return "i64".to_string(),
        "double" | "Double" => return "f64".to_string(),
        "float" | "Float" => return "f32".to_string(),
        "boolean" | "Boolean" => return "bool".to_string(),
        "char" | "Character" => return "char".to_string(),
        "byte" | "Byte" => return "u8".to_string(),
        "short" | "Short" => return "i16".to_string(),
        "void" => return "()".to_string(),
        _ => {}
    }
    // Handle generics like List<String> → Vec<String>
    if java_type.contains('<') {
        // Replace Java wildcards with Box<dyn Any>
        let java_type = java_type
            .replace("? extends", "Box<dyn std::any::Any>")
            .replace("? super", "Box<dyn std::any::Any>")
            .replace("?", "Box<dyn std::any::Any>");
        let java_type = java_type.as_str();
        // Isolate outer type name
        let outer = if let Some(idx) = java_type.find('<') {
            &java_type[..idx]
        } else {
            java_type
        };
        match outer {
            "List" | "ArrayList" | "LinkedList" => {
                return java_type
                    .replace("ArrayList<", "Vec<")
                    .replace("LinkedList<", "Vec<")
                    .replace("List<", "Vec<");
            }
            "Map" | "HashMap" | "TreeMap" | "LinkedHashMap" => {
                return java_type
                    .replace("LinkedHashMap<", "std::collections::HashMap<")
                    .replace("TreeMap<", "std::collections::BTreeMap<")
                    .replace("HashMap<", "std::collections::HashMap<")
                    .replace("Map<", "std::collections::HashMap<");
            }
            "Optional" => {
                return java_type.replace("Optional<", "Option<");
            }
            "Set" | "HashSet" | "TreeSet" => {
                return java_type
                    .replace("TreeSet<", "std::collections::BTreeSet<")
                    .replace("HashSet<", "std::collections::HashSet<")
                    .replace("Set<", "std::collections::HashSet<");
            }
            "Iterable" | "Iterator" | "Collection" => {
                // Erase inner type — use Vec
                return "Vec<Box<dyn std::any::Any>>".to_string();
            }
            "Supplier" | "Callable" | "Function" | "BiFunction" | "Predicate" | "Consumer"
            | "BiConsumer" | "Comparator" => {
                // Java functional interfaces → opaque function type
                return "Box<dyn std::any::Any>".to_string();
            }
            _ => {
                // Unknown generic — keep as-is (will be annotated by fallback)
            }
        }
        return format!("/* {} */ Box<dyn std::any::Any>", java_type);
    }
    // Arrays
    if let Some(inner) = java_type.strip_suffix("[]") {
        return format!("Vec<{}>", sanitize_type(inner));
    }
    // Java functional interfaces
    match java_type {
        "Supplier" | "Callable" => return "Box<dyn Fn() -> Box<dyn std::any::Any>>".to_string(),
        "Runnable" => return "Box<dyn Fn()>".to_string(),
        "Iterable" | "Iterator" => {
            return "Box<dyn Iterator<Item = Box<dyn std::any::Any>>>".to_string();
        }
        "Comparable" => return "Box<dyn std::cmp::PartialOrd>".to_string(),
        _ => {}
    }
    // Single-letter or short generic type parameters (T, E, K, V, etc.)
    if java_type.len() <= 2 && java_type.chars().all(|c| c.is_uppercase()) {
        return "Box<dyn std::any::Any>".to_string();
    }
    // Varargs without ... already handled above
    // Fallback: annotate as unknown
    format!("/* {} */ Box<dyn std::any::Any>", java_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("isEmpty"), "is_empty");
        assert_eq!(to_snake_case("isNotBlank"), "is_not_blank");
        assert_eq!(to_snake_case("toUpperCase"), "to_upper_case");
        assert_eq!(to_snake_case("strip"), "strip");
        assert_eq!(to_snake_case("length"), "length");
    }

    #[test]
    fn test_sanitize_type() {
        assert_eq!(sanitize_type("String[]"), "Vec<String>");
        assert!(sanitize_type("List<String>").contains("Vec"));
        assert!(sanitize_type("Optional<String>").contains("Option"));
    }

    #[test]
    fn test_convert_param_type() {
        let conv = PatternConverter::new();
        assert_eq!(conv.convert_param_type("CharSequence"), "Option<&str>");
        assert_eq!(conv.convert_param_type("String"), "Option<&str>");
        assert_eq!(conv.convert_param_type("int"), "i32");
        assert_eq!(conv.convert_param_type("boolean"), "bool");
    }

    #[test]
    fn test_convert_return_type_plain() {
        let conv = PatternConverter::new();
        let (ty, uses_result) = conv.convert_return_type("boolean", false);
        assert_eq!(ty, "bool");
        assert!(!uses_result);
    }

    #[test]
    fn test_convert_return_type_throws() {
        let conv = PatternConverter::new();
        let (ty, uses_result) = conv.convert_return_type("String", true);
        assert!(ty.contains("Result"));
        assert!(uses_result);
    }
}
