use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum MANAGECommandType {
    #[serde(rename = "door.open")]
    DoorOpen,
    #[serde(rename = "door.close")]
    DoorClose,
    #[serde(rename = "door.stop")]
    DoorStop,
}

#[derive(Serialize, Deserialize)]
pub struct MANAGECommand {
    pub command: MANAGECommandType,
}

#[derive(Serialize, Deserialize)]
pub enum MANAGEReportType {
    #[serde(rename = "heartbeat")]
    Heartbeat,
}

#[derive(Serialize, Deserialize)]
pub struct MANAGEReport{
    pub command: MANAGEReportType,
    pub is_healthy: bool,
}