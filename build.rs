fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "windows" {
        let mut res = winres::WindowsResource::new();
        if std::path::Path::new("icon.ico").exists() {
            res.set_icon("icon.ico");
        }

        // Set metadata for Task Manager / Startup / Properties
        res.set("FileDescription", "Wallp - Wallpaper changer");
        res.set("ProductName", "Wallp - Wallpaper changer");
        res.set("CompanyName", "Joao Castilho");
        res.set("LegalCopyright", "Copyright (c) 2026 Joao Castilho");
        res.set("InternalName", "wallp.exe");
        res.set("OriginalFilename", "wallp.exe");
        res.set("FileVersion", "1.1.0.0");
        res.set("ProductVersion", "1.1.0.0");
        res.set_language(0x0409); // English (United States)

        // When cross-compiling from Linux to Windows (GNU), 
        // winres might need help finding the resource compiler.
        #[cfg(not(windows))]
        {
            if std::env::var("WINDRES").is_err() {
                if std::process::Command::new("x86_64-w64-mingw32-windres").arg("--version").output().is_ok() {
                    res.set_windres_path("x86_64-w64-mingw32-windres");
                }
            }
        }

        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {e}");
            // Don't exit(1) here to allow the build to proceed even without metadata/icon
            // if the tools are missing in the current environment.
        }
    }

    set_build_timestamp();
}

fn set_build_timestamp() {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Format as human-readable date/time (UTC)
    let datetime = format_timestamp(timestamp);
    println!("cargo:rustc-env=BUILD_DATETIME={datetime}");
}

fn format_timestamp(timestamp: u64) -> String {
    // Manual formatting of Unix timestamp to avoid dependencies
    const SECONDS_PER_DAY: u64 = 86400;

    let days_since_epoch = timestamp / SECONDS_PER_DAY;
    let seconds_of_day = timestamp % SECONDS_PER_DAY;

    // Calculate year, month, day (simplified algorithm)
    let mut year = 1970;
    let mut days_remaining = days_since_epoch;

    // Account for leap years
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days_remaining < days_in_year {
            break;
        }
        days_remaining -= days_in_year;
        year += 1;
    }

    let (month, day) = days_to_month_day(days_remaining, is_leap_year(year));

    // Calculate hours, minutes, seconds
    let hours = seconds_of_day / 3600;
    let minutes = (seconds_of_day % 3600) / 60;
    let seconds = seconds_of_day % 60;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        year,
        month,
        day + 1,
        hours,
        minutes,
        seconds
    )
}

const fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn days_to_month_day(days: u64, is_leap: bool) -> (u64, u64) {
    let month_lengths = if is_leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut days_remaining = days;
    for (month_idx, &days_in_month) in month_lengths.iter().enumerate() {
        if days_remaining < days_in_month {
            return ((month_idx + 1) as u64, days_remaining);
        }
        days_remaining -= days_in_month;
    }
    (12, days_remaining)
}
