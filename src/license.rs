use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseClaims {
    pub sub: String,
    pub email: String,
    pub pro: bool,
    pub exp: usize,
}

pub fn create_license_token(
    user_id: &str,
    email: &str,
    is_pro: bool,
    ttl_seconds: u64,
    secret: &str,
) -> Result<String, AppError> {
    let now = now_unix();
    let claims = LicenseClaims {
        sub: user_id.to_string(),
        email: email.to_string(),
        pro: is_pro,
        exp: (now + ttl_seconds) as usize,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| AppError::Internal(format!("Failed to sign license token: {}", err)))
}

pub fn verify_license_token(token: &str, secret: &str) -> Result<LicenseClaims, AppError> {
    let validation = Validation::new(Algorithm::HS256);
    let token_data = decode::<LicenseClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized("Invalid or expired license token".to_string()))?;
    Ok(token_data.claims)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
