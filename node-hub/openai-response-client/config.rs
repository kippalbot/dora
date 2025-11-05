use std::{fs, path::PathBuf};

use eyre::{self, Context};
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct Config {
    pub default_model: String,
    pub system_prompt: Option<String>,
    pub max_history_exchanges: usize,
    pub enable_streaming: bool,
    pub log_level: String,
    pub enable_tools: bool,
    pub enable_local_mcp: bool,
    pub status_timeout_seconds: Option<u64>,
    pub providers: Vec<ProviderConfig>,
    pub models: Vec<ModelConfig>,
    pub api_base: Option<String>,
}

impl Config {
    pub fn load() -> eyre::Result<Self> {
        let raw = load_file_config().unwrap_or_default();

        let default_model = env_or("OPENAI_RESPONSE_MODEL", raw.default_model, || "gpt-5".to_string());
        let system_prompt = env_or_opt("SYSTEM_PROMPT", raw.system_prompt);
        let max_history_exchanges_first = env_or(
            "MAX_HISTORY_EXCHANGES",
            raw.max_history_exchanges,
            default_max_history,
        );
        let max_history_exchanges =
            env_or("MAX_HISTORY_MESSAGES", Some(max_history_exchanges_first), || {
                max_history_exchanges_first
            });
        let enable_streaming = env_or_bool("OPENAI_ENABLE_STREAMING", raw.enable_streaming)
            .unwrap_or(default_enable_streaming());
        let log_level = env_or("LOG_LEVEL", raw.log_level, default_log_level);
        let enable_tools = env_or_bool("OPENAI_ENABLE_TOOLS", raw.enable_tools).unwrap_or(false);
        let enable_local_mcp =
            env_or_bool("OPENAI_ENABLE_LOCAL_MCP", raw.enable_local_mcp).unwrap_or(false);
        let status_timeout_seconds =
            env_or_opt("OPENAI_STATUS_TIMEOUT_SECONDS", raw.status_timeout_seconds);
        let api_base = env_or_opt("OPENAI_API_BASE", raw.api_base);

        if raw.providers.is_empty() {
            return Err(eyre::eyre!(
                "No providers configured in openai_response_config.toml"
            ));
        }
        if raw.models.is_empty() {
            return Err(eyre::eyre!(
                "No models configured in openai_response_config.toml"
            ));
        }

        Ok(Self {
            default_model,
            system_prompt,
            max_history_exchanges,
            enable_streaming,
            log_level,
            enable_tools,
            enable_local_mcp,
            status_timeout_seconds,
            providers: raw.providers,
            models: raw.models,
            api_base,
        })
    }

    pub fn log_level(&self) -> &str {
        &self.log_level
    }

    pub fn enable_streaming(&self) -> bool {
        self.enable_streaming
    }

    pub fn resolve_model(&self) -> eyre::Result<(OpenaiConfig, String)> {
        let (provider_id, model_name) = self
            .route_model(&self.default_model)
            .ok_or_else(|| eyre::eyre!("No route found for model {}", self.default_model))?;

        for provider in &self.providers {
            if provider.id() == provider_id {
                if let ProviderConfig::Openai(cfg) = provider {
                    return Ok((cfg.clone(), model_name));
                } else {
                    return Err(eyre::eyre!(
                        "Model {} routed to unsupported provider kind {}",
                        self.default_model,
                        provider.kind()
                    ));
                }
            }
        }

        Err(eyre::eyre!(
            "Provider {} referenced by model {} not found",
            provider_id,
            self.default_model
        ))
    }

    pub fn route_model(&self, model_id: &str) -> Option<(String, String)> {
        self.models.iter().find(|m| m.id == model_id).map(|m| {
            let provider = m.route.provider.clone();
            let model = m.route.model.clone().unwrap_or_else(|| m.id.clone());
            (provider, model)
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct RawConfig {
    default_model: Option<String>,
    system_prompt: Option<String>,
    #[serde(default, alias = "max_history_messages")]
    max_history_exchanges: Option<usize>,
    #[serde(default)]
    enable_streaming: Option<bool>,
    log_level: Option<String>,
    #[serde(default)]
    enable_tools: Option<bool>,
    #[serde(default)]
    enable_local_mcp: Option<bool>,
    #[serde(default)]
    status_timeout_seconds: Option<u64>,
    #[serde(default)]
    api_base: Option<String>,
    #[serde(default)]
    providers: Vec<ProviderConfig>,
    #[serde(default)]
    models: Vec<ModelConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderConfig {
    Openai(OpenaiConfig),
    Gemini(GeminiConfig),
    Alicloud(AlicloudConfig),
}

impl ProviderConfig {
    pub fn id(&self) -> &str {
        match self {
            ProviderConfig::Openai(cfg) => &cfg.id,
            ProviderConfig::Gemini(cfg) => &cfg.id,
            ProviderConfig::Alicloud(cfg) => &cfg.id,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            ProviderConfig::Openai(_) => "openai",
            ProviderConfig::Gemini(_) => "gemini",
            ProviderConfig::Alicloud(_) => "alicloud",
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
pub struct OpenaiConfig {
    pub id: String,
    pub api_key: String,
    pub api_url: String,
    #[serde(default)]
    pub proxy: bool,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
pub struct GeminiConfig {
    pub id: String,
    pub api_key: String,
    pub api_url: String,
    #[serde(default)]
    pub proxy: bool,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
pub struct AlicloudConfig {
    pub id: String,
    pub api_key: String,
    pub api_url: String,
    #[serde(default)]
    pub proxy: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub route: ModelRoute,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ModelRoute {
    pub provider: String,
    pub model: Option<String>,
}

fn load_file_config() -> eyre::Result<RawConfig> {
    let config_file = std::env::var("OPENAI_RESPONSE_CONFIG_PATH")
        .unwrap_or_else(|_| "openai_response_config.toml".to_string());
    let config_path = PathBuf::from(&config_file);

    if !config_path.exists() {
        return Ok(RawConfig::default());
    }

    let contents = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file {}", config_path.display()))?;
    let parsed: RawConfig = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse {}", config_path.display()))?;
    Ok(parsed)
}

fn env_or<T, F>(key: &str, file_value: Option<T>, default: F) -> T
where
    T: std::str::FromStr,
    F: Fn() -> T,
{
    if let Ok(raw) = std::env::var(key) {
        raw.parse().ok().unwrap_or_else(default)
    } else if let Some(value) = file_value {
        value
    } else {
        default()
    }
}

fn env_or_opt<T>(key: &str, file_value: Option<T>) -> Option<T>
where
    T: std::str::FromStr,
{
    if let Ok(raw) = std::env::var(key) {
        raw.parse().ok()
    } else {
        file_value
    }
}

fn env_or_bool(key: &str, file_value: Option<bool>) -> Option<bool> {
    if let Ok(raw) = std::env::var(key) {
        parse_bool(&raw)
    } else {
        file_value
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn default_model() -> String {
    "gpt-5".to_string()
}

fn default_max_history() -> usize {
    12
}

fn default_enable_streaming() -> bool {
    true
}

fn default_log_level() -> String {
    "INFO".to_string()
}
