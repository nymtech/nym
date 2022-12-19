use client_core::{
    client::{
        base_client::{ClientInput, ClientOutput, ClientState},
        received_buffer::ReconstructedMessagesReceiver,
    },
    config::GatewayEndpointConfig,
};
use nymsphinx::addressing::clients::Recipient;
use task::TaskManager;

/// States that the client connection can be in. Currently the states can only progress linearly
/// from New -> Registered -> Connected -> Disconnected. In the future it could be useful to be able
/// to re-connect a disconnected client.
// WIP(JON): consider adding inner types
#[allow(clippy::large_enum_variant)]
pub(super) enum ConnectionState {
    New,
    Registered {
        nym_address: Recipient,
        gateway_endpoint_config: GatewayEndpointConfig,
    },
    Connected {
        nym_address: Recipient,
        client_input: ClientInput,
        #[allow(dead_code)]
        client_output: ClientOutput,
        #[allow(dead_code)]
        client_state: ClientState,
        reconstructed_receiver: ReconstructedMessagesReceiver,
        task_manager: TaskManager,
    },
    Disconnected,
}

impl std::fmt::Debug for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New => write!(f, "New"),
            Self::Registered {
                nym_address,
                gateway_endpoint_config: gateway_config,
            } => f
                .debug_struct("Registered")
                .field("nym_address", nym_address)
                .field("gateway_config", gateway_config)
                .finish(),
            Self::Connected {
                nym_address,
                client_input: _,
                client_output: _,
                client_state: _,
                reconstructed_receiver,
                task_manager,
            } => f
                .debug_struct("Connected")
                .field("nym_address", nym_address)
                .field("reconstructed_receiver", reconstructed_receiver)
                .field("task_manager", task_manager)
                .finish(),
            Self::Disconnected => write!(f, "Disconnected"),
        }
    }
}

impl ConnectionState {
    pub(super) fn client_input(&self) -> Option<&ClientInput> {
        match self {
            ConnectionState::New
            | ConnectionState::Registered { .. }
            | ConnectionState::Disconnected => None,
            ConnectionState::Connected { client_input, .. } => Some(client_input),
        }
    }

    pub(super) fn reconstructed_receiver(&mut self) -> Option<&mut ReconstructedMessagesReceiver> {
        match self {
            ConnectionState::New
            | ConnectionState::Registered { .. }
            | ConnectionState::Disconnected => None,
            ConnectionState::Connected {
                reconstructed_receiver,
                ..
            } => Some(reconstructed_receiver),
        }
    }

    pub(super) fn gateway_endpoint_config(&self) -> Option<&GatewayEndpointConfig> {
        match self {
            ConnectionState::New
            | ConnectionState::Connected { .. }
            | ConnectionState::Disconnected => None,
            ConnectionState::Registered {
                gateway_endpoint_config: gateway_config,
                ..
            } => Some(gateway_config),
        }
    }

    pub(super) fn nym_address(&self) -> Option<&Recipient> {
        match self {
            ConnectionState::New | ConnectionState::Disconnected => None,
            ConnectionState::Registered { nym_address, .. }
            | ConnectionState::Connected { nym_address, .. } => Some(nym_address),
        }
    }

    pub(super) fn task_manager(&mut self) -> Option<&mut TaskManager> {
        match self {
            ConnectionState::New
            | ConnectionState::Registered { .. }
            | ConnectionState::Disconnected => None,
            ConnectionState::Connected { task_manager, .. } => Some(task_manager),
        }
    }
}
