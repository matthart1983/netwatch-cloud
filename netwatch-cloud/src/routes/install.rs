use axum::response::{IntoResponse, Response};
use axum::http::{header, StatusCode};

const INSTALL_SCRIPT: &str = include_str!("../../install.sh");

pub async fn install_script() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        INSTALL_SCRIPT,
    )
        .into_response()
}
