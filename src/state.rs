use axum::extract::FromRef;

use crate::repository::startup_repository::StartupRepository;
use crate::services::report_service::ReportService;

/// Axum permite só um tipo de State por Router — como agora temos duas
/// dependências (repository de startups e o service de relatório),
/// agrupamos as duas aqui. `FromRef` abaixo ensina o Axum a "extrair"
/// cada campo individualmente, então os handlers continuam pedindo só
/// o que precisam (`State<StartupRepository>` ou `State<ReportService>`),
/// sem precisar saber que o outro existe.
#[derive(Clone)]
pub struct AppState {
    pub repo: StartupRepository,
    pub report_service: ReportService,
}

impl FromRef<AppState> for StartupRepository {
    fn from_ref(state: &AppState) -> Self {
        state.repo.clone()
    }
}

impl FromRef<AppState> for ReportService {
    fn from_ref(state: &AppState) -> Self {
        state.report_service.clone()
    }
}
