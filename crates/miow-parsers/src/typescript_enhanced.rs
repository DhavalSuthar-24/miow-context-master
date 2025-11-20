use crate::types::*;
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser, Query, QueryCursor};
use std::collections::HashMap;

pub struct EnhancedTypeScriptParser {
    parser: Parser,
}

impl EnhancedTypeScriptParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        Self { parser }
    }

    pub fn parse(&self, content: &str, is_tsx: bool) -> Result<ParsedFile> {
        let mut parser = Parser::new();
        
        let language = if is_tsx {
            tree_sitter_typescript::language_tsx()
        } else {
            tree_sitter_typescript::language_typescript()
        };
        
        parser
            .set_language(language)
            .context("Failed to set TypeScript language")?;

        let tree = parser
            .parse(content, None)
            .context("Failed to parse TypeScript content")?;

        let root_node = tree.root_node();

        // Extract everything
        let symbols = self.extract_symbols(&root_node, content, is_tsx)?;
        let imports = self.extract_imports(&root_node, content)?;
        let exports = self.extract_exports(&root_node, content)?;
        let design_tokens = self.extract_design_tokens(&root_node, content)?;
        let type_definitions = self.extract_type_definitions(&root_node, content)?;
        let constants = self.extract_constants(&root_node, content)?;
        let schemas = self.extract_validation_schemas(&root_node, content)?;

        Ok(ParsedFile {
            symbols,
            imports,
            exports,
            design_tokens,
            type_definitions,
            constants,
            schemas,
            language: if is_tsx { "tsx".to_string() } else { "typescript".to_string() },
        })
    }

    // ... (keeping existing symbol extraction methods)
    
    /// Extract ALL type definitions (interfaces, type aliases, enums)
    fn extract_type_definitions(&self, node: &Node, source: &str) -> Result<Vec<TypeDefinition>> {
        let mut types = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "interface_declaration" => {
                    if let Some(type_def) = self.extract_interface(&child, source)? {
                        types.push(type_def);
                    }
                }
                "type_alias_declaration" => {
                    if let Some(type_def) = self.extract_type_alias(&child, source)? {
                        types.push(type_def);
                    }
                }
                "enum_declaration" => {
                    if let Some(type_def) = self.extract_enum_type(&child, source)? {
                        types.push(type_def);
                    }
                }
                _ => {}
            }
        }

        Ok(types)
    }

    fn extract_interface(&self, node: &Node, source: &str) -> Result<Option<TypeDefinition>> {
        let name = self.get_child_text(node, "type_identifier", source).unwrap_or_default();
        let definition = node.utf8_text(source.as_bytes())?.to_string();
        
        let mut properties = Vec::new();
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "property_signature" {
                    let prop_name = self.get_child_text(&child, "property_identifier", source).unwrap_or_default();
                    let type_annotation = child.child_by_field_name("type")
                        .map(|n| n.utf8_text(source.as_bytes()).unwrap().to_string())
                        .unwrap_or_default();
                    let is_optional = child.utf8_text(source.as_bytes())?.contains('?');
                    
                    properties.push(TypeProperty {
                        name: prop_name,
                        type_annotation,
                        is_optional,
                        description: None, // TODO: Extract from JSDoc
                    });
                }
            }
        }

        Ok(Some(TypeDefinition {
            name,
            kind: TypeKind::Interface,
            definition,
            properties,
            generic_params: vec![],
            range: self.get_range(node),
        }))
    }

    fn extract_type_alias(&self, node: &Node, source: &str) -> Result<Option<TypeDefinition>> {
        let name = self.get_child_text(node, "type_identifier", source).unwrap_or_default();
        let definition = node.utf8_text(source.as_bytes())?.to_string();

        Ok(Some(TypeDefinition {
            name,
            kind: TypeKind::TypeAlias,
            definition,
            properties: vec![],
            generic_params: vec![],
            range: self.get_range(node),
        }))
    }

    fn extract_enum_type(&self, node: &Node, source: &str) -> Result<Option<TypeDefinition>> {
        let name = self.get_child_text(node, "identifier", source).unwrap_or_default();
        let definition = node.utf8_text(source.as_bytes())?.to_string();

        Ok(Some(TypeDefinition {
            name,
            kind: TypeKind::Enum,
            definition,
            properties: vec![],
            generic_params: vec![],
            range: self.get_range(node),
        }))
    }

    /// Extract ALL constants and configuration values
    fn extract_constants(&self, node: &Node, source: &str) -> Result<Vec<Constant>> {
        let mut constants = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "lexical_declaration" || child.kind() == "variable_declaration" {
                // Check if it's a const
                let text = child.utf8_text(source.as_bytes())?;
                if text.starts_with("const") || text.starts_with("export const") {
                    if let Some(constant) = self.extract_constant_from_declaration(&child, source)? {
                        constants.push(constant);
                    }
                }
            }
        }

        Ok(constants)
    }

    fn extract_constant_from_declaration(&self, node: &Node, source: &str) -> Result<Option<Constant>> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                let name_node = child.child_by_field_name("name");
                let value_node = child.child_by_field_name("value");
                
                if let (Some(name_node), Some(value_node)) = (name_node, value_node) {
                    let name = name_node.utf8_text(source.as_bytes())?.to_string();
                    let value = value_node.utf8_text(source.as_bytes())?.to_string();
                    
                    // Categorize the constant
                    let category = self.categorize_constant(&name, &value);
                    
                    // Skip if it's a function
                    if value_node.kind() == "arrow_function" || value_node.kind() == "function" {
                        continue;
                    }

                    return Ok(Some(Constant {
                        name,
                        value,
                        type_annotation: None,
                        category,
                        range: self.get_range(node),
                    }));
                }
            }
        }
        Ok(None)
    }

    fn categorize_constant(&self, name: &str, value: &str) -> ConstantCategory {
        let name_lower = name.to_lowercase();
        let value_lower = value.to_lowercase();

        if name_lower.contains("api") || name_lower.contains("endpoint") || name_lower.contains("url") {
            ConstantCategory::APIEndpoint
        } else if name_lower.contains("config") || name_lower.contains("settings") {
            ConstantCategory::Config
        } else if name_lower.contains("error") || name_lower.contains("message") {
            ConstantCategory::ErrorMessage
        } else if name_lower.contains("default") {
            ConstantCategory::DefaultValue
        } else {
            ConstantCategory::Other
        }
    }

    /// Extract validation schemas (Zod, Yup, etc.)
    fn extract_validation_schemas(&self, node: &Node, source: &str) -> Result<Vec<ValidationSchema>> {
        let mut schemas = Vec::new();
        let text = node.utf8_text(source.as_bytes())?;

        // Look for Zod schemas
        if text.contains("z.object") || text.contains("zod") {
            schemas.extend(self.extract_zod_schemas(node, source)?);
        }

        // Look for Yup schemas
        if text.contains("yup.object") || text.contains("Yup.object") {
            schemas.extend(self.extract_yup_schemas(node, source)?);
        }

        Ok(schemas)
    }

    fn extract_zod_schemas(&self, node: &Node, source: &str) -> Result<Vec<ValidationSchema>> {
        let mut schemas = Vec::new();
        // TODO: Implement Zod schema extraction using tree-sitter queries
        // This is a placeholder - full implementation would parse z.object() calls
        Ok(schemas)
    }

    fn extract_yup_schemas(&self, node: &Node, source: &str) -> Result<Vec<ValidationSchema>> {
        let mut schemas = Vec::new();
        // TODO: Implement Yup schema extraction
        Ok(schemas)
    }

    /// Enhanced design token extraction - extract ALL CSS variables, colors, etc.
    fn extract_design_tokens(&self, node: &Node, source: &str) -> Result<Vec<DesignToken>> {
        let mut tokens = Vec::new();
        
        // Extract from className attributes
        tokens.extend(self.extract_from_classnames(node, source)?);
        
        // Extract from CSS-in-JS
        tokens.extend(self.extract_from_css_in_js(node, source)?);
        
        // Extract from style objects
        tokens.extend(self.extract_from_style_objects(node, source)?);

        Ok(tokens)
    }

    fn extract_from_classnames(&self, node: &Node, source: &str) -> Result<Vec<DesignToken>> {
        let mut tokens = Vec::new();
        
        let query = Query::new(
            tree_sitter_typescript::language_tsx(),
            r#"(jsx_attribute (property_identifier) @prop_name (#eq? @prop_name "className") (string (string_fragment) @class_value))"#
        ).unwrap();

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, *node, source.as_bytes());

        for m in matches {
            for capture in m.captures {
                if capture.index == 1 {
                    let text = capture.node.utf8_text(source.as_bytes())?;
                    for class in text.split_whitespace() {
                        let token_type = self.classify_tailwind_class(class);
                        tokens.push(DesignToken {
                            token_type,
                            name: class.to_string(),
                            value: class.to_string(),
                            context: "className".to_string(),
                            range: self.get_range(&capture.node),
                        });
                    }
                }
            }
        }

        Ok(tokens)
    }

    fn classify_tailwind_class(&self, class: &str) -> DesignTokenType {
        if class.starts_with("bg-") || class.starts_with("text-") || class.starts_with("border-") {
            DesignTokenType::Color
        } else if class.starts_with("p-") || class.starts_with("m-") || class.starts_with("gap-") {
            DesignTokenType::Spacing
        } else if class.starts_with("text-") && (class.contains("sm") || class.contains("lg") || class.contains("xl")) {
            DesignTokenType::FontSize
        } else if class.starts_with("font-") {
            DesignTokenType::FontWeight
        } else if class.starts_with("rounded-") {
            DesignTokenType::BorderRadius
        } else if class.starts_with("shadow-") {
            DesignTokenType::Shadow
        } else {
            DesignTokenType::TailwindClass
        }
    }

    fn extract_from_css_in_js(&self, _node: &Node, _source: &str) -> Result<Vec<DesignToken>> {
        // TODO: Extract from styled-components, emotion, etc.
        Ok(vec![])
    }

    fn extract_from_style_objects(&self, _node: &Node, _source: &str) -> Result<Vec<DesignToken>> {
        // TODO: Extract from inline style objects
        Ok(vec![])
    }

    // Helper methods
    fn get_child_text(&self, node: &Node, field: &str, source: &str) -> Option<String> {
        node.child_by_field_name(field)
            .map(|n| n.utf8_text(source.as_bytes()).unwrap().to_string())
    }

    fn get_range(&self, node: &Node) -> Range {
        Range {
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        }
    }

    // Placeholder for existing methods - these would be kept from the original parser
    fn extract_symbols(&self, _node: &Node, _source: &str, _is_tsx: bool) -> Result<Vec<Symbol>> {
        Ok(vec![])
    }

    fn extract_imports(&self, _node: &Node, _source: &str) -> Result<Vec<Import>> {
        Ok(vec![])
    }

    fn extract_exports(&self, _node: &Node, _source: &str) -> Result<Vec<Export>> {
        Ok(vec![])
    }
}

impl Default for EnhancedTypeScriptParser {
    fn default() -> Self {
        Self::new()
    }
}
