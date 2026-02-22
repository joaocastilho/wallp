#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::tempdir; // Assuming tempfile is used for temp_dir
    use wallp::config::AppData;

    #[test]
    fn test_notification_build() -> anyhow::Result<()> {
        use notify_rust::Notification;
        let temp_dir = tempdir()?; // Added temp_dir initialization
        let icon_path = temp_dir.path().join("icon.ico");
        fs::write(&icon_path, b"dummy icon data")?;

        let app_data = AppData::default();
        let _ = app_data.config.unsplash_access_key; // Keep for demonstration of config access but without unused warning

        let _notification = Notification::new() // Changed to _notification to avoid unused variable warning
            .appname("Wallp")
            .summary("Test")
            .body("Body");
        Ok(())
    }

    #[test]
    fn test_notification_with_app_name() {
        use notify_rust::Notification;
        let _notif = Notification::new()
            .appname("Wallp")
            .summary("Test")
            .body("Body");
    }
}
