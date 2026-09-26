use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::models::startup::Startup;

/// DTO de entrada — o que o client manda no POST /startups.
/// Sem id, created_at, updated_at: isso é responsabilidade do Firestore.
#[derive(Debug, Deserialize)]
pub struct CreateStartupRequest {
    pub fantasy_name: String,
    pub legal_name: String,
    pub cnpj: String,
    pub description: String,
    pub founded_date: NaiveDate,
    pub website: Option<String>,
}

impl CreateStartupRequest {
    pub fn validar(&self) -> Result<(), String> {
        if self.fantasy_name.trim().is_empty() {
            return Err("nome fantasia é obrigatório".to_string());
        }
        if self.legal_name.trim().is_empty() {
            return Err("razão social é obrigatória".to_string());
        }
        if self.cnpj.len() != 14 {
            return Err("cnpj deve ter 14 dígitos".to_string());
        }
        Ok(())
    }
}

/// DTO de entrada — PATCH parcial. Tudo Option: só atualiza o que veio preenchido.
#[derive(Debug, Deserialize)]
pub struct UpdateStartupRequest {
    pub fantasy_name: Option<String>,
    pub legal_name: Option<String>,
    pub description: Option<String>,
    pub website: Option<String>,
}

/// DTO de saída — o que a API devolve ao client.
/// Note que não expõe legal_name/created_at/updated_at: só o necessário.
#[derive(Debug, Serialize)]
pub struct StartupResponse {
    pub id: String,
    pub fantasy_name: String,
    pub cnpj: String,
    pub description: String,
    pub founded_date: NaiveDate,
    pub website: Option<String>,
    pub registered_by: String,
}

impl From<Startup> for StartupResponse {
    fn from(s: Startup) -> Self {
        StartupResponse {
            id: s.id,
            fantasy_name: s.fantasy_name,
            cnpj: s.cnpj,
            description: s.description,
            founded_date: s.founded_date,
            website: s.website,
            registered_by: s.registered_by,
        }
    }
}
