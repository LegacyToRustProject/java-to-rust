use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JavaVersion {
    Java6,
    Java7,
    Java8,
    Java11,
    Java17,
    Java21,
    Unknown,
}

impl std::fmt::Display for JavaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Java6 => write!(f, "6"),
            Self::Java7 => write!(f, "7"),
            Self::Java8 => write!(f, "8"),
            Self::Java11 => write!(f, "11"),
            Self::Java17 => write!(f, "17"),
            Self::Java21 => write!(f, "21"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Framework {
    SpringBoot,
    JavaEE,
    Android,
    Plain,
}

impl std::fmt::Display for Framework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpringBoot => write!(f, "Spring Boot"),
            Self::JavaEE => write!(f, "Java EE"),
            Self::Android => write!(f, "Android"),
            Self::Plain => write!(f, "Plain Java"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BuildSystem {
    Maven,
    Gradle,
    None,
}

impl std::fmt::Display for BuildSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Maven => write!(f, "Maven"),
            Self::Gradle => write!(f, "Gradle"),
            Self::None => write!(f, "None"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaProject {
    pub path: PathBuf,
    pub version: JavaVersion,
    pub framework: Framework,
    pub build_system: BuildSystem,
    pub files: Vec<JavaFile>,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaFile {
    pub path: PathBuf,
    pub package: Option<String>,
    pub imports: Vec<String>,
    pub classes: Vec<JavaClass>,
    pub interfaces: Vec<JavaInterface>,
    pub enums: Vec<JavaEnum>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaClass {
    pub name: String,
    pub modifiers: Vec<Modifier>,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub fields: Vec<JavaField>,
    pub methods: Vec<JavaMethod>,
    pub annotations: Vec<Annotation>,
    pub is_abstract: bool,
    pub generic_params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaInterface {
    pub name: String,
    pub extends: Vec<String>,
    pub methods: Vec<JavaMethod>,
    pub annotations: Vec<Annotation>,
    pub generic_params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaEnum {
    pub name: String,
    pub variants: Vec<String>,
    pub methods: Vec<JavaMethod>,
    pub annotations: Vec<Annotation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaField {
    pub name: String,
    pub type_name: String,
    pub modifiers: Vec<Modifier>,
    pub annotations: Vec<Annotation>,
    pub initial_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaMethod {
    pub name: String,
    pub return_type: Option<String>,
    pub params: Vec<JavaParam>,
    pub modifiers: Vec<Modifier>,
    pub annotations: Vec<Annotation>,
    pub throws: Vec<String>,
    pub body: Option<String>,
    pub generic_params: Vec<String>,
    pub is_constructor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaParam {
    pub name: String,
    pub type_name: String,
    pub annotations: Vec<Annotation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub name: String,
    pub attributes: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Modifier {
    Public,
    Private,
    Protected,
    Static,
    Final,
    Abstract,
    Synchronized,
    Volatile,
    Transient,
    Native,
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub group_id: String,
    pub artifact_id: String,
    pub version: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub project_path: PathBuf,
    pub java_version: JavaVersion,
    pub framework: Framework,
    pub build_system: BuildSystem,
    pub total_files: usize,
    pub total_classes: usize,
    pub total_interfaces: usize,
    pub total_enums: usize,
    pub total_methods: usize,
    pub dependencies: Vec<Dependency>,
    pub annotations_used: Vec<String>,
}
