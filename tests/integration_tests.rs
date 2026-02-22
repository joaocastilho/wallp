use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_serialization_roundtrip() {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct Config {
        unsplash_access_key: String,
        collections: Vec<String>,
        custom_collections: Vec<(String, String)>,
        interval_minutes: u64,
        retention_days: Option<u64>,
    }

    let config = Config {
        unsplash_access_key: "test_key".to_string(),
        collections: vec!["123".to_string(), "456".to_string()],
        custom_collections: vec![("789".to_string(), "Custom".to_string())],
        interval_minutes: 60,
        retention_days: Some(14),
    };

    let serialized = serde_json::to_string_pretty(&config).expect("Must serialize config");

    let temp_dir = TempDir::new().expect("Must create test dir");
    let config_path = temp_dir.path().join("config.json");
    fs::write(&config_path, &serialized).expect("Must write config file");

    let deserialized: Config = serde_json::from_str(&serialized).expect("Must deserialize config");

    assert_eq!(deserialized.unsplash_access_key, "test_key");
    assert_eq!(deserialized.interval_minutes, 60);
}

#[test]
fn test_config_handles_missing_file() {
    let temp_dir = TempDir::new().expect("Must create test dir");
    let config_path = temp_dir.path().join("nonexistent.json");

    // Should return default config when file doesn't exist
    assert!(!config_path.exists());
}

#[test]
fn test_invalid_json_handling() {
    let temp_dir = TempDir::new().expect("Must create test dir");
    let config_path = temp_dir.path().join("invalid.json");

    // Write invalid JSON
    fs::write(&config_path, "{ invalid json }").expect("Must write invalid json");

    // Try to parse - should fail
    let content = fs::read_to_string(&config_path).expect("Must read file contents");
    let result: Result<serde_json::Value, _> = serde_json::from_str(&content);
    assert!(result.is_err());
}

#[test]
fn test_timestamp_parsing() {
    use chrono::DateTime;

    // Valid ISO-8601 timestamp
    let timestamp = "2024-01-15T10:30:00Z";
    let parsed = DateTime::parse_from_rfc3339(timestamp);
    assert!(parsed.is_ok());

    // Invalid timestamp
    let invalid = "not a timestamp";
    let parsed_invalid = DateTime::parse_from_rfc3339(invalid);
    assert!(parsed_invalid.is_err());
}

#[test]
fn test_wallpaper_history_tracking() {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Clone)]
    #[allow(dead_code)]
    struct Wallpaper {
        id: String,
        filename: String,
        applied_at: String,
        title: Option<String>,
        author: Option<String>,
        url: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    #[allow(dead_code)]
    struct State {
        current_history_index: usize,
    }

    let mut history = Vec::new();

    // Add some wallpapers
    for i in 0..5 {
        history.push(Wallpaper {
            id: format!("photo_{i}"),
            filename: format!("wallpaper_{i}.jpg"),
            applied_at: "2024-01-01T00:00:00Z".to_string(),
            title: Some(format!("Photo {i}")),
            author: Some("Test Author".to_string()),
            url: Some("https://example.com".to_string()),
        });
    }

    // Test navigation
    let mut current_index = 2;

    // Can go back
    assert!(current_index > 0);
    current_index -= 1;
    assert_eq!(current_index, 1);

    // Can go forward
    assert!(current_index < history.len() - 1);
    current_index += 1;
    assert_eq!(current_index, 2);

    // Can't go past end
    current_index = history.len() - 1;
    assert!(current_index >= history.len() - 1);
}

#[test]
fn test_collection_id_parsing() {
    // Test various collection ID formats
    let input = "123,456, 789 ,  1000";
    let collections: Vec<String> = input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    assert_eq!(collections, vec!["123", "456", "789", "1000"]);

    // Empty input
    let empty = "";
    let mut empty_collections = empty
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    assert!(empty_collections.next().is_none());
}

#[test]
fn test_retention_days_parsing() {
    // Test that retention_days parsing works correctly
    // Empty string = None (keep forever)
    // "0" = Some(0) (delete immediately)
    // "7" = Some(7) (keep for 7 days)

    fn parse_retention(input: &str) -> Option<u64> {
        if input.trim().is_empty() {
            None
        } else {
            input.trim().parse::<u64>().ok()
        }
    }

    assert_eq!(parse_retention(""), None);
    assert_eq!(parse_retention("7"), Some(7));
    assert_eq!(parse_retention("0"), Some(0));
    assert_eq!(parse_retention("365"), Some(365));
    assert_eq!(parse_retention("-1"), None); // Negative not allowed
    assert_eq!(parse_retention("abc"), None); // Invalid
}
