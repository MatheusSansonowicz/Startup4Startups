use reqwest::{Client, StatusCode};
use serde_json::Value;

use crate::config::FirebaseConfig;
use crate::error::AppError;
use crate::firebase::convert::FirestoreDocument;

/// Cliente fino sobre a API REST do Firestore.
///
/// Importante: essa camada não sabe o que é uma "Startup" — ela só
/// fala a língua genérica de "documentos" do Firestore (get/list/
/// create/patch/delete). Quem traduz "Startup" <-> "documento" é o
/// StartupRepository, que usa este client por baixo.
#[derive(Clone)]
pub struct FirestoreClient {
    http: Client,
    config: FirebaseConfig,
}

impl FirestoreClient {
    pub fn new(config: FirebaseConfig) -> Self {
        Self {
            http: Client::new(),
            config,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.access_token)
    }

    pub async fn create_document(
        &self,
        collection: &str,
        body: Value,
    ) -> Result<FirestoreDocument, AppError> {
        let url = format!("{}/{}", self.config.firestore_base_url(), collection);

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .await?;

        Self::parse_documento(resp).await
    }

    pub async fn get_document(&self, collection: &str, id: &str) -> Result<FirestoreDocument, AppError> {
        let url = format!("{}/{}/{}", self.config.firestore_base_url(), collection, id);

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound);
        }

        Self::parse_documento(resp).await
    }

    pub async fn list_documents(&self, collection: &str) -> Result<Vec<FirestoreDocument>, AppError> {
        let url = format!("{}/{}", self.config.firestore_base_url(), collection);

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        let status = resp.status();
        let corpo: Value = resp.json().await?;

        if !status.is_success() {
            return Err(AppError::Firestore(corpo.to_string()));
        }

        let documentos = corpo
            .get("documents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        documentos
            .into_iter()
            .map(|v| serde_json::from_value(v).map_err(|e| AppError::Internal(e.to_string())))
            .collect()
    }

    pub async fn patch_document(
        &self,
        collection: &str,
        id: &str,
        body: Value,
        update_mask_fields: &[&str],
    ) -> Result<FirestoreDocument, AppError> {
        // O Firestore exige um updateMask explícito no PATCH, senão ele
        // sobrescreve o documento inteiro (apagando campos não enviados).
        let mask_query: String = update_mask_fields
            .iter()
            .map(|campo| format!("updateMask.fieldPaths={campo}"))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!(
            "{}/{}/{}?{}",
            self.config.firestore_base_url(),
            collection,
            id,
            mask_query
        );

        let resp = self
            .http
            .patch(&url)
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .await?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound);
        }

        Self::parse_documento(resp).await
    }

    pub async fn delete_document(&self, collection: &str, id: &str) -> Result<(), AppError> {
        let url = format!("{}/{}/{}", self.config.firestore_base_url(), collection, id);

        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        match resp.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(AppError::NotFound),
            status => Err(AppError::Firestore(format!("status inesperado do Firestore: {status}"))),
        }
    }

    /// Cria OU sobrescreve um documento por completo, sem exigir que ele
    /// já exista (diferente de patch_document, que usa updateMask e falha
    /// com 404 se o documento não existir). Sem updateMask na query, o
    /// Firestore substitui o documento inteiro — ótimo pra cache, onde
    /// você sempre manda o objeto completo de novo.
    pub async fn upsert_document(
        &self,
        collection: &str,
        id: &str,
        body: Value,
    ) -> Result<FirestoreDocument, AppError> {
        let url = format!("{}/{}/{}", self.config.firestore_base_url(), collection, id);

        let resp = self
            .http
            .patch(&url)
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .await?;

        Self::parse_documento(resp).await
    }

    async fn parse_documento(resp: reqwest::Response) -> Result<FirestoreDocument, AppError> {
        let status = resp.status();
        let corpo: Value = resp.json().await?;

        if !status.is_success() {
            return Err(AppError::Firestore(corpo.to_string()));
        }

        serde_json::from_value(corpo).map_err(|e| AppError::Internal(e.to_string()))
    }
}
