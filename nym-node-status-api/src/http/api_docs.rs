use utoipa::OpenApi;
use utoipauto::utoipauto;

#[utoipauto(paths = "./nym-node-status-api/src")]
#[derive(OpenApi)]
#[openapi(info(title = "Nym API"), tags(), components(schemas()))]
pub(super) struct ApiDoc;
