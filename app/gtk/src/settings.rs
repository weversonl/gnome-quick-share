use std::cell::RefCell;

use gio::prelude::SettingsExt;

use crate::config::APP_ID;
#[cfg(debug_assertions)]
use crate::config::SCHEMA_DIR;

pub const DEFAULT_WINDOW_WIDTH: i32 = 400;
pub const DEFAULT_WINDOW_HEIGHT: i32 = 650;

thread_local! {
    static SETTINGS: RefCell<Option<gio::Settings>> = RefCell::new(None);
}

pub fn settings() -> gio::Settings {
    SETTINGS.with(|cell| {
        if let Some(s) = cell.borrow().as_ref() {
            return s.clone();
        }

        #[cfg(debug_assertions)]
        {
            unsafe { std::env::set_var("GSETTINGS_SCHEMA_DIR", SCHEMA_DIR) };
        }
        let s = gio::Settings::new(APP_ID);
        *cell.borrow_mut() = Some(s.clone());
        s
    })
}

pub fn get_autostart() -> bool {
    settings().boolean("autostart")
}

pub fn get_keep_running_on_close() -> bool {
    settings().boolean("keep-running-on-close")
}

pub fn get_start_minimized() -> bool {
    settings().boolean("start-minimized")
}

pub fn get_remember_window_size() -> bool {
    settings().boolean("remember-window-size")
}

pub fn get_visibility_raw() -> i32 {
    settings().int("visibility")
}

pub fn get_wifi_direct_enabled() -> bool {
    settings().boolean("wifi-direct-enabled")
}

pub fn set_visibility_raw(v: i32) {
    let _ = settings().set_int("visibility", v);
}

pub fn get_port() -> Option<u32> {
    let p = settings().int("port") as u32;
    if p < 1024 { None } else { Some(p) }
}

pub fn get_download_folder() -> Option<std::path::PathBuf> {
    let s = settings().string("download-folder");
    if s.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(s.as_str()))
    }
}

pub fn get_language() -> String {
    settings().string("language").to_string()
}

pub fn get_font_size() -> i32 {
    settings().int("font-size")
}

pub fn get_history_retention_days() -> i32 {
    settings().int("history-retention-days").clamp(1, 365)
}

pub fn get_history_max_items() -> i32 {
    settings().int("history-max-items").clamp(1, 500)
}

pub fn get_save_transfer_history() -> bool {
    settings().boolean("save-transfer-history")
}

pub fn font_size_css_px() -> i32 {
    match get_font_size() {
        0 => 13,
        2 => 17,
        3 => 19,
        _ => 15,
    }
}

const AUTOSTART_CONTENT: &str = "[Desktop Entry]\n\
Type=Application\n\
Name=GnomeQS\n\
Exec=gnomeqs\n\
Icon=io.github.weversonl.GnomeQuickShare\n\
Hidden=false\n\
X-GNOME-Autostart-enabled=true\n";

pub fn set_autostart(enable: bool) -> anyhow::Result<()> {
    let xdg = xdg::BaseDirectories::new()?;
    let path = xdg.place_config_file("autostart/io.github.weversonl.GnomeQuickShare.desktop")?;
    if enable {
        std::fs::write(&path, AUTOSTART_CONTENT)?;
        log::debug!("Autostart enabled: {}", path.display());
    } else if path.exists() {
        std::fs::remove_file(&path)?;
        log::debug!("Autostart disabled");
    }
    Ok(())
}

pub fn apply_color_scheme() {
    let scheme = settings().string("color-scheme");
    let cs = match scheme.as_str() {
        "light" => libadwaita::ColorScheme::ForceLight,
        "dark" => libadwaita::ColorScheme::ForceDark,
        _ => libadwaita::ColorScheme::Default,
    };
    libadwaita::StyleManager::default().set_color_scheme(cs);
}

pub fn save_window_state(width: i32, height: i32, maximized: bool) {
    if !get_remember_window_size() {
        return;
    }

    save_window_state_unchecked(width, height, maximized);
}

pub fn save_window_state_unchecked(width: i32, height: i32, maximized: bool) {
    let s = settings();
    let _ = s.set_int("window-width", width);
    let _ = s.set_int("window-height", height);
    let _ = s.set_boolean("window-maximized", maximized);
}

pub fn window_state() -> (i32, i32, bool) {
    if !get_remember_window_size() {
        return (DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT, false);
    }

    let s = settings();
    (
        s.int("window-width"),
        s.int("window-height"),
        s.boolean("window-maximized"),
    )
}

pub fn reset_window_state() {
    save_window_state_unchecked(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT, false);
}
