use libosdp::OsdpEventCardRead;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum MANAGECommandType {
    #[serde(rename = "door.open")]
    DoorOpen,
    #[serde(rename = "door.close")]
    DoorClose,
    #[serde(rename = "door.stop")]
    DoorStop,
    #[serde(rename = "door.unlock")]
    DoorUnlock(u32),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MANAGECommand {
    pub command: MANAGECommandType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "command")]
pub enum MANAGEReport {
    #[serde(rename = "heartbeat")]
    Heartbeat { is_healthy: bool },
    #[serde(rename = "osdp.card_read")]
    OsdpCardRead { event: OsdpEventCardRead },
}
