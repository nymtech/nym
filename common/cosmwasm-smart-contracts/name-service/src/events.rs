use cosmwasm_std::{Coin, Event};

use crate::RegisteredName;

pub enum NameEventType {
    Register,
    DeleteId,
    DeleteName,
    UpdateDepositRequired,
}

impl std::fmt::Display for NameEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameEventType::Register => write!(f, "register"),
            NameEventType::DeleteId => write!(f, "delete_id"),
            NameEventType::DeleteName => write!(f, "delete_name"),
            NameEventType::UpdateDepositRequired => write!(f, "update_deposit_required"),
        }
    }
}

impl From<NameEventType> for String {
    fn from(event_type: NameEventType) -> Self {
        event_type.to_string()
    }
}

pub const ACTION: &str = "action";

pub const NAME_ID: &str = "name_id";
pub const NAME: &str = "name";
pub const OWNER: &str = "owner";

pub const DEPOSIT_REQUIRED: &str = "deposit_required";

pub fn new_register_event(name: RegisteredName) -> Event {
    Event::new(NameEventType::Register)
        .add_attribute(ACTION, NameEventType::Register)
        .add_attribute(NAME_ID, name.id.to_string())
        .add_attribute(NAME, name.name.name.to_string())
        .add_attribute(name.name.address.event_tag(), name.name.address.to_string())
        .add_attribute(OWNER, name.owner.to_string())
}

pub fn new_delete_id_event(name: RegisteredName) -> Event {
    Event::new(NameEventType::DeleteId)
        .add_attribute(ACTION, NameEventType::DeleteId)
        .add_attribute(NAME_ID, name.id.to_string())
        .add_attribute(NAME, name.name.name.to_string())
        .add_attribute(name.name.address.event_tag(), name.name.address.to_string())
}

pub fn new_delete_name_event(name: RegisteredName) -> Event {
    Event::new(NameEventType::DeleteId)
        .add_attribute(ACTION, NameEventType::DeleteName)
        .add_attribute(NAME_ID, name.id.to_string())
        .add_attribute(NAME, name.name.name.to_string())
        .add_attribute(name.name.address.event_tag(), name.name.address.to_string())
}

pub fn new_update_deposit_required_event(deposit_required: Coin) -> Event {
    Event::new(NameEventType::UpdateDepositRequired)
        .add_attribute(ACTION, NameEventType::UpdateDepositRequired)
        .add_attribute(DEPOSIT_REQUIRED, deposit_required.to_string())
}
