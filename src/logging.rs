use crate::config::AppData;
use tracing_appender::rolling;
use tracing_subscriber::fmt;

/// Initialize file-based logging and a panic hook.
///
/// Logs are written to `{data_dir}/wallp.log` with daily rotation.
/// A panic hook is installed so panics are captured to the log file
/// before the default panic behavior runs.
///
/// # Errors
///
/// Returns an error if the data directory cannot be determined.
pub fn init() -> anyhow::Result<()> {
    let data_dir = AppData::get_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;

    let file_appender = rolling::daily(&data_dir, "wallp.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Leak the guard so it lives for the entire process lifetime.
    // This is intentional — the tray process runs until exit.
    std::mem::forget(_guard);

    fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false)
        .init();

    // Install panic hook that logs panics before running the default hook.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("PANIC: {info}");
        default_hook(info);
    }));

    tracing::info!("wallp started");
    Ok(())
}
