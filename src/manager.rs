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
        set_wallpaper_from_history(&wallpaper, &app_data)?;
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
        set_wallpaper_from_history(&wallpaper, &app_data)?;
        app_data.save()?;
    }
    
    Ok(())
}

pub async fn new() -> Result<()> {
    let mut app_data = AppData::load()?;
    fetch_and_set_new(&mut app_data).await
}

// Ensure local file exists before setting
fn set_wallpaper_from_history(wallpaper: &Wallpaper, _app_data: &AppData) -> Result<()> {
    let data_dir = AppData::get_data_dir()?;
    let path = data_dir.join("wallpapers").join(&wallpaper.filename);
    
    if !path.exists() {
        // If missing, we might need to re-download if we have the URL? 
        // For now, let's error or try to re-download if url present?
        // Simplicity: Error.
        anyhow::bail!("Wallpaper file not found: {:?}", path);
    }
    
    wallpaper::set_from_path(path.to_str().unwrap())
        .map_err(|e| anyhow::anyhow!("Failed to set wallpaper: {}", e))?;
        
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
    
    wallpaper::set_from_path(file_path.to_str().unwrap())
        .map_err(|e| anyhow::anyhow!("Failed to set wallpaper: {}", e))?;
        
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
