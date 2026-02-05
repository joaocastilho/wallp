use crate::config::AppData;
use crate::manager;
use std::time::Duration;
use chrono::Utc;

pub async fn start_background_task() {
    let mut interval = tokio::time::interval(Duration::from_secs(60)); // Check every minute

    loop {
        interval.tick().await;

        if let Err(e) = check_and_run().await {
            tracing::error!("Scheduler error: {}", e);
        }
    }
}

async fn check_and_run() -> anyhow::Result<()> {
    let app_data = AppData::load()?;
    
    // Parse next_run_at
    let next_run = chrono::DateTime::parse_from_rfc3339(&app_data.state.next_run_at)?;
    
    if Utc::now() >= next_run {
        tracing::info!("Scheduled run triggered.");
        manager::next().await?;
    }
    
    Ok(())
}
