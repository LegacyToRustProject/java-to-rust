use crate::types::{BuildSystem, Dependency, JavaVersion};
use anyhow::Result;
use regex::Regex;
use std::path::Path;

pub fn detect_build_system(project_path: &Path) -> BuildSystem {
    if project_path.join("pom.xml").exists() {
        BuildSystem::Maven
    } else if project_path.join("build.gradle").exists()
        || project_path.join("build.gradle.kts").exists()
    {
        BuildSystem::Gradle
    } else {
        BuildSystem::None
    }
}

pub fn parse_dependencies(project_path: &Path) -> Result<Vec<Dependency>> {
    let build_system = detect_build_system(project_path);
    match build_system {
        BuildSystem::Maven => parse_maven_dependencies(project_path),
        BuildSystem::Gradle => parse_gradle_dependencies(project_path),
        BuildSystem::None => Ok(Vec::new()),
    }
}

pub fn detect_java_version_from_build(project_path: &Path) -> JavaVersion {
    let build_system = detect_build_system(project_path);
    match build_system {
        BuildSystem::Maven => detect_version_from_maven(project_path),
        BuildSystem::Gradle => detect_version_from_gradle(project_path),
        BuildSystem::None => JavaVersion::Unknown,
    }
}

fn parse_maven_dependencies(project_path: &Path) -> Result<Vec<Dependency>> {
    let pom_path = project_path.join("pom.xml");
    let content = std::fs::read_to_string(pom_path)?;
    let mut deps = Vec::new();

    let dep_re = Regex::new(
        r"(?s)<dependency>\s*<groupId>([^<]+)</groupId>\s*<artifactId>([^<]+)</artifactId>(?:\s*<version>([^<]+)</version>)?(?:\s*<scope>([^<]+)</scope>)?\s*</dependency>",
    )?;

    for cap in dep_re.captures_iter(&content) {
        deps.push(Dependency {
            group_id: cap[1].trim().to_string(),
            artifact_id: cap[2].trim().to_string(),
            version: cap.get(3).map(|m| m.as_str().trim().to_string()),
            scope: cap.get(4).map(|m| m.as_str().trim().to_string()),
        });
    }

    Ok(deps)
}

fn parse_gradle_dependencies(project_path: &Path) -> Result<Vec<Dependency>> {
    let gradle_path = if project_path.join("build.gradle.kts").exists() {
        project_path.join("build.gradle.kts")
    } else {
        project_path.join("build.gradle")
    };
    let content = std::fs::read_to_string(gradle_path)?;
    let mut deps = Vec::new();

    // Match patterns like: implementation 'group:artifact:version'
    let dep_re = Regex::new(
        r#"(?:implementation|compile|api|testImplementation)\s*['"(]([^:]+):([^:]+):([^'")\s]+)"#,
    )?;

    for cap in dep_re.captures_iter(&content) {
        deps.push(Dependency {
            group_id: cap[1].to_string(),
            artifact_id: cap[2].to_string(),
            version: Some(cap[3].to_string()),
            scope: None,
        });
    }

    Ok(deps)
}

fn detect_version_from_maven(project_path: &Path) -> JavaVersion {
    let pom_path = project_path.join("pom.xml");
    let content = match std::fs::read_to_string(pom_path) {
        Ok(c) => c,
        Err(_) => return JavaVersion::Unknown,
    };

    // Check <maven.compiler.source> or <java.version>
    let version_re =
        Regex::new(r"<(?:maven\.compiler\.source|java\.version)>(\d+\.?\d*)</").unwrap();
    if let Some(cap) = version_re.captures(&content) {
        return parse_version_string(&cap[1]);
    }

    // Check <source> inside maven-compiler-plugin
    let source_re = Regex::new(r"<source>(\d+\.?\d*)</source>").unwrap();
    if let Some(cap) = source_re.captures(&content) {
        return parse_version_string(&cap[1]);
    }

    JavaVersion::Unknown
}

fn detect_version_from_gradle(project_path: &Path) -> JavaVersion {
    let gradle_path = if project_path.join("build.gradle.kts").exists() {
        project_path.join("build.gradle.kts")
    } else {
        project_path.join("build.gradle")
    };
    let content = match std::fs::read_to_string(gradle_path) {
        Ok(c) => c,
        Err(_) => return JavaVersion::Unknown,
    };

    // sourceCompatibility = '11' or sourceCompatibility = JavaVersion.VERSION_11
    let compat_re =
        Regex::new(r#"sourceCompatibility\s*=\s*(?:['"]?(\d+)['"]?|JavaVersion\.VERSION_(\d+))"#)
            .unwrap();
    if let Some(cap) = compat_re.captures(&content) {
        let version_str = cap.get(1).or(cap.get(2)).map(|m| m.as_str()).unwrap_or("");
        return parse_version_string(version_str);
    }

    JavaVersion::Unknown
}

fn parse_version_string(s: &str) -> JavaVersion {
    match s {
        "1.6" | "6" => JavaVersion::Java6,
        "1.7" | "7" => JavaVersion::Java7,
        "1.8" | "8" => JavaVersion::Java8,
        "11" => JavaVersion::Java11,
        "17" => JavaVersion::Java17,
        "21" => JavaVersion::Java21,
        _ => JavaVersion::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_detect_build_system_maven() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("pom.xml"), "<project></project>").unwrap();
        assert_eq!(detect_build_system(dir.path()), BuildSystem::Maven);
    }

    #[test]
    fn test_detect_build_system_gradle() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("build.gradle"), "apply plugin: 'java'").unwrap();
        assert_eq!(detect_build_system(dir.path()), BuildSystem::Gradle);
    }

    #[test]
    fn test_detect_build_system_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(detect_build_system(dir.path()), BuildSystem::None);
    }

    #[test]
    fn test_parse_maven_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("pom.xml"),
            r#"<project>
                <dependencies>
                    <dependency>
                        <groupId>org.springframework.boot</groupId>
                        <artifactId>spring-boot-starter-web</artifactId>
                        <version>3.2.0</version>
                    </dependency>
                    <dependency>
                        <groupId>junit</groupId>
                        <artifactId>junit</artifactId>
                        <version>4.13.2</version>
                        <scope>test</scope>
                    </dependency>
                </dependencies>
            </project>"#,
        )
        .unwrap();

        let deps = parse_maven_dependencies(dir.path()).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].group_id, "org.springframework.boot");
        assert_eq!(deps[0].artifact_id, "spring-boot-starter-web");
        assert_eq!(deps[0].version.as_deref(), Some("3.2.0"));
        assert_eq!(deps[1].scope.as_deref(), Some("test"));
    }

    #[test]
    fn test_parse_gradle_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("build.gradle"),
            r#"
            dependencies {
                implementation 'org.springframework.boot:spring-boot-starter-web:3.2.0'
                testImplementation 'junit:junit:4.13.2'
            }
            "#,
        )
        .unwrap();

        let deps = parse_gradle_dependencies(dir.path()).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].artifact_id, "spring-boot-starter-web");
    }

    #[test]
    fn test_detect_java_version_maven() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("pom.xml"),
            r#"<project>
                <properties>
                    <java.version>11</java.version>
                </properties>
            </project>"#,
        )
        .unwrap();
        assert_eq!(
            detect_java_version_from_build(dir.path()),
            JavaVersion::Java11
        );
    }

    #[test]
    fn test_parse_version_string() {
        assert_eq!(parse_version_string("1.8"), JavaVersion::Java8);
        assert_eq!(parse_version_string("8"), JavaVersion::Java8);
        assert_eq!(parse_version_string("11"), JavaVersion::Java11);
        assert_eq!(parse_version_string("17"), JavaVersion::Java17);
        assert_eq!(parse_version_string("21"), JavaVersion::Java21);
    }
}
