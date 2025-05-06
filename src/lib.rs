// src/lib.rs 或 src/main.rs
use rocket::{Build, Rocket, get, post, put, delete, routes, State};
use rocket::serde::json::Json;
use rocket::http::Status;
// Remove unused NotFound import
use rocket::response::status::Created;
use std::sync::Arc;
use std::sync::Mutex; // Use std::sync::Mutex
use tokio::task; // For spawn_blocking

pub mod db;
mod models;
// Ensure models.rs has correct Note/NoteResponse definitions (tags: Vec<String>)
use models::{Note, CreateNotePayload, NoteResponse, DetailedTag};
use crate::models::UpdateNotePayload;
// 删除未使用的导入
// use crate::db::DbConnection;

// --- Use correct DbConnection type ---
pub type SharedDb = Arc<Mutex<db::DbConnection>>;

// --- note_to_response expects Note with tags: Vec<String> ---
fn note_to_response(note: &Note) -> NoteResponse {
    NoteResponse {
        id: note.id,
        content: note.content.clone(),
        tags: note.tags.clone(), // Directly clone Vec<String>
        created_at: note.created_at.to_rfc3339(),
        updated_at: note.updated_at.to_rfc3339(),
    }
}

// --- 辅助函数处理 DB 错误 (uses rusqlite::Error) ---
fn handle_db_error(db_err: rusqlite::Error) -> Status { // Use full path
    let msg = format!("DB function failed: {:?}", db_err);
    eprintln!("[ERROR] {}", msg);
    match db_err {
        e if e.to_string().contains("no such table") => Status::BadRequest,
        // Use full path for QueryReturnedNoRows
        rusqlite::Error::QueryReturnedNoRows => Status::NotFound,
        _ => Status::InternalServerError,
    }
}

// --- 辅助函数处理 spawn_blocking 错误 (returns Status) ---
fn handle_spawn_error(spawn_err: task::JoinError) -> Status { // Return Status directly
     eprintln!("[ERROR] Spawn blocking task failed: {:?}", spawn_err);
     Status::InternalServerError
}


#[get("/tags/detailed")]
async fn get_detailed_tags(db_state: &State<SharedDb>) -> Result<Json<Vec<DetailedTag>>, Status> {
    let db_arc = db_state.inner().clone();

    let tags = task::spawn_blocking(move || {
        let conn = db_arc.lock().map_err(|_| Status::InternalServerError)?;
        match db::get_detailed_tags_db(&conn) {
            Ok(tags) => Ok(tags),
            Err(e) => Err(handle_db_error(e))
        }
    })
    .await
    .map_err(handle_spawn_error)??;

    Ok(Json(tags))
}


#[get("/tags")]
async fn get_tags(db_state: &State<SharedDb>) -> Result<Json<Vec<String>>, Status> {
    let db_arc = db_state.inner().clone();

    task::spawn_blocking(move || {
        let conn = db_arc.lock().map_err(|_| Status::InternalServerError)?;
        db::get_all_tags_db(&conn)
            .map_err(handle_db_error)
    })
    .await
    .map_err(handle_spawn_error)? // Single '?'
    .map(Json)
}

// mount_rocket remains the same
pub fn mount_rocket(rocket: Rocket<Build>, db: SharedDb) -> Rocket<Build> {
    println!("[INFO] 开始注册 Inbox Server 路由...");
    println!("[INFO] 注册数据库连接池 (同步包装)...");
    let rocket = rocket.manage(db);

    println!("[INFO] 注册 API 路由:");
    // ... (routes) ...

    let rocket = rocket.mount("/inbox", routes![
        root,
        create_note,
        get_notes,
        get_note,
        update_note,
        delete_note,
        get_tags,
        get_detailed_tags,
    ]);

    println!("[INFO] Inbox Server 路由注册完成");
    rocket
}

#[get("/")]
fn root() -> &'static str {
    "📥 Welcome to Inbox Inbox Server (Rust Version)"
}

#[post("/notes", data = "<payload>", format = "json")]
async fn create_note(db_state: &State<SharedDb>, payload: Json<CreateNotePayload>) -> Result<Created<Json<NoteResponse>>, Status> {
    let db_arc = db_state.inner().clone();
    let note_payload = payload.into_inner();

    let created_note = task::spawn_blocking(move || {
        let mut conn_guard = db_arc.lock().map_err(|_| Status::InternalServerError)?;
        db::create_note_db(&mut conn_guard, note_payload)
            .map_err(handle_db_error)
    })
    .await
    .map_err(handle_spawn_error)??; // Double '?' handles JoinError and then DB Result

    Ok(Created::new("/inbox/notes").body(Json(note_to_response(&created_note))))
}


#[get("/notes")]
async fn get_notes(db_state: &State<SharedDb>) -> Result<Json<Vec<NoteResponse>>, Status> {
     let db_arc = db_state.inner().clone();

    let notes = task::spawn_blocking(move || {
        let conn = db_arc.lock().map_err(|_| Status::InternalServerError)?;
        db::get_notes_db(&conn, None, None, None, None)
            .map_err(handle_db_error)
    })
    .await
    .map_err(handle_spawn_error)??; // Double '?'

    let response = notes.iter().map(note_to_response).collect();
    Ok(Json(response))
}


#[get("/notes/<id>")]
async fn get_note(db_state: &State<SharedDb>, id: i64) -> Result<Json<NoteResponse>, Status> {
    let db_arc = db_state.inner().clone();

    let maybe_note = task::spawn_blocking(move || {
        let conn = db_arc.lock().map_err(|_| Status::InternalServerError)?;
        db::get_note_db(&conn, id)
            .map_err(handle_db_error)
    })
    .await
    .map_err(handle_spawn_error)??; // Double '?'

    match maybe_note {
        Some(note) => Ok(Json(note_to_response(&note))),
        None => Err(Status::NotFound),
    }
}


#[put("/notes/<id>", data = "<payload>", format = "json")]
async fn update_note(db_state: &State<SharedDb>, id: i64, payload: Json<UpdateNotePayload>) -> Result<Json<NoteResponse>, Status> {
    let db_arc = db_state.inner().clone();
    let note_payload = payload.into_inner();

    let updated_note_option = task::spawn_blocking(move || {
        let mut conn_guard = db_arc.lock().map_err(|_| Status::InternalServerError)?;
        db::update_note_db(&mut conn_guard, id, note_payload)
             .map_err(handle_db_error)
    })
    .await
    .map_err(handle_spawn_error)??; // Double '?'

    match updated_note_option {
        Some(note) => Ok(Json(note_to_response(&note))),
        None => Err(Status::NotFound),
    }
}


#[delete("/notes/<id>")]
async fn delete_note(db_state: &State<SharedDb>, id: i64) -> Result<Status, Status> {
    let db_arc = db_state.inner().clone();

    let deleted = task::spawn_blocking(move || {
        let mut conn_guard = db_arc.lock().map_err(|_| Status::InternalServerError)?;
        db::delete_note_db(&mut conn_guard, id)
             .map_err(handle_db_error)
    })
    .await
    .map_err(handle_spawn_error)??; // Double '?'

    if deleted {
        Ok(Status::NoContent)
    } else {
        Err(Status::NotFound)
    }
}

// 修改migrate_db函数，解决借用问题
pub async fn migrate_db(db_path: &str) -> Result<(), Status> {
    // 复制路径字符串，以便在闭包中使用
    let db_path = db_path.to_string();
    
    // 在独立线程上运行数据库迁移
    tokio::task::spawn_blocking(move || {
        // 在新线程中创建新连接
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| {
            eprintln!("无法打开数据库连接: {:?}", e);
            handle_db_error(e)
        })?;
        
        // 执行迁移
        db::migrate(&conn).map_err(|e| {
            eprintln!("数据库迁移操作失败: {:?}", e);
            handle_db_error(e)
        })
    }).await.map_err(|_| Status::InternalServerError)?
}