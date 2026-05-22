use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use actix_web::HttpResponse;
use actix_web::web;
use uuid::Uuid;

use crate::macros::macros::require_project_permission;
use crate::model::app_state::AppState;
use crate::model::overlay::Comment;
use crate::model::overlay::CreateCommentReq;
use crate::model::user::MiddlewareData;

#[utoipa::path(
    get,
    path = "/api/comments/{proj_id}/{file_name}",
    params(
        ("proj_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("file_name" = String, Path, example = "src/main.rs"),
    ),
    tag = "comment"
)]
pub async fn list_comments(
    state: web::Data<AppState>,
    path: web::Path<(Uuid, String)>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let (proj_id, file_name) = path.into_inner();
    require_project_permission!(&state, &proj_id, &ext_data.user_id);

    let comments = match state.repo_states.get(&proj_id) {
        Some(proj) => match proj.overlays.get(&file_name) {
            Some(overlay) => overlay
                .comments
                .iter()
                .map(|e| e.value().clone())
                .collect::<Vec<Comment>>(),
            None => Vec::new(),
        },
        None => Vec::new(),
    };

    HttpResponse::Ok().json(comments)
}

#[utoipa::path(
    post,
    path = "/api/comments/{proj_id}/{file_name}",
    params(
        ("proj_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("file_name" = String, Path, example = "src/main.rs"),
    ),
    request_body = CreateCommentReq,
    tag = "comment"
)]
pub async fn create_comment(
    state: web::Data<AppState>,
    path: web::Path<(Uuid, String)>,
    body: web::Json<CreateCommentReq>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let (proj_id, file_name) = path.into_inner();
    require_project_permission!(&state, &proj_id, &ext_data.user_id);

    let body = body.into_inner();
    if body.text.trim().is_empty() {
        return HttpResponse::BadRequest().body("empty comment");
    }

    let Some(proj) = state.repo_states.get(&proj_id) else {
        return HttpResponse::NotFound().body("project not active");
    };
    let Some(overlay) = proj.overlays.get(&file_name) else {
        return HttpResponse::NotFound().body("file not active");
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let comment = Comment {
        id: Uuid::new_v4(),
        user_id: ext_data.user_id,
        line: body.line,
        text: body.text,
        created_at: now,
    };
    overlay.comments.insert(comment.id, comment.clone());

    HttpResponse::Ok().json(comment)
}

#[utoipa::path(
    delete,
    path = "/api/comments/{proj_id}/{comment_id}/{file_name}",
    params(
        ("proj_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("comment_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("file_name" = String, Path, example = "src/main.rs"),
    ),
    tag = "comment"
)]
pub async fn delete_comment(
    state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, String)>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let (proj_id, comment_id, file_name) = path.into_inner();
    require_project_permission!(&state, &proj_id, &ext_data.user_id);

    let Some(proj) = state.repo_states.get(&proj_id) else {
        return HttpResponse::NotFound().finish();
    };
    let Some(overlay) = proj.overlays.get(&file_name) else {
        return HttpResponse::NotFound().finish();
    };

    let owner = overlay.comments.get(&comment_id).map(|c| c.user_id);
    match owner {
        Some(uid) if uid == ext_data.user_id => {
            overlay.comments.remove(&comment_id);
            HttpResponse::NoContent().finish()
        }
        Some(_) => HttpResponse::Forbidden().finish(),
        None => HttpResponse::NotFound().finish(),
    }
}
