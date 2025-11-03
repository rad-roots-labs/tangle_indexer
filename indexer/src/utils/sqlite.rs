use rusqlite::{Connection, Row, Statement};
use thiserror::Error;

pub use rusqlite::{
    types::Type as SqliteType, Error as RustqliteError, Result as SqliteResult, Row as SqliteRow,
};

#[derive(Debug, Error)]
pub enum SqliteError {
    #[error("Failed to open SQLite DB at `{path}`: {source}")]
    ConnectionError {
        path: String,
        #[source]
        source: rusqlite::Error,
    },
    #[error("Failed to prepare SQL statement: {0}")]
    PrepareError(#[source] rusqlite::Error),
    #[error("Failed to run SQL query: {0}")]
    QueryError(#[source] rusqlite::Error),
    #[error("Failed to collect query results: {0}")]
    CollectError(#[source] rusqlite::Error),
}

pub fn sqlite_conn(db_path: &str) -> Result<Connection, SqliteError> {
    Connection::open(db_path).map_err(|e| SqliteError::ConnectionError {
        path: db_path.to_string(),
        source: e,
    })
}

pub fn sqlite_stmt<'a>(conn: &'a Connection, stmt: &'a str) -> Result<Statement<'a>, SqliteError> {
    conn.prepare(stmt).map_err(SqliteError::PrepareError)
}

pub fn sqlite_stmt_querymap<'a, T, F>(
    stmt: &'a mut Statement<'a>,
    map_fn: F,
) -> Result<Vec<T>, SqliteError>
where
    F: Fn(&Row) -> rusqlite::Result<T>,
    T: 'a,
{
    let mapped = stmt
        .query_map([], map_fn)
        .map_err(SqliteError::QueryError)?;
    mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(SqliteError::CollectError)
}

pub fn row_u64_at(row: &SqliteRow, idx: usize) -> SqliteResult<u64> {
    let v: u32 = row.get(idx)?;
    Ok(v as u64)
}
