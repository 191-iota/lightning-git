/// Requires the caller to be at least a member of the given project.
/// Org owners of the project's org also satisfy this check.
macro_rules! require_project_permission {
    ($sb_client:expr, $project_id:expr, $user_id:expr) => {
        match crate::service::permission_service::check_project_permission(
            &$sb_client,
            &$project_id,
            &$user_id,
            crate::model::project::ProjectRole::Member,
        )
        .await
        {
            Ok(true) => {}
            Ok(false) => {
                log::error!(
                    "Unauthorized user tried accessing a project: user_id: {}, project_id: {}",
                    $user_id,
                    $project_id
                );
                return actix_web::HttpResponse::Unauthorized().finish();
            }
            Err(e) => {
                log::error!("Permission check failed: {e}");
                return actix_web::HttpResponse::InternalServerError().finish();
            }
        }
    };
}

/// Requires the caller to be a project admin.
/// Org owners of the projects org also satisfy this check
macro_rules! require_project_admin {
    ($sb_client:expr, $project_id:expr, $user_id:expr) => {
        match crate::service::permission_service::check_project_permission(
            &$sb_client,
            &$project_id,
            &$user_id,
            crate::model::project::ProjectRole::Admin,
        )
        .await
        {
            Ok(true) => {}
            Ok(false) => {
                log::error!(
                    "Non-admin user tried mutating a project: user_id: {}, project_id: {}",
                    $user_id,
                    $project_id
                );
                return actix_web::HttpResponse::Unauthorized().finish();
            }
            Err(e) => {
                log::error!("Permission check failed: {e}");
                return actix_web::HttpResponse::InternalServerError().finish();
            }
        }
    };
}

/// Requires the caller to be at least a member of the given org.
macro_rules! require_org_permission {
    ($sb_client:expr, $org_id:expr, $user_id:expr) => {
        match crate::service::permission_service::check_org_permission(
            &$sb_client,
            &$org_id,
            &$user_id,
            crate::model::org::OrgRole::Member,
        )
        .await
        {
            Ok(true) => {}
            Ok(false) => {
                log::error!(
                    "Unauthorized user tried accessing an org: user_id: {}, org_id: {}",
                    $user_id,
                    $org_id
                );
                return actix_web::HttpResponse::Unauthorized().finish();
            }
            Err(e) => {
                log::error!("Org permission check failed: {e}");
                return actix_web::HttpResponse::InternalServerError().finish();
            }
        }
    };
}

/// Requires the caller to be the org owner.
macro_rules! require_org_owner {
    ($sb_client:expr, $org_id:expr, $user_id:expr) => {
        match crate::service::permission_service::check_org_permission(
            &$sb_client,
            &$org_id,
            &$user_id,
            crate::model::org::OrgRole::Owner,
        )
        .await
        {
            Ok(true) => {}
            Ok(false) => {
                log::error!(
                    "Non-owner user tried owner-only org operation: user_id: {}, org_id: {}",
                    $user_id,
                    $org_id
                );
                return actix_web::HttpResponse::Unauthorized().finish();
            }
            Err(e) => {
                log::error!("Org owner check failed: {e}");
                return actix_web::HttpResponse::InternalServerError().finish();
            }
        }
    };
}

pub(crate) use require_org_owner;
pub(crate) use require_org_permission;
pub(crate) use require_project_admin;
pub(crate) use require_project_permission;
