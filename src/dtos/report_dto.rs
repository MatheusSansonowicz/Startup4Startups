use chrono::NaiveDateTime;
use serde::Serialize;

use crate::models::report::StartupReport;

#[derive(Debug, Serialize)]
pub struct ReportResponse {
    pub startup_id: String,
    pub content: String,
    pub sources: Vec<String>,
    pub generated_at: NaiveDateTime,
    /// Diz ao client se isso veio do cache ou foi gerado agora —
    /// útil pro frontend mostrar "atualizado há Xh" vs "gerando...".
    pub from_cache: bool,
}

impl ReportResponse {
    pub fn new(report: StartupReport, from_cache: bool) -> Self {
        Self {
            startup_id: report.startup_id,
            content: report.content,
            sources: report.sources,
            generated_at: report.generated_at,
            from_cache,
        }
    }
}
