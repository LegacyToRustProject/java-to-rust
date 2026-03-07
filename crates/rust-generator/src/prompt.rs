use java_parser::types::*;

pub fn system_prompt(framework: &Framework, java_version: &JavaVersion) -> String {
    let framework_instructions = match framework {
        Framework::SpringBoot => SPRING_BOOT_INSTRUCTIONS,
        Framework::JavaEE => JAVA_EE_INSTRUCTIONS,
        Framework::Android => ANDROID_INSTRUCTIONS,
        Framework::Plain => "",
    };

    format!(
        r#"You are an expert Java-to-Rust code converter. Your task is to convert Java source code into idiomatic, safe Rust code that preserves the same behavior.

## Core Conversion Rules

1. **Classes → Structs + impl blocks**: Convert Java classes to Rust structs with impl blocks.
2. **Inheritance → Traits + Composition**: Convert abstract classes/interfaces to traits. Use composition instead of class inheritance.
3. **null → Option<T>**: All nullable references must become Option<T>. Never use raw pointers.
4. **Exceptions → Result<T, E>**: Convert try/catch/throws to Result types with ? operator.
5. **Generics**: Convert type-erased Java generics to monomorphized Rust generics.
6. **synchronized → Mutex/RwLock**: Convert synchronized blocks to std::sync primitives.
7. **Collections**: ArrayList → Vec, HashMap → HashMap, HashSet → HashSet, etc.
8. **Garbage Collection → Ownership**: Use Rust ownership/borrowing. Prefer &str over String for parameters.
9. **static methods → associated functions or free functions**.
10. **final fields → immutable by default** (Rust default).
11. **Getters/Setters → pub fields or accessor methods** as appropriate.

## Java Version: {java_version}

## Type Mapping Reference

| Java | Rust |
|------|------|
| int | i32 |
| long | i64 |
| float | f32 |
| double | f64 |
| boolean | bool |
| char | char |
| byte | u8 |
| short | i16 |
| String | String |
| Object | Box<dyn Any> (avoid if possible) |
| List<T> | Vec<T> |
| Map<K,V> | HashMap<K,V> / BTreeMap<K,V> |
| Set<T> | HashSet<T> / BTreeSet<T> |
| Optional<T> | Option<T> |
| int[] | Vec<i32> or &[i32] |
| Iterable<T> | impl Iterator<Item=T> |
| Runnable | impl Fn() |
| Callable<T> | impl Fn() -> T |
| Future<T> | impl Future<Output=T> |

## Output Format

Return ONLY the Rust source code wrapped in a ```rust code block. Add TODO comments for anything that cannot be directly converted. Do not include explanations outside the code block.

{framework_instructions}"#,
        java_version = java_version,
        framework_instructions = framework_instructions,
    )
}

const SPRING_BOOT_INSTRUCTIONS: &str = r#"
## Spring Boot Conversion Rules

| Spring Annotation | Rust Equivalent |
|---|---|
| @RestController | Axum Router + handler functions |
| @GetMapping("/path") | .route("/path", get(handler)) |
| @PostMapping("/path") | .route("/path", post(handler)) |
| @PutMapping("/path") | .route("/path", put(handler)) |
| @DeleteMapping("/path") | .route("/path", delete(handler)) |
| @PathVariable | axum::extract::Path |
| @RequestBody | axum::extract::Json |
| @RequestParam | axum::extract::Query |
| @Autowired | State<T> / function parameter |
| @Service / @Component | plain struct |
| @Transactional | sea_orm transaction block |
| @Scheduled | tokio::spawn + tokio::time::interval |
| @Entity | sea_orm Entity derive macros |
| @Table | #[sea_orm(table_name = "...")] |
| @Column | #[sea_orm(column_name = "...")] |
| @Id + @GeneratedValue | #[sea_orm(primary_key, auto_increment)] |

Convert Spring Boot applications to Axum web framework with:
- axum for HTTP routing
- sea-orm for database access
- tokio for async runtime
- tower for middleware
"#;

const JAVA_EE_INSTRUCTIONS: &str = r#"
## Java EE Conversion Rules

| Java EE | Rust Equivalent |
|---|---|
| @Stateless / @Stateful EJB | plain struct with methods |
| @Singleton | lazy_static or OnceCell |
| @Inject / @EJB | function parameter / State |
| @PersistenceContext | sea_orm DatabaseConnection |
| @WebServlet | axum handler |
| @WebFilter | tower middleware / axum layer |
| @MessageDriven | tokio channel consumer |
| JNDI lookup | direct construction or config |
| JMS | tokio channels or message queue client |

Convert Java EE applications to modern Rust equivalents with simpler patterns.
"#;

const ANDROID_INSTRUCTIONS: &str = r#"
## Android Backend Conversion Rules

Convert Android backend/server code to standard Rust web services.
Activity/Fragment patterns should be converted to appropriate server-side equivalents.
"#;

pub fn conversion_prompt(java_file: &JavaFile) -> String {
    let mut prompt = format!(
        "Convert the following Java source code to Rust:\n\n```java\n{}\n```\n",
        java_file.source
    );

    if !java_file.imports.is_empty() {
        prompt.push_str("\n## Context\n\nThe file uses these imports:\n");
        for import in &java_file.imports {
            prompt.push_str(&format!("- {}\n", import));
        }
    }

    if let Some(ref pkg) = java_file.package {
        prompt.push_str(&format!("\nPackage: {}\n", pkg));
    }

    prompt
}

pub fn fix_prompt(rust_code: &str, error: &str) -> String {
    format!(
        r#"The following Rust code has compilation errors. Fix the errors and return the corrected code.

## Current Code

```rust
{}
```

## Errors

```
{}
```

Return ONLY the fixed Rust code in a ```rust code block. Do not explain the changes."#,
        rust_code, error
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_plain() {
        let prompt = system_prompt(&Framework::Plain, &JavaVersion::Java8);
        assert!(prompt.contains("Java-to-Rust"));
        assert!(prompt.contains("Option<T>"));
        assert!(prompt.contains("8"));
    }

    #[test]
    fn test_system_prompt_spring() {
        let prompt = system_prompt(&Framework::SpringBoot, &JavaVersion::Java11);
        assert!(prompt.contains("Spring Boot"));
        assert!(prompt.contains("Axum"));
    }

    #[test]
    fn test_conversion_prompt() {
        let file = JavaFile {
            path: "Test.java".into(),
            package: Some("com.example".to_string()),
            imports: vec!["java.util.List".to_string()],
            classes: Vec::new(),
            interfaces: Vec::new(),
            enums: Vec::new(),
            source: "public class Test {}".to_string(),
        };
        let prompt = conversion_prompt(&file);
        assert!(prompt.contains("public class Test"));
        assert!(prompt.contains("java.util.List"));
    }

    #[test]
    fn test_fix_prompt() {
        let prompt = fix_prompt("fn main() { let x: i32 = \"hello\"; }", "mismatched types");
        assert!(prompt.contains("mismatched types"));
        assert!(prompt.contains("Fix the errors"));
    }
}
