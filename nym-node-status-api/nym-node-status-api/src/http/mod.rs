use utoipa::ToSchema;

pub(crate) mod api;
pub(crate) mod api_docs;
pub(crate) mod error;
pub(crate) mod middleware;
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
        let (size, mut page) = pagination.into_inner_values();

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

const SIZE_DEFAULT: usize = 10;
const SIZE_MAX: usize = 200;
const PAGE_DEFAULT: usize = 0;

impl Default for Pagination {
    fn default() -> Self {
        Self {
            size: Some(SIZE_DEFAULT),
            page: Some(PAGE_DEFAULT),
        }
    }
}

impl Pagination {
    pub(crate) fn new(size: Option<usize>, page: Option<usize>) -> Self {
        Self { size, page }
    }

    pub(crate) fn into_inner_values(self) -> (usize, usize) {
        (
            self.size.unwrap_or(SIZE_DEFAULT).min(SIZE_MAX),
            self.page.unwrap_or(PAGE_DEFAULT),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Use a simple type for testing instead of a custom struct
    type TestItem = String;

    #[test]
    fn test_pagination_default() {
        let pagination = Pagination::default();
        let (size, page) = pagination.into_inner_values();
        assert_eq!(size, SIZE_DEFAULT);
        assert_eq!(page, PAGE_DEFAULT);
    }

    #[test]
    fn test_pagination_new() {
        let pagination = Pagination::new(Some(50), Some(3));
        let (size, page) = pagination.into_inner_values();
        assert_eq!(size, 50);
        assert_eq!(page, 3);
    }

    #[test]
    fn test_pagination_max_size_limit() {
        let pagination = Pagination::new(Some(1000), Some(0));
        let (size, page) = pagination.into_inner_values();
        assert_eq!(size, SIZE_MAX);
        assert_eq!(page, 0);
    }

    #[test]
    fn test_pagination_none_values() {
        let pagination = Pagination::new(None, None);
        let (size, page) = pagination.into_inner_values();
        assert_eq!(size, SIZE_DEFAULT);
        assert_eq!(page, PAGE_DEFAULT);
    }

    #[test]
    fn test_paged_result_empty_list() {
        let items: Vec<TestItem> = vec![];
        let pagination = Pagination::new(Some(10), Some(0));
        let result = PagedResult::paginate(pagination, items);

        assert_eq!(result.page, 0);
        assert_eq!(result.size, 10);
        assert_eq!(result.total, 0);
        assert_eq!(result.items.len(), 0);
    }

    #[test]
    fn test_paged_result_single_page() {
        let items: Vec<TestItem> = (0..5).map(|i| format!("Item {i}")).collect();

        let pagination = Pagination::new(Some(10), Some(0));
        let result = PagedResult::paginate(pagination, items.clone());

        assert_eq!(result.page, 0);
        assert_eq!(result.size, 10);
        assert_eq!(result.total, 5);
        assert_eq!(result.items.len(), 5);
        assert_eq!(result.items[0], "Item 0");
        assert_eq!(result.items[4], "Item 4");
    }

    #[test]
    fn test_paged_result_multiple_pages() {
        let items: Vec<TestItem> = (0..25).map(|i| format!("Item {i}")).collect();

        // First page
        let pagination = Pagination::new(Some(10), Some(0));
        let result = PagedResult::paginate(pagination, items.clone());
        assert_eq!(result.page, 0);
        assert_eq!(result.size, 10);
        assert_eq!(result.total, 25);
        assert_eq!(result.items.len(), 10);
        assert_eq!(result.items[0], "Item 0");
        assert_eq!(result.items[9], "Item 9");

        // Second page
        let pagination = Pagination::new(Some(10), Some(1));
        let result = PagedResult::paginate(pagination, items.clone());
        assert_eq!(result.page, 1);
        assert_eq!(result.size, 10);
        assert_eq!(result.total, 25);
        assert_eq!(result.items.len(), 10);
        assert_eq!(result.items[0], "Item 10");
        assert_eq!(result.items[9], "Item 19");

        // Last page (partial)
        let pagination = Pagination::new(Some(10), Some(2));
        let result = PagedResult::paginate(pagination, items);
        assert_eq!(result.page, 2);
        assert_eq!(result.size, 10);
        assert_eq!(result.total, 25);
        assert_eq!(result.items.len(), 5);
        assert_eq!(result.items[0], "Item 20");
        assert_eq!(result.items[4], "Item 24");
    }

    #[test]
    fn test_paged_result_page_out_of_bounds() {
        let items: Vec<TestItem> = (0..15).map(|i| format!("Item {i}")).collect();

        // Page way out of bounds
        let pagination = Pagination::new(Some(10), Some(10));
        let result = PagedResult::paginate(pagination, items);

        // Should adjust to last valid page
        assert_eq!(result.page, 1); // 15 items / 10 per page = 1 (0-indexed)
        assert_eq!(result.size, 10);
        assert_eq!(result.total, 15);
        assert_eq!(result.items.len(), 5);
        assert_eq!(result.items[0], "Item 10");
    }

    #[test]
    fn test_paged_result_exact_page_boundary() {
        let items: Vec<TestItem> = (0..20).map(|i| format!("Item {i}")).collect();

        // Exactly 2 pages of 10 items each
        let pagination = Pagination::new(Some(10), Some(1));
        let result = PagedResult::paginate(pagination, items);

        assert_eq!(result.page, 1);
        assert_eq!(result.size, 10);
        assert_eq!(result.total, 20);
        assert_eq!(result.items.len(), 10);
    }

    #[test]
    fn test_paged_result_single_item_per_page() {
        let items: Vec<TestItem> = (0..5).map(|i| format!("Item {i}")).collect();

        let pagination = Pagination::new(Some(1), Some(3));
        let result = PagedResult::paginate(pagination, items);

        assert_eq!(result.page, 3);
        assert_eq!(result.size, 1);
        assert_eq!(result.total, 5);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0], "Item 3");
    }

    #[test]
    fn test_pagination_serialization() {
        let pagination = Pagination::new(Some(25), Some(2));
        let json = serde_json::to_string(&pagination).unwrap();
        assert!(json.contains("\"size\":25"));
        assert!(json.contains("\"page\":2"));

        let deserialized: Pagination = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.size, Some(25));
        assert_eq!(deserialized.page, Some(2));
    }

    #[test]
    fn test_paged_result_serialization() {
        let items = vec!["First".to_string(), "Second".to_string()];
        let pagination = Pagination::new(Some(10), Some(0));
        let result = PagedResult::paginate(pagination, items);

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"page\":0"));
        assert!(json.contains("\"size\":10"));
        assert!(json.contains("\"total\":2"));
        assert!(json.contains("\"items\":"));
        assert!(json.contains("First"));
        assert!(json.contains("Second"));
    }
}
