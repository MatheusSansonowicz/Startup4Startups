use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::dtos::startup_dto::CreateStartupRequest;
use crate::error::AppError;
use crate::models::report::StartupReport;
use crate::models::startup::Startup;

/// A API REST do Firestore não devolve JSON "cru" — cada valor vem
/// tipado, ex: {"fields": {"nome": {"stringValue": "Mirkos"}}}.
/// Essa struct representa esse envelope genérico de documento.
#[derive(Debug, Deserialize)]
pub struct FirestoreDocument {
    /// Path completo, ex: projects/x/databases/(default)/documents/startups/abc123
    pub name: String,
    #[serde(default)]
    pub fields: Value,
}

impl FirestoreDocument {
    /// O Firestore não devolve o ID isolado — só o path inteiro.
    /// Isso extrai só a última parte (o ID do documento).
    pub fn id(&self) -> String {
        self.name.rsplit('/').next().unwrap_or_default().to_string()
    }
}

/// Monta o corpo (body) da requisição de criação a partir do DTO de entrada.
///
/// `registered_by` NÃO vem do `dto` de propósito: se ele viesse do JSON
/// que o client manda, qualquer um poderia forjar esse campo dizendo
/// ser outra pessoa. Ele é passado à parte, vindo do uid já verificado
/// pelo middleware de autenticação.
pub fn create_request_to_fields(dto: &CreateStartupRequest, registered_by: &str) -> Value {
    let agora = Utc::now();
    let timestamp_agora = agora.to_rfc3339();

    json!({
        "fields": {
            "fantasy_name": { "stringValue": dto.fantasy_name },
            "legal_name": { "stringValue": dto.legal_name },
            "cnpj": { "stringValue": dto.cnpj },
            "description": { "stringValue": dto.description },
            "founded_date": { "stringValue": dto.founded_date.to_string() },
            "website": match &dto.website {
                Some(url) => json!({ "stringValue": url }),
                None => json!({ "nullValue": Value::Null }),
            },
            "registered_by": { "stringValue": registered_by },
            "created_at": { "timestampValue": timestamp_agora },
            "updated_at": { "timestampValue": timestamp_agora },
        }
    })
}

/// Converte um documento cru do Firestore de volta pra struct Startup.
/// Isso é o "espelho" da função acima — e é aqui que erros de schema
/// (campo faltando, tipo errado) viram AppError::Internal em vez de
/// panic ou null silencioso.
pub fn document_to_startup(doc: &FirestoreDocument) -> Result<Startup, AppError> {
    let campo_ausente = |campo: &str| AppError::Internal(format!("campo '{campo}' ausente ou com tipo inesperado no Firestore"));

    let get_str = |campo: &str| -> Result<String, AppError> {
        doc.fields
            .get(campo)
            .and_then(|v| v.get("stringValue"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| campo_ausente(campo))
    };

    let get_opt_str = |campo: &str| -> Option<String> {
        doc.fields
            .get(campo)
            .and_then(|v| v.get("stringValue"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };

    let get_timestamp = |campo: &str| -> Result<chrono::NaiveDateTime, AppError> {
        let bruto = doc
            .fields
            .get(campo)
            .and_then(|v| v.get("timestampValue"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| campo_ausente(campo))?;

        chrono::DateTime::parse_from_rfc3339(bruto)
            .map(|dt| dt.naive_utc())
            .map_err(|e| AppError::Internal(format!("timestamp inválido em '{campo}': {e}")))
    };

    let founded_date_bruto = get_str("founded_date")?;
    let founded_date = NaiveDate::parse_from_str(&founded_date_bruto, "%Y-%m-%d")
        .map_err(|e| AppError::Internal(format!("founded_date inválida: {e}")))?;

    Ok(Startup {
        id: doc.id(),
        fantasy_name: get_str("fantasy_name")?,
        legal_name: get_str("legal_name")?,
        cnpj: get_str("cnpj")?,
        description: get_str("description")?,
        founded_date,
        website: get_opt_str("website"),
        registered_by: get_str("registered_by")?,
        created_at: get_timestamp("created_at")?,
        updated_at: get_timestamp("updated_at")?,
    })
}

/// Monta o corpo de upsert do relatório — inclui o array `sources`,
/// que no Firestore é representado como `arrayValue` com uma lista de
/// `stringValue`s (o formato tipado exige isso pra cada item).
pub fn report_to_fields(report: &StartupReport) -> Value {
    let sources_values: Vec<Value> = report
        .sources
        .iter()
        .map(|s| json!({ "stringValue": s }))
        .collect();

    json!({
        "fields": {
            "startup_id": { "stringValue": report.startup_id },
            "content": { "stringValue": report.content },
            "sources": { "arrayValue": { "values": sources_values } },
            "generated_at": { "timestampValue": report.generated_at.and_utc().to_rfc3339() },
        }
    })
}

pub fn document_to_report(doc: &FirestoreDocument) -> Result<StartupReport, AppError> {
    let campo_ausente = |campo: &str| {
        AppError::Internal(format!("campo '{campo}' ausente no relatório do Firestore"))
    };

    let content = doc
        .fields
        .get("content")
        .and_then(|v| v.get("stringValue"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| campo_ausente("content"))?;

    let sources = doc
        .fields
        .get("sources")
        .and_then(|v| v.get("arrayValue"))
        .and_then(|v| v.get("values"))
        .and_then(Value::as_array)
        .map(|itens| {
            itens
                .iter()
                .filter_map(|v| v.get("stringValue").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let generated_at_bruto = doc
        .fields
        .get("generated_at")
        .and_then(|v| v.get("timestampValue"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| campo_ausente("generated_at"))?;

    let generated_at = chrono::DateTime::parse_from_rfc3339(generated_at_bruto)
        .map(|dt| dt.naive_utc())
        .map_err(|e| AppError::Internal(format!("generated_at inválido: {e}")))?;

    Ok(StartupReport {
        startup_id: doc.id(),
        content,
        sources,
        generated_at,
    })
}
