use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use websocket_requests::requests::ClientRequest;

/// A return address is a way to send a message back to the original sender. It can be either
/// an explicitly known Recipient, or a surb AnonymousSenderTag.
#[derive(Debug, Clone)]
pub enum ReturnAddress {
    Known(Box<Recipient>),
    Anonymous(AnonymousSenderTag),
}
impl ReturnAddress {
    pub fn new(
        explicit_return_address: Option<Recipient>,
        implicit_tag: Option<AnonymousSenderTag>,
    ) -> Option<Self> {
        // if somehow we received both, always prefer the explicit address since it's way easier to use
        if let Some(recipient) = explicit_return_address {
            return Some(ReturnAddress::Known(Box::new(recipient)));
        }
        if let Some(sender_tag) = implicit_tag {
            return Some(ReturnAddress::Anonymous(sender_tag));
        }
        None
    }

    pub(super) fn send_back_to(self, message: Vec<u8>, connection_id: u64) -> ClientRequest {
        match self {
            ReturnAddress::Known(recipient) => ClientRequest::Send {
                recipient: *recipient,
                message,
                connection_id: Some(connection_id),
            },
            ReturnAddress::Anonymous(sender_tag) => ClientRequest::Reply {
                message,
                sender_tag,
                connection_id: Some(connection_id),
            },
        }
    }
}

impl From<Recipient> for ReturnAddress {
    fn from(recipient: Recipient) -> Self {
        ReturnAddress::Known(Box::new(recipient))
    }
}

impl From<AnonymousSenderTag> for ReturnAddress {
    fn from(sender_tag: AnonymousSenderTag) -> Self {
        ReturnAddress::Anonymous(sender_tag)
    }
}
