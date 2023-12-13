use crate::fs::{config::AppConfig, data::AppData, storage::AppStorage};
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod app;

pub type SharedAppState = Arc<Mutex<app::AppState>>;
pub type SharedAppData = Arc<Mutex<AppStorage<AppData>>>;
pub type SharedAppConfig = Arc<Mutex<AppStorage<AppConfig>>>;
