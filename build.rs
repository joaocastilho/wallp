fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        if std::path::Path::new("icon.ico").exists() {
            res.set_icon("icon.ico");
        }
        
        // Set metadata for Task Manager / Startup
        res.set("FileDescription", "Wallp - Wallpaper Manager");
        res.set("ProductName", "Wallp");
        res.set("LegalCopyright", "Copyright (c) 2024");
        
        res.compile().unwrap();
    }
}
