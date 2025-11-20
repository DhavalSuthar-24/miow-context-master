use crate::PromptRegistry;
use anyhow::{Context, Result};
use async_trait::async_trait;
use miow_core::ProjectSignature;
use miow_llm::{LLMProvider, Message, Role};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// A single semantic search query the router wants to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Natural language or keyword-based query.
    pub query: String,
    /// Optional hint about what we expect to find (`component`, `type`, `schema`, `api`, etc.).
    #[serde(default)]
    pub kind: Option<String>,
    /// Optional list of directories / path prefixes the router thinks are relevant.
    #[serde(default)]
    pub target_paths: Vec<String>,
}

/// A plan for running one or more specialized workers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPlan {
    /// Logical worker id, e.g. "ui", "auth", "api", "schema".
    pub worker_id: String,
    /// High-level description of what this worker should focus on.
    pub description: String,
    /// Queries this worker should execute.
    #[serde(default)]
    pub queries: Vec<SearchQuery>,
}

/// Top‑level router output describing how to search the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPlan {
    /// High‑level intent label, e.g. "create_login_page", "add_auth", "fix_bug".
    #[serde(default)]
    pub global_intent: String,
    /// General queries that all workers / retrievers can use.
    #[serde(default)]
    pub search_queries: Vec<SearchQuery>,
    /// Optional worker‑specific plans.
    #[serde(default)]
    pub workers: Vec<WorkerPlan>,
    /// Execution order for workers (considering dependencies)
    #[serde(default)]
    pub execution_plan: Vec<String>, // Worker IDs in execution order
}

impl SearchPlan {
    /// Convenience helper to flatten all queries into plain strings.
    pub fn all_query_strings(&self) -> Vec<String> {
        let mut out = Vec::new();
        for q in &self.search_queries {
            if !q.query.trim().is_empty() {
                out.push(q.query.trim().to_string());
            }
        }
        for w in &self.workers {
            for q in &w.queries {
                if !q.query.trim().is_empty() {
                    out.push(q.query.trim().to_string());
                }
            }
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.global_intent.trim().is_empty()
            && self.search_queries.is_empty()
            && self.workers.is_empty()
    }
}

/// Trait for router agents that take a task + project context and produce a search plan.
#[async_trait]
pub trait RouterAgent: Send + Sync {
    async fn plan(
        &self,
        user_prompt: &str,
        project_signature: &ProjectSignature,
    ) -> Result<SearchPlan>;
}

/// LLM‑backed router that uses a generic `LLMProvider` (e.g. Gemini) to plan searches.
pub struct GeminiRouterAgent {
    llm: Arc<dyn LLMProvider>,
    registry: Arc<PromptRegistry>,
}

impl GeminiRouterAgent {
    pub fn new(llm: Arc<dyn LLMProvider>) -> Self {
        Self {
            llm,
            registry: Arc::new(PromptRegistry::new()),
        }
    }

    pub fn with_registry(llm: Arc<dyn LLMProvider>, registry: Arc<PromptRegistry>) -> Self {
        Self { llm, registry }
    }
}

#[async_trait]
impl RouterAgent for GeminiRouterAgent {
    async fn plan(
        &self,
        user_prompt: &str,
        project_signature: &ProjectSignature,
    ) -> Result<SearchPlan> {
        // First, classify the task to get recommended workers
        let task_classification = self.classify_task(user_prompt, project_signature).await?;
        let recommended_workers = self.registry.get_recommended_prompts(&task_classification.task_type);

        // Get available worker descriptions for the LLM
        let available_workers = self.get_available_workers_description();

        let project_description = project_signature.to_description();

        let system_prompt = format!(r#"You are a Senior Architect Router Agent for an autonomous code-understanding system.
Your job is to:
- Read the user's task and a short project description.
- Decide the high-level intent.
- Plan how to search the codebase (queries + which "workers" to activate).

Available specialized workers:
{}

You MUST respond with a single JSON object ONLY, no extra commentary, matching this schema:
{{
  "global_intent": "short_snake_case_label",
  "search_queries": [
    {{ "query": "string", "kind": "component|type|schema|api|style|helper|any", "target_paths": ["optional/path", "..."] }}
  ],
  "workers": [
    {{
      "worker_id": "worker_key_from_available_list",
      "description": "what this worker should focus on",
      "queries": [
        {{ "query": "string", "kind": "component|type|schema|api|style|helper|any", "target_paths": ["optional/path"] }}
      ]
    }}
  ]
}}

Guidelines:
- Use 3–8 strong search queries, not 1 generic query.
- Include at least one query for types/schemas if the task touches data or forms.
- Include at least one UI query if the task has any frontend or page aspect.
- Use target_paths hints when obvious (e.g. React: src/components, Next.js: app, pages).
- Select 2-4 workers from the available list based on task needs.
- If unsure, leave target_paths empty.
"#, available_workers);

        let user_message = format!(
            "User task:\n{}\n\nDetected project description:\n{}\n\nRecommended workers based on task type: {}\n",
            user_prompt, project_description, recommended_workers.join(", ")
        );

        let messages = vec![
            Message {
                role: Role::System,
                content: system_prompt.to_string(),
            },
            Message {
                role: Role::User,
                content: user_message,
            },
        ];

        let response = self
            .llm
            .generate_with_context(messages)
            .await
            .context("Router LLM call failed")?;

        // Strip possible markdown fences.
        let raw = response.content.trim();
        let clean = raw
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let plan: Result<SearchPlan> =
            serde_json::from_str(clean).context("Failed to parse router JSON plan");

        match plan {
            Ok(mut p) if !p.is_empty() => {
                // Build execution plan based on dependencies
                let worker_ids: Vec<String> = p.workers.iter().map(|w| w.worker_id.clone()).collect();
                p.execution_plan = self.build_execution_plan(&worker_ids);
                Ok(p)
            },
            _ => {
                // Fallback: use recommended workers with basic queries
                let mut fallback_plan = self.create_fallback_plan(user_prompt, &recommended_workers)?;
                let worker_ids: Vec<String> = fallback_plan.workers.iter().map(|w| w.worker_id.clone()).collect();
                fallback_plan.execution_plan = self.build_execution_plan(&worker_ids);
                Ok(fallback_plan)
            }
        }
    }
}

impl GeminiRouterAgent {
    /// Build execution plan considering worker dependencies
    fn build_execution_plan(&self, worker_ids: &[String]) -> Vec<String> {
        let mut execution_order = Vec::new();
        let mut remaining = worker_ids.to_vec();
        let mut processed = std::collections::HashSet::new();

        // Continue until all workers are processed or we can't resolve dependencies
        while !remaining.is_empty() {
            let mut progressed = false;

            // Find workers whose dependencies are satisfied
            remaining.retain(|worker_id| {
                if let Some(prompt) = self.registry.get_prompt(worker_id) {
                    // Check if all dependencies are already processed
                    let deps_satisfied = prompt.dependencies.iter().all(|dep| processed.contains(dep));

                    if deps_satisfied {
                        execution_order.push(worker_id.clone());
                        processed.insert(worker_id.clone());
                        progressed = true;
                        false // Remove from remaining
                    } else {
                        true // Keep in remaining
                    }
                } else {
                    // Unknown worker, add anyway to avoid infinite loop
                    execution_order.push(worker_id.clone());
                    processed.insert(worker_id.clone());
                    progressed = true;
                    false
                }
            });

            // If no progress was made, add remaining workers in arbitrary order to break cycles
            if !progressed && !remaining.is_empty() {
                for worker_id in remaining.drain(..) {
                    execution_order.push(worker_id.clone());
                }
            }
        }

        execution_order
    }

    /// Classify the task type using the task_classifier worker
    async fn classify_task(&self, user_prompt: &str, project_signature: &ProjectSignature) -> Result<TaskClassification> {
        let classifier = self.registry.get_prompt("task_classifier")
            .ok_or_else(|| anyhow::anyhow!("task_classifier prompt not found"))?;

        let template = &classifier.template;
        let project_info = project_signature.to_description();

        let full_prompt = template
            .replace("{user_prompt}", user_prompt)
            .replace("{project_info}", &project_info)
            .replace("{project_stack}", &project_info);

        let messages = vec![
            Message {
                role: Role::System,
                content: "You are a task classification specialist.".to_string(),
            },
            Message {
                role: Role::User,
                content: full_prompt,
            },
        ];

        let response = self.llm.generate_with_context(messages).await?;

        // Try to parse JSON response
        let clean = response.content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(clean) {
            let task_type = json.get("task_type")
                .and_then(|v| v.as_str())
                .unwrap_or("feature")
                .to_string();

            return Ok(TaskClassification { task_type });
        }

        // Fallback classification
        Ok(TaskClassification { task_type: "feature".to_string() })
    }

    /// Get description of available workers for the LLM
    fn get_available_workers_description(&self) -> String {
        let mut descriptions = Vec::new();
        for (key, prompt) in self.registry.get_all_prompts() {
            descriptions.push(format!("- {}: {}", key, prompt.description));
        }
        descriptions.join("\n")
    }

    /// Create a fallback plan when LLM parsing fails
    fn create_fallback_plan(&self, user_prompt: &str, recommended_workers: &[String]) -> Result<SearchPlan> {
        let mut workers = Vec::new();

        // Convert recommended worker keys to WorkerPlan objects
        for worker_key in recommended_workers.iter().take(3) {
            if let Some(prompt) = self.registry.get_prompt(worker_key) {
                workers.push(WorkerPlan {
                    worker_id: worker_key.clone(),
                    description: prompt.description.clone(),
                    queries: vec![SearchQuery {
                        query: user_prompt.to_string(),
                        kind: Some("any".to_string()),
                        target_paths: Vec::new(),
                    }],
                });
            }
        }

        Ok(SearchPlan {
            global_intent: "fallback_plan".to_string(),
            search_queries: vec![SearchQuery {
                query: user_prompt.to_string(),
                kind: Some("any".to_string()),
                target_paths: Vec::new(),
            }],
            workers,
            execution_plan: vec![],
        })
    }
}

/// Task classification result
#[derive(Debug)]
struct TaskClassification {
    task_type: String,
}


