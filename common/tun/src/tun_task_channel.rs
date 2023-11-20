#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::time::Duration;

use tokio::sync::mpsc::{
    self,
    error::{SendError, SendTimeoutError, TrySendError},
};

pub(crate) type TunTaskPayload = (u64, Vec<u8>);

#[derive(Clone)]
pub struct TunTaskTx(mpsc::Sender<TunTaskPayload>);
pub(crate) struct TunTaskRx(mpsc::Receiver<TunTaskPayload>);

impl TunTaskTx {
    pub async fn send(&self, data: TunTaskPayload) -> Result<(), SendError<TunTaskPayload>> {
        self.0.send(data).await
    }

    pub fn try_send(&self, data: TunTaskPayload) -> Result<(), TrySendError<TunTaskPayload>> {
        self.0.try_send(data)
    }
}

impl TunTaskRx {
    pub(crate) async fn recv(&mut self) -> Option<TunTaskPayload> {
        self.0.recv().await
    }
}

pub(crate) fn tun_task_channel() -> (TunTaskTx, TunTaskRx) {
    let (tun_task_tx, tun_task_rx) = tokio::sync::mpsc::channel(128);
    (TunTaskTx(tun_task_tx), TunTaskRx(tun_task_rx))
}

const TUN_TASK_RESPONSE_SEND_TIMEOUT_MS: u64 = 1_000;

// Send responses back from the tun device back to the PacketRelayer
pub(crate) struct TunTaskResponseTx(mpsc::Sender<TunTaskPayload>);
pub struct TunTaskResponseRx(mpsc::Receiver<TunTaskPayload>);

#[derive(thiserror::Error, Debug)]
pub enum TunTaskResponseSendError {
    #[error("failed to send tun response: {0}")]
    SendTimeoutError(#[from] SendTimeoutError<TunTaskPayload>),

    #[error("failed to send tun response: {0}")]
    SendError(#[from] SendError<TunTaskPayload>),

    #[error("failed to send tun response: {0}")]
    TrySendError(#[from] TrySendError<TunTaskPayload>),
}

impl TunTaskResponseTx {
    #[allow(unused)]
    pub(crate) async fn send(&self, data: TunTaskPayload) -> Result<(), TunTaskResponseSendError> {
        Ok(self
            .0
            .send_timeout(
                data,
                Duration::from_millis(TUN_TASK_RESPONSE_SEND_TIMEOUT_MS),
            )
            .await?)
    }

    pub(crate) fn try_send(&self, data: TunTaskPayload) -> Result<(), TunTaskResponseSendError> {
        Ok(self.0.try_send(data)?)
    }
}

impl TunTaskResponseRx {
    pub async fn recv(&mut self) -> Option<TunTaskPayload> {
        self.0.recv().await
    }
}

pub(crate) fn tun_task_response_channel() -> (TunTaskResponseTx, TunTaskResponseRx) {
    let (tun_task_tx, tun_task_rx) = tokio::sync::mpsc::channel(128);
    (
        TunTaskResponseTx(tun_task_tx),
        TunTaskResponseRx(tun_task_rx),
    )
}
