use async_channel::{Receiver, Sender};

use crate::bridge::{FromUi, ToUi};

#[derive(Clone)]
pub struct AppState {
    pub from_ui_tx: Sender<FromUi>,
    pub to_ui_rx: Receiver<ToUi>,
}
