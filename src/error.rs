use axum::{http::StatusCode, response::IntoResponse, response::Response, Json};
use serde_json::json;

/// Erro único da aplicação. Cada handler retorna Result<_, AppError>,
/// e o `?` propaga automaticamente — sem try/catch, sem exceção
/// escapando silenciosamente.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("startup não encontrada")]
    NotFound,

    #[error("dados inválidos: {0}")]
    Validation(String),

    #[error("erro ao comunicar com o Firestore: {0}")]
    Firestore(String),

    #[error("erro no serviço de IA/busca: {0}")]
    ExternalService(String),

    #[error("erro inesperado: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, mensagem) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Firestore(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            AppError::ExternalService(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, Json(json!({ "erro": mensagem }))).into_response()
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Firestore(e.to_string())
    }
}
