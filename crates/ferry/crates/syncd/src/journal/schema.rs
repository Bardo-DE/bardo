//! The journal file's schema. A file of any other version than this build's is emptied and made
//! again: the journal is rebuilt from the replica, never carried forward.

use super::JournalError;
use rusqlite::Connection;

/// Items, anchors, tombstones, conflicts (design/31 §6.1 "Journal").
const SCHEMA: &str = "
CREATE TABLE items (
    local_id        TEXT PRIMARY KEY NOT NULL,
    remote_id       TEXT UNIQUE,
    path            TEXT NOT NULL,
    size            INTEGER NOT NULL,
    hash            TEXT,
    remote_version  TEXT,
    base_version    TEXT,
    state           TEXT NOT NULL
);
CREATE TABLE anchors (
    id          INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
    anchor      TEXT NOT NULL,
    updated_at  INTEGER NOT NULL
);
CREATE TABLE tombstones (
    remote_id   TEXT PRIMARY KEY NOT NULL,
    version     TEXT NOT NULL,
    deleted_at  INTEGER NOT NULL,
    origin      TEXT NOT NULL,
    ack         TEXT NOT NULL
);
CREATE TABLE conflicts (
    number      INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    item        TEXT NOT NULL,
    base        TEXT,
    remote      TEXT NOT NULL,
    local_id    TEXT NOT NULL,
    local_hash  TEXT,
    at          INTEGER NOT NULL
);
";

/// The schema version this build writes, kept in the file's `user_version`.
const VERSION: u32 = 1;

/// Makes `connection` hold this build's schema: a file of another version loses its tables and
/// gets the current ones.
pub(crate) fn prepare(connection: &mut Connection) -> Result<(), JournalError> {
    prepare_with(connection, SCHEMA, VERSION)
}

/// [`prepare`] for any `schema` and `version`; the drive listing's file has its own.
pub(crate) fn prepare_with(
    connection: &mut Connection,
    schema: &str,
    version: u32,
) -> Result<(), JournalError> {
    let have: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if have == version {
        return Ok(());
    }
    let tx = connection.transaction()?;
    let tables = tx
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for table in tables {
        tx.execute_batch(&format!("DROP TABLE \"{table}\";"))?;
    }
    tx.execute_batch(schema)?;
    tx.execute_batch(&format!("PRAGMA user_version = {version};"))?;
    tx.commit()?;
    Ok(())
}
