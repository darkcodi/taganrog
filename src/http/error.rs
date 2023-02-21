use axum::http::header::WWW_AUTHENTICATE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::borrow::Cow;
use std::collections::HashMap;
use s3::error::S3Error;
use tracing::error;
use crate::db::surreal_http::SurrealDbError;
use crate::http::auth::AuthError;
use crate::http::{APPLICATION_JSON, CONTENT_TYPE_HEADER};

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("authentication required")]
    Unauthorized,

    #[error("authentication failed")]
    AuthError(#[from] AuthError),

    #[error("user may not perform that action")]
    Forbidden,

    #[error("request path not found")]
    NotFound,

    #[error("such entity already exists")]
    Conflict {
        serialized_entity: String,
    },

    #[error("error in the request body")]
    UnprocessableEntity {
        errors: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>,
    },

    #[error("an error occurred with the database: {0}")]
    DbErr(#[from] SurrealDbError),

    #[error("an error occurred with the S3: {0}")]
    S3Error(#[from] S3Error),

    #[error("an internal server error occurred: {0}")]
    Anyhow(#[from] anyhow::Error),
}

impl ApiError {
    pub fn conflict<K: serde::Serialize>(entity: K) -> Self {
        let serialized_entity = serde_json::to_string(&entity).unwrap();
        Self::Conflict { serialized_entity }
    }

    pub fn unprocessable_entity<K, V>(errors: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>,
    {
        let mut error_map = HashMap::new();

        for (key, val) in errors {
            error_map
                .entry(key.into())
                .or_insert_with(Vec::new)
                .push(val.into());
        }

        Self::UnprocessableEntity { errors: error_map }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized | Self::AuthError(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::DbErr(_) | Self::S3Error(_) | Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::UnprocessableEntity { errors } => {
                #[derive(serde::Serialize)]
                struct Errors {
                    errors: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>,
                }

                return (StatusCode::UNPROCESSABLE_ENTITY, Json(Errors { errors })).into_response();
            }

            Self::Conflict { serialized_entity } => {
                let mut response = (StatusCode::CONFLICT, serialized_entity).into_response();
                response.headers_mut().insert(CONTENT_TYPE_HEADER, HeaderValue::from_static(APPLICATION_JSON));
                return response;
            }

            Self::Unauthorized => {
                return (
                    self.status_code(),
                    [(WWW_AUTHENTICATE, HeaderValue::from_static("Token"))]
                        .into_iter()
                        .collect::<HeaderMap>(),
                    self.to_string(),
                )
                    .into_response();
            }

            Self::DbErr(ref e) => {
                error!("Database error: {:?}", e);
            }

            Self::Anyhow(ref e) => {
                error!("Generic error: {:?}", e);
            }

            _ => (),
        }

        (self.status_code(), self.to_string()).into_response()
    }
}
