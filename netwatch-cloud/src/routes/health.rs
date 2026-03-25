use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

#[derive(Serialize)]
pub struct VersionInfo {
    pub service: &'static str,
    pub version: &'static str,
    pub git_hash: &'static str,
    pub build_time: &'static str,
}

pub async fn version() -> Json<VersionInfo> {
    Json(VersionInfo {
        service: "netwatch-cloud",
        version: env!("CARGO_PKG_VERSION"),
        git_hash: env!("GIT_HASH"),
        build_time: env!("BUILD_TIME"),
    })
}
