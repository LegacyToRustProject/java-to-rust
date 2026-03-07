use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Trait for LLM providers used in code generation.
/// Aligned with cobol-to-rust's LlmRequest/LlmResponse pattern.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a prompt and get a response.
    async fn generate(&self, request: &LlmRequest) -> Result<LlmResponse>;

    /// Provider name for logging.
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub tokens_used: Option<u32>,
}

/// Claude API implementation.
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| "claude-sonnet-4-20250514".to_string()),
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY environment variable not set"))?;
        Ok(Self::new(api_key, None))
    }
}

#[derive(Serialize)]
struct ClaudeApiRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
}

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeApiResponse {
    content: Vec<ClaudeContent>,
    usage: Option<ClaudeUsage>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    text: String,
}

#[derive(Deserialize)]
struct ClaudeUsage {
    output_tokens: Option<u32>,
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn generate(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let api_request = ClaudeApiRequest {
            model: self.model.clone(),
            max_tokens: request.max_tokens,
            system: request.system_prompt.clone(),
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: request.user_prompt.clone(),
            }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&api_request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await?;
            anyhow::bail!("Claude API error ({}): {}", status, body);
        }

        let api_response: ClaudeApiResponse = response.json().await?;
        let content = api_response
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| anyhow::anyhow!("Empty response from Claude API"))?;

        let tokens_used = api_response.usage.and_then(|u| u.output_tokens);

        Ok(LlmResponse {
            content,
            tokens_used,
        })
    }

    fn name(&self) -> &str {
        "Claude"
    }
}

/// Mock LLM provider for testing.
pub struct MockProvider {
    response: String,
}

impl MockProvider {
    pub fn new(response: String) -> Self {
        Self { response }
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn generate(&self, _request: &LlmRequest) -> Result<LlmResponse> {
        Ok(LlmResponse {
            content: self.response.clone(),
            tokens_used: None,
        })
    }

    fn name(&self) -> &str {
        "Mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockProvider::new("fn hello() {}".to_string());
        let request = LlmRequest {
            system_prompt: "system".to_string(),
            user_prompt: "convert".to_string(),
            max_tokens: 4096,
            temperature: 0.0,
        };
        let result = provider.generate(&request).await.unwrap();
        assert_eq!(result.content, "fn hello() {}");
    }

    #[test]
    fn test_provider_name() {
        let provider = MockProvider::new(String::new());
        assert_eq!(provider.name(), "Mock");
    }

    #[test]
    fn test_llm_request_serialization() {
        let req = LlmRequest {
            system_prompt: "You are a Java expert.".to_string(),
            user_prompt: "Convert this Java to Rust.".to_string(),
            max_tokens: 8192,
            temperature: 0.0,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Java expert"));
    }
}
