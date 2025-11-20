use crate::ContextData;
use tracing::{info, debug};

/// Smart context pruner to manage token budget and relevance
pub struct SmartPruner {
    token_budget: usize,
}

impl SmartPruner {
    pub fn new(token_budget: usize) -> Self {
        Self { token_budget }
    }

    /// Prune context to fit within token budget
    pub fn prune(&self, context: &mut ContextData) {
        let current_usage = self.calculate_usage(context);
        
        if current_usage <= self.token_budget {
            debug!("Context usage {} within budget {}", current_usage, self.token_budget);
            return;
        }

        info!("✂️ Pruning context: usage {} > budget {}", current_usage, self.token_budget);

        // Strategy 1: Remove test files and mocks
        self.remove_test_files(context);
        
        if self.calculate_usage(context) <= self.token_budget {
            return;
        }

        // Strategy 2: Limit number of items per category
        self.limit_items(context);
        
        if self.calculate_usage(context) <= self.token_budget {
            return;
        }
        
        // Strategy 3: Truncate large content (keep signatures if possible)
        // For now, just remove lowest priority items
        self.aggressive_prune(context);
    }
    
    fn calculate_usage(&self, context: &ContextData) -> usize {
        let mut chars = 0;
        
        for s in &context.relevant_symbols { chars += s.content.len(); }
        for s in &context.similar_symbols { chars += s.content.len(); }
        for t in &context.types { chars += t.definition.len(); }
        for c in &context.constants { chars += c.value.len(); }
        for d in &context.design_tokens { chars += d.value.len(); }
        for s in &context.schemas { chars += s.definition.len(); }
        
        // Approx 4 chars per token
        chars / 4
    }
    
    fn remove_test_files(&self, context: &mut ContextData) {
        let is_test = |path: &str| {
            path.contains(".test.") || 
            path.contains(".spec.") || 
            path.contains("__tests__") ||
            path.contains("mock")
        };
        
        context.relevant_symbols.retain(|s| !is_test(&s.file_path));
        context.similar_symbols.retain(|s| !is_test(&s.file_path));

        // Constants and tokens usually don't have file paths in the same way or are less likely to be test-only
        // But if they do, filter them too
    }
    
    fn limit_items(&self, context: &mut ContextData) {
        // Keep top N items
        const MAX_ITEMS: usize = 10;
        
        if context.relevant_symbols.len() > MAX_ITEMS {
            context.relevant_symbols.truncate(MAX_ITEMS);
        }
        if context.similar_symbols.len() > MAX_ITEMS {
            context.similar_symbols.truncate(MAX_ITEMS);
        }
        if context.types.len() > MAX_ITEMS {
            context.types.truncate(MAX_ITEMS);
        }
        // ... others
    }
    
    fn aggressive_prune(&self, context: &mut ContextData) {
        // Keep only relevant symbols and types, remove others if still over budget
        context.similar_symbols.clear();
        context.constants.clear();
        context.design_tokens.clear();
        
        // If still over, truncate relevant symbols
        while self.calculate_usage(context) > self.token_budget && !context.relevant_symbols.is_empty() {
            context.relevant_symbols.pop();
        }
    }
}
