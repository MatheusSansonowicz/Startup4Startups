use crate::config::ReportConfig;
use crate::error::AppError;
use crate::firebase::client::FirestoreClient;
use crate::firebase::convert::{document_to_report, report_to_fields};
use crate::models::report::StartupReport;
use crate::models::startup::Startup;
use crate::services::{ai_service, search_service};

const COLLECTION: &str = "startup_reports";

#[derive(Clone)]
pub struct ReportService {
    firestore: FirestoreClient,
    config: ReportConfig,
}

impl ReportService {
    pub fn new(firestore: FirestoreClient, config: ReportConfig) -> Self {
        Self { firestore, config }
    }

    async fn buscar_cache(&self, startup_id: &str) -> Option<StartupReport> {
        match self.firestore.get_document(COLLECTION, startup_id).await {
            Ok(doc) => document_to_report(&doc).ok(),
            Err(_) => None, // sem cache ainda, ou erro — trata como "precisa gerar"
        }
    }

    /// Ponto de entrada principal: devolve o relatório em cache se ainda
    /// estiver fresco; senão, gera um novo (busca + IA) e salva.
    pub async fn obter_ou_gerar(
        &self,
        startup: &Startup,
    ) -> Result<(StartupReport, bool), AppError> {
        if let Some(cache) = self.buscar_cache(&startup.id).await {
            if cache.is_fresh(self.config.cache_ttl_hours) {
                return Ok((cache, true));
            }
        }

        let relatorio = self.gerar(startup).await?;
        Ok((relatorio, false))
    }

    /// Força a geração de um relatório novo, ignorando o cache —
    /// usado pelo endpoint de refresh manual e pelo job de background.
    pub async fn gerar(&self, startup: &Startup) -> Result<StartupReport, AppError> {
        let dados = search_service::coletar_dados_mercado(startup, &self.config.serpapi_key).await?;

        let texto =
            ai_service::gerar_relatorio_texto(startup, &dados, &self.config.ai_api_key).await?;

        let relatorio = StartupReport {
            startup_id: startup.id.clone(),
            content: texto,
            sources: dados.into_iter().map(|d| d.fonte).collect(),
            generated_at: chrono::Utc::now().naive_utc(),
        };

        let body = report_to_fields(&relatorio);
        self.firestore
            .upsert_document(COLLECTION, &startup.id, body)
            .await?;

        Ok(relatorio)
    }
}
