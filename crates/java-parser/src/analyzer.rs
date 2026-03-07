use crate::annotations::parse_annotations;
use crate::types::*;
use anyhow::Result;
use regex::Regex;
use std::path::Path;
use walkdir::WalkDir;

pub fn analyze_project(project_path: &Path) -> Result<JavaProject> {
    let build_system = crate::dependency::detect_build_system(project_path);
    let version = crate::dependency::detect_java_version_from_build(project_path);
    let dependencies = crate::dependency::parse_dependencies(project_path)?;

    let mut files = Vec::new();
    for entry in WalkDir::new(project_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "java") {
            match analyze_file(path) {
                Ok(java_file) => files.push(java_file),
                Err(e) => tracing::warn!("Failed to parse {}: {}", path.display(), e),
            }
        }
    }

    let framework = detect_framework(&files, &dependencies);

    // If version not detected from build files, try source analysis
    let version = if version == JavaVersion::Unknown {
        detect_version_from_source(&files)
    } else {
        version
    };

    Ok(JavaProject {
        path: project_path.to_path_buf(),
        version,
        framework,
        build_system,
        files,
        dependencies,
    })
}

pub fn analyze_file(file_path: &Path) -> Result<JavaFile> {
    let source = std::fs::read_to_string(file_path)?;
    let package = extract_package(&source);
    let imports = extract_imports(&source);
    let classes = extract_classes(&source);
    let interfaces = extract_interfaces(&source);
    let enums = extract_enums(&source);

    Ok(JavaFile {
        path: file_path.to_path_buf(),
        package,
        imports,
        classes,
        interfaces,
        enums,
        source,
    })
}

fn extract_package(source: &str) -> Option<String> {
    let re = Regex::new(r"package\s+([\w.]+)\s*;").unwrap();
    re.captures(source).map(|cap| cap[1].to_string())
}

fn extract_imports(source: &str) -> Vec<String> {
    let re = Regex::new(r"import\s+(?:static\s+)?([\w.*]+)\s*;").unwrap();
    re.captures_iter(source)
        .map(|cap| cap[1].to_string())
        .collect()
}

fn extract_classes(source: &str) -> Vec<JavaClass> {
    let re = Regex::new(
        r"(?m)^((?:\s*@\w+(?:\([^)]*\))?\s*\n)*)\s*((?:public|private|protected|abstract|static|final)\s+)*class\s+(\w+)(?:<([^>]+)>)?(?:\s+extends\s+(\w+))?(?:\s+implements\s+([\w,\s]+))?\s*\{",
    )
    .unwrap();

    let mut classes = Vec::new();
    for cap in re.captures_iter(source) {
        let annotation_block = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let modifier_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let name = cap[3].to_string();
        let generic_params = cap
            .get(4)
            .map(|m| {
                m.as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            })
            .unwrap_or_default();
        let extends = cap.get(5).map(|m| m.as_str().to_string());
        let implements = cap
            .get(6)
            .map(|m| {
                m.as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            })
            .unwrap_or_default();

        let modifiers = parse_modifiers(modifier_str);
        let is_abstract = modifiers.contains(&Modifier::Abstract);
        let annotations = parse_annotations(annotation_block);

        // Extract class body for methods and fields
        let class_body = extract_block_body(source, cap.get(0).unwrap().end() - 1);
        let methods = extract_methods(&class_body, &name);
        let fields = extract_fields(&class_body);

        classes.push(JavaClass {
            name,
            modifiers,
            extends,
            implements,
            fields,
            methods,
            annotations,
            is_abstract,
            generic_params,
        });
    }

    classes
}

fn extract_interfaces(source: &str) -> Vec<JavaInterface> {
    let re = Regex::new(
        r"(?m)^((?:\s*@\w+(?:\([^)]*\))?\s*\n)*)\s*(?:public\s+)?interface\s+(\w+)(?:<([^>]+)>)?(?:\s+extends\s+([\w,\s<>]+))?\s*\{",
    )
    .unwrap();

    let mut interfaces = Vec::new();
    for cap in re.captures_iter(source) {
        let annotation_block = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let name = cap[2].to_string();
        let generic_params = cap
            .get(3)
            .map(|m| {
                m.as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            })
            .unwrap_or_default();
        let extends = cap
            .get(4)
            .map(|m| {
                m.as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            })
            .unwrap_or_default();

        let annotations = parse_annotations(annotation_block);
        let body = extract_block_body(source, cap.get(0).unwrap().end() - 1);
        let methods = extract_interface_methods(&body);

        interfaces.push(JavaInterface {
            name,
            extends,
            methods,
            annotations,
            generic_params,
        });
    }

    interfaces
}

fn extract_enums(source: &str) -> Vec<JavaEnum> {
    let re =
        Regex::new(r"(?m)^((?:\s*@\w+(?:\([^)]*\))?\s*\n)*)\s*(?:public\s+)?enum\s+(\w+)\s*\{")
            .unwrap();

    let mut enums = Vec::new();
    for cap in re.captures_iter(source) {
        let annotation_block = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let name = cap[2].to_string();
        let annotations = parse_annotations(annotation_block);
        let body = extract_block_body(source, cap.get(0).unwrap().end() - 1);

        // Extract enum variants (before the first semicolon or closing brace)
        let variants_section = body.split(';').next().unwrap_or(&body);
        let variants: Vec<String> = variants_section
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with('}'))
            .collect();

        enums.push(JavaEnum {
            name,
            variants,
            methods: Vec::new(),
            annotations,
        });
    }

    enums
}

fn extract_block_body(source: &str, open_brace_pos: usize) -> String {
    let bytes = source.as_bytes();
    let mut depth = 0;
    let mut start = open_brace_pos;
    let mut end = open_brace_pos;

    for (i, &b) in bytes.iter().enumerate().skip(open_brace_pos) {
        match b {
            b'{' => {
                if depth == 0 {
                    start = i + 1;
                }
                depth += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }

    source[start..end].to_string()
}

fn extract_fields(class_body: &str) -> Vec<JavaField> {
    let field_re = Regex::new(
        r"(?m)^\s*((?:(?:public|private|protected|static|final|volatile|transient)\s+)*)([\w<>\[\],\s]+?)\s+(\w+)\s*(?:=\s*([^;]+))?\s*;",
    ).unwrap();

    let mut fields = Vec::new();
    for cap in field_re.captures_iter(class_body) {
        let modifier_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let type_name = cap[2].trim().to_string();
        let name = cap[3].to_string();
        let initial_value = cap.get(4).map(|m| m.as_str().trim().to_string());

        // Skip if it looks like a local variable in a method
        if type_name.contains('(') || type_name.contains(')') {
            continue;
        }

        let modifiers = parse_modifiers(modifier_str);

        fields.push(JavaField {
            name,
            type_name,
            modifiers,
            annotations: Vec::new(),
            initial_value,
        });
    }

    fields
}

fn extract_methods(class_body: &str, class_name: &str) -> Vec<JavaMethod> {
    let method_re = Regex::new(
        r"(?m)((?:\s*@\w+(?:\([^)]*\))?\s*\n)*)\s*((?:(?:public|private|protected|static|final|abstract|synchronized|default)\s+)*)((?:<[^>]+>\s+)?(?:\w+(?:\[\])?(?:<[^>]+>)?)\s+)?(\w+)\s*\(([^)]*)\)(?:\s*throws\s+([\w,\s]+))?\s*\{?",
    ).unwrap();

    let mut methods = Vec::new();
    for cap in method_re.captures_iter(class_body) {
        let annotation_block = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let modifier_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let return_type_raw = cap.get(3).map(|m| m.as_str().trim().to_string());
        let name = cap[4].to_string();
        let params_str = cap.get(5).map(|m| m.as_str()).unwrap_or("");
        let throws_str = cap.get(6).map(|m| m.as_str()).unwrap_or("");

        // Skip if it looks like a control statement
        if matches!(
            name.as_str(),
            "if" | "for" | "while" | "switch" | "catch" | "try" | "return"
        ) {
            continue;
        }

        let is_constructor = name == class_name;
        let return_type = if is_constructor {
            None
        } else {
            return_type_raw.filter(|t| !t.is_empty())
        };

        let modifiers = parse_modifiers(modifier_str);
        let annotations = parse_annotations(annotation_block);
        let params = parse_params(params_str);
        let throws: Vec<String> = throws_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Try to extract method body
        let method_match = cap.get(0).unwrap();
        let body = if class_body[method_match.start()..].contains('{') {
            let body_start = class_body[method_match.start()..]
                .find('{')
                .map(|p| method_match.start() + p);
            body_start.map(|s| extract_block_body(class_body, s))
        } else {
            None
        };

        methods.push(JavaMethod {
            name,
            return_type,
            params,
            modifiers,
            annotations,
            throws,
            body,
            generic_params: Vec::new(),
            is_constructor,
        });
    }

    methods
}

fn extract_interface_methods(body: &str) -> Vec<JavaMethod> {
    let method_re = Regex::new(
        r"(?m)((?:\s*@\w+(?:\([^)]*\))?\s*\n)*)\s*((?:(?:public|default|static)\s+)*)(?:(\w+(?:\[\])?(?:<[^>]+>)?)\s+)?(\w+)\s*\(([^)]*)\)(?:\s*throws\s+([\w,\s]+))?\s*;",
    ).unwrap();

    let mut methods = Vec::new();
    for cap in method_re.captures_iter(body) {
        let annotation_block = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let modifier_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let return_type = cap.get(3).map(|m| m.as_str().trim().to_string());
        let name = cap[4].to_string();
        let params_str = cap.get(5).map(|m| m.as_str()).unwrap_or("");
        let throws_str = cap.get(6).map(|m| m.as_str()).unwrap_or("");

        let modifiers = parse_modifiers(modifier_str);
        let annotations = parse_annotations(annotation_block);
        let params = parse_params(params_str);
        let throws: Vec<String> = throws_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        methods.push(JavaMethod {
            name,
            return_type,
            params,
            modifiers,
            annotations,
            throws,
            body: None,
            generic_params: Vec::new(),
            is_constructor: false,
        });
    }

    methods
}

fn parse_params(params_str: &str) -> Vec<JavaParam> {
    let trimmed = params_str.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    trimmed
        .split(',')
        .filter_map(|p| {
            let p = p.trim();
            if p.is_empty() {
                return None;
            }
            let annotations = parse_annotations(p);
            // Remove annotations and 'final' modifier from the param string
            let clean = Regex::new(r"@\w+(?:\([^)]*\))?\s*")
                .unwrap()
                .replace_all(p, "")
                .to_string();
            // Strip leading 'final' keyword (parameter modifier, not part of type)
            let clean = clean.trim().trim_start_matches("final").trim().to_string();
            let parts: Vec<&str> = clean.split_whitespace().collect();
            if parts.len() >= 2 {
                Some(JavaParam {
                    type_name: parts[..parts.len() - 1].join(" "),
                    name: parts[parts.len() - 1].to_string(),
                    annotations,
                })
            } else {
                None
            }
        })
        .collect()
}

fn parse_modifiers(modifier_str: &str) -> Vec<Modifier> {
    let mut modifiers = Vec::new();
    for word in modifier_str.split_whitespace() {
        match word {
            "public" => modifiers.push(Modifier::Public),
            "private" => modifiers.push(Modifier::Private),
            "protected" => modifiers.push(Modifier::Protected),
            "static" => modifiers.push(Modifier::Static),
            "final" => modifiers.push(Modifier::Final),
            "abstract" => modifiers.push(Modifier::Abstract),
            "synchronized" => modifiers.push(Modifier::Synchronized),
            "volatile" => modifiers.push(Modifier::Volatile),
            "transient" => modifiers.push(Modifier::Transient),
            "native" => modifiers.push(Modifier::Native),
            "default" => modifiers.push(Modifier::Default),
            _ => {}
        }
    }
    modifiers
}

fn detect_framework(files: &[JavaFile], dependencies: &[Dependency]) -> Framework {
    // Check dependencies first
    for dep in dependencies {
        if dep.group_id.contains("springframework") || dep.artifact_id.contains("spring") {
            return Framework::SpringBoot;
        }
        if dep.group_id.contains("javax") && dep.artifact_id.contains("javaee") {
            return Framework::JavaEE;
        }
        if dep.group_id.contains("android") {
            return Framework::Android;
        }
    }

    // Check imports/annotations
    for file in files {
        for import in &file.imports {
            if import.starts_with("org.springframework") {
                return Framework::SpringBoot;
            }
            if import.starts_with("javax.ejb") || import.starts_with("javax.servlet") {
                return Framework::JavaEE;
            }
            if import.starts_with("android.") {
                return Framework::Android;
            }
        }
    }

    Framework::Plain
}

fn detect_version_from_source(files: &[JavaFile]) -> JavaVersion {
    let record_re = Regex::new(r"(?m)^\s*(?:public\s+)?record\s+\w+").unwrap();

    for file in files {
        let source = &file.source;

        // Java 21: virtual threads, pattern matching for switch
        if source.contains("Thread.ofVirtual()") || source.contains("case String") {
            return JavaVersion::Java21;
        }

        // Java 17: sealed classes, records
        if source.contains("sealed class")
            || source.contains("sealed interface")
            || record_re.is_match(source)
        {
            return JavaVersion::Java17;
        }

        // Java 11: var in lambda, HttpClient
        if source.contains("java.net.http.HttpClient") {
            return JavaVersion::Java11;
        }

        // Java 8: streams, lambdas, Optional
        if source.contains(".stream()")
            || source.contains("->")
            || source.contains("Optional<")
            || source.contains("@FunctionalInterface")
        {
            return JavaVersion::Java8;
        }
    }

    JavaVersion::Java8 // Default to Java 8 as it's most common
}

pub fn generate_report(project: &JavaProject) -> AnalysisReport {
    let mut total_classes = 0;
    let mut total_interfaces = 0;
    let mut total_enums = 0;
    let mut total_methods = 0;
    let mut annotations_used = std::collections::HashSet::new();

    for file in &project.files {
        total_classes += file.classes.len();
        total_interfaces += file.interfaces.len();
        total_enums += file.enums.len();

        for class in &file.classes {
            total_methods += class.methods.len();
            for ann in &class.annotations {
                annotations_used.insert(ann.name.clone());
            }
            for method in &class.methods {
                for ann in &method.annotations {
                    annotations_used.insert(ann.name.clone());
                }
            }
        }
        for iface in &file.interfaces {
            total_methods += iface.methods.len();
        }
    }

    AnalysisReport {
        project_path: project.path.clone(),
        java_version: project.version.clone(),
        framework: project.framework.clone(),
        build_system: project.build_system.clone(),
        total_files: project.files.len(),
        total_classes,
        total_interfaces,
        total_enums,
        total_methods,
        dependencies: project.dependencies.clone(),
        annotations_used: annotations_used.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_package() {
        assert_eq!(
            extract_package("package com.example.app;"),
            Some("com.example.app".to_string())
        );
        assert_eq!(extract_package("// no package"), None);
    }

    #[test]
    fn test_extract_imports() {
        let source = r#"
import java.util.List;
import java.util.Map;
import static java.lang.Math.PI;
"#;
        let imports = extract_imports(source);
        assert_eq!(imports.len(), 3);
        assert_eq!(imports[0], "java.util.List");
        assert_eq!(imports[2], "java.lang.Math.PI");
    }

    #[test]
    fn test_extract_simple_class() {
        let source = r#"
public class Dog extends Animal implements Speakable {
    private String name;

    public Dog(String name) {
        this.name = name;
    }

    public String speak() {
        return "Woof!";
    }
}
"#;
        let classes = extract_classes(source);
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "Dog");
        assert_eq!(classes[0].extends, Some("Animal".to_string()));
        assert_eq!(classes[0].implements, vec!["Speakable"]);
    }

    #[test]
    fn test_extract_annotated_class() {
        let source = r#"
@RestController
@RequestMapping("/api")
public class UserController {
    @GetMapping("/users")
    public List<User> getUsers() {
        return userService.findAll();
    }
}
"#;
        let classes = extract_classes(source);
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].annotations.len(), 2);
        assert_eq!(classes[0].annotations[0].name, "RestController");
    }

    #[test]
    fn test_extract_generic_class() {
        let source = r#"
public class Box<T> {
    private T value;

    public Box(T value) {
        this.value = value;
    }

    public T getValue() {
        return value;
    }
}
"#;
        let classes = extract_classes(source);
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].generic_params, vec!["T"]);
    }

    #[test]
    fn test_extract_interface() {
        let source = r#"
public interface Repository<T> extends CrudRepository<T> {
    List<T> findAll();
    T findById(Long id);
}
"#;
        let interfaces = extract_interfaces(source);
        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].name, "Repository");
    }

    #[test]
    fn test_extract_enum() {
        let source = r#"
public enum Color {
    RED,
    GREEN,
    BLUE
}
"#;
        let enums = extract_enums(source);
        assert_eq!(enums.len(), 1);
        assert_eq!(enums[0].name, "Color");
        assert_eq!(enums[0].variants.len(), 3);
    }

    #[test]
    fn test_parse_params() {
        let params = parse_params("String name, int age, @NotNull List<String> items");
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].type_name, "String");
        assert_eq!(params[0].name, "name");
        assert_eq!(params[1].type_name, "int");
        assert_eq!(params[2].name, "items");
    }

    #[test]
    fn test_parse_modifiers() {
        let modifiers = parse_modifiers("public static final ");
        assert_eq!(modifiers.len(), 3);
        assert!(modifiers.contains(&Modifier::Public));
        assert!(modifiers.contains(&Modifier::Static));
        assert!(modifiers.contains(&Modifier::Final));
    }
}
