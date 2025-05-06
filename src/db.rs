// src/db.rs
use rusqlite::{params, Connection, Error, Row, ToSql}; // Ensure rusqlite is in Cargo.toml!
use std::env;
use std::path::Path;
use crate::models::{Note, CreateNotePayload, UpdateNotePayload, DetailedTag}; // Ensure Note has tags: Vec<String>
use chrono::{DateTime, Utc};
use serde_json;

// --- 错误处理助手 ---
fn map_serde_error(e: serde_json::Error) -> Error {
    Error::InvalidParameterName(format!("JSON serialization/deserialization error: {}", e))
}

// --- 数据库连接类型 ---
pub type DbConnection = Connection;

// --- 常量 ---
const DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";
const DEFAULT_DATABASE_URL: &str = "inbox.db";

// --- 初始化 ---
pub async fn init_pool() -> Result<DbConnection, Error> {
    let database_url = if cfg!(target_os = "android") {
        // Android环境下使用应用私有数据目录
        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| ".".to_string());
        let db_path = Path::new(&data_dir).join(DEFAULT_DATABASE_URL);
        
        // 确保父目录存在
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                    Some(format!("Failed to create parent directory: {}", e)),
                ))?;
            }
        }
        
        db_path.to_string_lossy().into_owned()
    } else {
        // 非Android环境保持原有逻辑
        env::var(DATABASE_URL_ENV_VAR)
            .unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string())
    };

    println!("🗄️ 连接到数据库 (同步): {}", database_url);

    let db_path = Path::new(&database_url);
    let conn = Connection::open(db_path)?;
    conn.execute("PRAGMA foreign_keys = ON;", [])?;
    Ok(conn)
}

// --- 迁移 ---
pub fn migrate(conn: &DbConnection) -> Result<(), Error> {
    conn.execute_batch(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            tags TEXT DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            note_id INTEGER NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
        );
        COMMIT;
        "#
    )?;
    
    println!("✅ 数据库迁移完成");
    Ok(())
}

// --- 笔记的 CRUD 操作 ---

fn map_row_to_note(row: &Row) -> Result<Note, Error> {
    let tags_json: String = row.get("tags")?;
    // Assuming Note in models.rs has tags: Vec<String>
    let tags: Vec<String> = serde_json::from_str(&tags_json).map_err(map_serde_error)?;
    let created_at: DateTime<Utc> = row.get("created_at")?;
    let updated_at: DateTime<Utc> = row.get("updated_at")?;

    Ok(Note {
        id: row.get("id")?,
        content: row.get("content")?,
        tags, // Store parsed Vec<String>
        created_at,
        updated_at,
    })
}

pub fn create_note_db(conn: &mut DbConnection, payload: CreateNotePayload) -> Result<Note, Error> {
    let created_at = payload.created_at.unwrap_or_else(Utc::now);
    let updated_at = created_at;
    let tags_json = serde_json::to_string(&payload.tags.unwrap_or_default())
        .map_err(map_serde_error)?;

    let tx = conn.transaction()?;
    tx.execute(
        r#"
        INSERT INTO notes (content, tags, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        params![
            payload.content,
            tags_json,
            created_at,
            updated_at,
        ],
    )?;

    let id = tx.last_insert_rowid();
    tx.commit()?;

    let parsed_tags: Vec<String> = serde_json::from_str(&tags_json).map_err(map_serde_error)?;

    Ok(Note {
        id,
        content: payload.content,
        tags: parsed_tags, // Ensure Note struct expects Vec<String>
        created_at,
        updated_at,
    })
}

pub fn get_note_db(conn: &DbConnection, note_id: i64) -> Result<Option<Note>, Error> {
    let mut stmt = conn.prepare(
        "SELECT id, content, tags, created_at, updated_at FROM notes WHERE id = ?1"
    )?;
    let result = stmt.query_row(params![note_id], map_row_to_note);

    match result {
        Ok(note) => Ok(Some(note)),
        Err(Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_notes_db(
    conn: &DbConnection,
    limit: Option<i64>,
    tag: Option<String>,
    created_after: Option<DateTime<Utc>>,
    created_before: Option<DateTime<Utc>>,
) -> Result<Vec<Note>, Error> {
    let mut query_str = "SELECT id, content, tags, created_at, updated_at FROM notes WHERE 1=1".to_string();
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();

    if let Some(t) = tag {
        query_str.push_str(" AND tags LIKE ?");
        params_vec.push(Box::new(format!("%\"{}\"%", t)));
    }
    if let Some(after) = created_after {
        query_str.push_str(" AND created_at >= ?");
        params_vec.push(Box::new(after));
    }
    if let Some(before) = created_before {
        query_str.push_str(" AND created_at < ?");
        params_vec.push(Box::new(before));
    }

    query_str.push_str(" ORDER BY created_at DESC");

    if let Some(l) = limit {
        query_str.push_str(&format!(" LIMIT {}", l));
    }

    let mut final_query_str = String::new();
    let mut param_index = 1;
    for c in query_str.chars() {
        if c == '?' {
            final_query_str.push_str(&format!("?{}", param_index));
            param_index += 1;
        } else {
            final_query_str.push(c);
        }
    }

    let mut stmt = conn.prepare(&final_query_str)?;
    let params_ref: Vec<&dyn ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

    // *** MUST FIX THIS LINE LOCALLY: Remove '¶', use 'params_ref' ***
    let notes_iter = stmt.query_map(&params_ref[..], map_row_to_note)?;

    let mut notes = Vec::new();
    for note_result in notes_iter {
        notes.push(note_result?);
    }

    Ok(notes)
}

pub fn update_note_db(
    conn: &mut DbConnection,
    note_id: i64,
    payload: UpdateNotePayload,
) -> Result<Option<Note>, Error> {
    let updated_at = Utc::now();
    let tags_json = serde_json::to_string(&payload.tags.unwrap_or_default())
        .map_err(map_serde_error)?;

    let rows_affected = conn.execute(
        r#"
        UPDATE notes
        SET content = ?1, tags = ?2, updated_at = ?3
        WHERE id = ?4
        "#,
        params![
            payload.content,
            tags_json,
            updated_at,
            note_id
        ],
    )?;

    if rows_affected == 0 {
        Ok(None)
    } else {
        get_note_db(conn, note_id)
    }
}

pub fn delete_note_db(conn: &mut DbConnection, note_id: i64) -> Result<bool, Error> {
    let rows_affected = conn.execute(
        "DELETE FROM notes WHERE id = ?1",
        params![note_id],
    )?;
    Ok(rows_affected > 0)
}

// --- 标签操作 ---

pub fn get_all_tags_db(conn: &DbConnection) -> Result<Vec<String>, Error> {
    let mut stmt = conn.prepare("SELECT tags FROM notes WHERE json_valid(tags) AND json_type(tags) = 'array'")?;
    let rows_iter = stmt.query_map(params![], |row| row.get::<_, String>(0))?;

    // *** Attempt to fix E0277 by collecting results first ***
    let tags_json_results: Vec<Result<String, Error>> = rows_iter.collect();

    let mut tag_set = std::collections::HashSet::new();
    for row_result in tags_json_results {
        match row_result {
            Ok(tags_json) => { // tags_json is String
                if let Ok(tags) = serde_json::from_str::<Vec<String>>(&tags_json) {
                     for tag in tags {
                        tag_set.insert(tag);
                    }
                } else {
                     eprintln!("警告：无法从数据库解析标签 JSON：{}", tags_json);
                }
            }
            Err(e) => {
                // Propagate error from collection step
                return Err(e);
            }
        }
    }
    Ok(tag_set.into_iter().collect())
}


pub fn get_detailed_tags_db(conn: &DbConnection) -> Result<Vec<DetailedTag>, Error> {
    let mut stmt = conn.prepare(
        r#"
        SELECT
            jt.value as tag_name,
            COUNT(*) as count,
            MAX(n.updated_at) as last_modified
        FROM
            notes n, json_each(n.tags) jt
        WHERE json_valid(n.tags) AND json_type(n.tags) = 'array'
        GROUP BY
            jt.value
        ORDER BY
            count DESC;
        "#
    )?;

    let tag_iter = stmt.query_map(params![], |row| {
        let last_modified: Option<DateTime<Utc>> = row.get("last_modified")?;
        Ok(DetailedTag {
            name: row.get("tag_name")?,
            count: row.get("count")?,
            last_modified,
        })
    })?;

    let mut result = Vec::new();
    for tag_result in tag_iter {
        result.push(tag_result?);
    }
    Ok(result)
}