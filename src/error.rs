use axum::{body::Body, response::{IntoResponse, Response}};
use http::StatusCode;

pub struct AppError(anyhow::Error);

pub type AppResult<T> = Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response<Body> {
        log::error!("{:?}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
