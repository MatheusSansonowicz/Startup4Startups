use std::time::Duration;

use crate::repository::startup_repository::StartupRepository;
use crate::services::report_service::ReportService;

/// Roda pra sempre em background (disparado via tokio::spawn no main.rs),
/// verificando periodicamente todas as startups e regenerando o relatório
/// de quem estiver com cache vencido — é isso que dá o "sempre atualizado
/// em tempo real" sem depender de alguém abrir o app pra disparar o refresh.
///
/// Cada startup é verificada/atualizada numa task separada (tokio::spawn),
/// então uma IA lenta numa startup não atrasa as outras — o mesmo padrão
/// de concorrência de I/O que vimos com tokio::join! na busca de mercado.
pub async fn iniciar(repo: StartupRepository, report_service: ReportService, intervalo: Duration) {
    let mut ticker = tokio::time::interval(intervalo);

    loop {
        ticker.tick().await;

        let startups = match repo.listar().await {
            Ok(lista) => lista,
            Err(e) => {
                tracing::warn!("job de relatório: falha ao listar startups: {e}");
                continue;
            }
        };

        tracing::info!("job de relatório: verificando {} startup(s)", startups.len());

        for startup in startups {
            let report_service = report_service.clone();

            tokio::spawn(async move {
                // `obter_ou_gerar` já checa o TTL — só chama a IA de
                // verdade se o cache estiver vencido. Startups com
                // relatório fresco não geram custo nenhum aqui.
                if let Err(e) = report_service.obter_ou_gerar(&startup).await {
                    tracing::warn!(
                        "job de relatório: falha ao atualizar '{}': {e}",
                        startup.fantasy_name
                    );
                }
            });
        }
    }
}
