use crate::types::*;
use anyhow::Result;
use ignore::WalkBuilder;
use miow_parsers::{parse_python, parse_rust, parse_typescript, ParsedFile};
use miow_vector::{SymbolVector, VectorStore};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

// Add project signature import
use crate::project_signature::ProjectSignature;

/// Indexes a codebase by traversing files and extracting metadata
pub struct CodebaseIndexer {
    root_path: PathBuf,
    config: IndexConfig,
    vector_store: Option<Arc<VectorStore>>,
    project_signature: Option<ProjectSignature>,
}

impl CodebaseIndexer {
    pub fn new(root_path: PathBuf) -> Result<Self> {
        if !root_path.exists() {
            anyhow::bail!("Path does not exist: {:?}", root_path);
        }

        if !root_path.is_dir() {
            anyhow::bail!("Path is not a directory: {:?}", root_path);
        }

        Ok(Self {
            root_path,
            config: IndexConfig::default(),
            vector_store: None,
            project_signature: None,
        })
    }

    pub fn with_config(mut self, config: IndexConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_vector_store(mut self, store: Arc<VectorStore>) -> Self {
        self.vector_store = Some(store);
        self
    }

    // New method to detect and set project signature
    pub fn detect_project_signature(&mut self) -> Result<&ProjectSignature> {
        if self.project_signature.is_none() {
            self.project_signature = Some(ProjectSignature::detect(&self.root_path)?);
            info!("Detected project signature: {:?}", self.project_signature.as_ref().unwrap());
        }
        Ok(self.project_signature.as_ref().unwrap())
    }

    pub async fn index(&mut self) -> Result<IndexReport> {
        let start = Instant::now();
        info!("Starting codebase indexing at {:?}", self.root_path);

        // Detect project signature first for smarter parsing
        let signature = self.detect_project_signature()?.clone();

        self.do_index_with_signature(signature, start).await
    }

    async fn do_index_with_signature(&mut self, signature: ProjectSignature, start: Instant) -> Result<IndexReport> {
        let config = &self.config;
        let root_path = &self.root_path;
        let vector_store = &self.vector_store;

        let mut files = Vec::new();
        let mut files_by_language: HashMap<String, usize> = HashMap::new();
        let mut total_size = 0u64;

        // Build walker with gitignore support
        let mut builder = WalkBuilder::new(&self.root_path);
        builder.git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .hidden(true); // Include hidden files
        
        // Try to ignore .miow directory (handle permission errors gracefully)
        let miow_ignore_path = format!("{}/.miow", self.root_path.display());
        if let Some(err) = builder.add_ignore(&miow_ignore_path) {
            // On Windows, we might get access denied - that's okay, we'll skip it manually
            debug!("Could not add .miow to ignore list (will skip manually): {}", err);
        }

        let walker = builder.build();

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(err) => {
                    warn!("Error walking directory: {}", err);
                    continue;
                }
            };

            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Manually skip .miow directory (in case add_ignore failed due to permissions)
            if path.to_string_lossy().contains(".miow") {
                continue;
            }

            // Check if file should be ignored
            if Self::should_ignore_static(path, &config.ignore_patterns) {
                continue;
            }

            // Get file extension
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

            // Check if extension is in include list
            if !config
                .include_extensions
                .contains(&extension.to_string())
            {
                continue;
            }

            // Get file metadata
            let metadata = match fs::metadata(path) {
                Ok(m) => m,
                Err(err) => {
                    warn!("Error reading metadata for {:?}: {}", path, err);
                    continue;
                }
            };

            let size = metadata.len();

            // Skip files that are too large
            if size > config.max_file_size {
                debug!("Skipping large file: {:?} ({} bytes)", path, size);
                continue;
            }

            // Read file content
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(err) => {
                    warn!("Error reading file {:?}: {}", path, err);
                    continue;
                }
            };

            let language = Language::from_extension(extension);
            let relative_path = path
                .strip_prefix(&root_path)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            // Enhanced parsing with project signature context
            if let Ok(parsed) = self.parse_file_enhanced(&content, extension, &signature, &config) {
                // Index symbols with enhanced metadata
                if let Some(store) = &vector_store {
                    for symbol in parsed.symbols {
                        let mut enhanced_metadata = symbol.metadata.clone();
                        
                        // Tag with UI library if applicable
                        if let Some(ui_lib) = &signature.ui_library {
                            enhanced_metadata.tags.push(format!("ui:{}", ui_lib.to_lowercase()));
                        }

                        // Tag with validation library
                        if let Some(val_lib) = &signature.validation_library {
                            enhanced_metadata.tags.push(format!("validation:{}", val_lib.to_lowercase()));
                        }

                        // Prioritize common UI components
                        if Self::is_common_ui_component(&symbol.name) {
                            enhanced_metadata.tags.push("common-ui".to_string());
                            enhanced_metadata.priority = Some(1.0); // High priority
                        }

                        // Tag Zod schemas and form-related symbols
                        if symbol.name.to_lowercase().contains("schema") || 
                           symbol.content.contains("z.object") ||
                           symbol.name.to_lowercase().contains("form") ||
                           symbol.name.to_lowercase().contains("input") ||
                           symbol.name.to_lowercase().contains("button") {
                            enhanced_metadata.tags.push("form-validation".to_string());
                        }

                        let symbol_vector = SymbolVector {
                            id: format!("{}:{}", relative_path, symbol.name),
                            name: symbol.name,
                            kind: format!("{:?}", symbol.kind),
                            content: symbol.content,
                            file_path: relative_path.clone(),
                            metadata: serde_json::to_string(&enhanced_metadata).unwrap_or_default(),
                        };

                        if let Err(e) = store.insert_symbol(&symbol_vector).await {
                            warn!(
                                "Failed to insert symbol {} into vector store: {}",
                                symbol_vector.name, e
                            );
                        }
                    }

                    // Index validation schemas separately for better search
                    for schema in &parsed.schemas {
                        let schema_vector = SymbolVector {
                            id: format!("schema:{}", schema.name),
                            name: format!("Validation Schema: {}", schema.name),
                            kind: "validation-schema".to_string(),
                            content: schema.definition.clone(),
                            file_path: relative_path.clone(),
                            metadata: serde_json::to_string(schema).unwrap_or_default(),
                        };
                        if let Err(e) = store.insert_symbol(&schema_vector).await {
                            warn!("Failed to insert schema {}: {}", schema.name, e);
                        }
                    }
                }
            }

            files.push(CodeFile {
                path: path.to_path_buf(),
                relative_path,
                language,
                size,
                content,
            });

            total_size += size;

            // Update language counts
            let lang_name = format!("{:?}", language);
            *files_by_language.entry(lang_name).or_insert(0) += 1;
        }

        let duration = start.elapsed();
        info!(
            "Indexed {} files in {:.2}s",
            files.len(),
            duration.as_secs_f64()
        );

        Ok(IndexReport {
            total_files: files.len(),
            files_by_language,
            total_size,
            duration_ms: duration.as_millis(),
            files,
        })
    }

    fn parse_file_enhanced(&self, content: &str, extension: &str, signature: &ProjectSignature, _config: &IndexConfig) -> Result<ParsedFile> {
        let mut parsed = match extension {
            "ts" => parse_typescript(content, false),
            "tsx" => parse_typescript(content, true),
            "rs" => parse_rust(content),
            "py" => parse_python(content),
            _ => anyhow::bail!("Unsupported extension: {}", extension),
        }?;

        // Enhance parsed data with signature context
        // For example, tag symbols based on detected libraries
        for symbol in &mut parsed.symbols {
            // If Zod detected, tag schema-related symbols
            if signature.validation_library.as_ref().map_or(false, |v| v == "Zod") {
                if symbol.content.contains("z.") || symbol.name.to_lowercase().contains("schema") {
                    symbol.metadata.tags.push("zod-schema".to_string());
                }
            }

            // Tag common UI components regardless of library
            if Self::is_common_ui_component(&symbol.name) {
                symbol.metadata.tags.push("common-ui-component".to_string());
            }

            // Next.js specific tagging
            if signature.framework.contains("Next.js") {
                if symbol.content.contains("usePathname") || symbol.content.contains("Link from 'next/link'") {
                    symbol.metadata.tags.push("nextjs-routing".to_string());
                }
                if symbol.content.contains("getServerSideProps") || symbol.content.contains("'use server'") {
                    symbol.metadata.tags.push("nextjs-server".to_string());
                }
            }
        }

        Ok(parsed)
    }

    fn is_common_ui_component(name: &str) -> bool {
        let common_ui = vec!["InputBox", "Button", "Form", "Modal", "Dialog", "Input", "Select", "Checkbox", "Textarea", "Label"];
        common_ui.iter().any(|c| name.contains(c))
    }

    fn should_ignore(&self, path: &std::path::Path) -> bool {
        Self::should_ignore_static(path, &self.config.ignore_patterns)
    }

    fn should_ignore_static(path: &std::path::Path, ignore_patterns: &[String]) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in ignore_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_indexer_creation() {
        let indexer = CodebaseIndexer::new(PathBuf::from("."));
        assert!(indexer.is_ok());
    }

    #[tokio::test]
    async fn test_indexer_invalid_path() {
        let indexer = CodebaseIndexer::new(PathBuf::from("/nonexistent/path"));
        assert!(indexer.is_err());
    }

    #[tokio::test]
    async fn test_project_signature_detection() {
        let mut indexer = CodebaseIndexer::new(PathBuf::from(".")).unwrap();
        let signature = indexer.detect_project_signature().unwrap();
        assert!(!signature.language.is_empty());
    }
}
