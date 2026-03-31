use anyhow::{Context, Result};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Provider-neutral user UUID.
    pub sub: String,
    pub username: String,
    pub exp: usize,
}

pub fn create_token(secret: &str, user_id: &str, username: &str, ttl_hours: i64) -> Result<String> {
    let exp = (chrono::Utc::now() + chrono::Duration::hours(ttl_hours)).timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .context("encode authentication token")?;
    Ok(token)
}

pub fn verify_token(secret: &str, token: &str) -> Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .context("decode authentication token")?;
    Ok(data.claims)
}
