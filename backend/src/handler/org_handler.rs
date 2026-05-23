use crate::macros::macros::require_org_owner;
use crate::macros::macros::require_org_permission;
use crate::model::app_state::AppState;
use crate::model::org::AddOrgMemberReq;
use crate::model::org::CreateOrgReq;
use crate::model::org::CreateOrgRes;
use crate::model::org::OrgRole;
use crate::model::org::TransferOrgOwnershipReq;
use crate::model::org::UpdateOrgReq;
use crate::model::user::MiddlewareData;
use crate::repository::org_repository;
use crate::service::permission_service;
use actix_web::HttpResponse;
use actix_web::web;
use log::error;
use uuid::Uuid;
use validator::Validate;

/// Create an organization. The caller is registered as its owner.
#[utoipa::path(
    post,
    path = "/api/orgs",
    request_body = CreateOrgReq,
    responses(
        (status = 200, body = CreateOrgRes),
        (status = 400, description = "Validation error"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "org",
)]
pub async fn create_org(
    state: web::Data<AppState>,
    req: web::Json<CreateOrgReq>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(e);
    }

    let org_id = Uuid::new_v4();
    match org_repository::save_org(&state.sb_client, &org_id, &req.name, &ext_data.user_id).await {
        Ok(_) => HttpResponse::Ok().json(CreateOrgRes { org_id }),
        Err(e) => {
            error!("Failed creating org: {e}");
            HttpResponse::BadRequest().body("Failed creating org")
        }
    }
}

/// List organizations the caller belongs to, including their role per org.
#[utoipa::path(
    get,
    path = "/api/orgs/mine",
    tag = "org",
)]
pub async fn list_my_orgs(
    state: web::Data<AppState>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    match org_repository::list_user_orgs(&state.sb_client, &ext_data.user_id).await {
        Ok(orgs) => HttpResponse::Ok().json(orgs),
        Err(e) => {
            error!("Failed listing user orgs: {e}");
            HttpResponse::BadRequest().body("Failed listing orgs")
        }
    }
}

/// Fetch a single organization by id. Requires org membership.
#[utoipa::path(
    get,
    path = "/api/orgs/{id}",
    params(("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")),
    tag = "org",
)]
pub async fn get_org(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let org_id = path.into_inner();
    require_org_permission!(&state, &org_id, &ext_data.user_id);

    match org_repository::find_org_by_id(&state.sb_client, &org_id).await {
        Ok(org) => HttpResponse::Ok().json(org),
        Err(e) => {
            error!("Failed retrieving org: {e}");
            HttpResponse::NotFound().body("Org not found")
        }
    }
}

/// Rename an organization. Owner only.
#[utoipa::path(
    put,
    path = "/api/orgs/{id}",
    params(("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")),
    request_body = UpdateOrgReq,
    tag = "org",
)]
pub async fn update_org(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: web::Json<UpdateOrgReq>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(e);
    }
    let org_id = path.into_inner();
    require_org_owner!(&state, &org_id, &ext_data.user_id);

    match org_repository::update_org(&state.sb_client, &org_id, &req.name).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("Failed updating org: {e}");
            HttpResponse::BadRequest().body("Failed updating org")
        }
    }
}

/// Delete an organization and cascade its projects. Owner only.
#[utoipa::path(
    delete,
    path = "/api/orgs/{id}",
    params(("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")),
    tag = "org",
)]
pub async fn delete_org(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let org_id = path.into_inner();
    require_org_owner!(&state, &org_id, &ext_data.user_id);

    match org_repository::delete_org(&state.sb_client, &org_id).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("Failed deleting org: {e}");
            HttpResponse::BadRequest().body("Failed deleting org")
        }
    }
}

/// List all members of an organization with their roles. Requires org membership.
#[utoipa::path(
    get,
    path = "/api/orgs/{id}/members",
    params(("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")),
    tag = "org",
)]
pub async fn list_org_members(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let org_id = path.into_inner();
    require_org_permission!(&state, &org_id, &ext_data.user_id);

    match org_repository::list_org_members(&state.sb_client, &org_id).await {
        Ok(members) => HttpResponse::Ok().json(members),
        Err(e) => {
            error!("Failed listing org members: {e}");
            HttpResponse::BadRequest().body("Failed listing org members")
        }
    }
}

/// Add a member to the organization with a chosen role. Owner only.
#[utoipa::path(
    post,
    path = "/api/orgs/{id}/members",
    params(("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")),
    request_body = AddOrgMemberReq,
    tag = "org",
)]
pub async fn add_org_member(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: web::Json<AddOrgMemberReq>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(e);
    }
    // an org has exactly one owner. promotion happens through the transfer
    // endpoint, not by adding a second owner row.
    if matches!(req.role, OrgRole::Owner) {
        return HttpResponse::BadRequest()
            .body("An org can only have one owner; use the transfer endpoint to change it");
    }
    let org_id = path.into_inner();
    require_org_owner!(&state, &org_id, &ext_data.user_id);

    match org_repository::add_org_member(&state.sb_client, &org_id, &req.user_id, req.role).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("Failed adding org member: {e}");
            HttpResponse::BadRequest().body("Failed adding org member")
        }
    }
}

/// Remove a member from the organization. Owner only.
/// Owners cannot remove themselves; deleting the org is the supported path.
#[utoipa::path(
    delete,
    path = "/api/orgs/{id}/members/{user_id}",
    params(
        ("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("user_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
    ),
    tag = "org",
)]
pub async fn remove_org_member(
    state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let (org_id, target_user) = path.into_inner();
    require_org_owner!(&state, &org_id, &ext_data.user_id);

    if target_user == ext_data.user_id {
        return HttpResponse::BadRequest()
            .body("Org owners cannot remove themselves; delete the org instead");
    }

    match org_repository::remove_org_member(&state.sb_client, &org_id, &target_user).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("Failed removing org member: {e}");
            HttpResponse::BadRequest().body("Failed removing org member")
        }
    }
}

/// Transfer ownership of the organization to another existing member. The
/// caller (current owner) is demoted to member in the same operation. Owner only.
#[utoipa::path(
    post,
    path = "/api/orgs/{id}/transfer",
    params(("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")),
    request_body = TransferOrgOwnershipReq,
    tag = "org",
)]
pub async fn transfer_org_ownership(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: web::Json<TransferOrgOwnershipReq>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let org_id = path.into_inner();
    require_org_owner!(&state, &org_id, &ext_data.user_id);

    if req.new_owner_id == ext_data.user_id {
        return HttpResponse::BadRequest().body("Cannot transfer ownership to yourself");
    }

    // verify the target is an existing member of this org
    let is_member = match permission_service::check_org_permission(
        &state,
        &org_id,
        &req.new_owner_id,
        OrgRole::Member,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            error!("Failed checking new owner membership: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };
    if !is_member {
        return HttpResponse::BadRequest()
            .body("New owner must already be a member of the org");
    }

    match org_repository::transfer_org_ownership(
        &state.sb_client,
        &org_id,
        &ext_data.user_id,
        &req.new_owner_id,
    )
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("Failed transferring org ownership: {e}");
            HttpResponse::BadRequest().body("Failed transferring ownership")
        }
    }
}

/// List projects in the organization. Owners see every project,
/// members see only the projects they belong to.
#[utoipa::path(
    get,
    path = "/api/orgs/{id}/projects",
    params(("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")),
    tag = "org",
)]
pub async fn list_org_projects(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let org_id = path.into_inner();
    require_org_permission!(&state, &org_id, &ext_data.user_id);

    let is_owner = match permission_service::check_org_permission(
        &state,
        &org_id,
        &ext_data.user_id,
        OrgRole::Owner,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            error!("Failed checking owner status: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match org_repository::list_org_projects(&state.sb_client, &org_id, &ext_data.user_id, is_owner)
        .await
    {
        Ok(projects) => HttpResponse::Ok().json(projects),
        Err(e) => {
            error!("Failed listing org projects: {e}");
            HttpResponse::BadRequest().body("Failed listing org projects")
        }
    }
}
