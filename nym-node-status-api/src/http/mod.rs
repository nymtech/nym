use models::{Gateway, GatewaySkinny};

pub(crate) mod api;
pub(crate) mod api_docs;
pub(crate) mod error;
pub(crate) mod models;
pub(crate) mod server;
pub(crate) mod state;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
// exclude generic from auto-discovery
#[utoipauto::utoipa_ignore]
// https://docs.rs/utoipa/latest/utoipa/derive.ToSchema.html#generic-schemas-with-aliases
// Generic structs can only be included via aliases, not directly, because they
// it would cause an error in generated Swagger docs.
// Instead, you have to manually monomorphize each generic struct that
// you wish to document
#[aliases(
    PagedGateway = PagedResult<Gateway>,
    PagedGatewaySkinny = PagedResult<GatewaySkinny>
)]
pub struct PagedResult<T> {
    pub page: usize,
    pub size: usize,
    pub total: usize,
    pub items: Vec<T>,
}

impl<T: Clone> PagedResult<T> {
    pub fn paginate(pagination: Pagination, res: Vec<T>) -> Self {
        let total = res.len();
        let (size, mut page) = pagination.to_inner_values();

        if page * size > total {
            page = total / size;
        }

        let chunks: Vec<&[T]> = res.chunks(size).collect();

        PagedResult {
            page,
            size,
            total,
            items: chunks.get(page).cloned().unwrap_or_default().into(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct Pagination {
    size: Option<usize>,
    page: Option<usize>,
}

impl Pagination {
    // unwrap stored values or use predefined defaults
    pub(crate) fn to_inner_values(self) -> (usize, usize) {
        const SIZE_DEFAULT: usize = 10;
        const SIZE_MAX: usize = 200;

        const PAGE_DEFAULT: usize = 0;

        (
            self.size.unwrap_or(SIZE_DEFAULT).min(SIZE_MAX),
            self.page.unwrap_or(PAGE_DEFAULT),
        )
    }
}
