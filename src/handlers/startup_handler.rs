use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};

use crate::auth::middleware::UsuarioAutenticado;
use crate::dtos::startup_dto::{CreateStartupRequest, StartupResponse, UpdateStartupRequest};
use crate::error::AppError;
use crate::repository::startup_repository::StartupRepository;

/// Equivalente ao @PostMapping do Controller Spring.
/// `Extension<UsuarioAutenticado>` só chega até aqui se o middleware
/// de autenticação já validou o token — não precisa checar de novo.
pub async fn criar_startup(
    State(repo): State<StartupRepository>,
    Extension(usuario): Extension<UsuarioAutenticado>,
    Json(dto): Json<CreateStartupRequest>,
) -> Result<(StatusCode, Json<StartupResponse>), AppError> {
    dto.validar().map_err(AppError::Validation)?;

    let startup = repo.criar(&dto, &usuario.uid).await?;
    Ok((StatusCode::CREATED, Json(startup.into())))
}

/// Equivalente ao @GetMapping("/{id}")
pub async fn buscar_startup(
    State(repo): State<StartupRepository>,
    Path(id): Path<String>,
) -> Result<Json<StartupResponse>, AppError> {
    let startup = repo.buscar_por_id(&id).await?;
    Ok(Json(startup.into()))
}

/// Equivalente ao @GetMapping (lista)
pub async fn listar_startups(
    State(repo): State<StartupRepository>,
) -> Result<Json<Vec<StartupResponse>>, AppError> {
    let startups = repo.listar().await?;
    Ok(Json(startups.into_iter().map(Into::into).collect()))
}

/// Equivalente ao @PutMapping("/{id}") / @PatchMapping("/{id}")
pub async fn atualizar_startup(
    State(repo): State<StartupRepository>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateStartupRequest>,
) -> Result<Json<StartupResponse>, AppError> {
    let startup = repo.atualizar(&id, dto).await?;
    Ok(Json(startup.into()))
}

/// Equivalente ao @DeleteMapping("/{id}")
pub async fn deletar_startup(
    State(repo): State<StartupRepository>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    repo.deletar(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
