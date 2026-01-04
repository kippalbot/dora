//! Provider data models

use serde::{Deserialize, Serialize};

/// Unique identifier for a provider
pub type ProviderId = String;

/// Type of AI provider API
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProviderType {
    #[default]
    OpenAi,
    DeepSeek,
    AlibabaCloud,
    Custom,
}

impl ProviderType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderType::OpenAi => "OpenAI",
            ProviderType::DeepSeek => "DeepSeek",
            ProviderType::AlibabaCloud => "Alibaba Cloud",
            ProviderType::Custom => "Custom",
        }
    }
}

/// Connection status of a provider
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ProviderConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl ProviderConnectionStatus {
    pub fn display_text(&self) -> &str {
        match self {
            ProviderConnectionStatus::Disconnected => "Disconnected",
            ProviderConnectionStatus::Connecting => "Connecting...",
            ProviderConnectionStatus::Connected => "Connected",
            ProviderConnectionStatus::Error(msg) => msg.as_str(),
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self, ProviderConnectionStatus::Connected)
    }
}

/// A configured AI provider
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Provider {
    pub id: ProviderId,
    pub name: String,
    pub url: String,
    pub api_key: Option<String>,
    pub provider_type: ProviderType,
    pub enabled: bool,
    pub models: Vec<String>,
    pub is_custom: bool,
    #[serde(skip)]
    pub connection_status: ProviderConnectionStatus,
}

impl Default for Provider {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            url: String::new(),
            api_key: None,
            provider_type: ProviderType::OpenAi,
            enabled: false,
            models: Vec::new(),
            is_custom: false,
            connection_status: ProviderConnectionStatus::Disconnected,
        }
    }
}

impl Provider {
    pub fn new_custom(name: String, url: String, provider_type: ProviderType) -> Self {
        let id = Self::generate_id(&name);
        Self {
            id,
            name,
            url,
            api_key: None,
            provider_type,
            enabled: false,
            models: Vec::new(),
            is_custom: true,
            connection_status: ProviderConnectionStatus::Disconnected,
        }
    }

    pub fn generate_id(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect()
    }

    pub fn status_color(&self) -> &'static str {
        if !self.enabled {
            "#9ca3af" // Gray - disabled
        } else {
            match &self.connection_status {
                ProviderConnectionStatus::Connected => "#22c55e", // Green
                ProviderConnectionStatus::Connecting => "#f59e0b", // Yellow
                ProviderConnectionStatus::Disconnected => "#6b7280", // Gray
                ProviderConnectionStatus::Error(_) => "#ef4444", // Red
            }
        }
    }
}

/// Pre-configured supported providers
pub fn get_supported_providers() -> Vec<Provider> {
    vec![
        Provider {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            url: "https://api.openai.com/v1".to_string(),
            api_key: None,
            provider_type: ProviderType::OpenAi,
            enabled: false,
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "o1-mini".to_string(),
            ],
            is_custom: false,
            connection_status: ProviderConnectionStatus::Disconnected,
        },
        Provider {
            id: "deepseek".to_string(),
            name: "DeepSeek".to_string(),
            url: "https://api.deepseek.com/v1".to_string(),
            api_key: None,
            provider_type: ProviderType::DeepSeek,
            enabled: false,
            models: vec![
                "deepseek-chat".to_string(),
                "deepseek-reasoner".to_string(),
            ],
            is_custom: false,
            connection_status: ProviderConnectionStatus::Disconnected,
        },
        Provider {
            id: "alibaba_cloud".to_string(),
            name: "Alibaba Cloud (Qwen)".to_string(),
            url: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
            api_key: None,
            provider_type: ProviderType::AlibabaCloud,
            enabled: false,
            models: vec![
                "qwen-plus".to_string(),
                "qwen-turbo".to_string(),
                "qwen-max".to_string(),
            ],
            is_custom: false,
            connection_status: ProviderConnectionStatus::Disconnected,
        },
    ]
}
