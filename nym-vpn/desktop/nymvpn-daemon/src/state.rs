use std::collections::HashMap;

use chrono::Utc;

use nymvpn_types::{
    location::Location,
    notification::{Notification, NotificationType},
    vpn_session::VpnStatus,
};
use uuid::Uuid;

use crate::{
    controller::{SERVER_UNAVAILABLE_PLEASE_TRY_AGAIN_LATER, VPN_SESSION_SERVICE_UNAVAILABLE},
    vpn_session::handler::VpnSessionError,
};

pub struct DaemonState {
    vpn_status: VpnStatus,
    notifications: HashMap<String, Notification>,
}

impl DaemonState {
    pub fn new() -> Self {
        Self {
            vpn_status: VpnStatus::Disconnected,
            notifications: Default::default(),
        }
    }

    pub fn set_vpn_status(&mut self, vpn_status: VpnStatus) {
        self.vpn_status = vpn_status;
    }

    pub fn vpn_status(&self) -> VpnStatus {
        self.vpn_status.clone()
    }

    pub fn update_state_on_disconnect(&mut self) -> VpnStatus {
        let new_status = match &self.vpn_status {
            VpnStatus::Accepted(_)
            | VpnStatus::ServerCreated(_)
            | VpnStatus::ServerRunning(_)
            | VpnStatus::Disconnected => VpnStatus::Disconnected,
            VpnStatus::ServerReady(location)
            | VpnStatus::Connecting(location)
            | VpnStatus::Connected(location, _)
            | VpnStatus::Disconnecting(location) => VpnStatus::Disconnecting(location.clone()),
        };

        self.set_vpn_status(new_status.clone());

        new_status
    }

    pub fn vpn_session_in_progress(&self) -> Option<Location> {
        match &self.vpn_status {
            VpnStatus::Accepted(location)
            | VpnStatus::ServerCreated(location)
            | VpnStatus::ServerRunning(location)
            | VpnStatus::ServerReady(location)
            | VpnStatus::Connecting(location)
            | VpnStatus::Connected(location, _)
            | VpnStatus::Disconnecting(location) => Some(location.clone()),
            VpnStatus::Disconnected => None,
        }
    }

    pub fn add_notification_for_failed_new_session(
        &mut self,
        request_id: Uuid,
        _location: Location,
        error: VpnSessionError,
    ) -> Notification {
        let timestamp = Utc::now();

        // user facing message of notification
        let message = match error {
            VpnSessionError::VpnSessionServiceDown => VPN_SESSION_SERVICE_UNAVAILABLE.to_string(),
            VpnSessionError::Connection(_) => SERVER_UNAVAILABLE_PLEASE_TRY_AGAIN_LATER.to_string(),
            VpnSessionError::Server(status) => status.message().to_string(),
        };

        let notification = Notification {
            id: request_id.to_string(),
            message,
            notification_type: NotificationType::ServerFailed,
            timestamp,
        };

        self.notifications
            .insert(request_id.to_string(), notification.clone());

        notification
    }

    pub fn accepted(&mut self, location: Location) {
        self.vpn_status = VpnStatus::Accepted(location)
    }

    pub fn add_notification(&mut self, notification: Notification) {
        self.notifications
            .insert(notification.id.clone(), notification);
    }

    pub fn remove_notification(&mut self, id: String) {
        self.notifications.remove(&id);
    }

    pub fn notifications(&self) -> Vec<Notification> {
        let mut res: Vec<Notification> = self.notifications.values().map(|r| r.clone()).collect();
        res.sort_by(|n1, n2| n2.timestamp.cmp(&n1.timestamp));
        res
    }
}
