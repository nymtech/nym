use crate::state::ExplorerApiStateContext;

pub(crate) struct MixNodesTasks {
    state: ExplorerApiStateContext,
}

impl MixNodesTasks {
    pub(crate) fn new(state: ExplorerApiStateContext) -> Self {
        MixNodesTasks { state }
    }

    pub(crate) fn start(self) {
        info!("Spawning mix nodes task runner...");
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                // wait for the next interval tick
                interval_timer.tick().await;

                info!("Updating mix node cache...");
                self.state.inner.mix_nodes.refresh().await;
                info!("Done");
            }
        });
    }
}
