use anyhow::{Context, Result};
use miow_graph::{KnowledgeGraph, SymbolSearchResult};
use miow_vector::{VectorStore, SymbolSearchResult as VectorResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::{LLMProvider, Message, Role};

/// Critical question for context gathering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalQuestion {
    pub question: String,
    pub search_query: String,
    pub expected_type: String, // "component", "function", "type", "constant", etc.
    pub priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    Critical,  // Must find
    High,      // Should find
    Medium,    // Nice to have
}

/// Verification result from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub is_correct: bool,
    pub reason: String,
    pub suggestion: Option<String>, // Suggested reformulation
}

/// Question execution result
#[derive(Debug, Clone)]
pub enum QuestionResult {
    Found(Vec<QuestionAnswer>),
    NotFound,
    PartiallyFound(Vec<QuestionAnswer>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionAnswer {
    pub question: String,
    pub symbols: Vec<SymbolSearchResult>,
    pub confidence: f32,
}

/// Question loop executor with rollback capability
pub struct QuestionLoop {
    llm: Arc<dyn LLMProvider>,
    vector_store: Option<Arc<VectorStore>>,
    graph: Arc<KnowledgeGraph>,
    max_retries: usize,
}

impl QuestionLoop {
    pub fn new(
        llm: Arc<dyn LLMProvider>,
        vector_store: Option<Arc<VectorStore>>,
        graph: Arc<KnowledgeGraph>,
    ) -> Self {
        Self {
            llm,
            vector_store,
            graph,
            max_retries: 3,
        }
    }
    
    /// Execute all questions and gather verified context
    pub async fn execute_questions(
        &self,
        questions: Vec<CriticalQuestion>,
    ) -> Result<Vec<QuestionAnswer>> {
        let mut answers = Vec::new();
        
        info!("📋 Executing {} questions", questions.len());
        
        for (i, question) in questions.iter().enumerate() {
            info!("❓ [QUESTION {}/{}] {}", i + 1, questions.len(), question.question);
            info!("   Search query: '{}', Expected type: {}, Priority: {:?}", 
                  question.search_query, question.expected_type, question.priority);
            
            match self.execute_single_question(question.clone()).await {
                Ok(result) => {
                    match result {
                        QuestionResult::Found(mut found) => {
                            debug!("✅ Found {} results", found.len());
                            answers.append(&mut found);
                        }
                        QuestionResult::PartiallyFound(mut partial) => {
                            debug!("⚠️  Partially found {} results", partial.len());
                            answers.append(&mut partial);
                        }
                        QuestionResult::NotFound => {
                            if question.priority == Priority::Critical {
                                warn!("❌ Critical question failed: {}", question.question);
                            } else {
                                debug!("ℹ️  Optional question not answered: {}", question.question);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Error executing question '{}': {}", question.question, e);
                }
            }
        }
        
        Ok(answers)
    }
    
    /// Execute a single question with retry logic
    async fn execute_single_question(&self, mut question: CriticalQuestion) -> Result<QuestionResult> {
        for attempt in 0..self.max_retries {
            debug!("🔄 Attempt {}/{}", attempt + 1, self.max_retries);
            
            // 1. Search using current query
            info!("🔍 [SEARCH] Query: '{}' (attempt {}/{})", 
                  question.search_query, attempt + 1, self.max_retries);
            let search_start = std::time::Instant::now();
            let search_results = self.search(&question.search_query).await?;
            let search_duration = search_start.elapsed();
            info!("   Found {} results in {:?}", search_results.len(), search_duration);
            
            if search_results.is_empty() && attempt < self.max_retries - 1 {
                // Try to reformulate before verifying
                debug!("No results found, reformulating query...");
                question = self.reformulate_question(question, &search_results).await?;
                continue;
            }
            
            if search_results.is_empty() {
                return Ok(QuestionResult::NotFound);
            }
            
            // 2. Verify results with LLM
            info!("💬 [LLM VERIFY] Verifying {} results against question...", search_results.len());
            let verify_start = std::time::Instant::now();
            let verification = self.verify_results(&question, &search_results).await?;
            let verify_duration = verify_start.elapsed();
            info!("   Verification result: is_correct={}, reason: '{}' (took {:?})", 
                  verification.is_correct, verification.reason, verify_duration);
            
            if verification.is_correct {
                // Success!
                return Ok(QuestionResult::Found(vec![QuestionAnswer {
                    question: question.question,
                    symbols: search_results,
                    confidence: 1.0,
                }]));
            }
            
            // 3. Rollback and retry
            if attempt < self.max_retries - 1 {
                debug!("🔙 Verification failed: {}", verification.reason);
                debug!("Reformulating query...");
                
                question = self.reformulate_question(question, &search_results).await?;
            } else {
                // Last attempt failed, return partial if we have something
                if !search_results.is_empty() {
                    return Ok(QuestionResult::PartiallyFound(vec![QuestionAnswer {
                        question: question.question,
                        symbols: search_results,
                        confidence: 0.5,
                    }]));
                } else {
                    return Ok(QuestionResult::NotFound);
                }
            }
        }
        
        Ok(QuestionResult::NotFound)
    }
    
    /// Search for symbols using vector store and/or knowledge graph
    async fn search(&self, query: &str) -> Result<Vec<SymbolSearchResult>> {
        let mut results = Vec::new();
        
        // Try vector search first if available (semantic understanding like Cursor)
        if let Some(vector_store) = &self.vector_store {
            info!("   [VECTOR_SEARCH] Searching for: '{}'", query);
            let vector_start = std::time::Instant::now();
            match vector_store.search_similar(query, 10).await {
                Ok(vector_results) => {
                    let vector_duration = vector_start.elapsed();
                    info!("   [VECTOR_SEARCH] Found {} results in {:?}", vector_results.len(), vector_duration);
                    
                    // Convert vector results to symbol results
                    for vr in vector_results {
                        // Try to find full symbol info from graph
                        if let Ok(symbols) = self.graph.find_symbols_by_name(&vr.symbol.name) {
                            results.extend(symbols);
                        }
                    }
                }
                Err(e) => {
                    warn!("   [VECTOR_SEARCH] Failed: {}, falling back to graph", e);
                }
            }
        }
        
        // Also search knowledge graph
        info!("   [GRAPH_SEARCH] Searching knowledge graph for: '{}'", query);
        let graph_start = std::time::Instant::now();
        if let Ok(graph_results) = self.graph.search_symbols(query) {
            let graph_duration = graph_start.elapsed();
            info!("   [GRAPH_SEARCH] Found {} results in {:?}", graph_results.len(), graph_duration);
            
            // Merge with vector results (deduplicate)
            for gr in graph_results {
                if !results.iter().any(|r| r.name == gr.name && r.file_path == gr.file_path) {
                    results.push(gr);
                }
            }
        }
        
        Ok(results)
    }
    
    /// Verify if search results answer the question
    async fn verify_results(
        &self,
        question: &CriticalQuestion,
        results: &[SymbolSearchResult],
    ) -> Result<VerificationResult> {
        let results_summary: Vec<String> = results
            .iter()
            .take(5)
            .map(|r| format!("- {} ({}) in {}", r.name, r.kind, r.file_path))
            .collect();
        
        let prompt = format!(
            r#"Question: {}
Expected type: {}
Search query used: {}

Search results found:
{}

Task: Verify if these results correctly answer the question.
Respond with JSON:
{{
  "is_correct": true/false,
  "reason": "explanation",
  "suggestion": "optional reformulated search query if incorrect"
}}

Return ONLY the JSON."#,
            question.question,
            question.expected_type,
            question.search_query,
            results_summary.join("\n")
        );
        
        info!("   [LLM] Calling LLM for verification...");
        let llm_start = std::time::Instant::now();
        let response = self.llm.generate(&prompt).await?;
        let llm_duration = llm_start.elapsed();
        info!("   [LLM] Response received in {:?} ({} chars)", llm_duration, response.content.len());
        
        // Parse JSON response
        let clean = response.content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        
        let verification: VerificationResult = serde_json::from_str(clean)
            .unwrap_or(VerificationResult {
                is_correct: !results.is_empty(),
                reason: "Failed to parse verification response".to_string(),
                suggestion: None,
            });
        
        Ok(verification)
    }
    
    /// Reformulate question based on failed search
    async fn reformulate_question(
        &self,
        question: CriticalQuestion,
        _failed_results: &[SymbolSearchResult],
    ) -> Result<CriticalQuestion> {
        let prompt = format!(
            r#"The search query "{}" for question "{}" did not find the correct results.

Suggest a better search query. Consider:
- More specific terms
- Alternate naming (e.g., "User" vs "UserModel" vs "UserStruct")
- Related terms
- Type-specific searches

Respond with JSON:
{{
  "new_query": "improved search query"
}}

Return ONLY the JSON."#,
            question.search_query, question.question
        );
        
        info!("   [LLM] Calling LLM for query reformulation...");
        let reformulate_start = std::time::Instant::now();
        let response = self.llm.generate(&prompt).await?;
        let reformulate_duration = reformulate_start.elapsed();
        info!("   [LLM] Reformulation response received in {:?}", reformulate_duration);
        
        let clean = response.content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(clean) {
            if let Some(new_query) = json["new_query"].as_str() {
                debug!("🔄 Reformulated: '{}' → '{}'", question.search_query, new_query);
                return Ok(CriticalQuestion {
                    search_query: new_query.to_string(),
                    ..question
                });
            }
        }
        
        // Fallback: Try common variations
        let new_query = if question.search_query.contains("User") {
            question.search_query.replace("User", "UserModel")
        } else {
            format!("{} {}", question.expected_type, question.search_query)
        };
        
        Ok(CriticalQuestion {
            search_query: new_query,
            ..question
        })
    }
}

/// Generate language-specific critical questions
pub async fn generate_critical_questions(
    llm: &dyn LLMProvider,
    user_prompt: &str,
    project_language: &str,
    framework: Option<&str>,
) -> Result<Vec<CriticalQuestion>> {
    let framework_context = framework
        .map(|f| format!("using {} framework", f))
        .unwrap_or_default();
    
    let prompt = format!(
        r#"You are analyzing a {} project {} for the following user request:
"{}"

Generate 3-5 critical questions to ask about the existing codebase to avoid duplicating existing code.

For each question, specify:
- question: The question to ask
- search_query: What to search for in the codebase
- expected_type: What type of code element (component/function/type/constant/schema)
- priority: critical/high/medium

Examples for different languages:
- React/TypeScript: "Is there a Button component?", search: "Button", type: "component"
- Rust: "Is there a User struct?", search: "User struct", type: "type"
- Python: "Is there an auth decorator?", search: "auth decorator", type: "function"

Respond with JSON array:
[
  {{
    "question": "...",
    "search_query": "...",
    "expected_type": "...",
    "priority": "critical"
  }}
]

Return ONLY the JSON array."#,
        project_language, framework_context, user_prompt
    );
    
    let response = llm.generate(&prompt).await?;
    
    let clean = response.content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    
    let questions: Vec<serde_json::Value> = serde_json::from_str(clean)
        .context("Failed to parse questions from LLM")?;
    
    let mut critical_questions = Vec::new();
    
    for q in questions {
        let question = q["question"].as_str().unwrap_or("").to_string();
        let search_query = q["search_query"].as_str().unwrap_or("").to_string();
        let expected_type = q["expected_type"].as_str().unwrap_or("unknown").to_string();
        let priority_str = q["priority"].as_str().unwrap_or("medium");
        
        let priority = match priority_str {
            "critical" => Priority::Critical,
            "high" => Priority::High,
            _ => Priority::Medium,
        };
        
        if !question.is_empty() && !search_query.is_empty() {
            critical_questions.push(CriticalQuestion {
                question,
                search_query,
                expected_type,
                priority,
            });
        }
    }
    
    Ok(critical_questions)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_question_creation() {
        let q = CriticalQuestion {
            question: "Is there a User struct?".to_string(),
            search_query: "User".to_string(),
            expected_type: "struct".to_string(),
            priority: Priority::Critical,
        };
        
        assert_eq!(q.priority, Priority::Critical);
    }
}
