use std::env;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use gio::prelude::SettingsExt;
use gnomeqs_core::Visibility;

use crate::bridge::FromUi;
use crate::settings;

pub const SOCKET_ENV: &str = "GNOMEQS_TRAY_SOCKET";
pub const STATUS_ENV: &str = "GNOMEQS_TRAY_STATUS";
pub const LANG_ENV: &str = "GNOMEQS_TRAY_LANG";
pub const MONO_ENV: &str = "GNOMEQS_TRAY_MONO";
pub const TOKEN_ENV: &str = "GNOMEQS_TRAY_TOKEN";

fn gen_tray_token() -> String {
    let mut buf = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        let _ = f.read_exact(&mut buf);
    }
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}

#[derive(Clone)]
pub struct TrayHandle {
    pub socket_path: PathBuf,
    pub status_path: PathBuf,
    pub token: String,
}

impl TrayHandle {
    pub fn set_visibility(&self, visibility: Visibility) {
        if let Err(e) = write_visibility_status(&self.status_path, visibility) {
            log::warn!("tray status write failed: {}", e);
        }
    }

    pub fn shutdown(&self) {
        let _ = send_command(&self.socket_path, &self.token, "shutdown");
        let _ = fs::remove_file(&self.socket_path);
        let _ = fs::remove_file(&self.status_path);
    }
}

fn runtime_dir() -> PathBuf {
    if let Ok(dir) = env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir);
    }
    env::temp_dir()
}

pub fn tray_socket_path() -> PathBuf {
    runtime_dir().join("gnomeqs-tray.sock")
}

pub fn tray_status_path() -> PathBuf {
    runtime_dir().join("gnomeqs-tray.status")
}

pub fn initialize_tray_runtime() -> Option<TrayHandle> {
    if std::env::var_os("FLATPAK_ID").is_some() {
        log::info!("running inside flatpak; tray helper disabled");
        return None;
    }

    let socket_path = tray_socket_path();
    let status_path = tray_status_path();

    let _ = fs::remove_file(&socket_path);

    if let Err(e) = write_visibility_status(
        &status_path,
        Visibility::from_raw_value(settings::get_visibility_raw() as u64),
    ) {
        log::warn!("tray initial status write failed: {}", e);
    }

    let helper_lang = settings::get_language();
    let helper_mono = if settings::settings().boolean("tray-monochrome") {
        "1"
    } else {
        "0"
    };

    let tray_exe = match env::current_exe() {
        Ok(exe) => exe.with_file_name("gnomeqs-tray"),
        Err(e) => {
            log::warn!("failed to resolve current exe for tray helper: {}", e);
            return None;
        }
    };

    let mut spawn = if tray_exe.exists() && cfg!(not(debug_assertions)) {
        Command::new(tray_exe)
    } else {
        let workspace_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut cmd = Command::new("cargo");
        cmd.arg("run")
            .arg("--quiet")
            .arg("-p")
            .arg("gnomeqs-tray")
            .current_dir(workspace_root);
        cmd
    };

    let token = gen_tray_token();

    if let Err(e) = spawn
        .env(SOCKET_ENV, &socket_path)
        .env(STATUS_ENV, &status_path)
        .env(LANG_ENV, helper_lang)
        .env(MONO_ENV, helper_mono)
        .env(TOKEN_ENV, &token)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        log::warn!("failed to spawn tray helper: {}", e);
        return None;
    }

    Some(TrayHandle {
        socket_path,
        status_path,
        token,
    })
}

pub fn handle_ipc_command(raw: &str, expected_token: &str, from_ui_tx: &async_channel::Sender<FromUi>) {
    let raw = raw.trim();
    let cmd = match raw.strip_prefix(expected_token).and_then(|s| s.strip_prefix(':')) {
        Some(c) => c.trim(),
        None => {
            log::warn!("tray ipc: rejected command with invalid token");
            return;
        }
    };

    match cmd {
        "show" => {
            let _ = from_ui_tx.try_send(FromUi::ShowWindow);
        }
        "toggle_visibility" => {
            let current = settings::get_visibility_raw();
            let new_vis = match current {
                0 => Visibility::Invisible,
                _ => Visibility::Visible,
            };
            settings::set_visibility_raw(new_vis as i32);
            let _ = from_ui_tx.try_send(FromUi::ChangeVisibility(new_vis));
        }
        "quit" => {
            let _ = from_ui_tx.try_send(FromUi::Quit);
        }
        _ => {}
    }
}

pub fn send_command(socket_path: &Path, token: &str, cmd: &str) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(token.as_bytes())?;
    stream.write_all(b":")?;
    stream.write_all(cmd.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(())
}

pub fn write_visibility_status(path: &Path, visibility: Visibility) -> std::io::Result<()> {
    fs::write(path, format!("{}", visibility as i32))
}
