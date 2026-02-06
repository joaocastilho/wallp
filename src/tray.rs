use crate::manager;
use crate::scheduler;
use tao::event_loop::{ControlFlow, EventLoop};
// use tao::platform::windows::EventLoopBuilderExtWindows; // Not needed if standard new works
use tray_icon::{TrayIconBuilder, menu::{Menu, MenuItem, CheckMenuItem, PredefinedMenuItem}};
use tray_icon::menu::MenuEvent;
use crate::config::AppData;
use anyhow::Context;
use notify_rust::Notification;

pub fn run() -> anyhow::Result<()> {
    // Single instance check
    let instance = single_instance::SingleInstance::new("wallp_tray_instance")?;
    if !instance.is_single() {
        return Ok(()); // Silently exit if already running
    }

    // Spawn Tokio Runtime for async tasks
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(scheduler::start_background_task());
    });

    // Create Event Loop
    let event_loop = EventLoop::new();

    // Menu Construction
    let tray_menu = Menu::new();
    
    // Check Autostart Status
    let autostart_enabled = check_autostart_status();

    let item_autostart = CheckMenuItem::new("Run at Startup", autostart_enabled, true, None);
    let item_new = MenuItem::new("✨ New Wallpaper", true, None);
    let item_next = MenuItem::new("⏭️ Next", true, None);
    let item_prev = MenuItem::new("⏮️ Previous", true, None);
    let item_folder = MenuItem::new("📂 Open Folder", true, None);
    let item_config = MenuItem::new("⚙️ Open Config", true, None);
    let item_quit = MenuItem::new("❌ Quit", true, None);

    tray_menu.append_items(&[
        &item_new,
        &item_next,
        &item_prev,
        &PredefinedMenuItem::separator(),
        &item_folder,
        &item_config,
        &PredefinedMenuItem::separator(),
        &item_autostart,
        &PredefinedMenuItem::separator(),
        &item_quit,
    ])?;

    // Load Icon
    let icon = load_icon()?;

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Wallp")
        .with_icon(icon)
        .build()?;

    // Event Loop
    event_loop.run(move |_event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == item_quit.id() {
                *control_flow = ControlFlow::Exit;
            } else if event.id == item_next.id() {
               spawn_oneshot(|| manager::next());
            } else if event.id == item_prev.id() {
                spawn_oneshot(|| manager::prev());
            } else if event.id == item_new.id() {
                spawn_oneshot(|| manager::new());
            } else if event.id == item_folder.id() {
                let _ = open::that(AppData::get_data_dir().unwrap().join("wallpapers"));
            } else if event.id == item_config.id() {
                 if let Ok(path) = AppData::get_config_path() {
                     let _ = open::that(path);
                 }
            } else if event.id == item_autostart.id() {
                let is_enabled = item_autostart.is_checked();
                // Get current exe for autostart path
                let result = if let Ok(exe_path) = std::env::current_exe() {
                    crate::cli::setup_autostart(is_enabled, &exe_path)
                } else {
                    Err(anyhow::anyhow!("Failed to determine current executable"))
                };

                if let Err(e) = result {
                    tracing::error!("Failed to toggle autostart: {}", e);
                    // Revert check state if failed
                    item_autostart.set_checked(!is_enabled);
                     let _ = Notification::new()
                        .summary("Wallp Error")
                        .body(&format!("Failed to toggle autostart: {}", e))
                        .show();
                }
            }
        }
    });
}

fn check_autostart_status() -> bool {
    if let Ok(current_exe) = std::env::current_exe() {
       let auto = auto_launch::AutoLaunchBuilder::new()
            .set_app_name("Wallp")
            .set_app_path(current_exe.to_str().unwrap())
            .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
            .build();
        
        if let Ok(a) = auto {
            return a.is_enabled().unwrap_or(false);
        }
    }
    false
}


fn spawn_oneshot<F, Fut>(f: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
{
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        if let Err(e) = rt.block_on(f()) {
            tracing::error!("Tray action error: {}", e);
            let _ = Notification::new()
                .summary("Wallp Error")
                .body(&e.to_string())
                .show();
        }
    });
}

fn load_icon() -> anyhow::Result<tray_icon::Icon> {
    let icon_bytes = include_bytes!("../icon.ico");
    let image = image::load_from_memory(icon_bytes)
        .context("Failed to load embedded icon")?
        .into_rgba8();
    
    let (width, height) = image.dimensions();
    let icon = tray_icon::Icon::from_rgba(image.into_raw(), width, height)?;
    Ok(icon)
}

