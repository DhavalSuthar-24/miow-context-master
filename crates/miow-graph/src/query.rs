// Query utilities for the knowledge graph
// This module can be expanded with more complex query logic

use anyhow::Result;

/// Query builder for complex symbol searches
pub struct QueryBuilder {
    conditions: Vec<String>,
    params: Vec<String>,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            params: Vec::new(),
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.conditions.push("s.name LIKE ?".to_string());
        self.params.push(format!("%{}%", name));
        self
    }

    pub fn with_kind(mut self, kind: &str) -> Self {
        self.conditions.push("s.kind = ?".to_string());
        self.params.push(kind.to_string());
        self
    }

    pub fn build(&self) -> (String, Vec<String>) {
        let where_clause = if self.conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", self.conditions.join(" AND "))
        };

        let query = format!(
            "SELECT s.id, s.name, s.kind, s.content, f.path, s.start_line, s.end_line \
             FROM symbols s \
             JOIN files f ON s.file_id = f.id \
             {}",
            where_clause
        );

        (query, self.params.clone())
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}
