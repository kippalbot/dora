//! User preferences storage

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use super::providers::{Provider, ProviderId, get_supported_providers};

/// User preferences for the dashboard
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Preferences {
    pub providers: Vec<Provider>,
    pub default_chat_provider: Option<ProviderId>,
    pub default_tts_provider: Option<ProviderId>,
    pub default_asr_provider: Option<ProviderId>,
    /// Dark mode preference (true = dark, false = light)
    #[serde(default)]
    pub dark_mode: bool,
}

impl Preferences {
    /// Get the preferences file path
    pub fn get_preferences_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".dora").join("dashboard").join("preferences.json")
    }

    /// Load preferences from disk, or create defaults if not found
    pub fn load() -> Self {
        let path = Self::get_preferences_path();

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<Preferences>(&content) {
                        Ok(mut prefs) => {
                            // Merge with supported providers to ensure all are present
                            prefs.merge_with_supported_providers();
                            return prefs;
                        }
                        Err(e) => {
                            eprintln!("Failed to parse preferences: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read preferences file: {}", e);
                }
            }
        }

        // Return defaults with supported providers
        let mut prefs = Self::default();
        prefs.providers = get_supported_providers();
        prefs
    }

    /// Save preferences to disk
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_preferences_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;

        Ok(())
    }

    /// Merge loaded preferences with supported providers
    fn merge_with_supported_providers(&mut self) {
        let supported = get_supported_providers();

        for supported_provider in supported {
            if !self.providers.iter().any(|p| p.id == supported_provider.id) {
                self.providers.push(supported_provider);
            }
        }
    }

    /// Get a provider by ID
    pub fn get_provider(&self, id: &str) -> Option<&Provider> {
        self.providers.iter().find(|p| p.id == id)
    }

    /// Get a mutable provider by ID
    pub fn get_provider_mut(&mut self, id: &str) -> Option<&mut Provider> {
        self.providers.iter_mut().find(|p| p.id == id)
    }

    /// Update or insert a provider
    pub fn upsert_provider(&mut self, provider: Provider) {
        if let Some(existing) = self.providers.iter_mut().find(|p| p.id == provider.id) {
            *existing = provider;
        } else {
            self.providers.push(provider);
        }
    }

    /// Remove a custom provider
    pub fn remove_provider(&mut self, id: &str) -> bool {
        if let Some(pos) = self.providers.iter().position(|p| p.id == id && p.is_custom) {
            self.providers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all enabled providers
    pub fn get_enabled_providers(&self) -> Vec<&Provider> {
        self.providers.iter().filter(|p| p.enabled).collect()
    }
}
