use cosmwasm_std::{Coin, Event};

use crate::{Service, ServiceId};

pub enum ServiceProviderEventType {
    Announce,
    DeleteId,
    DeleteNymAddress,
    UpdateDepositRequired,
}

impl std::fmt::Display for ServiceProviderEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceProviderEventType::Announce => write!(f, "announce"),
            ServiceProviderEventType::DeleteId => write!(f, "delete_id"),
            ServiceProviderEventType::DeleteNymAddress => write!(f, "delete_nym_address"),
            ServiceProviderEventType::UpdateDepositRequired => write!(f, "update_deposit_required"),
        }
    }
}

impl From<ServiceProviderEventType> for String {
    fn from(event_type: ServiceProviderEventType) -> Self {
        event_type.to_string()
    }
}

pub const ACTION: &str = "action";

pub const SERVICE_ID: &str = "service_id";
pub const SERVICE_TYPE: &str = "service_type";
pub const NYM_ADDRESS: &str = "nym_address";
pub const OWNER: &str = "owner";

pub const DEPOSIT_REQUIRED: &str = "deposit_required";

pub fn new_announce_event(service_id: ServiceId, service: Service) -> Event {
    Event::new(ServiceProviderEventType::Announce)
        .add_attribute(ACTION, ServiceProviderEventType::Announce)
        .add_attribute(SERVICE_ID, service_id.to_string())
        .add_attribute(SERVICE_TYPE, service.service_type.to_string())
        .add_attribute(NYM_ADDRESS, service.nym_address.to_string())
        .add_attribute(OWNER, service.owner.to_string())
}

pub fn new_delete_id_event(service_id: ServiceId, service: Service) -> Event {
    Event::new(ServiceProviderEventType::DeleteId)
        .add_attribute(ACTION, ServiceProviderEventType::DeleteId)
        .add_attribute(SERVICE_ID, service_id.to_string())
        .add_attribute(NYM_ADDRESS, service.nym_address.to_string())
}

pub fn new_update_deposit_required_event(deposit_required: Coin) -> Event {
    Event::new(ServiceProviderEventType::UpdateDepositRequired)
        .add_attribute(ACTION, ServiceProviderEventType::UpdateDepositRequired)
        .add_attribute(DEPOSIT_REQUIRED, deposit_required.to_string())
}
