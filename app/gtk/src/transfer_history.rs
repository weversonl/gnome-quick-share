use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::settings;

const HISTORY_FILE: &str = "transfer-history.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HistoryDirection {
    Send,
    Receive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub created_at: u64,
    pub direction: HistoryDirection,
    pub title: String,
    pub subtitle: String,
    pub open_target: Option<String>,
}

pub fn load(direction: HistoryDirection) -> Vec<HistoryEntry> {
    prune_and_load()
        .into_iter()
        .filter(|entry| entry.direction == direction)
        .collect()
}

pub fn append(mut entry: HistoryEntry) -> HistoryEntry {
    entry.created_at = unix_now();
    let mut entries = prune_entries(read_all());
    entries.insert(0, entry.clone());
    entries = prune_entries(entries);

    if let Err(e) = write_all(&entries) {
        log::warn!("failed to write transfer history: {e}");
    }

    entry
}

pub fn remove(target: &HistoryEntry) -> anyhow::Result<()> {
    let mut removed = false;
    let entries: Vec<HistoryEntry> = read_all()
        .into_iter()
        .filter(|entry| {
            if !removed && history_entry_matches(entry, target) {
                removed = true;
                false
            } else {
                true
            }
        })
        .collect();
    write_or_remove_all(&entries)
}

pub fn clear_direction(direction: HistoryDirection) -> anyhow::Result<()> {
    let entries: Vec<HistoryEntry> = read_all()
        .into_iter()
        .filter(|entry| entry.direction != direction)
        .collect();
    write_or_remove_all(&entries)
}

pub fn clear() -> anyhow::Result<()> {
    let path = history_path();
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn prune_and_load() -> Vec<HistoryEntry> {
    let entries = prune_entries(read_all());
    if let Err(e) = write_all(&entries) {
        log::warn!("failed to prune transfer history: {e}");
    }
    entries
}

fn prune_entries(entries: Vec<HistoryEntry>) -> Vec<HistoryEntry> {
    let now = unix_now();
    let retention_secs = (settings::get_history_retention_days().max(1) as u64) * 24 * 60 * 60;
    let max_entries = settings::get_history_max_items().max(1) as usize;

    entries
        .into_iter()
        .filter(|entry| now.saturating_sub(entry.created_at) <= retention_secs)
        .take(max_entries)
        .collect()
}

fn read_all() -> Vec<HistoryEntry> {
    let path = history_path();
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_else(|e| {
        log::warn!("failed to parse transfer history: {e}");
        Vec::new()
    })
}

fn write_all(entries: &[HistoryEntry]) -> anyhow::Result<()> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(entries)?)?;
    Ok(())
}

fn write_or_remove_all(entries: &[HistoryEntry]) -> anyhow::Result<()> {
    if entries.is_empty() {
        return clear();
    }

    write_all(entries)
}

fn history_entry_matches(entry: &HistoryEntry, target: &HistoryEntry) -> bool {
    entry.created_at == target.created_at
        && entry.direction == target.direction
        && entry.title == target.title
        && entry.subtitle == target.subtitle
        && entry.open_target == target.open_target
}

fn history_path() -> PathBuf {
    if let Ok(state_home) = std::env::var("XDG_STATE_HOME") {
        return PathBuf::from(state_home)
            .join("gnome-quick-share")
            .join(HISTORY_FILE);
    }

    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("gnome-quick-share")
            .join(HISTORY_FILE);
    }

    std::env::temp_dir()
        .join("gnome-quick-share")
        .join(HISTORY_FILE)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
