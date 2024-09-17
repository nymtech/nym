pub(crate) mod api;
pub(crate) mod api_docs;
pub(crate) mod models;
pub(crate) mod server;
pub(crate) mod state;
pub(crate) mod error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PagedResult<T> {
    pub page: u32,
    pub size: u32,
    pub total: i32,
    pub items: Vec<T>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct Pagination {
    size: Option<u32>,
    page: Option<u32>,
}

impl Pagination {
    // unwrap stored values or use predefined defaults
    pub(crate) fn to_inner_values(self) -> (u32, u32) {
        const SIZE_DEFAULT: u32 = 10;
        const SIZE_MAX: u32 = 200;

        const PAGE_DEFAULT: u32 = 0;

        (
            self.size.unwrap_or(SIZE_DEFAULT).min(SIZE_MAX),
            self.page.unwrap_or(PAGE_DEFAULT),
        )
    }
}
