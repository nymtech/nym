use tokio::sync::{mpsc, oneshot};
use nymvpn_controller::auth::Auth;
use nymvpn_migration::sea_orm::DatabaseConnection;
use nymvpn_server::{ServerApi, ServerApiNoAuth};
use nymvpn_types::nymvpn_server::{AddDeviceRequest, UserCredentials};

use crate::{token_storage::TokenStorage, AckTx, ResponseTx};

use super::{storage::DeviceStorage, DeviceError};

#[derive(Debug, Clone)]
pub struct DeviceHandler {
    tx: mpsc::UnboundedSender<DeviceCommand>,
}

impl DeviceHandler {
    pub async fn start(db: DatabaseConnection) -> Result<Self, DeviceError> {
        let (sender, receiver) = mpsc::unbounded_channel();

        let mut device_service = DeviceService::new(db, receiver).await?;

        tokio::spawn(async move { device_service.run().await });

        Ok(Self { tx: sender })
    }

    pub async fn sign_in(&self, user_creds: UserCredentials) -> Result<(), DeviceError> {
        self.send_command(move |tx| DeviceCommand::SignIn(tx, user_creds))
            .await
    }

    pub async fn sign_out(&self) -> Result<(), DeviceError> {
        self.send_command(move |tx| DeviceCommand::SignOut(tx))
            .await
    }

    pub async fn latest_app_version(&self) -> Result<String, DeviceError> {
        self.send_command(move |tx| DeviceCommand::LatestAppVersion(tx))
            .await
    }

    pub async fn shutdown(&self) -> Result<(), DeviceError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(DeviceCommand::Shutdown(tx))
            .map_err(|_| DeviceError::DeviceServiceUnavailable)?;
        rx.await
            .map_err(|_| DeviceError::DeviceServiceUnavailable)?;
        Ok(())
    }

    pub async fn send_command<T>(
        &self,
        make_cmd: impl FnOnce(oneshot::Sender<Result<T, DeviceError>>) -> DeviceCommand,
    ) -> Result<T, DeviceError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(make_cmd(tx))
            .map_err(|_| DeviceError::DeviceServiceUnavailable)?;
        rx.await
            .map_err(|_| DeviceError::DeviceServiceUnavailable)?
    }
}

#[async_trait::async_trait]
impl Auth for DeviceHandler {
    async fn is_authenticated(&self) -> bool {
        let response = self
            .send_command(|tx| DeviceCommand::IsAuthenticated(tx))
            .await
            .map_err(|e| tracing::error!("failed to check if device is authenticated: {e}"))
            .ok();

        response.is_some() && response.unwrap()
    }
}

pub enum DeviceCommand {
    SignIn(ResponseTx<(), DeviceError>, UserCredentials),
    SignOut(ResponseTx<(), DeviceError>),
    BearerToken(ResponseTx<Option<String>, DeviceError>),
    IsAuthenticated(ResponseTx<bool, DeviceError>),
    Shutdown(AckTx),
    LatestAppVersion(ResponseTx<String, DeviceError>),
}

pub struct DeviceService {
    token: Option<String>,
    rx: mpsc::UnboundedReceiver<DeviceCommand>,
    device_storage: DeviceStorage,
    token_storage: TokenStorage,
}

impl DeviceService {
    pub async fn new(
        db: DatabaseConnection,
        rx: mpsc::UnboundedReceiver<DeviceCommand>,
    ) -> Result<Self, DeviceError> {
        let device_storage = DeviceStorage::new(db.clone());
        let token_storage = TokenStorage::new(db);
        let token = token_storage.get_token().await?;
        Ok(Self {
            token,
            rx,
            device_storage,
            token_storage,
        })
    }

    pub async fn run(&mut self) {
        let mut shutdown_tx = None;
        while let Some(msg) = self.rx.recv().await {
            if let DeviceCommand::Shutdown(tx) = msg {
                shutdown_tx = Some(tx);
                break;
            }
            self.handle_message(msg).await;
        }

        tracing::info!("Device service shutting down");
        if shutdown_tx.is_some() {
            let _ = shutdown_tx.unwrap().send(());
        }
    }

    async fn handle_message(&mut self, msg: DeviceCommand) {
        match msg {
            DeviceCommand::SignIn(tx, user_creds) => self.handle_sign_in(tx, user_creds).await,
            DeviceCommand::SignOut(tx) => self.handle_sign_out(tx).await,
            DeviceCommand::BearerToken(tx) => self.handle_bearer_token(tx).await,
            DeviceCommand::IsAuthenticated(tx) => self.handle_is_authenticated(tx).await,
            DeviceCommand::Shutdown(_) => {}
            DeviceCommand::LatestAppVersion(tx) => self.handle_latest_app_version(tx).await,
        }
    }

    async fn handle_latest_app_version_inner(&mut self) -> Result<String, DeviceError> {
        let mut server_api = ServerApi::new(self.token_storage.clone()).await?;
        let version = server_api.latest_app_version().await?;

        Ok(version)
    }

    async fn handle_latest_app_version(&mut self, tx: ResponseTx<String, DeviceError>) {
        Self::oneshot_send(
            tx,
            self.handle_latest_app_version_inner().await,
            "handle_latest_app_version_inner",
        );
    }

    async fn handle_sign_in_inner(
        &mut self,
        user_creds: UserCredentials,
    ) -> Result<(), DeviceError> {
        let mut nymvpn_service = ServerApiNoAuth::new().await?;
        self.device_storage
            .init()
            .await
            .map_err(DeviceError::InitError)?;
        let device_details = self.device_storage.get_device().await?.unwrap();

        let add_device_request = AddDeviceRequest {
            user_creds,
            device_info: device_details.clone().into(),
        };

        // make API call
        let add_device_response = nymvpn_service.add_device(add_device_request).await?;

        // save token
        self.token_storage
            .save_token(add_device_response.token.clone())
            .await?;

        // update device ip addresses
        let device_details = self
            .device_storage
            .update_ipv4_address(
                device_details.unique_id,
                add_device_response.device_addresses.ipv4_address,
            )
            .await?;

        tracing::info!("Successfully signed in {device_details}");

        // keep this new token in memory
        self.token = Some(add_device_response.token);

        Ok(())
    }

    async fn handle_sign_in(
        &mut self,
        tx: ResponseTx<(), DeviceError>,
        user_creds: UserCredentials,
    ) {
        Self::oneshot_send(
            tx,
            self.handle_sign_in_inner(user_creds).await,
            "handle_sign_in",
        );
    }

    async fn handle_sign_out_inner(&mut self) -> Result<(), DeviceError> {
        // make API call to invalidate token
        let mut server_api = ServerApi::new(self.token_storage.clone()).await?;

        server_api.sign_out().await?;

        // reinitialize device
        self.device_storage.reinitialize("sign out").await?;
        // remove from DB and memory
        self.token_storage.remove_all().await?;
        self.token = None;

        Ok(())
    }

    async fn handle_sign_out(&mut self, tx: ResponseTx<(), DeviceError>) {
        Self::oneshot_send(tx, self.handle_sign_out_inner().await, "handle_sign_out");
    }

    async fn handle_bearer_token(&self, tx: ResponseTx<Option<String>, DeviceError>) {
        let token = self.token.clone();
        tokio::spawn(async move {
            Self::oneshot_send(tx, Ok(token), "handle_bearer_token");
        });
    }

    async fn handle_is_authenticated(&self, tx: ResponseTx<bool, DeviceError>) {
        let token = self.token.clone();
        tokio::spawn(async move {
            // todo: validate token from backend; if invalid purge from DB and memory
            Self::oneshot_send(tx, Ok(token.is_some()), "handle_is_authenticated")
        });
    }

    fn oneshot_send<T>(tx: oneshot::Sender<T>, t: T, msg: &'static str) {
        if tx.send(t).is_err() {
            tracing::warn!("Failed to respond from DeviceService {}", msg);
        }
    }
}
