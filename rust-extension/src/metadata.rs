use rusqlite::{params, Connection, Result};
use std::path::PathBuf;

pub struct IndexMeta {
    pub file_path: String,
    pub file_hash: String,
    #[allow(dead_code)]
    pub indexed_at: String,
}

pub fn get_db_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("data")
        .join("index.db")
}

fn ensure_data_dir() {
    let data_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("data");
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).ok();
    }
}

fn open_db() -> Result<Connection> {
    ensure_data_dir();
    let db_path = get_db_path();
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS index_meta (
            file_path TEXT PRIMARY KEY NOT NULL,
            file_hash TEXT NOT NULL,
            indexed_at TEXT NOT NULL
        )",
    )?;
    Ok(conn)
}

pub fn insert_meta(file_path: &str, file_hash: &str, indexed_at: &str) -> Result<()> {
    let conn = open_db()?;
    conn.execute(
        "INSERT OR REPLACE INTO index_meta (file_path, file_hash, indexed_at) VALUES (?1, ?2, ?3)",
        params![file_path, file_hash, indexed_at],
    )?;
    Ok(())
}

pub fn get_all_meta() -> Result<Vec<IndexMeta>> {
    let conn = open_db()?;
    let mut stmt =
        conn.prepare("SELECT file_path, file_hash, indexed_at FROM index_meta")?;
    let rows = stmt.query_map([], |row| {
        Ok(IndexMeta {
            file_path: row.get(0)?,
            file_hash: row.get(1)?,
            indexed_at: row.get(2)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn delete_meta(file_path: &str) -> Result<()> {
    let conn = open_db()?;
    conn.execute("DELETE FROM index_meta WHERE file_path = ?1", params![file_path])?;
    Ok(())
}