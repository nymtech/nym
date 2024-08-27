use crate::error::NodeStatusApiResult;

pub(crate) fn init() -> NodeStatusApiResult<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).map_err(|_| crate::error::Error::InitFailed)
}
