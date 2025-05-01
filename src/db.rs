use sqlx::{sqlite::SqlitePoolOptions, SqlitePool, Error};
use std::env;
use crate::models::{Note, CreateNotePayload, UpdateNotePayload};
use chrono::{DateTime, Utc};

// 数据库连接池类型别名
pub type DbPool = SqlitePool;

// 数据库文件路径，可以从环境变量读取或使用默认值
const DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";
const DEFAULT_DATABASE_URL: &str = "sqlite:inbox.db"; // 默认指向项目根目录的 inbox.db

// 初始化数据库连接池
pub async fn init_pool() -> Result<DbPool, Error> {
    let database_url = env::var(DATABASE_URL_ENV_VAR)
        .unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());

    println!("🗄️ 连接到数据库: {}", database_url);

    // 确保数据库文件存在，如果不存在则创建
    if !std::path::Path::new(database_url.trim_start_matches("sqlite:")).exists() {
        println!("数据库文件不存在，正在创建...");
        std::fs::File::create(database_url.trim_start_matches("sqlite:"))
            .map_err(|e| sqlx::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5) // 根据需要调整连接池大小
        .connect(&database_url)
        .await?;

    Ok(pool)
}

// 初始化测试数据库连接池
pub async fn init_db(database_url: &str) -> Result<DbPool, Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn migrate(pool: &DbPool) -> Result<(), Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            tags TEXT DEFAULT '[]',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#
    ).execute(pool).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            note_id INTEGER NOT NULL,
            content TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
        );
        "#
    ).execute(pool).await?;
    println!("✅ 数据库迁移完成");
    Ok(())
}


// 创建新笔记
pub async fn create_note_db(pool: &DbPool, payload: CreateNotePayload) -> Result<Note, Error> {
    let created_at = payload.created_at.unwrap_or_else(Utc::now);
    let updated_at = created_at; // 初始创建时，更新时间等于创建时间
    let tags_json = serde_json::to_string(&payload.tags.unwrap_or_default())
        .map_err(|e| sqlx::Error::Decode(Box::new(e)))?; // 将 Vec<String> 序列化为 JSON 字符串

    let result = sqlx::query(
        r#"
        INSERT INTO notes (content, tags, created_at, updated_at)
        VALUES (?, ?, ?, ?)
        "#
    )
    .bind(&payload.content)
    .bind(&tags_json)
    .bind(created_at)
    .bind(updated_at)
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();

    // 返回创建的笔记对象
    Ok(Note {
        id,
        content: payload.content,
        tags: tags_json,
        created_at,
        updated_at,
    })
}

// 获取单条笔记
pub async fn get_note_db(pool: &DbPool, note_id: i64) -> Result<Option<Note>, Error> {
    let note = sqlx::query_as::<_, Note>(
        "SELECT id, content, tags, created_at, updated_at FROM notes WHERE id = ?"
    )
    .bind(note_id)
    .fetch_optional(pool)
    .await?;
    Ok(note)
}

// 获取笔记列表 (带过滤和分页)
pub async fn get_notes_db(
    pool: &DbPool,
    limit: Option<i64>,
    tag: Option<String>,
    created_after: Option<DateTime<Utc>>,
    created_before: Option<DateTime<Utc>>,
) -> Result<Vec<Note>, Error> {
    let mut query_str = "SELECT id, content, tags, created_at, updated_at FROM notes WHERE 1=1".to_string();
    let mut conditions = Vec::<String>::new();

    if tag.is_some() {
        conditions.push("tags LIKE ?".to_string());
    }
    if created_after.is_some() {
        conditions.push("created_at >= ?".to_string());
    }
    if created_before.is_some() {
        conditions.push("created_at < ?".to_string());
    }

    if !conditions.is_empty() {
        query_str.push_str(" AND ");
        query_str.push_str(&conditions.join(" AND "));
    }

    query_str.push_str(" ORDER BY created_at DESC");

    if let Some(l) = limit {
        query_str.push_str(&format!(" LIMIT {}", l)); // LIMIT 不接受占位符
    }

    // 使用 query_as! 构建查询
    let mut query = sqlx::query_as::<_, Note>(&query_str);

    // 按顺序绑定参数
    if let Some(t) = tag {
        query = query.bind(format!("%\"{}\"%", t));
    }
    if let Some(after) = created_after {
        query = query.bind(after);
    }
    if let Some(before) = created_before {
        query = query.bind(before);
    }

    // 执行查询
    let notes = query.fetch_all(pool).await?;

    Ok(notes)
}

// 更新笔记
pub async fn update_note_db(
    pool: &DbPool,
    note_id: i64,
    payload: UpdateNotePayload,
) -> Result<Option<Note>, Error> {
    let updated_at = Utc::now();
    let tags_json = serde_json::to_string(&payload.tags.unwrap_or_default())
        .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

    let result = sqlx::query(
        r#"
        UPDATE notes
        SET content = ?, tags = ?, updated_at = ?
        WHERE id = ?
        "#
    )
    .bind(&payload.content)
    .bind(&tags_json)
    .bind(updated_at)
    .bind(note_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        // 如果没有行被更新，说明笔记不存在
        return Ok(None);
    }

    // 获取并返回更新后的笔记
    let updated_note = sqlx::query_as::<_, Note>(
        "SELECT id, content, tags, created_at, updated_at FROM notes WHERE id = ?"
    )
    .bind(note_id)
    .fetch_one(pool)
    .await?;

    Ok(Some(updated_note))
}

// 删除笔记
pub async fn delete_note_db(pool: &DbPool, note_id: i64) -> Result<bool, Error> {
    // 首先删除关联的评论 (如果需要保持外键约束)
    sqlx::query("DELETE FROM comments WHERE note_id = ?")
        .bind(note_id)
        .execute(pool)
        .await?;

    // 然后删除笔记本身
    let result = sqlx::query("DELETE FROM notes WHERE id = ?")
        .bind(note_id)
        .execute(pool)
        .await?;

    // 如果有行被删除，则返回 true
    Ok(result.rows_affected() > 0)
}


// 获取所有唯一标签
pub async fn get_all_tags_db(pool: &DbPool) -> Result<Vec<String>, Error> {
    use sqlx::Row;
    let rows = sqlx::query("SELECT tags FROM notes WHERE tags IS NOT NULL")
        .fetch_all(pool)
        .await?;
    let mut tag_set = std::collections::HashSet::new();
    for row in rows {
        let tags_json: String = row.get(0);
        if let Ok(tags) = serde_json::from_str::<Vec<String>>(&tags_json) {
            for tag in tags {
                tag_set.insert(tag);
            }
        }
    }
    Ok(tag_set.into_iter().collect())
}

// 获取详细标签信息
use crate::models::DetailedTag;
use sqlx::Row;

pub async fn get_detailed_tags_db(pool: &DbPool) -> Result<Vec<DetailedTag>, Error> {
    let rows = sqlx::query(
        r#"
        SELECT json_each.value as tag, COUNT(*) as count, MAX(updated_at) as last_modified
        FROM notes, json_each(notes.tags)
        GROUP BY tag
        ORDER BY count DESC
        "#
    )
    .fetch_all(pool)
    .await?;
    let mut result = Vec::new();
    for row in rows {
        let name: String = row.get("tag");
        let count: i64 = row.get("count");
        let last_modified: Option<String> = row.get("last_modified");
        result.push(DetailedTag {
            name,
            count,
            last_modified,
        });
    }
    Ok(result)
}

// 后续将在此处添加数据库操作函数，例如：
// pub async fn get_tags_db(...) -> Result<Vec<Tag>, Error> { ... }
// ... 等等