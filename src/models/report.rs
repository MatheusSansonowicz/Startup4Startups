use chrono::NaiveDateTime;
use serde::Serialize;

/// Relatório de mercado gerado por IA sobre uma startup.
/// Guardado no Firestore na coleção `startup_reports`, com o MESMO id
/// da startup — assim dá pra buscar direto por id sem precisar de query.
#[derive(Debug, Clone, Serialize)]
pub struct StartupReport {
    pub startup_id: String,
    pub content: String,
    /// URLs/fontes usadas na geração — transparência de onde veio o dado
    pub sources: Vec<String>,
    pub generated_at: NaiveDateTime,
}

impl StartupReport {
    /// Um relatório é "fresco" se foi gerado há menos de `ttl_hours`.
    /// É essa checagem que evita chamar a IA de novo a cada request.
    pub fn is_fresh(&self, ttl_hours: i64) -> bool {
        let idade = chrono::Utc::now().naive_utc() - self.generated_at;
        idade.num_hours() < ttl_hours
    }
}
