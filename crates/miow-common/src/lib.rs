use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a chunk of code with metadata for vector storage and retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub id: String,
    pub content: String,
    pub file_path: String,
    pub language: String,
    pub start_line: usize,
    pub end_line: usize,
    pub kind: String, // "function", "class", "interface", etc.
    pub metadata: serde_json::Value,
}

/// Lightweight file map for the indexer, showing project structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMap {
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub language: String,
    pub is_binary: bool,
}

impl FileMap {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add_file(&mut self, path: PathBuf, size: u64, language: String, is_binary: bool) {
        self.files.push(FileEntry {
            path: path.to_string_lossy().to_string(),
            size,
            language,
            is_binary,
        });
    }

    pub fn get_directories(&self) -> Vec<String> {
        let mut dirs = std::collections::HashSet::new();
        for file in &self.files {
            if let Some(parent) = std::path::Path::new(&file.path).parent() {
                dirs.insert(parent.to_string_lossy().to_string());
            }
        }
        dirs.into_iter().collect()
    }
}

/// Common error types
#[derive(thiserror::Error, Debug)]
pub enum MiowError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("LLM API error: {0}")]
    Llm(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Indexing error: {0}")]
    Indexing(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, MiowError>;