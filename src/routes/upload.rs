use axum::{extract::State, routing::post, Json, Router};
use axum::extract::Multipart;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::pipeline::{parse, process};
use crate::state::AppState;
use crate::types::activity::{AvailableData, FileFormat, Metrics};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/upload", post(upload))
}

#[derive(Serialize, Deserialize)]
struct UploadResponse {
    file_id: String,
    file_type: String,
    metrics: Metrics,
    available_data: AvailableData,
}

async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "file" {
            filename = field.file_name().map(|s| s.to_string());
            file_bytes = Some(field.bytes().await.map_err(|e| {
                AppError::BadRequest(format!("Failed to read file bytes: {}", e))
            })?.to_vec());
        }
    }

    let bytes = file_bytes.ok_or_else(|| AppError::BadRequest("No file provided".to_string()))?;
    let filename = filename.ok_or_else(|| AppError::BadRequest("No filename provided".to_string()))?;

    let format = FileFormat::from_filename(&filename)
        .ok_or_else(|| AppError::BadRequest("Unsupported file format".to_string()))?;

    tracing::info!("Parsing {} file: {}", format_name(format), filename);

    let parsed = parse::parse(&bytes, format)?;
    let processed = process::process(&parsed)?;

    let file_id = Uuid::new_v4().to_string();
    state.insert(file_id.clone(), processed.clone());

    tracing::info!(
        "Uploaded file {} with ID {} ({} points, {:.2} km)",
        filename,
        file_id,
        processed.points.len(),
        processed.metrics.distance_km
    );

    Ok(Json(UploadResponse {
        file_id,
        file_type: format_name(format).to_string(),
        metrics: processed.metrics,
        available_data: processed.available_data,
    }))
}

fn format_name(format: FileFormat) -> &'static str {
    match format {
        FileFormat::Gpx => "gpx",
        FileFormat::Fit => "fit",
    }
}

