// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! SQLCipher support for Nym's SQLite storages.
//!
//! The `bundled` feature builds the bundled SQLCipher in place of SQLite for the whole dependency
//! graph. Without it, [`ensure_available`] fails unless SQLCipher is linked some other way.
//! Storages open their databases encrypted by first calling [`encrypt_database_if_needed`], and
//! then connecting with options passed through [`connect_options_with_key`].

use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};
use sqlx::{AssertSqlSafe, ConnectOptions, Connection};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{debug, info, warn};
use zeroize::Zeroizing;

/// Length of a raw SQLCipher key in bytes.
pub const KEY_LEN: usize = 32;

/// Raw 256-bit SQLCipher key. It is used as is, without passphrase key derivation.
pub type Key = [u8; KEY_LEN];

/// Owned raw SQLCipher [`Key`], zeroized on drop and redacted from `Debug` output.
///
/// The key is kept in a single shared heap allocation, so moving a `DatabaseKey` doesn't leave
/// copies of it behind, and clones share it. It's zeroized when the last clone is dropped.
#[derive(Clone)]
pub struct DatabaseKey(Arc<Zeroizing<Key>>);

impl DatabaseKey {
    pub fn new(key: Key) -> Self {
        DatabaseKey(Arc::new(Zeroizing::new(key)))
    }

    /// Parses a key from its hex encoding: exactly [`KEY_LEN`] bytes, i.e. 64 hex characters.
    ///
    /// The error doesn't include the input, so that it can't leak the key.
    pub fn from_hex(hex_key: &str) -> Result<Self, InvalidHexKey> {
        let mut key = Zeroizing::new([0u8; KEY_LEN]);
        hex::decode_to_slice(hex_key.trim(), key.as_mut_slice()).map_err(|_| InvalidHexKey)?;
        Ok(DatabaseKey::new(*key))
    }

    pub fn as_bytes(&self) -> &Key {
        &self.0
    }
}

/// Returned by [`DatabaseKey::from_hex`] when the input isn't a hex-encoded 256-bit key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the database key must be exactly {KEY_LEN} bytes encoded as {} hex characters", KEY_LEN * 2)]
pub struct InvalidHexKey;

/// Parses the hex encoding, see [`DatabaseKey::from_hex`].
impl std::str::FromStr for DatabaseKey {
    type Err = InvalidHexKey;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DatabaseKey::from_hex(s)
    }
}

impl From<Key> for DatabaseKey {
    fn from(key: Key) -> Self {
        DatabaseKey::new(key)
    }
}

impl std::fmt::Debug for DatabaseKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DatabaseKey(<redacted>)")
    }
}

/// `SQLITE_NOTADB`: returned when a database can't be read, i.e. it's encrypted and the key is
/// wrong or missing, or it's plaintext and a key was given.
const SQLITE_NOTADB: &str = "26";

/// Extension appended to the database file name for the encrypted copy made during migration.
const MIGRATION_EXTENSION: &str = "sqlcipher-migration";

#[derive(Debug, thiserror::Error)]
pub enum SqlCipherError {
    #[error("SQLite is not built with SQLCipher, cannot open encrypted database")]
    NotSupported,

    #[error("cannot decrypt the database at {}: wrong key or corrupted database", path.display())]
    InvalidKeyOrCorrupted { path: PathBuf },

    #[error("failed to encrypt the existing plaintext database at {}: {source}", path.display())]
    MigrationFailed {
        path: PathBuf,
        #[source]
        source: sqlx::Error,
    },

    #[error("invalid database path {}: it has no file name", path.display())]
    InvalidPath { path: PathBuf },

    #[error("I/O error on {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl SqlCipherError {
    fn io(path: &Path, source: io::Error) -> Self {
        SqlCipherError::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

/// Prepares connect options for opening the database encrypted with `key`.
///
/// Without a key, the options are returned unchanged. With a key, the database at the options'
/// filename is encrypted with [`encrypt_database_if_needed`], and the key is set on the options
/// with [`connect_options_with_key`].
pub async fn apply_key(
    opts: SqliteConnectOptions,
    key: Option<&DatabaseKey>,
) -> Result<SqliteConnectOptions, SqlCipherError> {
    match key {
        Some(key) => {
            encrypt_database_if_needed(opts.get_filename(), key).await?;
            Ok(connect_options_with_key(opts, key))
        }
        None => Ok(opts),
    }
}

/// Sets the SQLCipher key on the connect options.
///
/// SQLx always issues `PRAGMA key` before any other PRAGMA, so the key is in place before
/// the database is read. The key is passed as `x'<HEX>'`, so SQLCipher uses it directly without
/// passphrase key derivation.
pub fn connect_options_with_key(
    opts: SqliteConnectOptions,
    key: &DatabaseKey,
) -> SqliteConnectOptions {
    with_raw_key(opts, key.as_bytes())
}

fn with_raw_key(opts: SqliteConnectOptions, key: &Key) -> SqliteConnectOptions {
    opts.pragma("key", format!("\"{}\"", raw_key_literal(key).as_str()))
}

/// Checks that SQLite is built with SQLCipher.
///
/// Plain SQLite silently ignores `PRAGMA key` and would create an unencrypted database.
/// `PRAGMA cipher_version` only returns a row when SQLite is built with SQLCipher. The check runs
/// on a separate in-memory connection, before any database file is touched. The linked SQLite
/// can't change at runtime, so the result is computed once and cached. Errors are not cached.
pub async fn ensure_available() -> Result<(), SqlCipherError> {
    static SQLCIPHER_AVAILABLE: OnceCell<bool> = OnceCell::const_new();

    let available = SQLCIPHER_AVAILABLE
        .get_or_try_init(|| async {
            let mut conn = SqliteConnection::connect("sqlite::memory:").await?;
            let cipher_version: Option<String> = sqlx::query_scalar("PRAGMA cipher_version")
                .fetch_optional(&mut conn)
                .await?;
            conn.close().await?;
            Ok::<_, SqlCipherError>(cipher_version.is_some())
        })
        .await?;

    if *available {
        Ok(())
    } else {
        Err(SqlCipherError::NotSupported)
    }
}

/// Encrypts the database at `database_path` with `key` if it's still plaintext, so it can then be
/// opened with options passed through [`connect_options_with_key`]. Fails with
/// [`SqlCipherError::NotSupported`] if SQLite is not built with SQLCipher.
///
/// The database is first opened with `key`. If that succeeds, it's already encrypted with `key`,
/// or it's a new database. If it can't be read, it's opened without a key: if that succeeds, it's
/// a plaintext database, which is then encrypted in place. Otherwise it's encrypted with another
/// key, or corrupted, failing with [`SqlCipherError::InvalidKeyOrCorrupted`].
///
/// The database must not be in use while this runs. Encrypting a plaintext database writes an
/// encrypted copy next to it, and then replaces the original with it. The copy keeps the
/// original's `auto_vacuum` and WAL journal mode, but not its `page_size`: it uses SQLCipher's
/// `cipher_page_size`, 4096 bytes by default. The data and schema are unaffected.
pub async fn encrypt_database_if_needed(
    database_path: &Path,
    key: &DatabaseKey,
) -> Result<(), SqlCipherError> {
    let key = key.as_bytes();
    ensure_available().await?;

    // left over from a migration interrupted before the copy replaced the original,
    // which is therefore still intact
    let migration_path = migration_path(database_path)?;
    remove_if_exists(&migration_path).await?;
    remove_if_exists(&journal_path(&migration_path, "-journal")).await?;

    // nothing to encrypt, SQLCipher creates it encrypted on connect
    if !tokio::fs::try_exists(database_path)
        .await
        .map_err(|err| SqlCipherError::io(database_path, err))?
    {
        return Ok(());
    }

    match try_open(database_path, Some(key)).await {
        Ok(()) => Ok(()),
        Err(err) if is_not_a_database(&err) => match try_open(database_path, None).await {
            Ok(()) => migrate_plaintext(database_path, &migration_path, key).await,
            Err(err) if is_not_a_database(&err) => Err(SqlCipherError::InvalidKeyOrCorrupted {
                path: database_path.to_path_buf(),
            }),
            Err(err) => Err(err.into()),
        },
        Err(err) => Err(err.into()),
    }
}

/// Opens the existing database, with `key` if given, and reads its schema, which fails with
/// `SQLITE_NOTADB` if the database can't be read that way.
async fn try_open(database_path: &Path, key: Option<&Key>) -> sqlx::Result<()> {
    let mut opts = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(false)
        .disable_statement_logging();
    if let Some(key) = key {
        opts = with_raw_key(opts, key);
    }

    let mut conn = opts.connect().await?;
    let result = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sqlite_master")
        .fetch_one(&mut conn)
        .await;
    if let Err(err) = conn.close().await {
        warn!("failed to close the connection used to check the database: {err}");
    }
    result.map(|_| ())
}

fn is_not_a_database(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some(SQLITE_NOTADB))
}

async fn migrate_plaintext(
    database_path: &Path,
    migration_path: &Path,
    key: &Key,
) -> Result<(), SqlCipherError> {
    info!(
        "encrypting the existing plaintext database at {}",
        database_path.display()
    );

    if let Err(source) = export_encrypted(database_path, migration_path, key).await {
        remove_if_exists(migration_path).await?;
        return Err(SqlCipherError::MigrationFailed {
            path: database_path.to_path_buf(),
            source,
        });
    }

    // closing the last connection checkpoints the WAL into the database and removes it, so these
    // only exist if something else still has the database open. They belong to the plaintext
    // database and would be replayed into the encrypted one, so they can't be kept.
    for suffix in ["-wal", "-shm"] {
        let path = journal_path(database_path, suffix);
        if remove_if_exists(&path).await? {
            warn!(
                "removed {} left over from the plaintext database",
                path.display()
            );
        }
    }

    tokio::fs::rename(migration_path, database_path)
        .await
        .map_err(|err| SqlCipherError::io(database_path, err))?;

    debug!("encrypted the database at {}", database_path.display());
    Ok(())
}

/// Copies the plaintext database at `database_path` into a new database at `migration_path`,
/// encrypted with `key`, using SQLCipher's `sqlcipher_export`.
async fn export_encrypted(
    database_path: &Path,
    migration_path: &Path,
    key: &Key,
) -> sqlx::Result<()> {
    let migration_path_str = migration_path.to_str().ok_or_else(|| {
        sqlx::Error::Configuration(
            format!(
                "database path {} is not valid UTF-8",
                migration_path.display()
            )
            .into(),
        )
    })?;

    // attached databases inherit the open flags of the connection, so creating the encrypted copy
    // needs `SQLITE_OPEN_CREATE`. The plaintext database exists already, so it isn't created.
    let mut conn = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true)
        .disable_statement_logging()
        .connect()
        .await?;

    let result = async {
        sqlx::query("ATTACH DATABASE ?1 AS encrypted KEY ?2")
            .bind(migration_path_str)
            .bind(raw_key_literal(key).as_str())
            .execute(&mut conn)
            .await?;
        // `sqlcipher_export` doesn't copy `auto_vacuum`, and it can only be set before the first
        // table is created
        let auto_vacuum: i64 = sqlx::query_scalar("PRAGMA main.auto_vacuum")
            .fetch_one(&mut conn)
            .await?;
        sqlx::query(AssertSqlSafe(format!(
            "PRAGMA encrypted.auto_vacuum = {auto_vacuum}"
        )))
        .execute(&mut conn)
        .await?;
        sqlx::query("SELECT sqlcipher_export('encrypted')")
            .execute(&mut conn)
            .await?;

        // the copy is exported in rollback journal mode, so all of it is in the one file that
        // replaces the original. WAL mode is stored in the database, so switch the copy over
        // only once it's complete.
        let journal_mode: String = sqlx::query_scalar("PRAGMA main.journal_mode")
            .fetch_one(&mut conn)
            .await?;
        if journal_mode.eq_ignore_ascii_case("wal") {
            sqlx::query("PRAGMA encrypted.journal_mode = WAL")
                .execute(&mut conn)
                .await?;
        }
        sqlx::query("DETACH DATABASE encrypted")
            .execute(&mut conn)
            .await?;
        Ok(())
    }
    .await;

    let closed = conn.close().await;
    result.and(closed)
}

/// `x'<HEX>'`: tells SQLCipher to use the key as is, without passphrase key derivation.
fn raw_key_literal(key: &Key) -> Zeroizing<String> {
    let hex = Zeroizing::new(hex::encode_upper(key));
    Zeroizing::new(format!("x'{}'", hex.as_str()))
}

fn migration_path(database_path: &Path) -> Result<PathBuf, SqlCipherError> {
    let mut file_name = database_path
        .file_name()
        .ok_or_else(|| SqlCipherError::InvalidPath {
            path: database_path.to_path_buf(),
        })?
        .to_owned();
    file_name.push(".");
    file_name.push(MIGRATION_EXTENSION);
    Ok(database_path.with_file_name(file_name))
}

/// Path of a SQLite sidecar file, e.g. `<db>-wal`.
fn journal_path(database_path: &Path, suffix: &str) -> PathBuf {
    let mut path = database_path.as_os_str().to_owned();
    path.push(suffix);
    PathBuf::from(path)
}

/// Removes the file at `path`, returning whether it existed.
async fn remove_if_exists(path: &Path) -> Result<bool, SqlCipherError> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(SqlCipherError::io(path, err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteAutoVacuum, SqliteJournalMode};
    use tempfile::TempDir;

    fn key() -> DatabaseKey {
        DatabaseKey::new([0x42; KEY_LEN])
    }

    fn other_key() -> DatabaseKey {
        DatabaseKey::new([0x24; KEY_LEN])
    }

    #[test]
    fn key_from_hex() {
        let parsed = DatabaseKey::from_hex(&"42".repeat(KEY_LEN)).unwrap();
        assert_eq!(parsed.as_bytes(), key().as_bytes());
        assert!(DatabaseKey::from_hex(&format!(" {} ", "aB".repeat(KEY_LEN))).is_ok());

        assert!(matches!(DatabaseKey::from_hex(""), Err(InvalidHexKey)));
        assert!(DatabaseKey::from_hex(&"42".repeat(KEY_LEN - 1)).is_err());
        assert!(DatabaseKey::from_hex(&"42".repeat(KEY_LEN + 1)).is_err());
        assert!(DatabaseKey::from_hex(&"zz".repeat(KEY_LEN)).is_err());
    }
    const SQLITE_HEADER: &[u8] = b"SQLite format 3\0";
    const ROWS: i64 = 100;

    fn database_path(dir: &TempDir) -> PathBuf {
        dir.path().join("test.db")
    }

    /// Connect options matching how the storages open their databases.
    fn storage_options(path: &Path) -> SqliteConnectOptions {
        SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
    }

    fn row_value(i: i64) -> String {
        format!("secret-row-{i:04}")
    }

    async fn create_plaintext_database(path: &Path, auto_vacuum: SqliteAutoVacuum) {
        let mut conn = storage_options(path)
            .auto_vacuum(auto_vacuum)
            .connect()
            .await
            .unwrap();
        sqlx::query("CREATE TABLE data (id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
            .execute(&mut conn)
            .await
            .unwrap();
        for i in 0..ROWS {
            sqlx::query("INSERT INTO data (id, value) VALUES (?, ?)")
                .bind(i)
                .bind(row_value(i))
                .execute(&mut conn)
                .await
                .unwrap();
        }
        conn.close().await.unwrap();
    }

    async fn connect_encrypted(path: &Path, key: &DatabaseKey) -> SqliteConnection {
        connect_options_with_key(storage_options(path).create_if_missing(false), key)
            .connect()
            .await
            .unwrap()
    }

    async fn read_rows(conn: &mut SqliteConnection) -> Vec<(i64, String)> {
        sqlx::query_as("SELECT id, value FROM data ORDER BY id")
            .fetch_all(conn)
            .await
            .unwrap()
    }

    async fn read_auto_vacuum(conn: &mut SqliteConnection) -> i64 {
        sqlx::query_scalar("PRAGMA auto_vacuum")
            .fetch_one(conn)
            .await
            .unwrap()
    }

    fn expected_rows() -> Vec<(i64, String)> {
        (0..ROWS).map(|i| (i, row_value(i))).collect()
    }

    fn directory_entries(dir: &TempDir) -> Vec<String> {
        let mut entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        entries.sort();
        entries
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    #[tokio::test]
    async fn encrypts_plaintext_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = database_path(&dir);
        create_plaintext_database(&path, SqliteAutoVacuum::Incremental).await;
        assert!(std::fs::read(&path).unwrap().starts_with(SQLITE_HEADER));

        encrypt_database_if_needed(&path, &key()).await.unwrap();

        // nothing in the file is readable anymore
        let raw = std::fs::read(&path).unwrap();
        assert!(!raw.starts_with(SQLITE_HEADER));
        assert!(!contains(&raw, row_value(0).as_bytes()));
        assert!(!contains(&raw, b"CREATE TABLE"));

        // no copy, journal or WAL files left behind
        assert_eq!(directory_entries(&dir), vec!["test.db".to_string()]);

        let mut conn = connect_encrypted(&path, &key()).await;
        assert_eq!(read_rows(&mut conn).await, expected_rows());
        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(journal_mode, "wal");
        conn.close().await.unwrap();
    }

    #[tokio::test]
    async fn encrypted_database_cannot_be_opened_without_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = database_path(&dir);
        create_plaintext_database(&path, SqliteAutoVacuum::Incremental).await;

        encrypt_database_if_needed(&path, &key()).await.unwrap();

        let err = try_open(&path, None).await.unwrap_err();
        assert!(is_not_a_database(&err));
        let err = try_open(&path, Some(other_key().as_bytes()))
            .await
            .unwrap_err();
        assert!(is_not_a_database(&err));
    }

    #[tokio::test]
    async fn preserves_auto_vacuum() {
        let cases = [
            (SqliteAutoVacuum::None, 0),
            (SqliteAutoVacuum::Full, 1),
            (SqliteAutoVacuum::Incremental, 2),
        ];

        for (auto_vacuum, expected) in cases {
            let dir = tempfile::tempdir().unwrap();
            let path = database_path(&dir);
            create_plaintext_database(&path, auto_vacuum).await;

            encrypt_database_if_needed(&path, &key()).await.unwrap();

            // opened without setting `auto_vacuum`, to read what the file has
            let mut conn =
                connect_options_with_key(SqliteConnectOptions::new().filename(&path), &key())
                    .connect()
                    .await
                    .unwrap();
            assert_eq!(read_auto_vacuum(&mut conn).await, expected);
            assert_eq!(read_rows(&mut conn).await, expected_rows());
            conn.close().await.unwrap();
        }
    }

    #[tokio::test]
    async fn preserves_journal_mode() {
        let cases = [
            (SqliteJournalMode::Wal, "wal"),
            (SqliteJournalMode::Delete, "delete"),
        ];

        for (journal_mode, expected) in cases {
            let dir = tempfile::tempdir().unwrap();
            let path = database_path(&dir);
            let mut conn = storage_options(&path)
                .journal_mode(journal_mode)
                .connect()
                .await
                .unwrap();
            sqlx::query("CREATE TABLE data (id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
                .execute(&mut conn)
                .await
                .unwrap();
            conn.close().await.unwrap();

            encrypt_database_if_needed(&path, &key()).await.unwrap();

            assert_eq!(directory_entries(&dir), vec!["test.db".to_string()]);
            // opened without setting `journal_mode`, to read what the file has
            let mut conn =
                connect_options_with_key(SqliteConnectOptions::new().filename(&path), &key())
                    .connect()
                    .await
                    .unwrap();
            let actual: String = sqlx::query_scalar("PRAGMA journal_mode")
                .fetch_one(&mut conn)
                .await
                .unwrap();
            conn.close().await.unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[tokio::test]
    async fn encrypted_database_is_left_as_is() {
        let dir = tempfile::tempdir().unwrap();
        let path = database_path(&dir);
        create_plaintext_database(&path, SqliteAutoVacuum::Incremental).await;
        encrypt_database_if_needed(&path, &key()).await.unwrap();
        let encrypted = std::fs::read(&path).unwrap();

        encrypt_database_if_needed(&path, &key()).await.unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), encrypted);
        let mut conn = connect_encrypted(&path, &key()).await;
        assert_eq!(read_rows(&mut conn).await, expected_rows());
        conn.close().await.unwrap();
    }

    #[tokio::test]
    async fn wrong_key_fails_without_touching_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = database_path(&dir);
        create_plaintext_database(&path, SqliteAutoVacuum::Incremental).await;
        encrypt_database_if_needed(&path, &key()).await.unwrap();
        let encrypted = std::fs::read(&path).unwrap();

        let err = encrypt_database_if_needed(&path, &other_key())
            .await
            .unwrap_err();

        assert!(matches!(err, SqlCipherError::InvalidKeyOrCorrupted { .. }));
        assert_eq!(std::fs::read(&path).unwrap(), encrypted);
        let mut conn = connect_encrypted(&path, &key()).await;
        assert_eq!(read_rows(&mut conn).await, expected_rows());
        conn.close().await.unwrap();
    }

    #[tokio::test]
    async fn missing_database_is_not_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = database_path(&dir);

        encrypt_database_if_needed(&path, &key()).await.unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn migration_path_requires_file_name() {
        for path in ["/", "db/.."] {
            assert!(matches!(
                migration_path(Path::new(path)),
                Err(SqlCipherError::InvalidPath { .. })
            ));
        }
    }

    #[tokio::test]
    async fn leftover_migration_files_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = database_path(&dir);
        let migration = migration_path(&path).unwrap();
        std::fs::write(&migration, b"partial copy").unwrap();
        std::fs::write(journal_path(&migration, "-journal"), b"journal").unwrap();
        create_plaintext_database(&path, SqliteAutoVacuum::Incremental).await;

        encrypt_database_if_needed(&path, &key()).await.unwrap();

        assert_eq!(directory_entries(&dir), vec!["test.db".to_string()]);
        let mut conn = connect_encrypted(&path, &key()).await;
        assert_eq!(read_rows(&mut conn).await, expected_rows());
        conn.close().await.unwrap();
    }
}
