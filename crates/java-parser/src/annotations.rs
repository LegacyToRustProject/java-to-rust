use crate::types::Annotation;
use regex::Regex;

pub fn parse_annotations(source: &str) -> Vec<Annotation> {
    let re = Regex::new(r#"@(\w+)(?:\(([^)]*)\))?"#).unwrap();
    let mut annotations = Vec::new();

    for cap in re.captures_iter(source) {
        let name = cap[1].to_string();
        let attributes = if let Some(attr_str) = cap.get(2) {
            parse_annotation_attributes(attr_str.as_str())
        } else {
            Vec::new()
        };
        annotations.push(Annotation { name, attributes });
    }

    annotations
}

fn parse_annotation_attributes(attr_str: &str) -> Vec<(String, String)> {
    let mut attributes = Vec::new();
    let trimmed = attr_str.trim();

    if trimmed.is_empty() {
        return attributes;
    }

    // Single value annotation like @GetMapping("/path")
    if !trimmed.contains('=') {
        attributes.push(("value".to_string(), trimmed.trim_matches('"').to_string()));
        return attributes;
    }

    // Key-value pairs like @RequestMapping(value = "/api", method = RequestMethod.GET)
    let kv_re = Regex::new(r#"(\w+)\s*=\s*(?:"([^"]*)"|([\w.]+))"#).unwrap();
    for cap in kv_re.captures_iter(trimmed) {
        let key = cap[1].to_string();
        let value = cap
            .get(2)
            .or(cap.get(3))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        attributes.push((key, value));
    }

    attributes
}

pub fn collect_annotation_names(annotations: &[Annotation]) -> Vec<String> {
    annotations.iter().map(|a| a.name.clone()).collect()
}

/// Detect Spring-specific annotations
pub fn is_spring_annotation(name: &str) -> bool {
    matches!(
        name,
        "RestController"
            | "Controller"
            | "Service"
            | "Repository"
            | "Component"
            | "Configuration"
            | "Bean"
            | "Autowired"
            | "GetMapping"
            | "PostMapping"
            | "PutMapping"
            | "DeleteMapping"
            | "RequestMapping"
            | "PathVariable"
            | "RequestBody"
            | "RequestParam"
            | "ResponseBody"
            | "SpringBootApplication"
            | "EnableAutoConfiguration"
            | "Transactional"
            | "Scheduled"
            | "Value"
    )
}

/// Detect JPA/Hibernate annotations
pub fn is_jpa_annotation(name: &str) -> bool {
    matches!(
        name,
        "Entity"
            | "Table"
            | "Column"
            | "Id"
            | "GeneratedValue"
            | "ManyToOne"
            | "OneToMany"
            | "ManyToMany"
            | "OneToOne"
            | "JoinColumn"
            | "Embeddable"
            | "Embedded"
            | "NamedQuery"
            | "Enumerated"
            | "Lob"
            | "Temporal"
    )
}

/// Detect Java EE annotations
pub fn is_java_ee_annotation(name: &str) -> bool {
    matches!(
        name,
        "Stateless"
            | "Stateful"
            | "Singleton"
            | "MessageDriven"
            | "EJB"
            | "Inject"
            | "Named"
            | "ManagedBean"
            | "SessionScoped"
            | "RequestScoped"
            | "ApplicationScoped"
            | "WebServlet"
            | "WebFilter"
            | "WebListener"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_annotation() {
        let annotations = parse_annotations("@Override");
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].name, "Override");
        assert!(annotations[0].attributes.is_empty());
    }

    #[test]
    fn test_parse_annotation_with_value() {
        let annotations = parse_annotations(r#"@GetMapping("/users")"#);
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].name, "GetMapping");
        assert_eq!(annotations[0].attributes[0].0, "value");
        assert_eq!(annotations[0].attributes[0].1, "/users");
    }

    #[test]
    fn test_parse_annotation_with_kv() {
        let annotations =
            parse_annotations(r#"@RequestMapping(value = "/api", method = RequestMethod.GET)"#);
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].attributes.len(), 2);
        assert_eq!(
            annotations[0].attributes[0],
            ("value".to_string(), "/api".to_string())
        );
        assert_eq!(
            annotations[0].attributes[1],
            ("method".to_string(), "RequestMethod.GET".to_string())
        );
    }

    #[test]
    fn test_spring_annotation_detection() {
        assert!(is_spring_annotation("RestController"));
        assert!(is_spring_annotation("Autowired"));
        assert!(!is_spring_annotation("Override"));
    }

    #[test]
    fn test_jpa_annotation_detection() {
        assert!(is_jpa_annotation("Entity"));
        assert!(is_jpa_annotation("Column"));
        assert!(!is_jpa_annotation("Override"));
    }
}
