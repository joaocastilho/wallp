#[cfg(test)]
mod tests {
    #[test]
    fn test_notification_build() {
        use notify_rust::Notification;
        let _notif = Notification::new().summary("Test Title").body("Test Body");
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
