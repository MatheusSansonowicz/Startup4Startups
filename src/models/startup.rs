use chrono::{NaiveDate, NaiveDateTime};
use serde::Serialize;

/// Entidade — representa a startup como ela existe no Firestore.
///
/// Diferença importante em relação a um banco relacional: `id` aqui é
/// uma String (o ID do documento gerado pelo Firestore), não um
/// inteiro SERIAL como seria numa tabela Postgres/MySQL.
#[derive(Debug, Clone, Serialize)]
pub struct Startup {
    pub id: String,
    pub fantasy_name: String,
    pub legal_name: String,
    pub cnpj: String,
    pub description: String,
    pub founded_date: NaiveDate,
    pub website: Option<String>,
    /// uid (Firebase Auth) de quem criou o registro — nunca vem do
    /// client, sempre extraído do token verificado no middleware.
    pub registered_by: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Startup {
    /// Regra de negócio simples que vive na entidade, não no repository
    /// nem no handler — ela só depende dos próprios dados da struct.
    pub fn is_valid(&self) -> bool {
        !self.fantasy_name.trim().is_empty() && self.cnpj.len() == 14
    }
}
