use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// 用于数据库交互的 Note 结构体
#[derive(Serialize, Deserialize, FromRow, Debug, Clone)]
pub struct Note {
    pub id: i64,
    pub content: String,
    pub tags: String, // 存储 JSON 字符串
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// 用于创建新笔记的请求体结构
#[derive(Deserialize, Debug)]
pub struct CreateNotePayload {
    pub content: String,
    pub tags: Option<Vec<String>>, // API 层面接收 Vec<String>
    pub created_at: Option<DateTime<Utc>>,
}

// 用于更新笔记的请求体结构
#[derive(Deserialize, Debug)]
pub struct UpdateNotePayload {
    pub content: String,
    pub tags: Option<Vec<String>>, // API 层面接收 Vec<String>
}

// 用于 API 响应的笔记结构
#[derive(Serialize, Debug)]
pub struct NoteResponse {
   pub id: i64,
   pub content: String,
   pub tags: Vec<String>, // API 层面返回 Vec<String>
   pub created_at: String, // ISO 8601 格式字符串
   pub updated_at: String, // ISO 8601 格式字符串
}

// 用于数据库交互和 API 响应的 Tag 结构体 (根据需要调整字段)
#[derive(Serialize, Deserialize, FromRow, Debug, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    // pub path: String, // 根据 Python 代码中的 get_child_tags 和 search_tags 可能需要
}

// 用于 API 响应的详细标签结构
#[derive(Serialize, Debug)]
pub struct DetailedTag {
    pub name: String,
    pub count: i64,
    pub last_modified: Option<String>, // ISO 8601 格式字符串
}

// 用于数据库交互和 API 响应的 Comment 结构体
#[derive(Serialize, Deserialize, FromRow, Debug, Clone)]
pub struct Comment {
    pub id: i64,
    pub note_id: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

// 用于创建评论的请求体结构
#[derive(Deserialize, Debug)]
pub struct CreateCommentPayload {
    pub content: String,
}

// 用于 API 响应的评论结构
#[derive(Serialize, Debug)]
pub struct CommentResponse {
    pub id: i64,
    pub content: String,
    pub created_at: String, // ISO 8601 格式字符串
}

// 其他模型可以后续添加