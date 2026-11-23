use std::{
    fs, path::Path, sync::{Mutex, MutexGuard, OnceLock},
};

use rusqlite::{Connection, Result};

use crate::runtime::db::schema;

static DB_CONN: OnceLock<Mutex<Connection>> = OnceLock::new();

pub async fn init_db(db_path: &str) -> Result<()> {
    if let Some(parent) = Path::new(&db_path).parent() {
        let _ = fs::create_dir_all(parent);
    }

    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;",
    )?;

    schema::create_tables(&conn)?;

    DB_CONN
        .set(Mutex::new(conn))
        .map_err(|_| rusqlite::Error::InvalidQuery)?;

    Ok(())
}

pub fn get_conn() -> MutexGuard<'static, Connection> {
    DB_CONN
    .get()
    .expect("Database is not initialized. Error database")
    .lock()
    .expect("Mutex database posioned")
}