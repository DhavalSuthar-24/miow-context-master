use crate::types::*;
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser, Query, QueryCursor};

pub struct PythonParser {
    parser: Parser,
}

impl PythonParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        let language = tree_sitter_python::language();
        parser
            .set_language(language)
            .expect("Error loading Python grammar");
        Self { parser }
    }

    pub fn parse(&self, content: &str) -> Result<ParsedFile> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_python::language())
            .context("Failed to set Python language")?;

        let tree = parser
            .parse(content, None)
            .context("Failed to parse Python content")?;

        let root_node = tree.root_node();

        let symbols = self.extract_symbols(&root_node, content)?;
        let imports = self.extract_imports(&root_node, content)?;

        Ok(ParsedFile {
            symbols,
            imports,
            exports: vec![], // Python exports are implicit (everything not starting with _)
            design_tokens: vec![],
            type_definitions: vec![], // TODO: Extract type hints
            constants: vec![],        // TODO: Extract constants
            schemas: vec![],          // TODO: Extract Pydantic models
            language: "python".to_string(),
        })
    }

    fn extract_symbols(&self, node: &Node, source: &str) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if let Some(symbol) = self.process_node(&child, source)? {
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    fn process_node(&self, node: &Node, source: &str) -> Result<Option<Symbol>> {
        let kind = node.kind();
        let text = node.utf8_text(source.as_bytes())?;

        match kind {
            "class_definition" => {
                let name = self
                    .get_child_text(node, "name", source)
                    .unwrap_or_else(|| "Anonymous".to_string());
                let range = self.get_range(node);
                let metadata = self.extract_metadata(node, source)?;

                Ok(Some(Symbol {
                    name,
                    kind: SymbolType::Class,
                    range,
                    content: text.to_string(),
                    metadata,
                    children: self.extract_class_members(node, source)?,
                    references: vec![],
                }))
            }
            "function_definition" => {
                let name = self
                    .get_child_text(node, "name", source)
                    .unwrap_or_else(|| "anonymous".to_string());
                let range = self.get_range(node);
                let metadata = self.extract_function_metadata(node, source)?;

                Ok(Some(Symbol {
                    name,
                    kind: SymbolType::Function,
                    range,
                    content: text.to_string(),
                    metadata,
                    children: vec![],
                    references: vec![],
                }))
            }
            "assignment" => {
                // Global variables
                if let Some(left) = node.child_by_field_name("left") {
                    let name = left.utf8_text(source.as_bytes())?.to_string();
                    Ok(Some(Symbol {
                        name,
                        kind: SymbolType::Variable,
                        range: self.get_range(node),
                        content: text.to_string(),
                        metadata: SymbolMetadata::default(),
                        children: vec![],
                        references: vec![],
                    }))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn extract_class_members(&self, node: &Node, source: &str) -> Result<Vec<Symbol>> {
        let mut members = Vec::new();
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "function_definition" {
                    let name = self
                        .get_child_text(&child, "name", source)
                        .unwrap_or_else(|| "method".to_string());
                    let metadata = self.extract_function_metadata(&child, source)?;

                    members.push(Symbol {
                        name,
                        kind: SymbolType::Method,
                        range: self.get_range(&child),
                        content: child.utf8_text(source.as_bytes())?.to_string(),
                        metadata,
                        children: vec![],
                        references: vec![],
                    });
                }
            }
        }
        Ok(members)
    }

    fn extract_metadata(&self, node: &Node, source: &str) -> Result<SymbolMetadata> {
        let mut metadata = SymbolMetadata::default();

        // Extract decorators
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "decorator" {
                metadata.decorators.push(child.utf8_text(source.as_bytes())?.to_string());
            }
        }

        // Check inheritance
        if let Some(superclasses) = node.child_by_field_name("superclasses") {
            let mut cursor = superclasses.walk();
            for child in superclasses.children(&mut cursor) {
                if child.kind() == "identifier" || child.kind() == "attribute" {
                    metadata
                        .extends
                        .push(child.utf8_text(source.as_bytes())?.to_string());
                }
            }
        }

        Ok(metadata)
    }

    fn extract_function_metadata(&self, node: &Node, source: &str) -> Result<SymbolMetadata> {
        let mut metadata = self.extract_metadata(node, source)?;

        // Extract parameters
        if let Some(params_node) = node.child_by_field_name("parameters") {
            metadata.parameters = self.extract_parameters(&params_node, source)?;
        }

        // Extract return type
        if let Some(return_type) = node.child_by_field_name("return_type") {
            metadata.return_type = Some(return_type.utf8_text(source.as_bytes())?.to_string());
        }

        metadata.is_async = node.utf8_text(source.as_bytes())?.starts_with("async");

        Ok(metadata)
    }

    fn extract_parameters(&self, node: &Node, source: &str) -> Result<Vec<Parameter>> {
        let mut params = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            let kind = child.kind();
            if kind == "identifier"
                || kind == "typed_parameter"
                || kind == "default_parameter"
                || kind == "typed_default_parameter"
            {
                let mut name = String::new();
                let mut type_annotation = None;
                let mut default_value = None;

                if kind == "identifier" {
                    name = child.utf8_text(source.as_bytes())?.to_string();
                } else if kind == "typed_parameter" {
                    name = self
                        .get_child_text(&child, "name", source)
                        .unwrap_or_default();
                    type_annotation = self.get_child_text(&child, "type", source);
                } else if kind == "default_parameter" {
                    name = self
                        .get_child_text(&child, "name", source)
                        .unwrap_or_default();
                    default_value = self.get_child_text(&child, "value", source);
                } else if kind == "typed_default_parameter" {
                    name = self
                        .get_child_text(&child, "name", source)
                        .unwrap_or_default();
                    type_annotation = self.get_child_text(&child, "type", source);
                    default_value = self.get_child_text(&child, "value", source);
                }

                params.push(Parameter {
                    name,
                    type_annotation,
                    default_value,
                    is_optional: false,
                });
            }
        }
        Ok(params)
    }

    fn extract_imports(&self, node: &Node, source: &str) -> Result<Vec<Import>> {
        let mut imports = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "import_statement" {
                // import x, y
                let text = child.utf8_text(source.as_bytes())?;
                imports.push(Import {
                    source: text.to_string(), // Simplified
                    names: vec![],
                    range: self.get_range(&child),
                });
            } else if child.kind() == "import_from_statement" {
                // from x import y
                let module_name = self
                    .get_child_text(&child, "module_name", source)
                    .unwrap_or_default();
                imports.push(Import {
                    source: module_name,
                    names: vec![],
                    range: self.get_range(&child),
                });
            }
        }
        Ok(imports)
    }

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
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}
