use sqlx::Database;

/// Converts SQLite-style ? placeholders to PostgreSQL $N format
#[cfg(feature = "pg")]
fn convert_placeholders(query: &str) -> String {
    let mut result = String::with_capacity(query.len() + 10);
    let mut placeholder_count = 0;
    let mut chars = query.chars();
    let mut in_string: Option<char> = None;
    let mut escape_next = false;

    #[allow(clippy::while_let_on_iterator)]
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
                result.push_str(&format!("${placeholder_count}"));
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

        // Double quotes
        assert_eq!(
            convert_placeholders(r#"SELECT * FROM "table" WHERE "column" = ? AND name = "test?""#),
            r#"SELECT * FROM "table" WHERE "column" = $1 AND name = "test?""#
        );

        // Mixed quotes
        assert_eq!(
            convert_placeholders(
                r#"SELECT * FROM table WHERE a = 'single?' AND b = "double?" AND c = ?"#
            ),
            r#"SELECT * FROM table WHERE a = 'single?' AND b = "double?" AND c = $1"#
        );

        // Escaped backslash before quote
        assert_eq!(
            convert_placeholders(r"SELECT * FROM table WHERE path = 'C:\\?' AND id = ?"),
            r"SELECT * FROM table WHERE path = 'C:\\?' AND id = $1"
        );

        // Multiple escaped quotes
        assert_eq!(
            convert_placeholders(
                r#"INSERT INTO table (msg) VALUES ('it\'s "complex" test') WHERE id = ?"#
            ),
            r#"INSERT INTO table (msg) VALUES ('it\'s "complex" test') WHERE id = $1"#
        );

        // Very long query with many placeholders
        let long_query = r"INSERT INTO very_long_table_name (col1, col2, col3, col4, col5, col6, col7, col8, col9, col10, col11, col12, col13, col14, col15) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
        let expected = r"INSERT INTO very_long_table_name (col1, col2, col3, col4, col5, col6, col7, col8, col9, col10, col11, col12, col13, col14, col15) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)";
        assert_eq!(convert_placeholders(long_query), expected);

        // Query with comments (question marks in comments are also converted)
        assert_eq!(
            convert_placeholders(
                r"-- This is a comment with ?
            SELECT * FROM table WHERE id = ? -- another comment ?"
            ),
            r"-- This is a comment with $1
            SELECT * FROM table WHERE id = $2 -- another comment $3"
        );

        // Multiline strings
        assert_eq!(
            convert_placeholders(
                r"SELECT * FROM table
            WHERE description = 'This is a
            multiline string with ?'
            AND id = ?"
            ),
            r"SELECT * FROM table
            WHERE description = 'This is a
            multiline string with ?'
            AND id = $1"
        );

        // Complex nested quotes
        assert_eq!(
            convert_placeholders(
                r#"SELECT json_extract(data, '$.items[?(@.name=="test?")]') FROM table WHERE id = ?"#
            ),
            r#"SELECT json_extract(data, '$.items[?(@.name=="test?")]') FROM table WHERE id = $1"#
        );

        // Empty string
        assert_eq!(convert_placeholders(""), "");

        // Only placeholders
        assert_eq!(convert_placeholders("???"), "$1$2$3");

        // Unicode in strings
        assert_eq!(
            convert_placeholders(r"SELECT * FROM table WHERE name = '测试?' AND id = ?"),
            r"SELECT * FROM table WHERE name = '测试?' AND id = $1"
        );

        // Test case with backslash at end of string
        assert_eq!(
            convert_placeholders(r"SELECT * FROM table WHERE path LIKE '%\\' AND id = ?"),
            r"SELECT * FROM table WHERE path LIKE '%\\' AND id = $1"
        );

        // Mismatched quotes
        assert_eq!(
            convert_placeholders(r#"SELECT * FROM foo WHERE bar = "'" AND baz = ?"#),
            r#"SELECT * FROM foo WHERE bar = "'" AND baz = $1"#
        );

        // Unmatched quote
        assert_eq!(convert_placeholders(r"SELECT 'oops?"), r"SELECT 'oops?");
    }
}
