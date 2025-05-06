// src/models.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
// Removed: use sqlx::FromRow;

// 用于数据库交互的 Note 结构体
// Removed FromRow, Updated tags type
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Note {
    pub id: i64,
    pub content: String,
    pub tags: Vec<String>, // <<< Changed from String to Vec<String>
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// 用于创建新笔记的请求体结构 (Remains the same)
#[derive(Deserialize, Debug)]
pub struct CreateNotePayload {
    pub content: String,
    pub tags: Option<Vec<String>>,
    pub created_at: Option<DateTime<Utc>>,
}

// 用于更新笔记的请求体结构 (Remains the same)
#[derive(Deserialize, Debug)]
pub struct UpdateNotePayload {
    pub content: String,
    pub tags: Option<Vec<String>>,
}

// 用于 API 响应的笔记结构 (Remains the same, tags is Vec<String>)
#[derive(Serialize, Debug)]
pub struct NoteResponse {
   pub id: i64,
   pub content: String,
   pub tags: Vec<String>, // API 层面返回 Vec<String>
   pub created_at: String, // ISO 8601 格式字符串
   pub updated_at: String, // ISO 8601 格式字符串
}

// 用于数据库交互和 API 响应的 Tag 结构体
// Removed FromRow
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    // pub path: String, // 根据需要添加
}

// 用于 API 响应的详细标签结构
// Updated last_modified type
#[derive(Serialize, Debug)]
pub struct DetailedTag {
    pub name: String,
    pub count: i64,
    // Changed to DateTime<Utc> to match data from db layer
    // Serde chrono feature usually handles serialization to string automatically
    pub last_modified: Option<DateTime<Utc>>, // <<< Changed from Option<String>
}

// 用于数据库交互和 API 响应的 Comment 结构体
// Removed FromRow
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Comment {
    pub id: i64,
    pub note_id: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

// 用于创建评论的请求体结构 (Remains the same)
#[derive(Deserialize, Debug)]
pub struct CreateCommentPayload {
    pub content: String,
}

// 用于 API 响应的评论结构 (Remains the same)
#[derive(Serialize, Debug)]
pub struct CommentResponse {
    pub id: i64,
    pub content: String,
    pub created_at: String, // ISO 8601 格式字符串
}