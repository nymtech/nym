use tokio::sync::mpsc;

pub(crate) type TunTaskPayload = (u64, Vec<u8>);

#[derive(Clone)]
pub struct TunTaskTx(mpsc::Sender<TunTaskPayload>);
pub(crate) struct TunTaskRx(mpsc::Receiver<TunTaskPayload>);

impl TunTaskTx {
    pub async fn send(
        &self,
        data: TunTaskPayload,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<TunTaskPayload>> {
        self.0.send(data).await
    }
}

impl TunTaskRx {
    pub(crate) async fn recv(&mut self) -> Option<TunTaskPayload> {
        self.0.recv().await
    }
}

pub(crate) fn tun_task_channel() -> (TunTaskTx, TunTaskRx) {
    let (tun_task_tx, tun_task_rx) = tokio::sync::mpsc::channel(16);
    (TunTaskTx(tun_task_tx), TunTaskRx(tun_task_rx))
}

// Send responses back from the tun device back to the PacketRelayer
pub(crate) struct TunTaskResponseTx(mpsc::Sender<TunTaskPayload>);
pub struct TunTaskResponseRx(mpsc::Receiver<TunTaskPayload>);

impl TunTaskResponseTx {
    pub(crate) async fn send(
        &self,
        data: TunTaskPayload,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<TunTaskPayload>> {
        self.0.send(data).await
    }
}

impl TunTaskResponseRx {
    pub async fn recv(&mut self) -> Option<TunTaskPayload> {
        self.0.recv().await
    }
}

pub(crate) fn tun_task_response_channel() -> (TunTaskResponseTx, TunTaskResponseRx) {
    let (tun_task_tx, tun_task_rx) = tokio::sync::mpsc::channel(16);
    (
        TunTaskResponseTx(tun_task_tx),
        TunTaskResponseRx(tun_task_rx),
    )
}
