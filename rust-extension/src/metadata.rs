use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use crate::file_utils;

pub struct IndexMeta {
    pub file_path: String,
    pub file_hash: String,
    #[allow(dead_code)]
    pub indexed_at: String,
}
pub fn get_db_path() -> PathBuf {
    file_utils::get_data_dir().join("index.db")
}

fn open_db() -> Result<Connection> {
    file_utils::ensure_data_dir().ok();
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

/// 清空所有元数据
pub fn clear_all() -> Result<()> {
    let conn = open_db()?;
    conn.execute("DELETE FROM index_meta", [])?;
    Ok(())
}

/// 按可选条件查询元数据。
pub fn query_meta(
    path_filter: Option<&str>,
    time_after: Option<&str>,
    time_before: Option<&str>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<IndexMeta>> {
    let conn = open_db()?;
    let mut sql = String::from("SELECT file_path, file_hash, indexed_at FROM index_meta WHERE 1=1");
    let mut param_values: Vec<String> = Vec::new();

    if let Some(f) = path_filter {
        if !f.is_empty() {
            sql.push_str(" AND file_path LIKE ?");
            param_values.push(format!("%{}%", f));
        }
    }
    if let Some(t) = time_after {
        if !t.is_empty() {
            sql.push_str(" AND indexed_at >= ?");
            param_values.push(t.to_string());
        }
    }
    if let Some(t) = time_before {
        if !t.is_empty() {
            sql.push_str(" AND indexed_at <= ?");
            param_values.push(t.to_string());
        }
    }
    sql.push_str(" ORDER BY indexed_at DESC");

    if let Some(l) = limit {
        sql.push_str(&format!(" LIMIT {}", l));
    }
    if let Some(o) = offset {
        sql.push_str(&format!(" OFFSET {}", o));
    }

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_refs.as_slice(), |row| {
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

/// 返回匹配筛选条件的记录总数（与 query_meta 相同的 WHERE 条件，不加 LIMIT/OFFSET）。
pub fn count_meta(
    path_filter: Option<&str>,
    time_after: Option<&str>,
    time_before: Option<&str>,
) -> Result<i64> {
    let conn = open_db()?;
    let mut sql = String::from("SELECT COUNT(*) FROM index_meta WHERE 1=1");
    let mut param_values: Vec<String> = Vec::new();

    if let Some(f) = path_filter {
        if !f.is_empty() {
            sql.push_str(" AND file_path LIKE ?");
            param_values.push(format!("%{}%", f));
        }
    }
    if let Some(t) = time_after {
        if !t.is_empty() {
            sql.push_str(" AND indexed_at >= ?");
            param_values.push(t.to_string());
        }
    }
    if let Some(t) = time_before {
        if !t.is_empty() {
            sql.push_str(" AND indexed_at <= ?");
            param_values.push(t.to_string());
        }
    }

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();

    let count: i64 = conn.query_row(&sql, params_refs.as_slice(), |row| row.get(0))?;
    Ok(count)
}

/// 批量删除元数据
pub fn delete_meta_batch(file_paths: &[String]) -> Result<()> {
    if file_paths.is_empty() {
        return Ok(());
    }
    let conn = open_db()?;
    // Build: file_path IN (?1, ?2, ...)
    let placeholders: Vec<String> = (1..=file_paths.len())
        .map(|i| format!("?{}", i))
        .collect();
    let sql = format!("DELETE FROM index_meta WHERE file_path IN ({})", placeholders.join(", "));
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        file_paths.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    conn.execute(&sql, params_refs.as_slice())?;
    Ok(())
}