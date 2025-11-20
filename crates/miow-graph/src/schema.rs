use serde::{Deserialize, Serialize};

/// Data structures for inserting into the knowledge graph
/// These mirror the parser types but are simplified for storage

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFileData {
    pub symbols: Vec<SymbolData>,
    pub imports: Vec<ImportData>,
    pub design_tokens: Vec<DesignTokenData>,
    pub type_definitions: Vec<TypeDefinitionData>,
    pub constants: Vec<ConstantData>,
    pub schemas: Vec<SchemaData>,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolData {
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub content: String,
    pub metadata: String, // JSON serialized metadata
    pub style_tags: Option<String>, // Comma-separated style tags
    pub children: Vec<SymbolData>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportData {
    pub source: String,
    pub names: Vec<String>,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignTokenData {
    pub token_type: String,
    pub name: String,
    pub value: String,
    pub context: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinitionData {
    pub name: String,
    pub kind: String,
    pub definition: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantData {
    pub name: String,
    pub value: String,
    pub category: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaData {
    pub name: String,
    pub schema_type: String,
    pub definition: String,
    pub start_line: usize,
    pub end_line: usize,
}
