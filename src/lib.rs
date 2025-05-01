use rocket::{Build, Rocket, get, post, put, delete, routes, State};
use rocket::serde::json::Json;
use rocket::http::Status;
use rocket::response::status::Created;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod db;
mod models;
use models::{Note, CreateNotePayload, UpdateNotePayload, NoteResponse, DetailedTag};

pub type SharedDb = Arc<Mutex<db::DbPool>>;

fn note_to_response(note: &Note) -> NoteResponse {
    NoteResponse {
        id: note.id,
        content: note.content.clone(),
        tags: serde_json::from_str(&note.tags).unwrap_or_default(),
        created_at: note.created_at.to_rfc3339(),
        updated_at: note.updated_at.to_rfc3339(),
    }
}

#[get("/tags/detailed")]
async fn get_detailed_tags(db: &State<SharedDb>) -> Result<Json<Vec<DetailedTag>>, Status> {
    let pool = db.lock().await;
    match db::get_detailed_tags_db(&*pool).await {
        Ok(tags) => Ok(Json(tags)),
        Err(e) => {
            let msg = format!("get_detailed_tags_db failed: {:?}", e);
            eprintln!("[ERROR] {}", msg);
            if msg.contains("no such table") {
                return Err(Status::BadRequest);
            }
            Err(Status::InternalServerError)
        }
    }
}

#[get("/tags")]
async fn get_tags(db: &State<SharedDb>) -> Result<Json<Vec<String>>, Status> {
    let pool = db.lock().await;
    match db::get_all_tags_db(&*pool).await {
        Ok(tags) => Ok(Json(tags)),
        Err(e) => {
            let msg = format!("get_all_tags_db failed: {:?}", e);
            eprintln!("[ERROR] {}", msg);
            if msg.contains("no such table") {
                return Err(Status::BadRequest);
            }
            Err(Status::InternalServerError)
        }
    }
}

pub fn mount_rocket(rocket: Rocket<Build>, db: SharedDb) -> Rocket<Build> {
    rocket.manage(db).mount("/inbox", routes![
        root,
        create_note,
        get_notes,
        get_note,
        update_note,
        delete_note,
        get_tags,
        get_detailed_tags,
    ])
}

#[get("/")]
fn root() -> &'static str {
    "📥 Welcome to Inbox Inbox Server (Rust Version)"
}

#[post("/notes", data = "<payload>", format = "json")]
async fn create_note(db: &State<SharedDb>, payload: Json<CreateNotePayload>) -> Result<Created<Json<NoteResponse>>, Status> {
    let pool = db.lock().await;
    match db::create_note_db(&*pool, payload.into_inner()).await {
        Ok(note) => Ok(Created::new("/inbox/notes").body(Json(note_to_response(&note)))),
        Err(e) => {
            let msg = format!("create_note_db failed: {:?}", e);
            eprintln!("[ERROR] {}", msg);
            if msg.contains("no such table") {
                return Err(Status::BadRequest);
            }
            Err(Status::InternalServerError)
        }
    }
}

#[get("/notes")]
async fn get_notes(db: &State<SharedDb>) -> Result<Json<Vec<NoteResponse>>, Status> {
    let pool = db.lock().await;
    match db::get_notes_db(&*pool, None, None, None, None).await {
        Ok(notes) => Ok(Json(notes.iter().map(note_to_response).collect())),
        Err(e) => {
            let msg = format!("get_notes_db failed: {:?}", e);
            eprintln!("[ERROR] {}", msg);
            if msg.contains("no such table") {
                return Err(Status::BadRequest);
            }
            Err(Status::InternalServerError)
        }
    }
}

#[get("/notes/<id>")]
async fn get_note(db: &State<SharedDb>, id: i64) -> Result<Json<NoteResponse>, Status> {
    let pool = db.lock().await;
    match db::get_note_db(&*pool, id).await {
        Ok(Some(note)) => Ok(Json(note_to_response(&note))),
        Ok(None) => Err(Status::NotFound),
        Err(e) => {
            let msg = format!("get_note_db failed: {:?}", e);
            eprintln!("[ERROR] {}", msg);
            if msg.contains("no such table") {
                return Err(Status::BadRequest);
            }
            Err(Status::InternalServerError)
        }
    }
}

#[put("/notes/<id>", data = "<payload>", format = "json")]
async fn update_note(db: &State<SharedDb>, id: i64, payload: Json<UpdateNotePayload>) -> Result<Json<NoteResponse>, Status> {
    let pool = db.lock().await;
    match db::update_note_db(&*pool, id, payload.into_inner()).await {
        Ok(Some(note)) => Ok(Json(note_to_response(&note))),
        Ok(None) => Err(Status::NotFound),
        Err(e) => {
            let msg = format!("update_note_db failed: {:?}", e);
            eprintln!("[ERROR] {}", msg);
            if msg.contains("no such table") {
                return Err(Status::BadRequest);
            }
            Err(Status::InternalServerError)
        }
    }
}

#[delete("/notes/<id>")]
async fn delete_note(db: &State<SharedDb>, id: i64) -> Result<Status, Status> {
    let pool = db.lock().await;
    match db::delete_note_db(&*pool, id).await {
        Ok(true) => Ok(Status::NoContent),
        Ok(false) => Err(Status::NotFound),
        Err(e) => {
            let msg = format!("delete_note_db failed: {:?}", e);
            eprintln!("[ERROR] {}", msg);
            if msg.contains("no such table") {
                return Err(Status::BadRequest);
            }
            Err(Status::InternalServerError)
        }
    }
}