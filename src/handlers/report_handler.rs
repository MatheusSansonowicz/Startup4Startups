use axum::{
    extract::{Path, State},
    Json,
};

use crate::dtos::report_dto::ReportResponse;
use crate::error::AppError;
use crate::repository::startup_repository::StartupRepository;
use crate::services::report_service::ReportService;

/// GET /startups/:id/report
/// Devolve o relatório em cache se ainda estiver fresco; senão gera um
/// novo na hora (busca de mercado + IA) e devolve.
pub async fn buscar_relatorio(
    State(repo): State<StartupRepository>,
    State(report_service): State<ReportService>,
    Path(id): Path<String>,
) -> Result<Json<ReportResponse>, AppError> {
    let startup = repo.buscar_por_id(&id).await?;
    let (relatorio, from_cache) = report_service.obter_ou_gerar(&startup).await?;

    Ok(Json(ReportResponse::new(relatorio, from_cache)))
}

/// POST /startups/:id/report/refresh
/// Ignora o cache e força uma geração nova — útil pro client pedir
/// "atualiza agora" em vez de esperar o TTL vencer sozinho.
pub async fn atualizar_relatorio(
    State(repo): State<StartupRepository>,
    State(report_service): State<ReportService>,
    Path(id): Path<String>,
) -> Result<Json<ReportResponse>, AppError> {
    let startup = repo.buscar_por_id(&id).await?;
    let relatorio = report_service.gerar(&startup).await?;

    Ok(Json(ReportResponse::new(relatorio, false)))
}
