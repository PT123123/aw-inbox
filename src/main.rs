use axum::{
    routing::{get, post, put},
    Router,
    extract::{State, Json, Query, Path}, // <-- 添加 Path
    response::{IntoResponse, Response},
    http::StatusCode,
};
use std::net::SocketAddr;
use tokio;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use serde::Deserialize;
use chrono::{DateTime, Utc};

use aw_inbox_rust::models::{CreateNotePayload, NoteResponse, UpdateNotePayload};
use aw_inbox_rust::db;

// 使用 aw_inbox_rust::AppState
use aw_inbox_rust::AppState;

// 定义 GET /inbox/notes 的查询参数结构体
#[derive(Deserialize, Debug)]
struct GetNotesQuery {
    limit: Option<i64>,
    tag: Option<String>,
    created_after: Option<DateTime<Utc>>,
    created_before: Option<DateTime<Utc>>,
}

#[tokio::main]
pub async fn app(db_pool: db::DbPool) -> Router {
    // 创建应用状态
    let app_state = aw_inbox_rust::AppState { pool: db_pool };

    // 配置 CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 创建 Axum 路由
    Router::new()
        .route("/", get(root))
        .route("/inbox/notes", post(create_note_handler).get(get_notes_handler))
        .route("/inbox/notes/:note_id", get(get_note_handler).put(update_note_handler).delete(delete_note_handler))
        .route("/inbox/tags", get(aw_inbox_rust::get_tags_handler)) // <-- 添加 DELETE 路由
        // 在这里添加其他路由...
        .with_state(app_state)
        .layer(cors)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "aw_inbox_rust=info,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 服务器启动中...");

    // 初始化数据库连接池
    let db_pool = match db::init_pool().await {
        Ok(pool) => {
            tracing::info!("✅ 数据库连接成功");
            pool
        }
        Err(e) => {
            tracing::error!("❌ 无法连接到数据库: {}", e);
            std::process::exit(1); // 连接失败则退出
        }
    };

    // 创建应用状态
    let app_state = aw_inbox_rust::AppState { pool: db_pool };

    // 配置 CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 创建 Axum 路由
    let app = Router::new()
        .route("/", get(root))
        .route("/inbox/notes", post(create_note_handler).get(get_notes_handler))
        .route("/inbox/notes/:note_id", get(get_note_handler).put(update_note_handler).delete(delete_note_handler))
        .route("/inbox/tags", get(aw_inbox_rust::get_tags_handler)) // <-- 添加 DELETE 路由
        // 在这里添加其他路由...
        .with_state(app_state)
        .layer(cors);

    // 定义监听地址和端口
    let addr = SocketAddr::from(([127, 0, 0, 1], 5061)); // 改为 5061端口
    tracing::info!("👂 监听于 {}", addr);
    println!("[MAIN] 服务即将监听于 {}", addr);

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("[MAIN] axum::serve 启动");
    axum::serve(listener, app).await.unwrap();
    Ok(())
    })
}

// 根路由处理程序
async fn root() -> &'static str {
    "📥 Welcome to Inbox Inbox Server (Rust Version)"
}

// --- API 处理程序 --- 

// GET /inbox/notes 处理程序
async fn get_notes_handler(
    State(state): State<AppState>,
    Query(params): Query<GetNotesQuery>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(?params, "接收到获取笔记列表请求");

    let notes = db::get_notes_db(
        &state.pool,
        params.limit,
        params.tag,
        params.created_after,
        params.created_before,
    ).await?;

    // 将数据库模型列表转换为 API 响应模型列表
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

// POST /inbox/notes 处理程序
async fn create_note_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateNotePayload>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(?payload, "接收到创建笔记请求");

    // 数据校验 (基础示例，可以根据需要扩展)
    if payload.content.trim().is_empty() {
        tracing::warn!("笔记内容为空");
        return Err(AppError::BadRequest("笔记内容不能为空".to_string()));
    }

    let note = db::create_note_db(&state.pool, payload).await?;

    // 将数据库模型转换为 API 响应模型
    let response = NoteResponse {
        id: note.id,
        content: note.content,
        // 从 JSON 字符串反序列化 tags
        tags: serde_json::from_str(&note.tags).unwrap_or_default(), 
        created_at: note.created_at.to_rfc3339(),
        updated_at: note.updated_at.to_rfc3339(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

// PUT /inbox/notes/:note_id 处理程序
async fn update_note_handler(
    State(state): State<AppState>,
    Path(note_id): Path<i64>, // <-- 从路径中提取 note_id
    Json(payload): Json<UpdateNotePayload>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(note_id, ?payload, "接收到更新笔记请求");

    // 数据校验
    if payload.content.trim().is_empty() {
        tracing::warn!("笔记内容为空");
        return Err(AppError::BadRequest("笔记内容不能为空".to_string()));
    }

    match db::update_note_db(&state.pool, note_id, payload).await? {
        Some(note) => {
            // 将数据库模型转换为 API 响应模型
            let response = NoteResponse {
                id: note.id,
                content: note.content,
                tags: serde_json::from_str(&note.tags).unwrap_or_default(),
                created_at: note.created_at.to_rfc3339(),
                updated_at: note.updated_at.to_rfc3339(),
            };
            Ok(Json(response))
        }
        None => Err(AppError::NotFound("未找到指定笔记".to_string())),
    }
}

// GET /inbox/notes/:note_id 处理程序
async fn get_note_handler(
    State(state): State<AppState>,
    Path(note_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(note_id, "接收到获取单条笔记请求");

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
        None => Err(AppError::NotFound("未找到指定笔记".to_string())),
    }
}

// DELETE /inbox/notes/:note_id 处理程序
async fn delete_note_handler(
    State(state): State<AppState>,
    Path(note_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(note_id, "接收到删除笔记请求");

    match db::delete_note_db(&state.pool, note_id).await? {
        true => Ok(StatusCode::NO_CONTENT), // 删除成功，返回 204 No Content
        false => Err(AppError::NotFound("未找到指定笔记".to_string())),
    }
}


// --- 自定义错误处理 ---
// 定义一个统一的错误类型，方便处理各种错误
enum AppError {
    SqlxError(sqlx::Error),
    BadRequest(String),
    NotFound(String), // <-- 添加 NotFound 错误类型
}

// 实现 IntoResponse trait，让 AppError 可以转换为 Axum 响应
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::SqlxError(e) => {
                tracing::error!("数据库错误: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误".to_string())
            }
            AppError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, msg)
            }
            AppError::NotFound(msg) => { // <-- 处理 NotFound
                (StatusCode::NOT_FOUND, msg)
            }
        };

        (status, Json(serde_json::json!({ "error": error_message }))).into_response()
    }
}

// 实现 From<sqlx::Error> trait，方便将 sqlx 错误转换为 AppError
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        // 可以根据具体的 sqlx 错误类型返回不同的 AppError
        // 例如，如果 sqlx::Error::RowNotFound，可以返回 AppError::NotFound
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("未找到资源".to_string()),
            _ => AppError::SqlxError(err),
        }
    }
}
