use sqlx::Database;

/// Converts SQLite-style ? placeholders to PostgreSQL $N format
#[cfg(feature = "pg")]
fn convert_placeholders(query: &str) -> String {
    let mut result = String::with_capacity(query.len() + 10);
    let mut placeholder_count = 0;
    let chars = query.chars().peekable();
    let mut in_string = false;
    let mut escape_next = false;

    for ch in chars {
        if escape_next {
            result.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' => {
                result.push(ch);
                escape_next = true;
            }
            '\'' => {
                result.push(ch);
                in_string = !in_string;
            }
            '?' if !in_string => {
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
        assert_eq!(
            convert_placeholders("SELECT * FROM table WHERE id = ?"),
            "SELECT * FROM table WHERE id = $1"
        );

        assert_eq!(
            convert_placeholders("INSERT INTO table (a, b, c) VALUES (?, ?, ?)"),
            "INSERT INTO table (a, b, c) VALUES ($1, $2, $3)"
        );

        assert_eq!(
            convert_placeholders("SELECT * FROM table WHERE name = 'test?' AND id = ?"),
            "SELECT * FROM table WHERE name = 'test?' AND id = $1"
        );

        assert_eq!(
            convert_placeholders("UPDATE table SET a = ?, b = ? WHERE id = ?"),
            "UPDATE table SET a = $1, b = $2 WHERE id = $3"
        );

        // Test with 10 placeholders (like in update_mixnodes)
        assert_eq!(
            convert_placeholders("INSERT INTO t VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"),
            "INSERT INTO t VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        );
    }
}
