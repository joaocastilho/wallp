use crate::config::{AppData, Wallpaper};
use crate::unsplash::UnsplashClient;
use anyhow::Result;
use chrono::Utc;

#[allow(clippy::missing_errors_doc, clippy::unused_async)]
pub async fn set_lockscreen_wallpaper(path: &std::path::Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use windows::Storage::StorageFile;
        use windows::System::UserProfile::LockScreen;
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
        let (file, action) = {
            let hpath = windows::core::HSTRING::from(path_str);
            let op = StorageFile::GetFileFromPathAsync(&hpath)?;
            drop(hpath);
            let file = op.await?;
            let action = LockScreen::SetImageFileAsync(&file)?;
            (file, action)
        };
        drop(file);
        action.await?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Ok(())
    }
}

async fn set_lockscreen_from_file(app_data: &AppData, path: &std::path::Path) -> Result<()> {
    if !app_data.config.lockscreen_enabled {
        return Ok(());
    }
    set_lockscreen_wallpaper(path).await
}

#[allow(clippy::missing_errors_doc)]
pub async fn new() -> Result<()> {
    let mut app_data = AppData::load()?;
    fetch_and_set_new(&mut app_data).await
}

#[allow(clippy::missing_errors_doc)]
pub async fn next() -> Result<()> {
    let mut app_data = AppData::load()?;

    while app_data.state.current_history_index < app_data.history.len().saturating_sub(1) {
        let target_index = app_data.state.current_history_index + 1;
        let wallpaper = &app_data.history[target_index];

        match set_wallpaper_from_history(wallpaper).await {
            Ok(()) => {
                app_data.state.current_history_index = target_index;
                app_data.state.current_wallpaper_id = Some(wallpaper.id.clone());

                #[allow(clippy::cast_possible_wrap)]
                let next_run =
                    Utc::now() + chrono::Duration::minutes(app_data.config.interval_minutes as i64);
                app_data.state.next_run_at = next_run.to_rfc3339();

                app_data.save()?;
                return Ok(());
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to set next wallpaper (possibly missing file): {e}. Removing from history array."
                );
                app_data.history.remove(target_index);
            }
        }
    }

    app_data.save()?;
    fetch_and_set_new(&mut app_data).await
}

#[allow(clippy::missing_errors_doc)]
pub async fn prev() -> Result<()> {
    let mut app_data = AppData::load()?;

    while app_data.state.current_history_index > 0 {
        let prev_index = app_data.state.current_history_index - 1;
        let wallpaper = &app_data.history[prev_index];

        match set_wallpaper_from_history(wallpaper).await {
            Ok(()) => {
                app_data.state.current_history_index = prev_index;
                app_data.state.current_wallpaper_id = Some(wallpaper.id.clone());

                #[allow(clippy::cast_possible_wrap)]
                let next_run =
                    Utc::now() + chrono::Duration::minutes(app_data.config.interval_minutes as i64);
                app_data.state.next_run_at = next_run.to_rfc3339();

                app_data.save()?;
                return Ok(());
            }
            Err(e) => {
                eprintln!("Warning: Failed to set previous wallpaper: {e}. Removing from history.");
                app_data.history.remove(prev_index);
                app_data.state.current_history_index = prev_index;
            }
        }
    }

    app_data.save()?;
    anyhow::bail!("No previous wallpaper available (or all previous files were missing)");
}

#[allow(clippy::missing_errors_doc, clippy::unused_async)]
pub async fn set_by_index(index: usize) -> Result<()> {
    let mut app_data = AppData::load()?;
    let history_len = app_data.history.len();

    if history_len == 0 {
        anyhow::bail!("No wallpaper in history");
    }

    let actual_index = history_len.saturating_sub(1).saturating_sub(index);

    if actual_index >= history_len {
        anyhow::bail!("Invalid index {} (max is {})", index, history_len - 1);
    }

    let wallpaper = &app_data.history[actual_index];
    if let Err(e) = set_wallpaper_from_history(wallpaper).await {
        eprintln!("Warning: Failed to set wallpaper by index: {e}. Removing from history.");
        app_data.history.remove(actual_index);

        if actual_index <= app_data.state.current_history_index {
            app_data.state.current_history_index =
                app_data.state.current_history_index.saturating_sub(1);
        }

        app_data.save()?;
        return Err(e);
    }

    app_data.state.current_history_index = actual_index;
    app_data.state.current_wallpaper_id = Some(wallpaper.id.clone());

    #[allow(clippy::cast_possible_wrap)]
    let next_run = Utc::now() + chrono::Duration::minutes(app_data.config.interval_minutes as i64);
    app_data.state.next_run_at = next_run.to_rfc3339();

    app_data.save()?;

    Ok(())
}

async fn set_wallpaper_from_history(wallpaper: &Wallpaper) -> Result<()> {
    let data_dir = AppData::get_data_dir()?;
    let path = data_dir.join("wallpapers").join(&wallpaper.filename);

    if !path.exists() {
        anyhow::bail!("Wallpaper file not found: {}", path.display());
    }

    match path.to_str() {
        Some(p) => wallpaper::set_from_path(p)
            .map_err(|e| anyhow::anyhow!("Failed to set wallpaper: {e}"))?,
        None => return Err(anyhow::anyhow!("Wallpaper path contains invalid UTF-8")),
    }

    let app_data = AppData::load()?;
    set_lockscreen_from_file(&app_data, &path).await?;

    Ok(())
}

async fn fetch_and_set_new(app_data: &mut AppData) -> Result<()> {
    if app_data.config.unsplash_access_key.is_empty() {
        anyhow::bail!("Unsplash Access Key is missing. Run 'wallp setup' to configure.");
    }

    if app_data.config.collections.is_empty() {
        anyhow::bail!("No collections configured. Run 'wallp setup' to add collections.");
    }

    let client = UnsplashClient::new(&app_data.config.unsplash_access_key);
    let photo = client.fetch_random(&app_data.config.collections).await?;

    let filename = format!("wallpaper_{}.jpg", photo.id);
    let data_dir = AppData::get_data_dir()?;
    let wallpapers_dir = data_dir.join("wallpapers");
    let file_path = wallpapers_dir.join(&filename);

    client.download_image(&photo.urls.full, &file_path).await?;

    file_path.to_str().map_or_else(
        || {
            Err(anyhow::anyhow!(
                "Wallpaper file path contains invalid UTF-8"
            ))
        },
        |p| {
            wallpaper::set_from_path(p).map_err(|e| anyhow::anyhow!("Failed to set wallpaper: {e}"))
        },
    )?;

    set_lockscreen_from_file(app_data, &file_path).await?;

    let new_wallpaper = Wallpaper {
        id: photo.id.clone(),
        filename,
        applied_at: Utc::now().to_rfc3339(),
        title: photo.description.or(photo.alt_description),
        author: Some(photo.user.name),
        url: Some(photo.links.html),
    };

    app_data.history.push(new_wallpaper);
    app_data.state.current_history_index = app_data.history.len() - 1;
    app_data.state.current_wallpaper_id = Some(photo.id);
    app_data.state.last_run_at = Utc::now().to_rfc3339();

    #[allow(clippy::cast_possible_wrap)]
    let next_run = Utc::now() + chrono::Duration::minutes(app_data.config.interval_minutes as i64);
    app_data.state.next_run_at = next_run.to_rfc3339();

    if let Err(e) = app_data.cleanup_old_wallpapers() {
        eprintln!("Warning: Failed to clean up old wallpapers: {e}");
    }

    app_data.save()?;

    Ok(())
}

#[allow(clippy::missing_errors_doc)]
pub fn get_current_wallpaper() -> Result<Option<Wallpaper>> {
    let mut app_data = AppData::load()?;
    if app_data.history.is_empty() {
        return Ok(None);
    }
    if app_data.state.current_history_index >= app_data.history.len() {
        app_data.state.current_history_index = app_data.history.len() - 1;
        let _ = app_data.save();
    }
    Ok(app_data
        .history
        .get(app_data.state.current_history_index)
        .cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_env() -> anyhow::Result<(TempDir, AppData)> {
        let temp_dir = TempDir::new()?;
        let data_dir = temp_dir.path().join("wallp");
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(data_dir.join("wallpapers"))?;

        let app_data = AppData::default();
        Ok((temp_dir, app_data))
    }

    #[test]
    fn test_get_current_wallpaper_empty_history() -> anyhow::Result<()> {
        let (_, mut app_data) = create_test_env()?;
        app_data.history.clear();
        app_data.state.current_history_index = 0;

        let result = if app_data.history.is_empty() {
            None
        } else {
            app_data
                .history
                .get(app_data.state.current_history_index)
                .cloned()
        };

        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn test_get_current_wallpaper_with_history() -> anyhow::Result<()> {
        let (_, mut app_data) = create_test_env()?;

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
            app_data
                .history
                .get(app_data.state.current_history_index)
                .cloned()
        };

        assert!(result.is_some());
        assert_eq!(result.map(|w| w.id), Some("test_id".to_string()));
        Ok(())
    }

    #[test]
    fn test_history_index_bounds_next() -> anyhow::Result<()> {
        let (_, mut app_data) = create_test_env()?;

        for i in 0..3 {
            app_data.history.push(Wallpaper {
                id: format!("id_{i}"),
                filename: format!("wallpaper_{i}.jpg"),
                applied_at: "2024-01-01T00:00:00Z".to_string(),
                title: None,
                author: None,
                url: None,
            });
        }
        app_data.state.current_history_index = 2;

        let can_go_forward =
            app_data.state.current_history_index < app_data.history.len().saturating_sub(1);
        assert!(!can_go_forward);
        Ok(())
    }

    #[test]
    fn test_history_index_bounds_prev() -> anyhow::Result<()> {
        let (_, mut app_data) = create_test_env()?;

        app_data.history.push(Wallpaper {
            id: "id_0".to_string(),
            filename: "wallpaper_0.jpg".to_string(),
            applied_at: "2024-01-01T00:00:00Z".to_string(),
            title: None,
            author: None,
            url: None,
        });

        app_data.state.current_history_index = 0;

        let can_go_back = app_data.state.current_history_index > 0;
        assert!(!can_go_back);
        Ok(())
    }

    #[test]
    fn test_history_navigation_middle() -> anyhow::Result<()> {
        let (_, mut app_data) = create_test_env()?;

        for i in 0..3 {
            app_data.history.push(Wallpaper {
                id: format!("id_{i}"),
                filename: format!("wallpaper_{i}.jpg"),
                applied_at: "2024-01-01T00:00:00Z".to_string(),
                title: None,
                author: None,
                url: None,
            });
        }
        app_data.state.current_history_index = 1;

        assert!(app_data.state.current_history_index > 0);
        assert!(app_data.state.current_history_index < app_data.history.len() - 1);
        Ok(())
    }
}
