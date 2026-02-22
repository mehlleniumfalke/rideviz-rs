use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Invalid GPX: {0}")]
    InvalidGpx(String),
    #[error("Invalid FIT: {0}")]
    InvalidFit(String),
    #[error("No track points found in file")]
    EmptyFile,
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Insufficient data points (need at least 2, got {0})")]
    InsufficientPoints(usize),
}

#[derive(Debug, thiserror::Error)]
pub enum PrepareError {
    #[error("No {0} data available in this activity")]
    MissingData(&'static str),
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("SVG generation failed: {0}")]
    SvgError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RasterError {
    #[error("PNG rendering failed: {0}")]
    RenderFailed(String),
    #[error("Animation rendering failed: {0}")]
    AnimationFailed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error(transparent)]
    Prepare(#[from] PrepareError),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error(transparent)]
    Raster(#[from] RasterError),
    #[error("Activity not found: {0}")]
    NotFound(String),
    #[error("Invalid request: {0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Parse(_) | AppError::Process(_) | AppError::Prepare(_) | AppError::BadRequest(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::Render(_) | AppError::Raster(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
        };

        let body = Json(json!({
            "error": message
        }));

        (status, body).into_response()
    }
}
