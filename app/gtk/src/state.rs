use async_channel::{Receiver, Sender};

use crate::bridge::{FromUi, ToUi};

/// Shared application state passed to every widget that needs to communicate
/// with the Tokio service layer or receive updates from it.
#[derive(Clone)]
pub struct AppState {
    /// Channel for GTK → Tokio commands.
    pub from_ui_tx: Sender<FromUi>,
    /// Channel for Tokio → GTK updates (consumed in the main window async loop).
    pub to_ui_rx: Receiver<ToUi>,
}
