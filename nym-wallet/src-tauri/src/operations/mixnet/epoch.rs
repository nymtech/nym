use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use mixnet_contract_common::Interval;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/epoch.ts"))]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Epoch {
  id: u32,
  start: i64,
  end: i64,
  duration_seconds: u64,
}

impl From<Interval> for Epoch {
  fn from(interval: Interval) -> Self {
    Self {
      id: interval.id(),
      start: interval.start_unix_timestamp(),
      end: interval.end_unix_timestamp(),
      duration_seconds: interval.length().as_secs(),
    }
  }
}

#[tauri::command]
pub async fn get_current_epoch(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Epoch, BackendError> {
  let interval = nymd_client!(state).get_current_epoch().await?;
  Ok(interval.into())
}
