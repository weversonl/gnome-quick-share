use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use gtk3::prelude::*;
use libayatana_appindicator::{AppIndicator, AppIndicatorStatus};

const SOCKET_ENV: &str = "GNOMEQS_TRAY_SOCKET";
const STATUS_ENV: &str = "GNOMEQS_TRAY_STATUS";
const LANG_ENV: &str = "GNOMEQS_TRAY_LANG";
const MONO_ENV: &str = "GNOMEQS_TRAY_MONO";
const TRAY_ICON: &str = "io.github.weversonl.GnomeQS-airdrop-symbolic";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Visibility {
    Visible,
    Invisible,
    Temporarily,
}

fn main() -> anyhow::Result<()> {
    gtk3::init()?;
    register_debug_icon_search_path();

    let socket_path = PathBuf::from(env::var(SOCKET_ENV)?);
    let status_path = PathBuf::from(env::var(STATUS_ENV)?);
    let lang = env::var(LANG_ENV).unwrap_or_else(|_| "en".into());
    let _monochrome = env::var(MONO_ENV).map(|v| v == "1").unwrap_or(false);
    let icon_theme_path = icon_theme_root();

    let mut indicator =
        AppIndicator::with_path("io.github.weversonl.GnomeQS", TRAY_ICON, &icon_theme_path);
    indicator.set_title("GnomeQS");
    indicator.set_status(AppIndicatorStatus::Active);
    indicator.set_icon_theme_path(&icon_theme_path);
    indicator.set_icon(TRAY_ICON);

    let (show_label, quit_label, visible_label, hidden_label, temporary_label) =
        tray_labels(&lang);

    let mut menu = gtk3::Menu::new();

    let show_item = gtk3::MenuItem::with_label(show_label);
    {
        let socket_path = socket_path.clone();
        show_item.connect_activate(move |_| {
            let _ = send_command(&socket_path, "show");
        });
    }
    menu.append(&show_item);

    let initial_visibility = read_visibility_status(&status_path).unwrap_or(Visibility::Visible);
    let visibility_item = gtk3::MenuItem::with_label(&visibility_menu_label(
        initial_visibility,
        visible_label,
        hidden_label,
        temporary_label,
    ));
    {
        let socket_path = socket_path.clone();
        visibility_item.connect_activate(move |_| {
            let _ = send_command(&socket_path, "toggle_visibility");
        });
    }
    menu.append(&visibility_item);

    menu.append(&gtk3::SeparatorMenuItem::new());

    let quit_item = gtk3::MenuItem::with_label(quit_label);
    {
        let socket_path = socket_path.clone();
        quit_item.connect_activate(move |_| {
            let _ = send_command(&socket_path, "quit");
        });
    }
    menu.append(&quit_item);

    menu.show_all();
    indicator.set_menu(&mut menu);

    glib::timeout_add_local(Duration::from_millis(250), move || {
        if !status_path.exists() {
            indicator.set_status(AppIndicatorStatus::Passive);
            gtk3::main_quit();
            return glib::ControlFlow::Break;
        }

        if let Some(vis) = read_visibility_status(&status_path) {
            visibility_item.set_label(&visibility_menu_label(
                vis,
                visible_label,
                hidden_label,
                temporary_label,
            ));
        }

        glib::ControlFlow::Continue
    });

    gtk3::main();
    Ok(())
}

fn register_debug_icon_search_path() {
    #[cfg(debug_assertions)]
    if let Some(icon_theme) = gtk3::IconTheme::default() {
        icon_theme.append_search_path(icon_theme_root());
    }
}

fn icon_theme_root() -> String {
    #[cfg(debug_assertions)]
    {
        format!("{}/../gtk/data/icons", env!("CARGO_MANIFEST_DIR"))
    }
    #[cfg(not(debug_assertions))]
    {
        "/usr/share/icons".to_string()
    }
}

fn send_command(socket_path: &std::path::Path, cmd: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(cmd.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(())
}

fn read_visibility_status(path: &std::path::Path) -> Option<Visibility> {
    let raw = fs::read_to_string(path).ok()?;
    match raw.trim() {
        "0" => Some(Visibility::Visible),
        "1" => Some(Visibility::Invisible),
        "2" => Some(Visibility::Temporarily),
        _ => None,
    }
}

fn tray_labels(
    lang: &str,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    if lang == "pt_BR" {
        (
            "Exibir",
            "Sair",
            "Visibilidade: Visivel",
            "Visibilidade: Oculto",
            "Visibilidade: Temporario",
        )
    } else {
        (
            "Show",
            "Quit",
            "Visibility: Visible",
            "Visibility: Hidden",
            "Visibility: Temporary",
        )
    }
}

fn visibility_menu_label(
    vis: Visibility,
    visible_label: &str,
    hidden_label: &str,
    temporary_label: &str,
) -> String {
    match vis {
        Visibility::Visible => format!("{visible_label} ✓"),
        Visibility::Invisible => hidden_label.to_string(),
        Visibility::Temporarily => temporary_label.to_string(),
    }
}
