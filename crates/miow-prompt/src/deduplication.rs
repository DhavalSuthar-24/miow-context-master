use crate::ContextData;
use std::collections::HashSet;
use tracing::debug;

/// Deduplication engine to remove redundant context
pub struct DeduplicationEngine;

impl DeduplicationEngine {
    /// Deduplicate context data
    pub fn deduplicate(context: &mut ContextData) {
        let initial_count = context.relevant_symbols.len() + context.types.len();
        
        // 1. Deduplicate relevant symbols (by name and path)
        let mut seen_symbols = HashSet::new();
        context.relevant_symbols.retain(|s| {
            let key = format!("{}:{}", s.name, s.file_path);
            seen_symbols.insert(key)
        });
        
        // 2. Remove symbols from similar_symbols that are already in relevant_symbols
        let relevant_names: HashSet<String> = context.relevant_symbols.iter()
            .map(|s| s.name.clone())
            .collect();
            
        context.similar_symbols.retain(|s| !relevant_names.contains(&s.name));
        
        // 3. Deduplicate types
        let mut seen_types = HashSet::new();
        context.types.retain(|t| {
            let key = format!("{}:{}", t.name, t.definition);
            seen_types.insert(key)
        });
        
        // 4. Deduplicate constants
        let mut seen_constants = HashSet::new();
        context.constants.retain(|c| {
            let key = format!("{}:{}", c.name, c.value);
            seen_constants.insert(key)
        });
        
        // 5. Deduplicate schemas
        let mut seen_schemas = HashSet::new();
        context.schemas.retain(|s| {
            let key = format!("{}:{}", s.name, s.definition);
            seen_schemas.insert(key)
        });
        
        let final_count = context.relevant_symbols.len() + context.types.len();
        if initial_count > final_count {
            debug!("Deduplicated {} items from context", initial_count - final_count);
        }
    }
}
