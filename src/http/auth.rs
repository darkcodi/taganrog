use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum_auth::{AuthBearerCustom, Rejection};
use axum::http::{request::Parts, StatusCode};
use thiserror::Error;

pub struct MyCustomBearerAuth(pub String);

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("bearer token is invalid")]
    InvalidToken,
}

impl AuthBearerCustom for MyCustomBearerAuth {
    const ERROR_CODE: StatusCode = StatusCode::UNAUTHORIZED;
    const ERROR_OVERWRITE: Option<&'static str> = None;

    fn from_header(contents: &str) -> Self {
        Self(contents.to_string())
    }
}

#[async_trait]
impl<B> FromRequestParts<B> for MyCustomBearerAuth
    where
        B: Send + Sync,
{
    type Rejection = Rejection;

    async fn from_request_parts(parts: &mut Parts, _: &B) -> Result<Self, Self::Rejection> {
        Self::decode_request_parts(parts)
    }
}

pub fn is_token_valid(token: &str, expected_token: &str) -> Result<(), AuthError> {
    if token  == expected_token {
        Ok(())
    } else {
        Err(AuthError::InvalidToken)
    }
}
