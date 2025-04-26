pub mod models;
pub mod db;

use axum::{
    routing::{get, post, put},
    Router,
    extract::{State, Json, Query, Path},
    response::{IntoResponse, Response},
    http::StatusCode,
};
use tower_http::cors::{Any, CorsLayer};
use serde::Deserialize;
use chrono::{DateTime, Utc};
use crate::models::{CreateNotePayload, NoteResponse, UpdateNotePayload};
use thiserror::Error;

#[derive(Clone)]
pub struct AppState {
    pub pool: db::DbPool,
}

#[derive(Deserialize, Debug)]
struct GetNotesQuery {
    limit: Option<i64>,
    tag: Option<String>,
    created_after: Option<DateTime<Utc>>,
    created_before: Option<DateTime<Utc>>,
}

pub async fn app(db_pool: db::DbPool) -> Router {
    let app_state = AppState { pool: db_pool };
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    Router::new()
        .route("/", get(root))
        .route("/inbox/notes", post(create_note_handler).get(get_notes_handler))
        .route("/inbox/notes/:note_id", get(get_note_handler).put(update_note_handler).delete(delete_note_handler))
        .route("/inbox/tags", get(get_tags_handler))
        .with_state(app_state)
        .layer(cors)
}

async fn root() -> &'static str {
    "📥 Welcome to Inbox Inbox Server (Rust Version)"
}

// --- API 处理程序 --- 

// GET /inbox/notes/:note_id 处理程序
async fn get_note_handler(
    State(state): State<AppState>,
    Path(note_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    match db::get_note_db(&state.pool, note_id).await? {
        Some(note) => {
            let response = NoteResponse {
                id: note.id,
                content: note.content,
                tags: serde_json::from_str(&note.tags).unwrap_or_default(),
                created_at: note.created_at.to_rfc3339(),
                updated_at: note.updated_at.to_rfc3339(),
            };
            Ok(Json(response))
        }
        None => Err(AppError::NotFound("未找到指定的笔记".to_string())),
    }
}

async fn get_notes_handler(
    State(state): State<AppState>,
    Query(params): Query<GetNotesQuery>,
) -> Result<impl IntoResponse, AppError> {
    let notes = db::get_notes_db(
        &state.pool,
        params.limit,
        params.tag,
        params.created_after,
        params.created_before,
    ).await?;
    let response: Vec<NoteResponse> = notes
        .into_iter()
        .map(|note| NoteResponse {
            id: note.id,
            content: note.content,
            tags: serde_json::from_str(&note.tags).unwrap_or_default(),
            created_at: note.created_at.to_rfc3339(),
            updated_at: note.updated_at.to_rfc3339(),
        })
        .collect();
    Ok(Json(response))
}

async fn create_note_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateNotePayload>,
) -> Result<impl IntoResponse, AppError> {
    if payload.content.trim().is_empty() {
        return Err(AppError::BadRequest("笔记内容不能为空".to_string()));
    }
    let note = db::create_note_db(&state.pool, payload).await?;
    let response = NoteResponse {
        id: note.id,
        content: note.content,
        tags: serde_json::from_str(&note.tags).unwrap_or_default(),
        created_at: note.created_at.to_rfc3339(),
        updated_at: note.updated_at.to_rfc3339(),
    };
    Ok((StatusCode::CREATED, Json(response)))
}

async fn update_note_handler(
    State(state): State<AppState>,
    Path(note_id): Path<i64>,
    Json(payload): Json<UpdateNotePayload>,
) -> Result<impl IntoResponse, AppError> {
    if payload.content.trim().is_empty() {
        return Err(AppError::BadRequest("笔记内容不能为空".to_string()));
    }
    match db::update_note_db(&state.pool, note_id, payload).await? {
        Some(note) => {
            let response = NoteResponse {
                id: note.id,
                content: note.content,
                tags: serde_json::from_str(&note.tags).unwrap_or_default(),
                created_at: note.created_at.to_rfc3339(),
                updated_at: note.updated_at.to_rfc3339(),
            };
            Ok(Json(response))
        }
        None => Err(AppError::NotFound("未找到指定的笔记".to_string())),
    }
}

async fn delete_note_handler(
    State(state): State<AppState>,
    Path(note_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = db::delete_note_db(&state.pool, note_id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("未找到指定的笔记".to_string()))
    }
}

// 错误类型定义
use axum::Json as AxumJson;
use serde_json::json;
use sqlx;

// GET /inbox/tags handler
pub async fn get_tags_handler(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    match db::get_all_tags_db(&state.pool).await {
        Ok(tags) => Ok(Json(tags)),
        Err(e) => {
            tracing::error!("get_all_tags_db error: {}", e);
            Err(AppError::Internal("获取标签失败".to_string()))
        }
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Internal(String),
    #[error("{0}")]
    DatabaseError(sqlx::Error)
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("未找到记录".to_string()),
            _ => AppError::DatabaseError(err),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status_code, error_msg) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::DatabaseError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("数据库错误: {}", err)
            ),
        };
        
        let body = AxumJson(json!({ "error": error_msg }));
        (status_code, body).into_response()
    }
}