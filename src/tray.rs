use crate::manager;
use crate::scheduler;
use tao::event_loop::{ControlFlow, EventLoop};
// use tao::platform::windows::EventLoopBuilderExtWindows; // Not needed if standard new works
use tray_icon::{TrayIconBuilder, menu::{Menu, MenuItem, PredefinedMenuItem}};
use tray_icon::menu::MenuEvent;
use crate::config::AppData;

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
    
    let item_new = MenuItem::new("✨ New Wallpaper", true, None);
    let item_next = MenuItem::new("⏭️ Next", true, None);
    let item_prev = MenuItem::new("⏮️ Previous", true, None);
    let item_folder = MenuItem::new("📂 Open Folder", true, None);
    let item_quit = MenuItem::new("❌ Quit", true, None);

    tray_menu.append_items(&[
        &item_new,
        &item_next,
        &item_prev,
        &PredefinedMenuItem::separator(),
        &item_folder,
        &PredefinedMenuItem::separator(),
        &item_quit,
    ])?;

    // Load Icon
    // Ideally use embedded icon or load from file. 
    // For simplicity, we need an RGBA icon buffer. 
    // We will use a dummy icon or load one if available? 
    // `tray-icon` requires `Icon` struct.
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
            }
        }
    });
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
        }
    });
}

fn load_icon() -> anyhow::Result<tray_icon::Icon> {
    // Generate a simple colored square icon programmatically to avoid file dependency for now
    // 32x32 red square
    let width = 32u32;
    let height = 32u32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..width*height {
        rgba.extend_from_slice(&[255, 0, 0, 255]); // Red
    }
    
    let icon = tray_icon::Icon::from_rgba(rgba, width, height)?;
    Ok(icon)
}
