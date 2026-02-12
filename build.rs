fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        if std::path::Path::new("icon.ico").exists() {
            res.set_icon("icon.ico");
        }

        // Set metadata for Task Manager / Startup
        res.set("FileDescription", "Wallp - Wallpaper changer");
        res.set("ProductName", "Wallp");
        res.set("CompanyName", "Joao Castilho");
        res.set("LegalCopyright", "Copyright (c) 2026 Joao Castilho");
        res.set("InternalName", "wallp.exe");
        res.set("OriginalFilename", "wallp.exe");
        res.set("FileVersion", "1.0.0.0");
        res.set("ProductVersion", "1.0.0.0");
        res.set("AssemblyVersion", "1.0.0.0");

        res.compile().unwrap();
    }
}
