use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a file in the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeFile {
    pub path: PathBuf,
    pub relative_path: String,
    pub language: Language,
    pub size: u64,
    pub content: String,
}

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    TypeScript,
    JavaScript,
    TSX,
    JSX,
    Python,
    Rust,
    CSS,
    JSON,
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "ts" => Language::TypeScript,
            "tsx" => Language::TSX,
            "js" => Language::JavaScript,
            "jsx" => Language::JSX,
            "py" => Language::Python,
            "rs" => Language::Rust,
            "css" => Language::CSS,
            "json" => Language::JSON,
            _ => Language::Unknown,
        }
    }

    pub fn is_parseable(&self) -> bool {
        matches!(
            self,
            Language::TypeScript
                | Language::TSX
                | Language::JavaScript
                | Language::JSX
                | Language::Python
                | Language::Rust
        )
    }
}

/// Report generated after indexing
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexReport {
    pub total_files: usize,
    pub files_by_language: std::collections::HashMap<String, usize>,
    pub total_size: u64,
    pub duration_ms: u128,
    pub files: Vec<CodeFile>,
}

/// Configuration for indexing
#[derive(Debug, Clone)]
pub struct IndexConfig {
    pub max_file_size: u64,
    pub ignore_patterns: Vec<String>,
    pub include_extensions: Vec<String>,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            max_file_size: 1024 * 1024, // 1MB
            ignore_patterns: vec![
                "node_modules".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".git".to_string(),
                ".next".to_string(),
                "coverage".to_string(),
            ],
            include_extensions: vec![
                "ts".to_string(),
                "tsx".to_string(),
                "js".to_string(),
                "jsx".to_string(),
                "py".to_string(),
                "rs".to_string(),
                "css".to_string(),
                "json".to_string(),
            ],
        }
    }
}
