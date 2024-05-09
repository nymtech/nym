use nym_client_core::client::base_client::ClientState;
use nym_socks5_client_core::config::Socks5;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::{connections::LaneQueueLengths, TaskHandle};

use nym_topology::NymTopology;

use crate::mixnet::client::MixnetClientBuilder;
use crate::Result;

/// Client connected to the Nym mixnet.
pub struct Socks5MixnetClient {
    /// The nym address of this connected client.
    pub(crate) nym_address: Recipient,

    /// The current state of the client that is exposed to the user. This includes things like
    /// current message send queue length.
    pub(crate) client_state: ClientState,

    /// The task manager that controls all the spawned tasks that the clients uses to do it's job.
    pub(crate) task_handle: TaskHandle,

    /// SOCKS5 configuration parameters.
    pub(crate) socks5_config: Socks5,
}

impl Socks5MixnetClient {
    /// Create a new client and connect to a service provider over the mixnet via SOCKS5 using
    /// ephemeral in-memory keys that are discarded at application close.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nym_sdk::mixnet;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let receiving_client = mixnet::MixnetClient::connect_new().await.unwrap();
    ///     let mut client = mixnet::Socks5MixnetClient::connect_new(receiving_client.nym_address().to_string()).await;
    /// }
    ///
    /// ```
    pub async fn connect_new<S: Into<String>>(provider_mix_address: S) -> Result<Self> {
        MixnetClientBuilder::new_ephemeral()
            .socks5_config(Socks5::new(provider_mix_address))
            .build()?
            .connect_to_mixnet_via_socks5()
            .await
    }

    /// Get the nym address for this client, if it is available. The nym address is composed of the
    /// client identity, the client encryption key, and the gateway identity.
    pub fn nym_address(&self) -> &Recipient {
        &self.nym_address
    }

    /// Get the SOCKS5 proxy URL that a HTTP(S) client can connect to.
    pub fn socks5_url(&self) -> String {
        format!("socks5h://{}", self.socks5_config.bind_address)
    }

    /// Get a shallow clone of [`LaneQueueLengths`]. This is useful to manually implement some form
    /// of backpressure logic.
    pub fn shared_lane_queue_lengths(&self) -> LaneQueueLengths {
        self.client_state.shared_lane_queue_lengths.clone()
    }

    /// Change the network topology used by this client for constructing sphinx packets into the
    /// provided one.
    pub async fn manually_overwrite_topology(&self, new_topology: NymTopology) {
        self.client_state
            .topology_accessor
            .manually_change_topology(new_topology)
            .await
    }

    /// Gets the value of the currently used network topology.
    pub async fn read_current_topology(&self) -> Option<NymTopology> {
        self.client_state.topology_accessor.current_topology().await
    }

    /// Restore default topology refreshing behaviour of this client.
    pub fn restore_automatic_topology_refreshing(&self) {
        self.client_state.topology_accessor.release_manual_control()
    }

    /// Disconnect from the mixnet. Currently it is not supported to reconnect a disconnected
    /// client.
    pub async fn disconnect(mut self) {
        if let TaskHandle::Internal(task_manager) = &mut self.task_handle {
            task_manager.signal_shutdown().ok();
            task_manager.wait_for_shutdown().await;
        }

        // note: it's important to take ownership of the struct as if the shutdown is `TaskHandle::External`,
        // it must be dropped to finalize the shutdown
    }
}
