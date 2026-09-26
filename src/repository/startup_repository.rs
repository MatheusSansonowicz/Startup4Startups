use serde_json::json;

use crate::dtos::startup_dto::{CreateStartupRequest, UpdateStartupRequest};
use crate::error::AppError;
use crate::firebase::client::FirestoreClient;
use crate::firebase::convert::{create_request_to_fields, document_to_startup};
use crate::models::startup::Startup;

const COLLECTION: &str = "startups";

/// Equivale ao `StartupRepository` do seu padrão Spring, mas em vez de
/// esconder um JpaRepository/Hibernate, ele esconde chamadas HTTP ao
/// Firestore. Handlers nunca falam com FirestoreClient diretamente —
/// só conhecem esta interface.
#[derive(Clone)]
pub struct StartupRepository {
    firestore: FirestoreClient,
}

impl StartupRepository {
    pub fn new(firestore: FirestoreClient) -> Self {
        Self { firestore }
    }

    pub async fn criar(
        &self,
        dto: &CreateStartupRequest,
        registered_by: &str,
    ) -> Result<Startup, AppError> {
        let body = create_request_to_fields(dto, registered_by);
        let documento = self.firestore.create_document(COLLECTION, body).await?;
        document_to_startup(&documento)
    }

    pub async fn buscar_por_id(&self, id: &str) -> Result<Startup, AppError> {
        let documento = self.firestore.get_document(COLLECTION, id).await?;
        document_to_startup(&documento)
    }

    pub async fn listar(&self) -> Result<Vec<Startup>, AppError> {
        let documentos = self.firestore.list_documents(COLLECTION).await?;
        documentos.iter().map(document_to_startup).collect()
    }

    pub async fn atualizar(&self, id: &str, dto: UpdateStartupRequest) -> Result<Startup, AppError> {
        let mut fields = serde_json::Map::new();
        let mut mask: Vec<&str> = vec![];

        if let Some(nome) = &dto.fantasy_name {
            fields.insert("fantasy_name".into(), json!({ "stringValue": nome }));
            mask.push("fantasy_name");
        }
        if let Some(razao) = &dto.legal_name {
            fields.insert("legal_name".into(), json!({ "stringValue": razao }));
            mask.push("legal_name");
        }
        if let Some(descricao) = &dto.description {
            fields.insert("description".into(), json!({ "stringValue": descricao }));
            mask.push("description");
        }
        if let Some(site) = &dto.website {
            fields.insert("website".into(), json!({ "stringValue": site }));
            mask.push("website");
        }

        if mask.is_empty() {
            return Err(AppError::Validation(
                "nenhum campo enviado para atualização".to_string(),
            ));
        }

        fields.insert(
            "updated_at".into(),
            json!({ "timestampValue": chrono::Utc::now().to_rfc3339() }),
        );
        mask.push("updated_at");

        let body = json!({ "fields": fields });

        let documento = self
            .firestore
            .patch_document(COLLECTION, id, body, &mask)
            .await?;

        document_to_startup(&documento)
    }

    pub async fn deletar(&self, id: &str) -> Result<(), AppError> {
        self.firestore.delete_document(COLLECTION, id).await
    }
}
