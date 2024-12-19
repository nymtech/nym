use utoipa::ToSchema;

pub(crate) mod api;
pub(crate) mod api_docs;
pub(crate) mod error;
pub(crate) mod models;
pub(crate) mod server;
pub(crate) mod state;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PagedResult<T: ToSchema> {
    pub page: usize,
    pub size: usize,
    pub total: usize,
    pub items: Vec<T>,
}

impl<T: Clone + ToSchema> PagedResult<T> {
    pub fn paginate(pagination: Pagination, res: Vec<T>) -> Self {
        let total = res.len();
        let (size, mut page) = pagination.intoto_inner_values();

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
    pub(crate) fn intoto_inner_values(self) -> (usize, usize) {
        const SIZE_DEFAULT: usize = 10;
        const SIZE_MAX: usize = 200;

        const PAGE_DEFAULT: usize = 0;

        (
            self.size.unwrap_or(SIZE_DEFAULT).min(SIZE_MAX),
            self.page.unwrap_or(PAGE_DEFAULT),
        )
    }
}
