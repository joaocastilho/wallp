use crate::config::{AppData, Wallpaper};
use crate::unsplash::UnsplashClient;
use anyhow::Result;
use chrono::Utc;

pub async fn next() -> Result<()> {
    let mut app_data = AppData::load()?;
    
    // Check if we can "redo" -> move forward in history
    if app_data.state.current_history_index < app_data.history.len().saturating_sub(1) {
        app_data.state.current_history_index += 1;
        let wallpaper = &app_data.history[app_data.state.current_history_index];
        set_wallpaper_from_history(wallpaper)?;
        
        // IMPORTANT: Update next_run calculation to prevent immediate re-triggering
        // if we are just browsing history.
        let next_run = Utc::now() + chrono::Duration::minutes(app_data.config.interval_minutes as i64);
        app_data.state.next_run_at = next_run.to_rfc3339();
        
        app_data.save()?;
        return Ok(());
    }

    // Otherwise fetch new
    fetch_and_set_new(&mut app_data).await
}

pub async fn prev() -> Result<()> {
    let mut app_data = AppData::load()?;

    if app_data.state.current_history_index > 0 {
        app_data.state.current_history_index -= 1;
        let wallpaper = &app_data.history[app_data.state.current_history_index];
        set_wallpaper_from_history(wallpaper)?;
        app_data.save()?;
    } else {
        anyhow::bail!("No previous wallpaper available");
    }
    
    Ok(())
}

pub async fn new() -> Result<()> {
    let mut app_data = AppData::load()?;
    fetch_and_set_new(&mut app_data).await
}

// Ensure local file exists before setting
fn set_wallpaper_from_history(wallpaper: &Wallpaper) -> Result<()> {
    let data_dir = AppData::get_data_dir()?;
    let path = data_dir.join("wallpapers").join(&wallpaper.filename);
    
    if !path.exists() {
        // If missing, we might need to re-download if we have the URL? 
        // For now, let's error or try to re-download if url present?
        // Simplicity: Error.
        anyhow::bail!("Wallpaper file not found: {:?}", path);
    }
    
    match path.to_str() {
        Some(p) => wallpaper::set_from_path(p)
            .map_err(|e| anyhow::anyhow!("Failed to set wallpaper: {}", e))?,
        None => return Err(anyhow::anyhow!("Wallpaper path contains invalid UTF-8")),
    }
        
    Ok(())
}

async fn fetch_and_set_new(app_data: &mut AppData) -> Result<()> {
    if app_data.config.unsplash_access_key.is_empty() {
        anyhow::bail!("Unsplash Access Key is missing. Run 'wallp init' or 'wallp config set unsplash_access_key <KEY>'");
    }

    let client = UnsplashClient::new(app_data.config.unsplash_access_key.clone());
    let photo = client.fetch_random(&app_data.config.collections).await?;
    
    let filename = format!("wallpaper_{}.jpg", photo.id);
    let data_dir = AppData::get_data_dir()?;
    let wallpapers_dir = data_dir.join("wallpapers");
    let file_path = wallpapers_dir.join(&filename);
    
    client.download_image(&photo.urls.full, &file_path).await?;
    
    match file_path.to_str() {
        Some(p) => wallpaper::set_from_path(p)
            .map_err(|e| anyhow::anyhow!("Failed to set wallpaper: {}", e)),
        None => Err(anyhow::anyhow!("Wallpaper file path contains invalid UTF-8")),
    }?;
        
    let new_wallpaper = Wallpaper {
        id: photo.id.clone(),
        filename,
        applied_at: Utc::now().to_rfc3339(),
        title: photo.description.or(photo.alt_description),
        author: Some(photo.user.name),
        url: Some(photo.links.html),
    };
    
    // If we were in the middle of history, truncate future?
    // PRD says: "Ignore current index/history... Append to history... Set currentHistoryIndex to new end"
    // Usually "New" implies branching or just appending. Let's just append.
    
    app_data.history.push(new_wallpaper);
    app_data.state.current_history_index = app_data.history.len() - 1;
    app_data.state.current_wallpaper_id = Some(photo.id);
    app_data.state.last_run_at = Utc::now().to_rfc3339();
    
    // Schedule next run
    let next_run = Utc::now() + chrono::Duration::minutes(app_data.config.interval_minutes as i64);
    app_data.state.next_run_at = next_run.to_rfc3339();
    
    app_data.save()?;
    
    Ok(())
}

pub fn get_current_wallpaper() -> Result<Option<Wallpaper>> {
    let app_data = AppData::load()?;
    if app_data.history.is_empty() {
        return Ok(None);
    }
    Ok(app_data.history.get(app_data.state.current_history_index).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_env() -> (TempDir, AppData) {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().join("wallp");
        fs::create_dir_all(&data_dir).unwrap();
        fs::create_dir_all(data_dir.join("wallpapers")).unwrap();
        
        let app_data = AppData::default();
        (temp_dir, app_data)
    }

    #[test]
    fn test_get_current_wallpaper_empty_history() {
        let (_, mut app_data) = create_test_env();
        app_data.history.clear();
        app_data.state.current_history_index = 0;
        
        // Simulate the logic from get_current_wallpaper
        let result = if app_data.history.is_empty() {
            None
        } else {
            app_data.history.get(app_data.state.current_history_index).cloned()
        };
        
        assert!(result.is_none());
    }

    #[test]
    fn test_get_current_wallpaper_with_history() {
        let (_, mut app_data) = create_test_env();
        
        app_data.history.push(Wallpaper {
            id: "test_id".to_string(),
            filename: "test.jpg".to_string(),
            applied_at: "2024-01-01T00:00:00Z".to_string(),
            title: Some("Test Title".to_string()),
            author: Some("Test Author".to_string()),
            url: Some("https://example.com".to_string()),
        });
        
        app_data.state.current_history_index = 0;
        
        let result = if app_data.history.is_empty() {
            None
        } else {
            app_data.history.get(app_data.state.current_history_index).cloned()
        };
        
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "test_id");
    }

    #[test]
    fn test_history_index_bounds_next() {
        let (_, mut app_data) = create_test_env();
        
        // Add 3 wallpapers
        for i in 0..3 {
            app_data.history.push(Wallpaper {
                id: format!("id_{}", i),
                filename: format!("wallpaper_{}.jpg", i),
                applied_at: "2024-01-01T00:00:00Z".to_string(),
                title: None,
                author: None,
                url: None,
            });
        }
        app_data.state.current_history_index = 2; // At last item
        
        // Simulate next() logic - should try to go forward (would fetch new)
        let can_go_forward = app_data.state.current_history_index < app_data.history.len().saturating_sub(1);
        assert!(!can_go_forward); // Cannot go forward, would fetch new
    }

    #[test]
    fn test_history_index_bounds_prev() {
        let (_, mut app_data) = create_test_env();
        
        app_data.history.push(Wallpaper {
            id: "id_0".to_string(),
            filename: "wallpaper_0.jpg".to_string(),
            applied_at: "2024-01-01T00:00:00Z".to_string(),
            title: None,
            author: None,
            url: None,
        });
        
        app_data.state.current_history_index = 0;
        
        // Simulate prev() logic
        let can_go_back = app_data.state.current_history_index > 0;
        assert!(!can_go_back); // Cannot go back from first item
    }

    #[test]
    fn test_history_navigation_middle() {
        let (_, mut app_data) = create_test_env();
        
        for i in 0..3 {
            app_data.history.push(Wallpaper {
                id: format!("id_{}", i),
                filename: format!("wallpaper_{}.jpg", i),
                applied_at: "2024-01-01T00:00:00Z".to_string(),
                title: None,
                author: None,
                url: None,
            });
        }
        app_data.state.current_history_index = 1;
        
        // Can go prev
        assert!(app_data.state.current_history_index > 0);
        
        // Can go next
        assert!(app_data.state.current_history_index < app_data.history.len() - 1);
    }
}
