use serde::{Deserialize, Serialize};

use crate::hdl::info::TransferMetadata;
use crate::hdl::State;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub enum ChannelDirection {
    #[default]
    FrontToLib,
    LibToFront,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum ChannelAction {
    AcceptTransfer,
    RejectTransfer,
    CancelTransfer,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum TransferType {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ChannelMessage {
    pub id: String,
    pub direction: ChannelDirection,
    pub action: Option<ChannelAction>,
    pub rtype: Option<TransferType>,
    pub state: Option<State>,
    pub meta: Option<TransferMetadata>,
}
