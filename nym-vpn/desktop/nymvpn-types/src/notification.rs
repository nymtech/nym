use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum NotificationType {
    ServerFailed,
    ClientFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Notification {
    pub id: String,
    pub message: String,
    pub notification_type: NotificationType,
    pub timestamp: DateTime<Utc>,
}

// for protobuf
impl TryFrom<i32> for NotificationType {
    type Error = String;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => NotificationType::ServerFailed,
            1 => NotificationType::ClientFailed,
            val => Err(format!("invalid notification type value: {val}"))?,
        })
    }
}

impl From<NotificationType> for i32 {
    fn from(value: NotificationType) -> Self {
        match value {
            NotificationType::ServerFailed => 0,
            NotificationType::ClientFailed => 1,
        }
    }
}
