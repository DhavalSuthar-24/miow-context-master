use crate::{SearchQuery, SpecializedPrompt, PromptRegistry};
use async_trait::async_trait;
use miow_common::{CodeChunk, Result as MiowResult};
use miow_core::ProjectSignature;
use miow_llm::{LLMProvider, Message, Role};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// Result from running a worker agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    pub worker_id: String,
    pub chunks: Vec<CodeChunk>,
    pub summary: String,
    pub confidence: f32,
}

/// Trait for worker agents that execute specialized prompts
#[async_trait]
pub trait WorkerAgent: Send + Sync {
    async fn execute(
        &self,
        prompt_key: &str,
        user_prompt: &str,
        project_signature: &ProjectSignature,
        search_queries: &[SearchQuery],
    ) -> MiowResult<WorkerResult>;
}

/// LLM-backed worker agent that can execute any specialized prompt
pub struct GeminiWorkerAgent {
    llm: Arc<dyn LLMProvider>,
    registry: Arc<PromptRegistry>,
}

impl GeminiWorkerAgent {
    pub fn new(llm: Arc<dyn LLMProvider>, registry: Arc<PromptRegistry>) -> Self {
        Self {
            llm,
            registry: registry.clone(),
        }
    }

    pub fn new_with_registry(llm: Arc<dyn LLMProvider>) -> Self {
        Self {
            llm,
            registry: Arc::new(PromptRegistry::new()),
        }
    }
}

#[async_trait]
impl WorkerAgent for GeminiWorkerAgent {
    async fn execute(
        &self,
        prompt_key: &str,
        user_prompt: &str,
        project_signature: &ProjectSignature,
        search_queries: &[SearchQuery],
    ) -> MiowResult<WorkerResult> {
        let prompt = self.registry.get_prompt(prompt_key)
            .ok_or_else(|| miow_common::MiowError::Generic(
                anyhow::anyhow!("Unknown prompt key: {}", prompt_key)
            ))?;

        // Build the full prompt by substituting variables
        let template = &prompt.template;
        let project_info = project_signature.to_description();
        let query_list = search_queries.iter()
            .map(|q| format!("- {} ({})", q.query, q.kind.as_deref().unwrap_or("any")))
            .collect::<Vec<_>>()
            .join("\n");

        let full_prompt = template
            .replace("{user_prompt}", user_prompt)
            .replace("{project_info}", &project_info)
            .replace("{project_stack}", &project_info)
            .replace("{file_path}", "") // Could be enhanced to pass specific files
            .replace("{error_message}", "") // Could be enhanced for error analysis
            .replace("{file_list}", "") // Could be enhanced with FileMap
            .replace("{package_managers}", "") // Could be enhanced with package info
            .replace("{config_files}", ""); // Could be enhanced with config detection

        // Create messages for LLM
        let messages = vec![
            Message {
                role: Role::System,
                content: format!("You are a {}. {}", prompt.description, prompt.description),
            },
            Message {
                role: Role::User,
                content: full_prompt,
            },
        ];

        // Call LLM
        let response = self.llm.generate_with_context(messages)
            .await
            .map_err(|e| miow_common::MiowError::Llm(e.to_string()))?;

        // Parse response (this would be specific to each prompt type)
        // For now, return a basic result - in practice, each worker would have custom parsing
        let chunks = self.parse_llm_response(prompt_key, &response.content)?;

        Ok(WorkerResult {
            worker_id: prompt_key.to_string(),
            chunks,
            summary: format!("Executed {} worker", prompt_key),
            confidence: 0.8, // Could be calculated based on response quality
        })
    }
}

impl GeminiWorkerAgent {
    /// Parse LLM response into CodeChunk objects (basic implementation)
    fn parse_llm_response(&self, prompt_key: &str, response: &str) -> MiowResult<Vec<CodeChunk>> {
        // This is a simplified parser - in practice, each worker type would have
        // custom JSON schema parsing based on what it returns

        // Try to parse as JSON first
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
            if let Some(array) = json.as_array() {
                let mut chunks = Vec::new();
                for item in array {
                    if let Some(obj) = item.as_object() {
                        let chunk = CodeChunk {
                            id: format!("{}-{}", prompt_key, chunks.len()),
                            content: obj.get("content")
                                .or_else(|| obj.get("definition"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            file_path: obj.get("file_path")
                                .or_else(|| obj.get("path"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            language: obj.get("language")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            start_line: 0,
                            end_line: 0,
                            kind: obj.get("kind")
                                .or_else(|| obj.get("type"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            metadata: json!({
                                "worker": prompt_key,
                                "description": obj.get("description").and_then(|v| v.as_str()).unwrap_or("")
                            }),
                        };
                        chunks.push(chunk);
                    }
                }
                return Ok(chunks);
            }
        }

        // Fallback: create a single chunk with the raw response
        Ok(vec![CodeChunk {
            id: format!("{}-fallback", prompt_key),
            content: response.to_string(),
            file_path: format!("{}_analysis.txt", prompt_key),
            language: "text".to_string(),
            start_line: 0,
            end_line: 0,
            kind: "analysis".to_string(),
            metadata: json!({
                "worker": prompt_key,
                "fallback": true
            }),
        }])
    }
}

/// Factory function to create worker agents
pub fn create_worker_agent(llm: Arc<dyn LLMProvider>) -> Box<dyn WorkerAgent> {
    Box::new(GeminiWorkerAgent::new_with_registry(llm))
}
