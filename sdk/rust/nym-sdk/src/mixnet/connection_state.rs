use client_core::config::GatewayEndpointConfig;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum BuilderState {
    New,
    Registered {
        gateway_endpoint_config: GatewayEndpointConfig,
    },
}

impl BuilderState {
    pub(super) fn gateway_endpoint_config(&self) -> Option<&GatewayEndpointConfig> {
        match self {
            BuilderState::New => None,
            BuilderState::Registered {
                gateway_endpoint_config,
                ..
            } => Some(gateway_endpoint_config),
        }
    }
}
