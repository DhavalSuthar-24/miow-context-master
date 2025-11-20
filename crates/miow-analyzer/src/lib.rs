use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Context analyzer - analyzes user prompts and finds relevant context
pub struct ContextAnalyzer;

impl ContextAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze a prompt and extract keywords/entities
    pub fn analyze_prompt(&self, prompt: &str) -> AnalyzedPrompt {
        let keywords = self.extract_keywords(prompt);
        let intent = self.infer_intent(prompt);
        let entities = self.extract_entities(prompt);

        AnalyzedPrompt {
            original: prompt.to_string(),
            keywords,
            intent,
            entities,
        }
    }

    /// Extract keywords from the prompt
    fn extract_keywords(&self, prompt: &str) -> Vec<String> {
        let stop_words: HashSet<&str> = [
            "a",
            "an",
            "the",
            "is",
            "are",
            "was",
            "were",
            "be",
            "been",
            "being",
            "have",
            "has",
            "had",
            "do",
            "does",
            "did",
            "will",
            "would",
            "should",
            "could",
            "may",
            "might",
            "must",
            "can",
            "to",
            "from",
            "in",
            "on",
            "at",
            "by",
            "for",
            "with",
            "about",
            "as",
            "of",
            "and",
            "or",
            "but",
            "not",
            "this",
            "that",
            "these",
            "those",
            "i",
            "you",
            "he",
            "she",
            "it",
            "we",
            "they",
            "me",
            "him",
            "her",
            "us",
            "them",
            "my",
            "your",
            "his",
            "its",
            "our",
            "their",
            "make",
            "create",
            "add",
            "build",
            "implement",
            "write",
        ]
        .iter()
        .cloned()
        .collect();

        prompt
            .to_lowercase()
            .split_whitespace()
            .filter(|word| {
                !stop_words.contains(word)
                    && word.len() > 2
                    && word.chars().all(|c| c.is_alphanumeric())
            })
            .map(String::from)
            .collect()
    }

    /// Infer the intent from the prompt
    fn infer_intent(&self, prompt: &str) -> PromptIntent {
        let lower = prompt.to_lowercase();

        if lower.contains("create")
            || lower.contains("make")
            || lower.contains("add")
            || lower.contains("new")
        {
            if lower.contains("component") {
                PromptIntent::CreateComponent
            } else if lower.contains("function") || lower.contains("helper") {
                PromptIntent::CreateFunction
            } else if lower.contains("page") || lower.contains("screen") {
                PromptIntent::CreatePage
            } else {
                PromptIntent::Create
            }
        } else if lower.contains("modify")
            || lower.contains("update")
            || lower.contains("change")
            || lower.contains("edit")
        {
            PromptIntent::Modify
        } else if lower.contains("fix") || lower.contains("debug") || lower.contains("solve") {
            PromptIntent::Fix
        } else if lower.contains("refactor")
            || lower.contains("improve")
            || lower.contains("optimize")
        {
            PromptIntent::Refactor
        } else {
            PromptIntent::Unknown
        }
    }

    /// Extract potential entity names (capitalized words, camelCase, etc.)
    fn extract_entities(&self, prompt: &str) -> Vec<String> {
        let mut entities = Vec::new();

        // Find capitalized words
        for word in prompt.split_whitespace() {
            let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric());
            if !cleaned.is_empty() && cleaned.chars().next().unwrap().is_uppercase() {
                entities.push(cleaned.to_string());
            }
        }

        // Find camelCase or PascalCase words
        let words: Vec<&str> = prompt.split_whitespace().collect();
        for word in words {
            if word.chars().any(|c| c.is_uppercase()) && word.chars().any(|c| c.is_lowercase()) {
                entities.push(word.to_string());
            }
        }

        entities.sort();
        entities.dedup();
        entities
    }
}

impl Default for ContextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzedPrompt {
    pub original: String,
    pub keywords: Vec<String>,
    pub intent: PromptIntent,
    pub entities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PromptIntent {
    Create,
    CreateComponent,
    CreateFunction,
    CreatePage,
    Modify,
    Fix,
    Refactor,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords() {
        let analyzer = ContextAnalyzer::new();
        let prompt = "Create a login page with email and password fields";
        let analyzed = analyzer.analyze_prompt(prompt);

        assert!(analyzed.keywords.contains(&"login".to_string()));
        assert!(analyzed.keywords.contains(&"page".to_string()));
        assert!(analyzed.keywords.contains(&"email".to_string()));
        assert!(analyzed.keywords.contains(&"password".to_string()));
    }

    #[test]
    fn test_infer_intent() {
        let analyzer = ContextAnalyzer::new();

        let prompt1 = "Create a new Button component";
        assert_eq!(
            analyzer.analyze_prompt(prompt1).intent,
            PromptIntent::CreateComponent
        );

        let prompt2 = "Fix the authentication bug";
        assert_eq!(analyzer.analyze_prompt(prompt2).intent, PromptIntent::Fix);
    }
}
