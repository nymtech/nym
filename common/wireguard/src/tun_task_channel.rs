pub(crate) type TunTaskPayload = (u64, Vec<u8>);

#[derive(Clone)]
pub struct TunTaskTx(tokio::sync::mpsc::UnboundedSender<TunTaskPayload>);

pub(crate) struct TunTaskRx(tokio::sync::mpsc::UnboundedReceiver<TunTaskPayload>);

impl TunTaskTx {
    pub(crate) fn send(
        &self,
        data: TunTaskPayload,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<TunTaskPayload>> {
        self.0.send(data)
    }
}

impl TunTaskRx {
    pub(crate) async fn recv(&mut self) -> Option<TunTaskPayload> {
        self.0.recv().await
    }
}

pub(crate) fn tun_task_channel() -> (TunTaskTx, TunTaskRx) {
    let (tun_task_tx, tun_task_rx) = tokio::sync::mpsc::unbounded_channel();
    (TunTaskTx(tun_task_tx), TunTaskRx(tun_task_rx))
}
