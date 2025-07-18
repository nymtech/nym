use sqlx::Database;

/// Converts SQLite-style ? placeholders to PostgreSQL $N format
#[cfg(feature = "pg")]
fn convert_placeholders(query: &str) -> String {
    let mut result = String::with_capacity(query.len() + 10);
    let mut placeholder_count = 0;
    let mut chars = query.chars();
    let mut in_string: Option<char> = None;
    let mut escape_next = false;

    while let Some(ch) = chars.next() {
        if escape_next {
            result.push(ch);
            escape_next = false;
            continue;
        }

        if let Some(quote_char) = in_string {
            result.push(ch);
            if ch == quote_char {
                in_string = None;
            } else if ch == '\\' {
                escape_next = true;
            }
            continue;
        }

        match ch {
            '\\' => {
                result.push(ch);
                escape_next = true;
            }
            '\'' | '"' => {
                result.push(ch);
                in_string = Some(ch);
            }
            '?' => {
                placeholder_count += 1;
                result.push_str(&format!("${}", placeholder_count));
            }
            _ => {
                result.push(ch);
            }
        }
    }

    result
}

/// Creates a query that automatically handles placeholder conversion
#[cfg(feature = "sqlite")]
pub fn query(
    sql: &str,
) -> sqlx::query::Query<'_, sqlx::Sqlite, <sqlx::Sqlite as Database>::Arguments<'_>> {
    sqlx::query(sql)
}

#[cfg(feature = "pg")]
pub fn query(
    sql: &str,
) -> sqlx::query::Query<'static, sqlx::Postgres, <sqlx::Postgres as Database>::Arguments<'static>> {
    let converted = convert_placeholders(sql);
    sqlx::query(Box::leak(converted.into_boxed_str()))
}

/// Creates a query_as that automatically handles placeholder conversion
#[cfg(feature = "sqlite")]
pub fn query_as<O>(
    sql: &str,
) -> sqlx::query::QueryAs<'_, sqlx::Sqlite, O, <sqlx::Sqlite as Database>::Arguments<'_>>
where
    O: for<'r> sqlx::FromRow<'r, <sqlx::Sqlite as Database>::Row>,
{
    sqlx::query_as(sql)
}

#[cfg(feature = "pg")]
pub fn query_as<O>(
    sql: &str,
) -> sqlx::query::QueryAs<
    'static,
    sqlx::Postgres,
    O,
    <sqlx::Postgres as Database>::Arguments<'static>,
>
where
    O: for<'r> sqlx::FromRow<'r, <sqlx::Postgres as Database>::Row>,
{
    let converted = convert_placeholders(sql);
    sqlx::query_as(Box::leak(converted.into_boxed_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "pg")]
    fn test_convert_placeholders() {
        // Basic conversion
        assert_eq!(
            convert_placeholders(r"SELECT * FROM table WHERE id = ?"),
            r"SELECT * FROM table WHERE id = $1"
        );

        // Multiple placeholders
        assert_eq!(
            convert_placeholders(r"INSERT INTO table (a, b, c) VALUES (?, ?, ?)"),
            r"INSERT INTO table (a, b, c) VALUES ($1, $2, $3)"
        );

        // Placeholder inside string literal should be ignored
        assert_eq!(
            convert_placeholders(r"SELECT * FROM table WHERE name = 'test?' AND id = ?"),
            r"SELECT * FROM table WHERE name = 'test?' AND id = $1"
        );

        // Update statement
        assert_eq!(
            convert_placeholders(r"UPDATE table SET a = ?, b = ? WHERE id = ?"),
            r"UPDATE table SET a = $1, b = $2 WHERE id = $3"
        );

        // Test with 10 placeholders (like in update_mixnodes)
        assert_eq!(
            convert_placeholders(r"INSERT INTO t VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"),
            r"INSERT INTO t VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        );

        // No placeholders
        assert_eq!(
            convert_placeholders(r"SELECT * FROM table"),
            r"SELECT * FROM table"
        );

        // Placeholder at the beginning
        assert_eq!(convert_placeholders(r"? AND ?"), r"$1 AND $2");

        // Placeholder at the end
        assert_eq!(
            convert_placeholders(r"SELECT * FROM table WHERE id = ?"),
            r"SELECT * FROM table WHERE id = $1"
        );

        // Adjacent placeholders
        assert_eq!(
            convert_placeholders(r"VALUES(?,?   ,?)"),
            r"VALUES($1,$2   ,$3)"
        );

        // Escaped single quote
        assert_eq!(
            convert_placeholders(r"SELECT * FROM foo WHERE bar = 'it\'s a test' AND baz = ?"),
            r"SELECT * FROM foo WHERE bar = 'it\'s a test' AND baz = $1"
        );

        // Escaped question mark (should not be replaced)
        assert_eq!(
            convert_placeholders(r"SELECT * FROM foo WHERE bar = '\\?' AND baz = ?"),
            r"SELECT * FROM foo WHERE bar = '\\?' AND baz = $1"
        );

        // Double quotes (not standard SQL for strings, but good to test)
        assert_eq!(
            convert_placeholders(r#"SELECT * FROM foo WHERE bar = "?" AND baz = ?"#),
            r#"SELECT * FROM foo WHERE bar = "?" AND baz = $1"#
        );

        // Mismatched quotes
        assert_eq!(
            convert_placeholders(r#"SELECT * FROM foo WHERE bar = "'" AND baz = ?"#),
            r#"SELECT * FROM foo WHERE bar = "'" AND baz = $1"#
        );

        // Unmatched quote
        assert_eq!(
            convert_placeholders(r"SELECT 'oops?"),
            r"SELECT 'oops?"
        );
    }
}