use crate::timestamp_to_datetime_utc;

impl From<crate::proto::NotificationType> for nymvpn_types::notification::NotificationType {
    fn from(value: crate::proto::NotificationType) -> Self {
        match value {
            crate::proto::NotificationType::ServerFailed => {
                nymvpn_types::notification::NotificationType::ServerFailed
            }
            crate::proto::NotificationType::ClientFailed => {
                nymvpn_types::notification::NotificationType::ClientFailed
            }
        }
    }
}

impl From<nymvpn_types::notification::NotificationType> for crate::proto::NotificationType {
    fn from(value: nymvpn_types::notification::NotificationType) -> Self {
        match value {
            nymvpn_types::notification::NotificationType::ServerFailed => {
                crate::proto::NotificationType::ServerFailed
            }
            nymvpn_types::notification::NotificationType::ClientFailed => {
                crate::proto::NotificationType::ClientFailed
            }
        }
    }
}

impl TryFrom<crate::proto::Notification> for nymvpn_types::notification::Notification {
    type Error = String;
    fn try_from(value: crate::proto::Notification) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            message: value.message,
            notification_type: value.notification_type.try_into()?,
            timestamp: timestamp_to_datetime_utc(value.timestamp)?,
        })
    }
}

impl From<nymvpn_types::notification::Notification> for crate::proto::Notification {
    fn from(value: nymvpn_types::notification::Notification) -> Self {
        let seconds = value.timestamp.timestamp();
        let nanos = value.timestamp.timestamp_subsec_nanos();
        Self {
            id: value.id,
            notification_type: value.notification_type.into(),
            message: value.message,
            timestamp: Some(prost_types::Timestamp {
                seconds,
                nanos: nanos as i32,
            }),
        }
    }
}

impl From<Vec<nymvpn_types::notification::Notification>> for crate::proto::Notifications {
    fn from(value: Vec<nymvpn_types::notification::Notification>) -> Self {
        Self {
            notification: value
                .into_iter()
                .map(crate::proto::Notification::from)
                .collect(),
        }
    }
}

impl TryFrom<crate::proto::Notifications> for Vec<nymvpn_types::notification::Notification> {
    type Error = String;
    fn try_from(value: crate::proto::Notifications) -> Result<Self, Self::Error> {
        let mut notifications = vec![];
        for notification in value.notification {
            notifications.push(notification.try_into()?)
        }
        Ok(notifications)
    }
}
