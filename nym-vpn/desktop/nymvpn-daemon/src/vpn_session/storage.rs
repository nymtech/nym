use std::fmt::Display;

use chrono::Utc;
use talpid_core::tunnel_state_machine::TunnelCommand;
use talpid_types::tunnel::{ErrorState, TunnelStateTransition};
use nymvpn_migration::{
    sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set},
    DbErr, Expr,
};
use nymvpn_types::{
    location::Location,
    notification::{Notification, NotificationType},
    nymvpn_server::{Accepted, VpnSessionStatus},
    vpn_session::VpnStatus,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct VpnSessionStorage {
    db: DatabaseConnection,
}

pub struct VpnSessionStatusProcessed {
    pub vpn_status: Option<VpnStatus>,
    pub notification: Option<Notification>,
}

pub struct TunnelTransitionProcessed {
    pub vpn_status: VpnStatus,
    pub tunnel_command: Option<TunnelCommand>,
    pub end_session: Option<String>,
    pub notification: Option<Notification>,
    pub client_connected: Option<SessionInfo>,
}

#[derive(Clone)]
pub struct SessionInfo {
    pub request_id: Uuid,
    pub vpn_session_id: Uuid,
}

pub enum StorageServerStatus {
    Accepted,
    Failed,
    ServerCreated,
    ServerRunning,
    ServerReady,
    ClientConnected,
    Ended,
}

impl From<StorageServerStatus> for String {
    fn from(value: StorageServerStatus) -> Self {
        match value {
            StorageServerStatus::Accepted => "Accepted",
            StorageServerStatus::Failed => "Failed",
            StorageServerStatus::ServerCreated => "ServerCreated",
            StorageServerStatus::ServerRunning => "ServerRunning",
            StorageServerStatus::ServerReady => "ServerReady",
            StorageServerStatus::ClientConnected => "ClientConnected",
            StorageServerStatus::Ended => "Ended",
        }
        .to_owned()
    }
}

impl Display for SessionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SessionInfo request_id: {}, vpn_session_uuid: {}",
            self.request_id, self.vpn_session_id
        )
    }
}

impl VpnSessionStorage {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn new_session(&self, location: Location) -> Result<Uuid, DbErr> {
        let request_id = Uuid::new_v4();

        let vpn_session = nymvpn_entity::vpn_session::ActiveModel {
            request_id: Set(request_id.to_string()),
            location_code: Set(location.code),
            location_city: Set(location.city),
            location_city_code: Set(location.city_code),
            location_country: Set(location.country),
            location_country_code: Set(location.country_code),
            location_state: Set(location.state),
            location_state_code: Set(location.state_code),
            server_status: Set(None),
            session_uuid: Set(None),
            server_ipv4_endpoint: Set(None),
            server_private_ipv4: Set(None),
            server_public_key: Set(None),
            requested_at: Set(Utc::now().to_rfc3339()),
            mark_for_deletion: Set(false),
        };

        let _vpn_session = vpn_session.insert(&self.db).await?;

        Ok(request_id)
    }

    pub async fn end_session(&self) -> Result<Option<SessionInfo>, DbErr> {
        let vpn_session = nymvpn_entity::vpn_session::Entity::find()
            .filter(nymvpn_entity::vpn_session::Column::MarkForDeletion.eq(false))
            .one(&self.db)
            .await?;

        if let Some(vpn_session) = vpn_session {
            // mark for deletion
            let marked_result = nymvpn_entity::vpn_session::Entity::update_many()
                .filter(
                    nymvpn_entity::vpn_session::Column::RequestId.eq(vpn_session.request_id.clone()),
                )
                .col_expr(
                    nymvpn_entity::vpn_session::Column::MarkForDeletion,
                    Expr::value(true),
                )
                .exec(&self.db)
                .await?;

            tracing::info!(
                "marked for deletion: request_id: {} vpn_session_uuid: {:?}. Rows: {}",
                &vpn_session.request_id,
                &vpn_session.session_uuid,
                marked_result.rows_affected
            );

            if let Some(vpn_session_uuid) = vpn_session.session_uuid {
                return Ok(Some(SessionInfo {
                    request_id: Uuid::parse_str(&vpn_session.request_id).unwrap(),
                    vpn_session_id: Uuid::parse_str(&vpn_session_uuid).unwrap(),
                }));
            }
        } else {
            tracing::info!("No session found to mark for deletion");
        }

        Ok(None)
    }

    pub async fn reclaim(&self) -> Result<(), DbErr> {
        let marked = nymvpn_entity::vpn_session::Entity::update_many()
            .col_expr(
                nymvpn_entity::vpn_session::Column::MarkForDeletion,
                Expr::value(true),
            )
            .exec(&self.db)
            .await?;

        tracing::info!(
            "reclaimer marked for deletion count: {}",
            marked.rows_affected
        );

        Ok(())
    }

    pub async fn to_reclaim(&self) -> Result<Vec<SessionInfo>, DbErr> {
        nymvpn_entity::vpn_session::Entity::find()
            .filter(nymvpn_entity::vpn_session::Column::MarkForDeletion.eq(true))
            .all(&self.db)
            .await
            .map(|sessions| {
                sessions
                    .into_iter()
                    .filter(|session| {
                        session.session_uuid.is_some()
                            && Uuid::parse_str(&session.session_uuid.as_ref().unwrap()).is_ok()
                            && Uuid::parse_str(&session.request_id).is_ok()
                    })
                    .map(|session| SessionInfo {
                        request_id: Uuid::parse_str(&session.request_id).unwrap(),
                        vpn_session_id: Uuid::parse_str(&session.session_uuid.unwrap()).unwrap(),
                    })
                    .collect()
            })
    }

    pub async fn update_on_accepted(&self, accepted: Accepted) -> Result<(), DbErr> {
        let _update_result = nymvpn_entity::vpn_session::Entity::update_many()
            .col_expr(
                nymvpn_entity::vpn_session::Column::ServerStatus,
                Expr::value(String::from(StorageServerStatus::Accepted)),
            )
            .col_expr(
                nymvpn_entity::vpn_session::Column::SessionUuid,
                Expr::value(accepted.vpn_session_uuid.to_string()),
            )
            .filter(
                nymvpn_entity::vpn_session::Column::RequestId.eq(accepted.request_id.to_string()),
            )
            .exec(&self.db)
            .await?;
        Ok(())
    }

    pub async fn delete(&self, request_id: Uuid) -> Result<(), DbErr> {
        let delete_result = nymvpn_entity::vpn_session::Entity::delete_many()
            .filter(nymvpn_entity::vpn_session::Column::RequestId.eq(request_id.to_string()))
            .exec(&self.db)
            .await?;
        tracing::info!(
            "deleted rows for {request_id}: {}",
            delete_result.rows_affected
        );
        Ok(())
    }

    fn get_session_info(vpn_session_status: &VpnSessionStatus) -> SessionInfo {
        match vpn_session_status {
            VpnSessionStatus::Accepted(accepted) => SessionInfo {
                request_id: accepted.request_id,
                vpn_session_id: accepted.vpn_session_uuid,
            },
            VpnSessionStatus::Failed(failed) => SessionInfo {
                request_id: failed.request_id,
                vpn_session_id: failed.vpn_session_uuid,
            },
            VpnSessionStatus::ServerCreated(server_created) => SessionInfo {
                request_id: server_created.request_id,
                vpn_session_id: server_created.vpn_session_uuid,
            },
            VpnSessionStatus::ServerRunning(server_running) => SessionInfo {
                request_id: server_running.request_id,
                vpn_session_id: server_running.vpn_session_uuid,
            },
            VpnSessionStatus::ServerReady(server_ready) => SessionInfo {
                request_id: server_ready.request_id,
                vpn_session_id: server_ready.vpn_session_uuid,
            },
            VpnSessionStatus::ClientConnected(client_connected) => SessionInfo {
                request_id: client_connected.request_id,
                vpn_session_id: client_connected.vpn_session_uuid,
            },
            VpnSessionStatus::Ended(ended) => SessionInfo {
                request_id: ended.request_id,
                vpn_session_id: ended.vpn_session_uuid,
            },
        }
    }

    // must be idempotent as same server update can arrive multiple times
    pub async fn updated_server_status(
        &self,
        vpn_session_status: VpnSessionStatus,
    ) -> Result<VpnSessionStatusProcessed, DbErr> {
        tracing::info!("Received updated status from server: {vpn_session_status}");

        let session_info = Self::get_session_info(&vpn_session_status);

        let vpn_session =
            nymvpn_entity::vpn_session::Entity::find_by_id(session_info.request_id.to_string())
                .filter(nymvpn_entity::vpn_session::Column::MarkForDeletion.eq(false))
                .one(&self.db)
                .await?;

        if vpn_session.is_none() {
            tracing::info!("vpn session not found locally");
            tracing::info!("dropping status update from server: {vpn_session_status}");
            Ok(VpnSessionStatusProcessed {
                vpn_status: None,
                notification: None,
            })
        } else {
            let vpn_session = vpn_session.unwrap();
            let location: Location = vpn_session.clone().into();
            let mut vpn_session: nymvpn_entity::vpn_session::ActiveModel = vpn_session.into();

            // update server status and other fields in DB
            let (vpn_status, notification) = match vpn_session_status {
                VpnSessionStatus::Accepted(_) => {
                    // This is initial state client knows so nothing to do here
                    vpn_session.server_status =
                        Set(Some(String::from(StorageServerStatus::Accepted)));
                    vpn_session.update(&self.db).await?;

                    (None, None)
                }
                VpnSessionStatus::Failed(_) => {
                    // server could not be provisioned, create client notification, delete record from DB
                    vpn_session.delete(&self.db).await?;

                    (
                        Some(VpnStatus::Disconnected),
                        Some(Notification {
                            id: format!("failed-{}", session_info.request_id),
                            message: "Server could not be provisioned, please try again later"
                                .into(),
                            notification_type:
                                nymvpn_types::notification::NotificationType::ServerFailed,
                            timestamp: Utc::now(),
                        }),
                    )
                }
                VpnSessionStatus::ServerCreated(_) => {
                    vpn_session.server_status =
                        Set(Some(String::from(StorageServerStatus::ServerCreated)));

                    vpn_session.update(&self.db).await?;

                    (Some(VpnStatus::ServerCreated(location)), None)
                }
                VpnSessionStatus::ServerRunning(_) => {
                    vpn_session.server_status =
                        Set(Some(String::from(StorageServerStatus::ServerRunning)));
                    vpn_session.update(&self.db).await?;

                    (Some(VpnStatus::ServerRunning(location)), None)
                }
                VpnSessionStatus::ServerReady(server_ready) => {
                    vpn_session.server_status =
                        Set(Some(String::from(StorageServerStatus::ServerReady)));
                    vpn_session.server_ipv4_endpoint =
                        Set(Some(server_ready.ipv4_endpoint.to_string()));
                    vpn_session.server_private_ipv4 =
                        Set(Some(server_ready.private_ipv4.to_string()));
                    vpn_session.server_public_key = Set(Some(server_ready.public_key));

                    vpn_session.update(&self.db).await?;

                    (Some(VpnStatus::ServerReady(location)), None)
                }
                VpnSessionStatus::ClientConnected(_) => {
                    vpn_session.server_status =
                        Set(Some(String::from(StorageServerStatus::ClientConnected)));

                    vpn_session.update(&self.db).await?;
                    (None, None)
                }
                VpnSessionStatus::Ended(_) => {
                    vpn_session.delete(&self.db).await?;
                    (Some(VpnStatus::Disconnected), None)
                }
            };

            Ok(VpnSessionStatusProcessed {
                vpn_status,
                notification,
            })
        }
    }

    fn message_from_error(&self, error_state: &ErrorState) -> String {
        format!("{}", error_state.cause())
    }

    // Process tunnel state transition to derive new state
    // and possibly tunnel action in case of client side failures
    pub async fn tunnel_state_transition(
        &self,
        transition: TunnelStateTransition,
        current_state: VpnStatus,
    ) -> Result<TunnelTransitionProcessed, DbErr> {
        // When a tunnel transition is received that means all vpn_session on server side
        // transitioned to successfully state ServerReady. Process tunnel state knowing that
        // if vpn_session is still not marked for delete, its ready.
        let vpn_session = nymvpn_entity::vpn_session::Entity::find()
            .filter(nymvpn_entity::vpn_session::Column::MarkForDeletion.eq(false))
            .one(&self.db)
            .await?;

        let (vpn_status, tunnel_command, end_session, notification, client_connected) =
            match vpn_session {
                Some(vpn_session) => {
                    tracing::info!(
                        "vpn session status {:?} during tunnel transition",
                        vpn_session.server_status
                    );
                    let location: Location = vpn_session.clone().into();
                    match transition {
                        TunnelStateTransition::Disconnected => {
                            (VpnStatus::Disconnected, None, None, None, None)
                        }
                        TunnelStateTransition::Connecting(_) => {
                            (VpnStatus::Connecting(location), None, None, None, None)
                        }
                        TunnelStateTransition::Connected(_) => {
                            let mut vpn_session_updated: nymvpn_entity::vpn_session::ActiveModel =
                                vpn_session.clone().into();
                            vpn_session_updated.server_status =
                                Set(Some(String::from(StorageServerStatus::ClientConnected)));
                            vpn_session_updated.update(&self.db).await?;

                            (
                                VpnStatus::Connected(location, Utc::now()),
                                None,
                                None,
                                None,
                                Some(SessionInfo {
                                    request_id: Uuid::parse_str(&vpn_session.request_id).unwrap(),
                                    vpn_session_id: Uuid::parse_str(
                                        &vpn_session.session_uuid.unwrap(),
                                    )
                                    .unwrap(),
                                }),
                            )
                        }
                        TunnelStateTransition::Disconnecting(_) => {
                            (VpnStatus::Disconnecting(location), None, None, None, None)
                        }
                        TunnelStateTransition::Error(error_state) => {
                            tracing::error!("tunnel errored: {error_state:?}");
                            (
                                VpnStatus::Disconnected,
                                Some(TunnelCommand::Disconnect),
                                Some(self.message_from_error(&error_state)),
                                Some(Notification {
                                    id: format!("ce-{}", &vpn_session.request_id),
                                    message: self.message_from_error(&error_state),
                                    notification_type: NotificationType::ClientFailed,
                                    timestamp: Utc::now(),
                                }),
                                None,
                            )
                        }
                    }
                }
                None => {
                    tracing::info!("No vpn session found during tunnel transition");
                    match transition {
                        TunnelStateTransition::Disconnected => {
                            (VpnStatus::Disconnected, None, None, None, None)
                        }
                        TunnelStateTransition::Connecting(_) => {
                            tracing::warn!(
                                "dropping connecting state transition as no vpn session found"
                            );
                            (VpnStatus::Disconnected, None, None, None, None)
                        }
                        TunnelStateTransition::Connected(_) => {
                            tracing::warn!(
                                "dropping connected state transition as no vpn session found"
                            );
                            (VpnStatus::Disconnected, None, None, None, None)
                        }
                        TunnelStateTransition::Disconnecting(_) => {
                            if let VpnStatus::Disconnecting(location) = current_state {
                                (VpnStatus::Disconnecting(location), None, None, None, None)
                            } else {
                                panic!("No vpn session found; current state is {current_state} and tunnel transitioned to disconnecting");
                            }
                        }
                        TunnelStateTransition::Error(error_state) => {
                            tracing::error!("tunnel errored: {error_state:?}");
                            (
                                VpnStatus::Disconnected,
                                Some(TunnelCommand::Disconnect),
                                Some(self.message_from_error(&error_state)),
                                Some(Notification {
                                    id: "unknown".into(),
                                    message: self.message_from_error(&error_state),
                                    notification_type: NotificationType::ClientFailed,
                                    timestamp: Utc::now(),
                                }),
                                None,
                            )
                        }
                    }
                }
            };

        Ok(TunnelTransitionProcessed {
            vpn_status,
            tunnel_command,
            end_session,
            notification,
            client_connected,
        })
    }
}
