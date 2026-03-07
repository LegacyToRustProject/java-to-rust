use std::collections::HashMap;

/// Maps common Java patterns to their Rust equivalents.
/// Used to provide additional context to the LLM.
pub struct PatternMapper {
    pub type_map: HashMap<String, String>,
    pub method_map: HashMap<String, String>,
    pub import_map: HashMap<String, String>,
}

impl PatternMapper {
    pub fn new() -> Self {
        Self {
            type_map: build_type_map(),
            method_map: build_method_map(),
            import_map: build_import_map(),
        }
    }

    pub fn map_type(&self, java_type: &str) -> Option<&str> {
        self.type_map.get(java_type).map(|s| s.as_str())
    }

    pub fn map_method(&self, java_method: &str) -> Option<&str> {
        self.method_map.get(java_method).map(|s| s.as_str())
    }

    pub fn map_import(&self, java_import: &str) -> Option<&str> {
        self.import_map.get(java_import).map(|s| s.as_str())
    }

    /// Generate a context section for the LLM prompt with relevant mappings
    pub fn generate_context(&self, java_imports: &[String]) -> String {
        let mut context = String::new();
        let mut mapped = Vec::new();

        for import in java_imports {
            if let Some(rust_equiv) = self.map_import(import) {
                mapped.push(format!("  {} → {}", import, rust_equiv));
            }
        }

        if !mapped.is_empty() {
            context.push_str("\n## Known Import Mappings\n\n");
            for m in &mapped {
                context.push_str(m);
                context.push('\n');
            }
        }

        context
    }
}

impl Default for PatternMapper {
    fn default() -> Self {
        Self::new()
    }
}

fn build_type_map() -> HashMap<String, String> {
    let mut m = HashMap::new();
    // Primitives
    m.insert("int".into(), "i32".into());
    m.insert("long".into(), "i64".into());
    m.insert("float".into(), "f32".into());
    m.insert("double".into(), "f64".into());
    m.insert("boolean".into(), "bool".into());
    m.insert("char".into(), "char".into());
    m.insert("byte".into(), "u8".into());
    m.insert("short".into(), "i16".into());
    m.insert("void".into(), "()".into());

    // Boxed types
    m.insert("Integer".into(), "i32".into());
    m.insert("Long".into(), "i64".into());
    m.insert("Float".into(), "f32".into());
    m.insert("Double".into(), "f64".into());
    m.insert("Boolean".into(), "bool".into());
    m.insert("Character".into(), "char".into());
    m.insert("Byte".into(), "u8".into());
    m.insert("Short".into(), "i16".into());

    // Common types
    m.insert("String".into(), "String".into());
    m.insert("Object".into(), "Box<dyn std::any::Any>".into());

    // Collections
    m.insert("ArrayList".into(), "Vec".into());
    m.insert("LinkedList".into(), "VecDeque".into());
    m.insert("HashMap".into(), "HashMap".into());
    m.insert("TreeMap".into(), "BTreeMap".into());
    m.insert("HashSet".into(), "HashSet".into());
    m.insert("TreeSet".into(), "BTreeSet".into());
    m.insert("List".into(), "Vec".into());
    m.insert("Map".into(), "HashMap".into());
    m.insert("Set".into(), "HashSet".into());
    m.insert("Queue".into(), "VecDeque".into());
    m.insert("Stack".into(), "Vec".into());
    m.insert("Vector".into(), "Vec".into());
    m.insert("Deque".into(), "VecDeque".into());

    // Concurrency
    m.insert("AtomicInteger".into(), "AtomicI32".into());
    m.insert("AtomicLong".into(), "AtomicI64".into());
    m.insert("AtomicBoolean".into(), "AtomicBool".into());
    m.insert(
        "ConcurrentHashMap".into(),
        "DashMap or Arc<RwLock<HashMap>>".into(),
    );
    m.insert("ReentrantLock".into(), "Mutex".into());
    m.insert("ReadWriteLock".into(), "RwLock".into());

    // Optional
    m.insert("Optional".into(), "Option".into());

    // IO
    m.insert("InputStream".into(), "impl Read".into());
    m.insert("OutputStream".into(), "impl Write".into());
    m.insert("File".into(), "std::path::PathBuf".into());
    m.insert("Path".into(), "std::path::Path".into());
    m.insert("BufferedReader".into(), "BufReader".into());
    m.insert("BufferedWriter".into(), "BufWriter".into());

    m
}

fn build_method_map() -> HashMap<String, String> {
    let mut m = HashMap::new();

    // String methods
    m.insert("length()".into(), "len()".into());
    m.insert("charAt".into(), "chars().nth".into());
    m.insert("substring".into(), "&s[start..end]".into());
    m.insert("indexOf".into(), "find".into());
    m.insert("contains".into(), "contains".into());
    m.insert("startsWith".into(), "starts_with".into());
    m.insert("endsWith".into(), "ends_with".into());
    m.insert("toUpperCase".into(), "to_uppercase".into());
    m.insert("toLowerCase".into(), "to_lowercase".into());
    m.insert("trim".into(), "trim".into());
    m.insert("split".into(), "split".into());
    m.insert("replace".into(), "replace".into());
    m.insert("equals".into(), "==".into());
    m.insert("isEmpty".into(), "is_empty".into());

    // Collection methods
    m.insert("add".into(), "push".into());
    m.insert("get".into(), "get / []".into());
    m.insert("set".into(), "[] =".into());
    m.insert("size".into(), "len".into());
    m.insert("remove".into(), "remove".into());
    m.insert("clear".into(), "clear".into());
    m.insert("iterator".into(), "iter".into());

    // Stream API
    m.insert("stream()".into(), "iter()".into());
    m.insert("filter".into(), "filter".into());
    m.insert("map".into(), "map".into());
    m.insert("collect".into(), "collect".into());
    m.insert("forEach".into(), "for_each".into());
    m.insert("reduce".into(), "fold / reduce".into());
    m.insert("flatMap".into(), "flat_map".into());
    m.insert("sorted".into(), "sorted (via itertools) or sort".into());
    m.insert("distinct".into(), "dedup (after sort)".into());
    m.insert("count".into(), "count".into());
    m.insert("anyMatch".into(), "any".into());
    m.insert("allMatch".into(), "all".into());
    m.insert("noneMatch".into(), "!any".into());
    m.insert("findFirst".into(), "next".into());

    // System
    m.insert("System.out.println".into(), "println!".into());
    m.insert("System.err.println".into(), "eprintln!".into());
    m.insert("System.exit".into(), "std::process::exit".into());

    m
}

fn build_import_map() -> HashMap<String, String> {
    let mut m = HashMap::new();

    m.insert("java.util.List".into(), "// Vec (built-in)".into());
    m.insert("java.util.ArrayList".into(), "// Vec (built-in)".into());
    m.insert(
        "java.util.Map".into(),
        "use std::collections::HashMap;".into(),
    );
    m.insert(
        "java.util.HashMap".into(),
        "use std::collections::HashMap;".into(),
    );
    m.insert(
        "java.util.Set".into(),
        "use std::collections::HashSet;".into(),
    );
    m.insert(
        "java.util.HashSet".into(),
        "use std::collections::HashSet;".into(),
    );
    m.insert("java.util.Optional".into(), "// Option (built-in)".into());
    m.insert(
        "java.util.stream.Collectors".into(),
        "// .collect() (built-in)".into(),
    );
    m.insert("java.io.IOException".into(), "use std::io;".into());
    m.insert("java.io.File".into(), "use std::path::PathBuf;".into());
    m.insert("java.nio.file.Files".into(), "use std::fs;".into());
    m.insert("java.nio.file.Path".into(), "use std::path::Path;".into());
    m.insert(
        "java.util.concurrent.atomic.AtomicInteger".into(),
        "use std::sync::atomic::AtomicI32;".into(),
    );
    m.insert(
        "java.util.concurrent.locks.ReentrantLock".into(),
        "use std::sync::Mutex;".into(),
    );

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_mapping() {
        let mapper = PatternMapper::new();
        assert_eq!(mapper.map_type("int"), Some("i32"));
        assert_eq!(mapper.map_type("ArrayList"), Some("Vec"));
        assert_eq!(mapper.map_type("Optional"), Some("Option"));
        assert_eq!(mapper.map_type("NonExistent"), None);
    }

    #[test]
    fn test_method_mapping() {
        let mapper = PatternMapper::new();
        assert_eq!(mapper.map_method("size"), Some("len"));
        assert_eq!(mapper.map_method("equals"), Some("=="));
    }

    #[test]
    fn test_generate_context() {
        let mapper = PatternMapper::new();
        let imports = vec![
            "java.util.List".to_string(),
            "java.util.HashMap".to_string(),
        ];
        let context = mapper.generate_context(&imports);
        assert!(context.contains("HashMap"));
    }
}
