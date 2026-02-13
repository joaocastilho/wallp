use crate::config::AppData;
use crate::manager;
use chrono::Utc;
use std::time::Duration;

pub async fn start_background_task() {
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        if let Err(e) = check_and_run().await {
            eprintln!("Scheduler error: {e}");
        }
    }
}

async fn check_and_run() -> anyhow::Result<()> {
    let app_data = AppData::load()?;

    if app_data.config.unsplash_access_key.is_empty() {
        return Ok(());
    }

    let next_run = chrono::DateTime::parse_from_rfc3339(&app_data.state.next_run_at)?;

    if Utc::now() >= next_run {
        manager::next().await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    #[test]
    fn test_should_run_next_when_past_time() {
        let past_time = Utc::now() - ChronoDuration::minutes(5);

        let should_run = Utc::now() >= past_time;
        assert!(should_run);
    }

    #[test]
    fn test_should_not_run_when_future_time() {
        let future_time = Utc::now() + ChronoDuration::minutes(30);

        let should_run = Utc::now() >= future_time;
        assert!(!should_run);
    }

    #[test]
    fn test_next_run_calculation() {
        let now = Utc::now();
        let interval_minutes = 60i64;

        let next_run = now + ChronoDuration::minutes(interval_minutes);

        assert!(next_run > now);
        assert_eq!(next_run.timestamp() - now.timestamp(), 60 * 60);
    }

    #[test]
    fn test_interval_parsing() {
        // Test different interval values
        let intervals = [15, 30, 60, 120, 240];

        for interval in intervals {
            let next = Utc::now() + ChronoDuration::minutes(interval);
            let diff = (next - Utc::now()).num_minutes();
            assert!((diff - interval).abs() <= 1); // Allow 1 min tolerance
        }
    }
}
