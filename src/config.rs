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
    pub custom_collections: Vec<(String, String)>,
    pub interval_minutes: u64,
    pub retention_days: Option<u64>,
}

impl Config {}

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
                "1065976".to_string(),
                "3330448".to_string(),
                "894".to_string(),
            ],
            custom_collections: Vec::new(),
            interval_minutes: 1440,
            retention_days: Some(7),
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
    /// Get the data directory for wallpapers and other app data
    /// - Linux: ~/.local/share/wallp/
    /// - Windows: %LOCALAPPDATA%\wallp\
    /// - macOS: ~/Library/Application Support/wallp/
    pub fn get_data_dir() -> anyhow::Result<PathBuf> {
        let base_dirs =
            directories::BaseDirs::new().context("Could not determine base directories")?;
        #[cfg(target_os = "windows")]
        {
            Ok(base_dirs.data_local_dir().join("wallp"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(base_dirs.data_dir().join("wallp"))
        }
    }

    /// Get the config directory
    /// - Linux: ~/.config/wallp/
    /// - Windows: %LOCALAPPDATA%\wallp\
    /// - macOS: ~/Library/Application Support/wallp/
    pub fn get_config_dir() -> anyhow::Result<PathBuf> {
        let base_dirs =
            directories::BaseDirs::new().context("Could not determine base directories")?;
        #[cfg(target_os = "linux")]
        {
            Ok(base_dirs.config_dir().join("wallp"))
        }
        #[cfg(target_os = "windows")]
        {
            Ok(base_dirs.data_local_dir().join("wallp"))
        }
        #[cfg(target_os = "macos")]
        {
            Ok(base_dirs.data_dir().join("wallp"))
        }
    }

    /// Get the config file path
    pub fn get_config_path() -> anyhow::Result<PathBuf> {
        Ok(Self::get_config_dir()?.join("wallp.json"))
    }

    /// Get the binary directory (Linux only)
    /// - Linux: ~/.local/bin/
    /// - Windows/macOS: returns error (not applicable)
    #[cfg(target_os = "linux")]
    pub fn get_binary_dir() -> anyhow::Result<PathBuf> {
        let base_dirs =
            directories::BaseDirs::new().context("Could not determine base directories")?;
        base_dirs
            .executable_dir()
            .map(std::path::Path::to_path_buf)
            .context("Could not determine executable directory")
    }

    #[cfg(not(target_os = "linux"))]
    #[allow(dead_code)]
    pub fn get_binary_dir() -> anyhow::Result<PathBuf> {
        anyhow::bail!("Binary directory only applicable on Linux")
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

    /// Clean up old wallpapers that exceed `retention_days`
    pub fn cleanup_old_wallpapers(&mut self) -> anyhow::Result<u32> {
        // None = keep forever, Some(0) = delete immediately on next run
        let retention = match self.config.retention_days {
            Some(0) => {
                // Delete all but the most recent wallpaper
                if self.history.len() > 1 {
                    self.history.drain(0..self.history.len() - 1);
                    return self.cleanup_old_wallpapers();
                }
                return Ok(0);
            }
            None => return Ok(0), // Keep forever
            Some(n) => n,
        };

        #[allow(clippy::cast_possible_wrap)]
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(retention as i64);
        let data_dir = Self::get_data_dir()?;
        let wallpapers_dir = data_dir.join("wallpapers");

        let mut removed_count = 0;
        let mut retained_history: Vec<Wallpaper> = Vec::new();

        for wallpaper in &self.history {
            // Parse the applied_at timestamp
            if let Ok(applied_at) = chrono::DateTime::parse_from_rfc3339(&wallpaper.applied_at)
                && applied_at < cutoff_date
            {
                // This wallpaper is too old, delete the file
                let file_path = wallpapers_dir.join(&wallpaper.filename);
                if file_path.exists() {
                    if let Err(e) = fs::remove_file(&file_path) {
                        eprintln!(
                            "Warning: Failed to delete old wallpaper file {}: {}",
                            wallpaper.filename, e
                        );
                    } else {
                        removed_count += 1;
                    }
                }
                // Skip adding to retained history
                continue;
            }
            // Keep this wallpaper
            retained_history.push(wallpaper.clone());
        }

        // Update history with retained items
        self.history = retained_history;

        // Adjust current_history_index if it's now out of bounds
        if self.state.current_history_index >= self.history.len() {
            self.state.current_history_index = self.history.len().saturating_sub(1);
        }

        Ok(removed_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.unsplash_access_key.is_empty());
        assert_eq!(config.collections.len(), 3);
        assert_eq!(config.interval_minutes, 1440);
        assert_eq!(config.retention_days, Some(7));
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
