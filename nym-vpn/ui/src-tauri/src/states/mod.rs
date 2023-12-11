use std::sync::Arc;

use nym_vpn_lib::NymVPN;
use tokio::sync::Mutex;

use crate::fs::{config::AppConfig, data::AppData, storage::AppStorage};

pub mod app;

pub type SharedAppState = Arc<Mutex<app::AppState>>;
pub type SharedAppData = Arc<Mutex<AppStorage<AppData>>>;
pub type SharedAppConfig = Arc<Mutex<AppStorage<AppConfig>>>;
pub type NymVPNState = Arc<Mutex<NymVPN>>;
