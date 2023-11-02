use std::fmt::Display;
use std::future::Future;
use std::sync::Arc;

use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use log::{debug, error, info};
use tokio::sync::Mutex;

use crate::core::shutdown::Shutdown;
#[cfg(feature = "rocksdb_storage")]
use crate::storage::rocksdb::RocksDbStorage;
#[cfg(feature = "sqlite_storage")]
use crate::storage::sqlite::SqliteStorage;
use crate::{
    api::{application::Application, http, ApiListener, CommandExecutor},
    block::{builder::BlockManagerBuilder, manager::BlockManager},
    broadcast::bracha::broadcast::Broadcaster,
    broadcast::group::BroadcastGroup,
    config::Configuration,
    core::{
        api_cmd::ApiCmdProcessor,
        shutdown::{Handle, ShutdownManager},
    },
    crypto::Keypair,
    membership,
    membership::PeerInfo,
    network::libp2p::{
        ephemera_sender::EphemeraToNetworkSender, network_sender::NetCommunicationReceiver,
        swarm_network::SwarmNetwork,
    },
    peer::{PeerId, ToPeerId},
    storage::EphemeraDatabase,
    utilities::crypto::key_manager::KeyManager,
    websocket::ws_manager::{WsManager, WsMessageBroadcaster},
    Ephemera,
};

#[derive(Clone)]
pub(crate) struct NodeInfo {
    pub(crate) ip: String,
    pub(crate) protocol_port: u16,
    pub(crate) http_port: u16,
    pub(crate) ws_port: u16,
    pub(crate) peer_id: PeerId,
    pub(crate) keypair: Arc<Keypair>,
    pub(crate) initial_config: Configuration,
}

impl NodeInfo {
    pub(crate) fn new(config: Configuration) -> anyhow::Result<Self> {
        let keypair = KeyManager::read_keypair_from_str(&config.node.private_key)?;
        let info = Self {
            ip: config.node.ip.clone(),
            protocol_port: config.libp2p.port,
            http_port: config.http.port,
            ws_port: config.websocket.port,
            peer_id: keypair.peer_id(),
            keypair,
            initial_config: config,
        };
        Ok(info)
    }

    pub(crate) fn protocol_address(&self) -> String {
        format!("/ip4/{}/tcp/{}", self.ip, self.protocol_port)
    }

    pub(crate) fn api_address_http(&self) -> String {
        format!("http://{}:{}", self.ip, self.http_port)
    }

    pub(crate) fn ws_address_ws(&self) -> String {
        format!("ws://{}:{}", self.ip, self.ws_port)
    }

    pub(crate) fn ws_address_ip_port(&self) -> String {
        format!("{}:{}", self.ip, self.ws_port)
    }
}

impl Display for NodeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NodeInfo {{ ip: {}, protocol_port: {}, http_port: {}, ws_port: {}, peer_id: {}, keypair: {} }}",
            self.ip,
            self.protocol_port,
            self.http_port,
            self.ws_port,
            self.peer_id,
            self.keypair
        )
    }
}

#[derive(Clone)]
pub struct EphemeraHandle {
    /// Ephemera API
    pub api: CommandExecutor,
    /// Allows to send shutdown signal to the node
    pub shutdown: Handle,
}

pub struct EphemeraStarterInit {
    config: Configuration,
    node_info: NodeInfo,
    broadcaster: Broadcaster,
    api_listener: ApiListener,
    api: CommandExecutor,
}

impl EphemeraStarterInit {
    /// Initialize Ephemera builder
    ///
    /// # Arguments
    /// * `config` - [Configuration]
    ///
    /// # Returns
    /// [`EphemeraStarterInit`]
    ///
    /// # Errors
    /// * If the node configuration is invalid
    pub fn new(config: Configuration) -> anyhow::Result<Self> {
        let instance_info = NodeInfo::new(config.clone())?;
        let broadcaster = Broadcaster::new(instance_info.peer_id);
        let (api, api_listener) = CommandExecutor::new();

        let builder = EphemeraStarterInit {
            config,
            node_info: instance_info,
            broadcaster,
            api_listener,
            api,
        };
        Ok(builder)
    }

    pub fn with_application<A: Application>(
        self,
        application: A,
    ) -> EphemeraStarterWithApplication<A> {
        EphemeraStarterWithApplication {
            init: self,
            application,
        }
    }
}

#[derive(Default)]
struct ServiceInfo {
    ws_message_broadcast: Option<WsMessageBroadcaster>,
    from_network: Option<NetCommunicationReceiver>,
    to_network: Option<EphemeraToNetworkSender>,
}

pub struct EphemeraStarterWithApplication<A: Application> {
    init: EphemeraStarterInit,
    application: A,
}

impl<A: Application> EphemeraStarterWithApplication<A> {
    /// Initialize Ephemera with the given application.
    /// It also tries to open the database connection.
    ///
    /// # Arguments
    /// * `application` - [Application] to be used
    ///
    /// # Returns
    /// [`EphemeraStarterWithApplication`]
    ///
    /// # Errors
    /// * If the node configuration is invalid or the database connection cannot be opened
    pub fn with_members_provider<
        P: Future<Output = membership::Result<Vec<PeerInfo>>> + Send + Unpin + 'static,
    >(
        mut self,
        provider: P,
    ) -> anyhow::Result<EphemeraStarterWithProvider<A>> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "sqlite_storage")] {
                let mut storage = self.connect_sqlite()?;
                debug!("Connected to sqlite database")
            } else if #[cfg(feature = "rocksdb_storage")] {
                let mut storage = self.connect_rocksdb()?;
                debug!("Connected to rocksdb database")
            } else {
                compile_error!("Must enable either sqlite or rocksdb feature");
            }
        }

        let block_manager = self.init_block_manager(&mut storage)?;

        let (shutdown_manager, shutdown_handle) = ShutdownManager::init();

        let mut service_data = ServiceInfo::default();
        let services = self.init_services(&mut service_data, &shutdown_manager, provider)?;

        Ok(EphemeraStarterWithProvider {
            with_application: self,
            block_manager: Some(block_manager),
            service_data,
            services,
            storage: Some(Box::new(storage)),
            shutdown_manager: Some(shutdown_manager),
            shutdown_handle: Some(shutdown_handle),
        })
    }

    //allocate database connection
    #[cfg(feature = "rocksdb_storage")]
    fn connect_rocksdb(&self) -> anyhow::Result<RocksDbStorage> {
        info!("Opening database...");
        RocksDbStorage::open(&self.init.config.storage)
            .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))
    }

    #[cfg(feature = "sqlite_storage")]
    fn connect_sqlite(&mut self) -> anyhow::Result<SqliteStorage> {
        info!("Opening database...");
        SqliteStorage::open(self.init.config.storage.clone())
            .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))
    }

    fn init_block_manager<D: EphemeraDatabase + ?Sized>(
        &mut self,
        db: &mut D,
    ) -> anyhow::Result<BlockManager> {
        let block_manager_configuration = self.init.config.block_manager.clone();
        let keypair = self.init.node_info.keypair.clone();
        let builder = BlockManagerBuilder::new(block_manager_configuration, keypair);
        builder.build(db)
    }

    fn init_services<
        P: Future<Output = membership::Result<Vec<PeerInfo>>> + Send + Unpin + 'static,
    >(
        &mut self,
        service_data: &mut ServiceInfo,
        shutdown_manager: &ShutdownManager,
        provider: P,
    ) -> anyhow::Result<Vec<BoxFuture<'static, anyhow::Result<()>>>> {
        let services = vec![
            self.init_libp2p(service_data, shutdown_manager.subscribe(), provider)?,
            self.init_http(shutdown_manager.subscribe())?,
            self.init_websocket(service_data, shutdown_manager.subscribe()),
        ];
        Ok(services)
    }

    fn init_websocket(
        &mut self,
        service_data: &mut ServiceInfo,
        mut shutdown: Shutdown,
    ) -> BoxFuture<'static, anyhow::Result<()>> {
        let (mut websocket, ws_message_broadcast) =
            WsManager::new(self.init.node_info.ws_address_ip_port());

        service_data.ws_message_broadcast = Some(ws_message_broadcast);

        async move {
            websocket.listen().await?;

            tokio::select! {
                _ = shutdown.shutdown_signal_rcv.recv() => {
                    info!("Shutting down websocket manager");
                }
                ws_stopped = websocket.run() => {
                    match ws_stopped {
                        Ok(()) => info!("Websocket stopped unexpectedly"),
                        Err(e) => error!("Websocket stopped with error: {}", e),
                    }
                }
            }
            info!("Websocket task finished");
            Ok(())
        }
        .boxed()
    }

    fn init_http(
        &mut self,
        mut shutdown: Shutdown,
    ) -> anyhow::Result<BoxFuture<'static, anyhow::Result<()>>> {
        let http = http::init(&self.init.node_info, self.init.api.clone())?;

        let fut = async move {
            let server_handle = http.handle();
            tokio::select! {
                _ = shutdown.shutdown_signal_rcv.recv() => {
                    info!("Shutting down http server");
                    server_handle.stop(true).await;
                }
                http_stopped = http => {
                    match http_stopped {
                        Ok(()) => info!("Http server stopped unexpectedly"),
                        Err(e) => error!("Http server stopped with error: {}", e),
                    }
                }
            }
            info!("Http task finished");
            Ok(())
        }
        .boxed();
        Ok(fut)
    }

    fn init_libp2p<
        P: Future<Output = membership::Result<Vec<PeerInfo>>> + Send + Unpin + 'static,
    >(
        &mut self,
        service_data: &mut ServiceInfo,
        mut shutdown: Shutdown,
        provider: P,
    ) -> anyhow::Result<BoxFuture<'static, anyhow::Result<()>>> {
        info!("Starting network...",);

        let (mut network, from_network, to_network) =
            SwarmNetwork::new(self.init.node_info.clone(), provider)?;

        service_data.from_network = Some(from_network);
        service_data.to_network = Some(to_network);

        let libp2p = async move {
            network.listen()?;

            tokio::select! {
                _ = shutdown.shutdown_signal_rcv.recv() => {
                    info!("Shutting down network");
                }
                nw_stopped = network.start() => {
                    match nw_stopped {
                        Ok(()) => info!("Network stopped unexpectedly"),
                        Err(e) => error!("Network stopped with error: {e}",),
                    }
                }
            }
            info!("Network task finished");
            Ok(())
        }
        .boxed();
        Ok(libp2p)
    }
}

pub struct EphemeraStarterWithProvider<A>
where
    A: Application + 'static,
{
    with_application: EphemeraStarterWithApplication<A>,
    block_manager: Option<BlockManager>,
    service_data: ServiceInfo,
    storage: Option<Box<dyn EphemeraDatabase>>,
    services: Vec<BoxFuture<'static, anyhow::Result<()>>>,
    shutdown_manager: Option<ShutdownManager>,
    shutdown_handle: Option<Handle>,
}

impl<A> EphemeraStarterWithProvider<A>
where
    A: Application + 'static,
{
    pub fn build(self) -> Ephemera<A> {
        self.ephemera()
    }

    fn ephemera(mut self) -> Ephemera<A> {
        let ephemera_handle = EphemeraHandle {
            api: self.with_application.init.api,
            shutdown: self.shutdown_handle.take().unwrap(),
        };

        let node_info = self.with_application.init.node_info;
        let application = self.with_application.application;
        let block_manager = self.block_manager.expect("Block manager not initialized");
        let broadcaster = self.with_application.init.broadcaster;
        let from_network = self
            .service_data
            .from_network
            .expect("From network not initialized");
        let to_network = self
            .service_data
            .to_network
            .expect("To network not initialized");
        let storage = self.storage.expect("Storage not initialized");
        let ws_message_broadcast = self
            .service_data
            .ws_message_broadcast
            .expect("WS message broadcast not initialized");
        let api_listener = self.with_application.init.api_listener;
        let shutdown_manager = self
            .shutdown_manager
            .expect("Shutdown manager not initialized");
        let services = self.services;

        Ephemera {
            node_info,
            block_manager,
            broadcaster,
            from_network,
            to_network,
            broadcast_group: BroadcastGroup::new(),
            storage: Arc::new(Mutex::new(storage)),
            ws_message_broadcast,
            api_listener,
            api_cmd_processor: ApiCmdProcessor::new(),
            application: application.into(),
            ephemera_handle,
            shutdown_manager,
            services,
        }
    }
}
