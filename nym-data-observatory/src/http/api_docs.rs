use utoipa::OpenApi;
use utoipauto::utoipauto;

// manually import external structs which are behind feature flags because they
// can't be automatically discovered
// https://github.com/ProbablyClem/utoipauto/issues/13#issuecomment-1974911829
#[utoipauto(paths = "./nym-data-observatory/src")]
#[derive(OpenApi)]
#[openapi(info(title = "Nym API"), tags(), components(schemas()))]
pub(super) struct ApiDoc;
