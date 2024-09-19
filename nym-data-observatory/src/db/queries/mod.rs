// group queries in files by theme
mod joke;

// re-exporting allows us to access all queries via `queries::bla``
pub(crate) use joke::{insert_joke, select_all, select_joke_by_id};
