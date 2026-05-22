use crate::macros::macros::require_project_permission;
use crate::model::app_state::AppState;
use crate::model::task::SetArchivedReq;
use crate::model::user::MiddlewareData;
use crate::repository::task_repository;
use actix_web::HttpResponse;
use actix_web::web;
use log::error;
use uuid::Uuid;

/// All Kanban tasks for a project, archived or not. The frontend filters by
/// the archived flag for board vs archive panel rendering.
#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/tasks",
    params(
        ("project_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
    ),
    tag = "task"
)]
pub async fn get_project_tasks(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let project_id = path.into_inner();
    require_project_permission!(&state, &project_id, &ext_data.user_id);

    let res = task_repository::find_by_proj(&state.sb_client, project_id.to_string()).await;
    match res {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(_e) => HttpResponse::BadRequest().body("Failed getting Task"),
    }
}

/// Fetch one task by id, scoped to a project for the permission check.
#[utoipa::path(
    get,
    path = "/api/tasks/{task_id}/{project_id}",
    params(
        ("task_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("project_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
    ),
    tag = "task"
)]
pub async fn get_task(
    state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    // ids.0 => project_id
    // ids.1 => task_id
    let ids = path.into_inner();

    require_project_permission!(&state, &ids.0, &ext_data.user_id);

    let res = task_repository::find_by_id(&state.sb_client, ids.1.to_string()).await;
    match res {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(_e) => HttpResponse::BadRequest().body("Failed getting task"),
    }
}

/// Flip a task's archived flag. Project members can archive or restore.
#[utoipa::path(
    patch,
    path = "/api/tasks/{task_id}/archive",
    params(("task_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")),
    request_body = SetArchivedReq,
    tag = "task",
)]
pub async fn set_task_archived(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<SetArchivedReq>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let task_id = path.into_inner();

    let project_id = match task_repository::project_id_of_task(&state.sb_client, &task_id).await {
        Ok(Some(pid)) => pid,
        Ok(None) => return HttpResponse::NotFound().finish(),
        Err(e) => {
            error!("Failed locating task project: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    require_project_permission!(&state, &project_id, &ext_data.user_id);

    match task_repository::set_archived(&state.sb_client, &task_id, body.archived).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("Failed updating archive flag: {e}");
            HttpResponse::BadRequest().body("Failed updating task")
        }
    }
}
