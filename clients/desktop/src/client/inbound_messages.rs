use futures::channel::mpsc;
use nymsphinx::{
    DestinationAddressBytes, Error, NodeAddressBytes, DESTINATION_ADDRESS_LENGTH,
    NODE_ADDRESS_LENGTH,
};

pub(crate) type InputMessageSender = mpsc::UnboundedSender<InputMessage>;
pub(crate) type InputMessageReceiver = mpsc::UnboundedReceiver<InputMessage>;

#[derive(Debug)]
pub(crate) struct InputMessage {
    recipient: Recipient,
    data: Vec<u8>,
}

impl InputMessage {
    pub(crate) fn new(recipient: Recipient, data: Vec<u8>) -> Self {
        InputMessage { recipient, data }
    }

    // I'm open to suggestions on how to rename this.
    pub(crate) fn destruct(self) -> (Recipient, Vec<u8>) {
        (self.recipient, self.data)
    }
}

#[derive(Debug)]
pub struct RecipientFormattingError;

impl From<nymsphinx::Error> for RecipientFormattingError {
    fn from(_: Error) -> Self {
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

    pub fn try_from_string(full_address: String) -> Result<Self, RecipientFormattingError> {
        let split: Vec<_> = full_address.split("@").collect();
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

    pub fn to_string(&self) -> String {
        format!(
            "{}@{}",
            self.destination.to_base58_string(),
            self.gateway.to_base58_string()
        )
    }
}
