// This is still not an ideal home for this struct, because it's not an
// universal nymsphinx addressing method, however, it needs to be
// accessible by both desktop and webassembly client (it's more
// of a helper/utils structure, because before it reaches the gateway
// it's already destructed).

use nymsphinx_types::{
    DestinationAddressBytes, NodeAddressBytes, DESTINATION_ADDRESS_LENGTH, NODE_ADDRESS_LENGTH,
};

#[derive(Debug)]
pub struct RecipientFormattingError;

impl From<nymsphinx_types::Error> for RecipientFormattingError {
    fn from(_: nymsphinx_types::Error) -> Self {
        Self
    }
}

// TODO: this should a different home... somewhere, but where?
#[derive(Clone, Debug)]
pub struct Recipient {
    destination: DestinationAddressBytes,
    gateway: NodeAddressBytes,
}

impl Recipient {
    pub const LEN: usize = DESTINATION_ADDRESS_LENGTH + NODE_ADDRESS_LENGTH;

    pub fn new(destination: DestinationAddressBytes, gateway: NodeAddressBytes) -> Self {
        Recipient {
            destination,
            gateway,
        }
    }

    pub fn destination(&self) -> DestinationAddressBytes {
        self.destination.clone()
    }

    pub fn gateway(&self) -> NodeAddressBytes {
        self.gateway.clone()
    }

    pub fn into_bytes(self) -> [u8; Self::LEN] {
        let mut out = [0u8; Self::LEN];
        out[..DESTINATION_ADDRESS_LENGTH].copy_from_slice(self.destination.as_bytes());
        out[DESTINATION_ADDRESS_LENGTH..].copy_from_slice(self.gateway.as_bytes());

        out
    }

    pub fn from_bytes(bytes: [u8; Self::LEN]) -> Self {
        let mut destination_bytes = [0u8; DESTINATION_ADDRESS_LENGTH];
        destination_bytes.copy_from_slice(&bytes[..DESTINATION_ADDRESS_LENGTH]);

        let mut gateway_address_bytes = [0u8; NODE_ADDRESS_LENGTH];
        gateway_address_bytes.copy_from_slice(&bytes[DESTINATION_ADDRESS_LENGTH..]);

        let destination = DestinationAddressBytes::from_bytes(destination_bytes);
        let gateway = NodeAddressBytes::from_bytes(gateway_address_bytes);

        Self {
            destination,
            gateway,
        }
    }

    pub fn try_from_string<S: Into<String>>(
        full_address: S,
    ) -> Result<Self, RecipientFormattingError> {
        let string_address = full_address.into();
        let split: Vec<_> = string_address.split('@').collect();
        if split.len() != 2 {
            return Err(RecipientFormattingError);
        }
        let destination = DestinationAddressBytes::try_from_base58_string(split[0])?;
        let gateway = NodeAddressBytes::try_from_base58_string(split[1])?;
        Ok(Recipient {
            destination,
            gateway,
        })
    }
}

impl std::fmt::Display for Recipient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{}",
            self.destination.to_base58_string(),
            self.gateway.to_base58_string()
        )
    }
}
