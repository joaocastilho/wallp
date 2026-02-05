fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        if std::path::Path::new("icon.ico").exists() {
            res.set_icon("icon.ico");
        }
        
        // Set metadata for Task Manager / Startup
        res.set("FileDescription", "Wallp - Modern Wallpaper Manager for Windows");
        res.set("ProductName", "Wallp");
        res.set("CompanyName", "Wallp Developers");
        res.set("LegalCopyright", "Copyright (c) 2025");
        res.set("InternalName", "wallp.exe");
        res.set("OriginalFilename", "wallp.exe");
        
        res.compile().unwrap();
    }
}
