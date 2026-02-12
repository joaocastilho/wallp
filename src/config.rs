use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppData {
    pub config: Config,
    pub state: State,
    #[serde(default)]
    pub history: Vec<Wallpaper>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub unsplash_access_key: String,
    pub collections: Vec<String>,
    pub interval_minutes: u64,
    pub aspect_ratio_tolerance: f64,
    pub retention_days: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    pub is_running: bool,
    pub next_run_at: String, // ISO-8601
    pub last_run_at: String, // ISO-8601
    pub current_wallpaper_id: Option<String>,
    pub current_history_index: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Wallpaper {
    pub id: String,
    pub filename: String,
    pub applied_at: String, // ISO-8601
    pub title: Option<String>,
    pub author: Option<String>,
    pub url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            unsplash_access_key: String::new(),
            collections: vec![
                "1053828".to_string(),
                "3330448".to_string(),
                "327760".to_string(),
                "894".to_string(),
            ],
            interval_minutes: 120,
            aspect_ratio_tolerance: 0.1,
            retention_days: 7,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_running: true,
            next_run_at: chrono::Utc::now().to_rfc3339(),
            last_run_at: chrono::Utc::now().to_rfc3339(),
            current_wallpaper_id: None,
            current_history_index: 0,
        }
    }
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            config: Config::default(),
            state: State::default(),
            history: Vec::new(),
        }
    }
}

impl AppData {
    pub fn get_data_dir() -> anyhow::Result<PathBuf> {
        let base_dirs =
            directories::BaseDirs::new().context("Could not determine base directories")?;
        Ok(base_dirs.config_dir().join("wallp"))
    }

    pub fn get_config_path() -> anyhow::Result<PathBuf> {
        Ok(Self::get_data_dir()?.join("wallp.json"))
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = Self::get_config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path).context("Failed to read wallp.json")?;

        let data: AppData = serde_json::from_str(&content).context("Failed to parse wallp.json")?;

        Ok(data)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::get_config_path()?;
        let dir = path.parent().context("Config path has no parent")?;

        fs::create_dir_all(dir).context("Failed to create config directory")?;

        let content = serde_json::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&path, content).context("Failed to write wallp.json")?;

        Ok(())
    }
}
