use crate::config::AppData;
use crate::manager;
use crate::scheduler;
use anyhow::Context;
use notify_rust::Notification;
use std::process::ExitCode;
use tao::event_loop::{ControlFlow, EventLoop};
use tray_icon::menu::MenuEvent;
use tray_icon::{
    TrayIconBuilder,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
};

/// Interval between watchdog checks when restarting the scheduler after a crash.
const WATCHDOG_RESTART_DELAY: std::time::Duration = std::time::Duration::from_secs(5);

#[allow(clippy::too_many_lines)]
#[must_use]
pub fn run() -> ExitCode {
    // Single instance check
    let instance = match single_instance::SingleInstance::new("wallp_tray_instance") {
        Ok(i) => i,
        Err(e) => {
            tracing::error!("Failed to create single instance: {e}");
            return ExitCode::FAILURE;
        }
    };
    if !instance.is_single() {
        tracing::info!("Another instance is already running, exiting");
        let _ = Notification::new()
            .summary("Wallp")
            .body("Wallp is already running in the system tray")
            .show();
        return ExitCode::SUCCESS;
    }

    // Spawn a watchdog thread that keeps the scheduler alive.
    // If the scheduler thread panics or exits, the watchdog restarts it.
    std::thread::Builder::new()
        .name("scheduler-watchdog".into())
        .spawn(|| {
            loop {
                tracing::info!("Watchdog: starting scheduler thread");

                let handle = std::thread::Builder::new()
                    .name("scheduler".into())
                    .spawn(|| match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt.block_on(scheduler::start_background_task()),
                        Err(e) => tracing::error!("Failed to create tokio runtime: {e}"),
                    });

                match handle {
                    Ok(h) => {
                        if let Err(e) = h.join() {
                            tracing::error!(
                                "Watchdog: scheduler thread panicked: {e:?}. Restarting in {}s...",
                                WATCHDOG_RESTART_DELAY.as_secs()
                            );
                        } else {
                            // start_background_task runs an infinite loop, so it should
                            // never return normally. If it does, restart anyway.
                            tracing::warn!(
                                "Watchdog: scheduler exited unexpectedly. Restarting in {}s...",
                                WATCHDOG_RESTART_DELAY.as_secs()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Watchdog: failed to spawn scheduler thread: {e}. Retrying in {}s...",
                            WATCHDOG_RESTART_DELAY.as_secs()
                        );
                    }
                }

                std::thread::sleep(WATCHDOG_RESTART_DELAY);
            }
        })
        .ok(); // If watchdog itself can't spawn, we still run the tray (degraded mode)

    // Create Event Loop
    let event_loop = EventLoop::new();

    // Menu Construction
    let tray_menu = Menu::new();

    // Check Autostart Status
    let autostart_enabled = check_autostart_status();

    let item_autostart = CheckMenuItem::new("Run at Startup", autostart_enabled, true, None);
    let item_new = MenuItem::new("New Wallpaper", true, None);
    let item_next = MenuItem::new("Next", true, None);
    let item_prev = MenuItem::new("Previous", true, None);
    let item_info = MenuItem::new("Info", true, None);
    let item_setup = MenuItem::new("Setup", true, None);
    let item_folder = MenuItem::new("Open Folder", true, None);
    let item_config = MenuItem::new("Open Config", true, None);
    let item_quit = MenuItem::new("Quit", true, None);

    if let Err(e) = tray_menu.append_items(&[
        &item_new,
        &item_next,
        &item_prev,
        &item_info,
        &PredefinedMenuItem::separator(),
        &item_folder,
        &item_config,
        &item_setup,
        &PredefinedMenuItem::separator(),
        &item_autostart,
        &PredefinedMenuItem::separator(),
        &item_quit,
    ]) {
        tracing::error!("Failed to create tray menu: {e}");
        return ExitCode::FAILURE;
    }

    // Load Icon
    let icon = match load_icon() {
        Ok(i) => i,
        Err(e) => {
            tracing::error!("Failed to load icon: {e}");
            return ExitCode::FAILURE;
        }
    };

    let _tray_icon = match TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Wallp")
        .with_icon(icon)
        .build()
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to create tray icon: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Event Loop (runs forever until exit)
    let exit_code = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        event_loop.run(move |_event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if let Ok(event) = MenuEvent::receiver().try_recv() {
                    if event.id == item_quit.id() {
                        tracing::info!("Quit requested, exiting");
                        *control_flow = ControlFlow::Exit;
                    } else if event.id == item_next.id() {
                        spawn_oneshot(manager::next);
                    } else if event.id == item_prev.id() {
                        spawn_oneshot(manager::prev);
                    } else if event.id == item_new.id() {
                        spawn_oneshot(manager::new);
                    } else if event.id == item_info.id() {
                        if let Ok(exe) = std::env::current_exe() {
                            #[cfg(target_os = "windows")]
                            {
                                let _ = std::process::Command::new("cmd")
                                    .args([
                                        "/c",
                                        "start",
                                        "cmd",
                                        "/k",
                                        &exe.display().to_string(),
                                        "info",
                                    ])
                                    .spawn();
                            }
                            #[cfg(target_os = "linux")]
                            {
                                let _ = std::process::Command::new("x-terminal-emulator")
                                    .args(["-e", &exe.display().to_string(), "info"])
                                    .spawn();
                            }
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("osascript")
                                    .args([
                                        "-e",
                                        &format!(
                                            "tell app \"Terminal\" to do script \"{} info\"",
                                            exe.display()
                                        ),
                                    ])
                                    .spawn();
                            }
                        }
                    } else if event.id == item_setup.id() {
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = std::process::Command::new(exe).arg("setup").spawn();
                        }
                    } else if event.id == item_folder.id() {
                        if let Ok(data_dir) = AppData::get_data_dir() {
                            let _ = open::that(data_dir.join("wallpapers"));
                        } else {
                            tracing::error!("Failed to get data directory");
                        }
                    } else if event.id == item_config.id() {
                        if let Ok(path) = AppData::get_config_path() {
                            let _ = open::that(path);
                        }
                    } else if event.id == item_autostart.id() {
                        let is_enabled = item_autostart.is_checked();
                        let result = std::env::current_exe()
                            .map(|exe| crate::cli::normalize_path_for_registry(&exe))
                            .map_or_else(
                                |_| Err(anyhow::anyhow!("Failed to determine current executable")),
                                |exe_path| crate::cli::setup_autostart(is_enabled, &exe_path),
                            );

                        if let Err(e) = result {
                            tracing::error!("Failed to toggle autostart: {e}");
                            item_autostart.set_checked(!is_enabled);
                            let _ = Notification::new()
                                .summary("Wallp Error")
                                .body(&format!("Failed to toggle autostart: {e}"))
                                .show();
                        }
                    }
                }
            })) {
                tracing::error!("Panic in menu event handler: {e:?}");
            }
        });
    }));

    match exit_code {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("Event loop panicked: {e:?}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(target_os = "macos")]
fn build_auto_launch_for_check(exe_path: &str) -> Option<auto_launch::AutoLaunch> {
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Wallp")
        .set_app_path(exe_path)
        .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
        .build()
        .ok()
}

#[cfg(not(target_os = "macos"))]
fn build_auto_launch_for_check(exe_path: &str) -> Option<auto_launch::AutoLaunch> {
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Wallp")
        .set_app_path(exe_path)
        .build()
        .ok()
}

fn check_autostart_status() -> bool {
    let Ok(current_exe) = std::env::current_exe() else {
        return false;
    };

    let exe_path = crate::cli::normalize_path_for_registry(&current_exe);

    let Some(exe_path) = exe_path.to_str() else {
        return false;
    };

    let Some(auto) = build_auto_launch_for_check(exe_path) else {
        return false;
    };

    auto.is_enabled().unwrap_or(false)
}

fn spawn_oneshot<F, Fut>(f: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
{
    std::thread::spawn(move || {
        // catch_unwind so a panic in a tray action doesn't kill the thread silently
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match tokio::runtime::Runtime::new() {
                Ok(rt) => {
                    if let Err(e) = rt.block_on(f()) {
                        tracing::error!("Tray action error: {e}");
                        let _ = Notification::new()
                            .summary("Wallp Error")
                            .body(&e.to_string())
                            .show();
                    }
                }
                Err(e) => tracing::error!("Failed to create tokio runtime: {e}"),
            }
        }));

        if let Err(e) = result {
            tracing::error!("Tray action panicked: {e:?}");
            let _ = Notification::new()
                .summary("Wallp Error")
                .body("An unexpected error occurred")
                .show();
        }
    });
}

fn load_icon() -> anyhow::Result<tray_icon::Icon> {
    #[cfg(target_os = "windows")]
    let icon_bytes = include_bytes!("../icon.ico");

    #[cfg(not(target_os = "windows"))]
    let icon_bytes = include_bytes!("../icon.png");

    let image = image::load_from_memory(icon_bytes)
        .context("Failed to load embedded icon")?
        .into_rgba8();

    let (width, height) = image.dimensions();
    let icon = tray_icon::Icon::from_rgba(image.into_raw(), width, height)?;
    Ok(icon)
}
