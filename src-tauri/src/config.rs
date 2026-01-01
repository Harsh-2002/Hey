use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::transcription::TranscriptionProvider;

/// Recording mode determines how the global shortcut works
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RecordingMode {
    #[default]
    Toggle, // Press to start, press again to stop
    Hold, // Hold to record, release to stop (push-to-talk)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // API keys are now stored in macOS Keychain, not here
    pub active_provider: TranscriptionProvider,
    pub shortcut: String,
    pub auto_paste: bool,
    pub format_with_ai: bool,
    #[serde(default = "default_openai_model")]
    pub openai_formatting_model: String,
    #[serde(default = "default_groq_model")]
    pub groq_formatting_model: String,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
    #[serde(default)]
    pub selected_device: Option<String>,
    #[serde(default = "default_save_history")]
    pub save_history: bool,
    #[serde(default)]
    pub launch_at_login: bool,
    #[serde(default)]
    pub recording_mode: RecordingMode,
}

fn default_save_history() -> bool {
    true
}

fn default_openai_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_groq_model() -> String {
    "llama-3.1-8b-instant".to_string()
}

fn default_system_prompt() -> String {
    "You are a transcript formatter. Clean up the following speech-to-text transcript by fixing punctuation, capitalization, and minor errors. Keep the original meaning and words as much as possible. Output only the cleaned text, nothing else.".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            active_provider: TranscriptionProvider::OpenAI,
            shortcut: "Option+Space".to_string(),
            auto_paste: false,
            format_with_ai: true,
            openai_formatting_model: default_openai_model(),
            groq_formatting_model: default_groq_model(),
            system_prompt: default_system_prompt(),
            selected_device: None,
            save_history: true,
            launch_at_login: false,
            recording_mode: RecordingMode::default(),
        }
    }
}

impl Config {
    pub fn config_path() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let config_dir = home.join(".hey");

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        Ok(config_dir.join("config.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let config: Config = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}
