use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("NO_AUTHORIZATION")]
    Unauthorized,
    #[error("NO_PERMISSION")]
    Forbidden,
    #[error("NOT_FOUND")]
    NotFound,
    #[error("INTERNAL_DB_ERROR")]
    Sqlx(sqlx::Error),
    #[error("INTERNAL_ERROR")]
    Anyhow(#[from] anyhow::Error),
}

impl Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Sqlx(_) | Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Serialize)]
pub struct ErrorMessage {
    pub msg: String,
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Error::NotFound,
            _ => Error::Sqlx(err),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Unauthorized => {
                return (
                    self.status_code(),
                    Json(ErrorMessage {
                        msg: self.to_string(),
                    }),
                )
                    .into_response();
            }

            Self::Sqlx(ref e) => {
                tracing::error!("SQLx error: {:?}", e);
            }

            Self::Anyhow(ref e) => {
                tracing::error!("Generic error: {:?}", e);
            }
            _ => (),
        }

        (
            self.status_code(),
            Json(ErrorMessage {
                msg: self.to_string(),
            }),
        )
            .into_response()
    }
}
