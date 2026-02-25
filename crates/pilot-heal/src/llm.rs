use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde_json::json;
use std::env;
use std::thread;
use std::time::Duration;

/// Trait for LLM clients used in healing
pub trait LlmClient {
    fn generate_fix(&mut self, prompt: &str) -> Result<String>;
}

/// HTTP-based LLM client compatible with OpenAI API (or Ollama)
pub struct RemoteLlm {
    client: Client,
    base_url: String,
    model: String,
    api_key: String,
    max_retries: u32,
    base_delay_ms: u64,
}

impl RemoteLlm {
    pub fn new() -> Result<Self> {
        let base_url =
            env::var("ARQON_LLM_URL").unwrap_or_else(|_| "http://localhost:11434/v1".to_string());
        let model =
            env::var("ARQON_LLM_MODEL").unwrap_or_else(|_| "deepseek-coder:1.3b".to_string());
        let api_key = env::var("ARQON_LLM_KEY").unwrap_or_else(|_| "ollama".to_string());

        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()?,
            base_url: base_url.trim_end_matches('/').to_string(),
            model,
            api_key,
            max_retries: 3,
            base_delay_ms: 1000,
        })
    }

    fn request_with_retry(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url);

        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": "You are an expert software engineer. Output only the fixed code block."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.2
        });

        let mut last_error = None;

        for attempt in 0..self.max_retries {
            if attempt > 0 {
                let delay = self.base_delay_ms * 2u64.pow(attempt - 1);
                println!(
                    "  Retrying in {}ms (attempt {}/{})",
                    delay,
                    attempt + 1,
                    self.max_retries
                );
                thread::sleep(Duration::from_millis(delay));
            }

            match self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
            {
                Ok(response) => {
                    let status = response.status();

                    // Handle rate limiting
                    if status.as_u16() == 429 {
                        last_error = Some(anyhow::anyhow!("Rate limited (429)"));
                        continue;
                    }

                    // Handle server errors (5xx) - retry
                    if status.is_server_error() {
                        last_error = Some(anyhow::anyhow!("Server error: {}", status));
                        continue;
                    }

                    // Handle client errors (4xx except 429) - don't retry
                    if status.is_client_error() {
                        let error_text = response.text().unwrap_or_default();
                        return Err(anyhow::anyhow!("Client error {}: {}", status, error_text));
                    }

                    // Success
                    let json: serde_json::Value =
                        response.json().context("Failed to parse LLM response")?;

                    let content = json["choices"][0]["message"]["content"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?;

                    // Clean up code fences if present
                    let clean_content = content
                        .lines()
                        .filter(|l| !l.trim().starts_with("```"))
                        .collect::<Vec<_>>()
                        .join("\n");

                    return Ok(clean_content);
                }
                Err(e) => {
                    // Network error - retry
                    last_error = Some(anyhow::anyhow!("Network error: {}", e));
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Max retries exceeded")))
    }
}

impl LlmClient for RemoteLlm {
    fn generate_fix(&mut self, prompt: &str) -> Result<String> {
        self.request_with_retry(prompt)
    }
}
