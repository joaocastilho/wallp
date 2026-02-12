use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.unsplash_access_key.is_empty());
        assert_eq!(config.collections.len(), 4);
        assert_eq!(config.interval_minutes, 120);
        assert_eq!(config.aspect_ratio_tolerance, 0.1);
        assert_eq!(config.retention_days, 7);
    }

    #[test]
    fn test_state_default() {
        let state = State::default();
        assert!(state.is_running);
        assert!(state.next_run_at.contains('T'));
        assert!(state.last_run_at.contains('T'));
        assert!(state.current_wallpaper_id.is_none());
        assert_eq!(state.current_history_index, 0);
    }

    #[test]
    fn test_app_data_default() {
        let app_data = AppData::default();
        assert_eq!(app_data.history.len(), 0);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config.unsplash_access_key, deserialized.unsplash_access_key);
        assert_eq!(config.collections, deserialized.collections);
    }

    #[test]
    fn test_wallpaper_serialization() {
        let wallpaper = Wallpaper {
            id: "test_id".to_string(),
            filename: "test.jpg".to_string(),
            applied_at: "2024-01-01T00:00:00Z".to_string(),
            title: Some("Test Title".to_string()),
            author: Some("Test Author".to_string()),
            url: Some("https://example.com".to_string()),
        };
        let serialized = serde_json::to_string(&wallpaper).unwrap();
        let deserialized: Wallpaper = serde_json::from_str(&serialized).unwrap();
        assert_eq!(wallpaper.id, deserialized.id);
        assert_eq!(wallpaper.filename, deserialized.filename);
        assert_eq!(wallpaper.title, deserialized.title);
    }

    #[test]
    fn test_app_data_serialization() {
        let app_data = AppData::default();
        let serialized = serde_json::to_string(&app_data).unwrap();
        let deserialized: AppData = serde_json::from_str(&serialized).unwrap();
        assert_eq!(
            app_data.config.unsplash_access_key,
            deserialized.config.unsplash_access_key
        );
    }
}
