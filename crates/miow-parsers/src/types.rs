use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a parsed file with extracted symbols and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFile {
    pub symbols: Vec<Symbol>,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
    pub design_tokens: Vec<DesignToken>,
    pub type_definitions: Vec<TypeDefinition>,
    pub constants: Vec<Constant>,
    pub schemas: Vec<ValidationSchema>,
    pub language: String,
}

/// A generic symbol (class, function, interface, variable, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolType,
    pub range: Range,
    pub content: String,
    pub metadata: SymbolMetadata,
    pub children: Vec<Symbol>,
    pub references: Vec<String>, // Names of other symbols referenced by this one
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SymbolType {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
    Component, // React/UI Component
    Hook,      // React Hook
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SymbolMetadata {
    pub documentation: Option<String>,
    pub jsdoc: Option<String>,
    pub access_modifier: Option<String>,
    pub is_static: bool,
    pub is_readonly: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub is_async: bool,
    // New field for tags
    pub tags: Vec<String>,
    pub priority: Option<f32>,
    pub decorators: Vec<String>,
    pub extends: Vec<String>,
    pub implements: Vec<String>,
    pub generic_params: Vec<String>,
    // Component-specific metadata
    pub props: Vec<PropDefinition>,
    pub hooks_used: Vec<String>,
    pub state_variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
    pub is_optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum AccessModifier {
    #[default]
    Public,
    Private,
    Protected,
    Internal,
}

/// An import statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub source: String,
    pub names: Vec<ImportName>,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportName {
    pub name: String,
    pub alias: Option<String>,
    pub is_default: bool,
    pub is_namespace: bool,
    pub is_type: bool,
}

/// An export statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    pub name: String,
    pub alias: Option<String>,
    pub is_default: bool,
    pub is_type: bool,
    pub range: Range,
}

/// Design tokens (colors, spacing, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignToken {
    pub token_type: DesignTokenType,
    pub name: String,
    pub value: String,
    pub context: String, // Where it was found (e.g., className, style prop)
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DesignTokenType {
    Color,
    Spacing,
    Typography,
    BorderRadius,
    Shadow,
    TailwindClass,
    CSSVariable,
    FontFamily,
    FontSize,
    FontWeight,
    ZIndex,
    Breakpoint,
    Animation,
    Transition,
    Opacity,
}

/// JSDoc documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSDoc {
    pub description: String,
    pub params: Vec<JSDocParam>,
    pub returns: Option<String>,
    pub examples: Vec<String>,
    pub deprecated: Option<String>,
    pub see: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSDocParam {
    pub name: String,
    pub type_info: Option<String>,
    pub description: Option<String>,
}

/// Component prop definition with full metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropDefinition {
    pub name: String,
    pub type_annotation: Option<String>,
    pub is_required: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
    pub validation: Option<String>,
}

/// Type definition (interface, type alias, enum)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub kind: TypeKind,
    pub definition: String,
    pub properties: Vec<TypeProperty>,
    pub generic_params: Vec<String>,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeKind {
    Interface,
    TypeAlias,
    Enum,
    Union,
    Intersection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeProperty {
    pub name: String,
    pub type_annotation: String,
    pub is_optional: bool,
    pub description: Option<String>,
}

/// Constant or configuration value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constant {
    pub name: String,
    pub value: String,
    pub type_annotation: Option<String>,
    pub category: ConstantCategory,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstantCategory {
    Config,
    APIEndpoint,
    ErrorMessage,
    DefaultValue,
    Other,
}

/// Validation schema (Zod, Yup, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSchema {
    pub name: String,
    pub schema_type: SchemaType,
    pub definition: String,
    pub fields: Vec<SchemaField>,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaType {
    Zod,
    Yup,
    JoiCustom,
    Other(String),
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SchemaField {
    pub name: String,
    pub validation_rules: Vec<String>,
    pub is_required: bool,
    pub default_value: Option<String>,
    // New fields for enhanced Zod support
    pub type_annotation: Option<String>,
    pub is_optional: bool,
    pub validators: Vec<String>,
    pub description: Option<String>,
}
