use utoipa::OpenApi;
use utoipauto::utoipauto;

// manually import external structs which are behind feature flags because they
// can't be automatically discovered
// https://github.com/ProbablyClem/utoipauto/issues/13#issuecomment-1974911829
#[utoipauto(paths = "./nyx-chain-watcher/sqlite/src")]
#[derive(OpenApi)]
#[openapi(info(title = "Nyx Chain Watcher API"), tags(), components(schemas()))]
pub(super) struct ApiDoc;
