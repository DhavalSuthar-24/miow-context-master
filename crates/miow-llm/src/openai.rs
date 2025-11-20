use super::*;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

pub struct OpenAIClient {
    client: Client,
    api_key: String,
    model: String,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "gpt-4-turbo-preview".to_string(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

#[async_trait]
impl LLMProvider for OpenAIClient {
    async fn generate(&self, prompt: &str) -> Result<LLMResponse> {
        let messages = vec![Message {
            role: Role::User,
            content: prompt.to_string(),
        }];
        self.generate_with_context(messages).await
    }

    async fn generate_with_context(&self, messages: Vec<Message>) -> Result<LLMResponse> {
        let url = "https://api.openai.com/v1/chat/completions";

        let openai_messages: Vec<serde_json::Value> = messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                };
                json!({
                    "role": role,
                    "content": msg.content
                })
            })
            .collect();

        let body = json!({
            "model": self.model,
            "messages": openai_messages,
            "temperature": 0.7,
            "max_tokens": 4096,
        });

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = json["usage"].as_object().map(|u| Usage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as usize,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as usize,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as usize,
        });

        Ok(LLMResponse {
            content,
            finish_reason: json["choices"][0]["finish_reason"]
                .as_str()
                .map(|s| s.to_string()),
            usage,
        })
    }

    async fn stream_generate(
        &self,
        _prompt: &str,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin>> {
        // TODO: Implement streaming
        unimplemented!("Streaming not yet implemented for OpenAI")
    }

    async fn generate_multi_step(&self, steps: Vec<String>, context: &str) -> Result<LLMResponse> {
        let mut final_content = String::new();

        for (i, step_prompt) in steps.iter().enumerate() {
            let full_prompt = format!("Step {}/{}: {}\nContext: {}", i + 1, steps.len(), step_prompt, context);
            let response = self.generate(&full_prompt).await?;
            final_content += &format!("Step {}: {}\n", i + 1, response.content);
        }

        Ok(LLMResponse {
            content: final_content,
            finish_reason: None,
            usage: None,
        })
    }

    async fn generate_with_framework(&self, prompt: &str, framework: &str, lang: &str) -> Result<LLMResponse> {
        let enhanced_prompt = format!(
            "You are an expert {} developer using {} framework.\n\n{}",
            lang, framework, prompt
        );
        self.generate(&enhanced_prompt).await
    }
}
