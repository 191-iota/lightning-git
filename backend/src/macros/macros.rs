macro_rules! require_project_permission {
    ($sb_client:expr, $project_id:expr, $user_id:expr) => {
        match crate::service::permission_service::check_repo_permission(
            &$sb_client,
            &$project_id,
            &$user_id,
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

pub(crate) use require_project_permission;