use anyhow::{Context, Result};
use miow_llm::{ContextItem, GatheredContext, LLMProvider, Message, Role};
use serde::Deserialize;
use std::sync::Arc;

/// Simple LLM-backed context auditor that selects only the most essential items.
pub struct GeminiContextAuditor {
    llm: Arc<dyn LLMProvider>,
}

impl GeminiContextAuditor {
    pub fn new(llm: Arc<dyn LLMProvider>) -> Self {
        Self { llm }
    }

    /// Audit and prune a gathered context in-place. Never fails hard – on error it leaves context unchanged.
    pub async fn audit(
        &self,
        user_prompt: &str,
        gathered: &mut GatheredContext,
    ) -> Result<()> {
        // Only bother the LLM if we have more than a handful of items.
        if gathered.components.len()
            + gathered.helpers.len()
            + gathered.types.len()
            + gathered.schemas.len()
            <= 12
        {
            return Ok(());
        }

        self.audit_category("components", user_prompt, &mut gathered.components)
            .await
            .ok();
        self.audit_category("helpers", user_prompt, &mut gathered.helpers)
            .await
            .ok();
        self.audit_category("types", user_prompt, &mut gathered.types)
            .await
            .ok();
        self.audit_category("schemas", user_prompt, &mut gathered.schemas)
            .await
            .ok();

        Ok(())
    }

    async fn audit_category(
        &self,
        category: &str,
        user_prompt: &str,
        items: &mut Vec<ContextItem>,
    ) -> Result<()> {
        if items.len() <= 8 {
            return Ok(());
        }

        // Build a lightweight summary of each item to keep tokens manageable.
        let summaries: Vec<ItemSummary> = items
            .iter()
            .enumerate()
            .map(|(idx, item)| ItemSummary {
                index: idx,
                name: item.name.clone(),
                kind: item.kind.clone(),
                file_path: item.file_path.clone(),
                preview: truncate_preview(&item.content, 320),
            })
            .collect();

        let system_prompt = r#"You are a Context Auditor Agent for an autonomous code-understanding system.
Given a user task and a list of candidate code items, decide which items are essential.

Rules:
- Prefer items that are directly useful for implementing the task.
- Prefer framework-/architecture-specific entry points and core domain types.
- Avoid generic utilities that are not clearly relevant.

You MUST respond with JSON only, matching:
{ "keep_indices": [0, 2, 5] }
"#;

        let user_message = format!(
            "User task:\n{}\n\nCategory: {}\n\nCandidate items:\n{}",
            user_prompt,
            category,
            serde_json::to_string_pretty(&summaries)?
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
            .context("Context auditor LLM call failed")?;

        let raw = response.content.trim();
        let clean = raw
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let parsed: AuditDecision = serde_json::from_str(clean)
            .or_else(|_| serde_json::from_str::<AuditDecision>(raw))
            .context("Failed to parse context auditor JSON")?;

        if parsed.keep_indices.is_empty() {
            return Ok(()); // Don't change anything on empty decision.
        }

        let mut new_items = Vec::new();
        for idx in parsed.keep_indices {
            if let Some(item) = items.get(idx) {
                new_items.push(item.clone());
            }
        }
        if !new_items.is_empty() {
            *items = new_items;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct AuditDecision {
    #[serde(default)]
    keep_indices: Vec<usize>,
}

#[derive(Debug, serde::Serialize)]
struct ItemSummary {
    index: usize,
    name: String,
    kind: String,
    file_path: String,
    preview: String,
}

fn truncate_preview(content: &str, max_chars: usize) -> String {
    let mut s: String = content.chars().take(max_chars).collect();
    if content.chars().count() > max_chars {
        s.push_str("…");
    }
    s
}


