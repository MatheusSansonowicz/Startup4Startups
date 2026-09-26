use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::auth::firebase_verify::verificar_id_token;
use crate::config::FirebaseConfig;

/// Dados do usuário autenticado, disponíveis nos handlers via
/// `Extension<UsuarioAutenticado>` depois que este middleware roda.
#[derive(Debug, Clone)]
pub struct UsuarioAutenticado {
    pub uid: String,
}

/// Middleware que bloqueia a requisição ANTES de chegar no handler,
/// caso não venha um ID Token do Firebase válido no header
/// Authorization: Bearer <token>.
///
/// É isso que faltava no protótipo original — sem isso, qualquer
/// requisição HTTP passava direto pro repository/Firestore.
pub async fn exigir_autenticacao(
    State(config): State<FirebaseConfig>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_string();

    let claims = verificar_id_token(&token, &config.project_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Descomente pra exigir email verificado antes de permitir escrita
    // (útil pro provedor email/senha, onde criar conta é instantâneo):
    //
    // if !claims.email_verified {
    //     return Err(StatusCode::FORBIDDEN);
    // }

    req.extensions_mut().insert(UsuarioAutenticado { uid: claims.sub });

    Ok(next.run(req).await)
}
