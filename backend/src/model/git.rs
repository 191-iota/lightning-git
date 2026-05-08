use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateRepoReq {
    repo_name: String,
}

