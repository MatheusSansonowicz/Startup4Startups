use std::time::{Duration, Instant};

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use once_cell::sync::Lazy;
use serde::Deserialize;
use tokio::sync::RwLock;

const JWK_URL: &str =
    "https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com";

// O Google roda as chaves periodicamente. 1h é uma margem segura pra
// não bater na API deles a cada request, mas ainda pegar rotações.
const CACHE_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize, Clone)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
}

struct CacheDeChaves {
    chaves: Vec<Jwk>,
    buscado_em: Instant,
}

// Cache global em memória do processo — evita buscar as chaves do
// Google a cada requisição. `once_cell::Lazy` é o jeito idiomático em
// Rust de ter um "estático" inicializado sob demanda (não existe
// static mutável direto por causa das garantias de thread-safety).
static CACHE: Lazy<RwLock<Option<CacheDeChaves>>> = Lazy::new(|| RwLock::new(None));

/// Claims relevantes de um ID Token do Firebase.
/// Referência: https://firebase.google.com/docs/auth/admin/verify-id-tokens
#[derive(Debug, Deserialize)]
pub struct FirebaseClaims {
    /// uid do usuário autenticado — é isso que vira "registered_by"
    pub sub: String,
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("token ausente ou mal formatado")]
    Malformado,
    #[error("token inválido: {0}")]
    Invalido(String),
    #[error("falha ao buscar chaves públicas do Google: {0}")]
    Chaves(String),
}

async fn buscar_chaves_atualizadas() -> Result<Vec<Jwk>, TokenError> {
    let resp = reqwest::get(JWK_URL)
        .await
        .map_err(|e| TokenError::Chaves(e.to_string()))?;

    let jwks: Jwks = resp
        .json()
        .await
        .map_err(|e| TokenError::Chaves(e.to_string()))?;

    Ok(jwks.keys)
}

async fn obter_chave(kid: &str) -> Result<Jwk, TokenError> {
    {
        let cache = CACHE.read().await;
        if let Some(c) = cache.as_ref() {
            if c.buscado_em.elapsed() < CACHE_TTL {
                if let Some(chave) = c.chaves.iter().find(|k| k.kid == kid) {
                    return Ok(chave.clone());
                }
            }
        }
    }

    // Cache vazio, expirado ou kid novo (rotação de chave) — busca de novo.
    let chaves = buscar_chaves_atualizadas().await?;
    let encontrada = chaves
        .iter()
        .find(|k| k.kid == kid)
        .cloned()
        .ok_or_else(|| TokenError::Invalido("kid não encontrado nas chaves do Google".into()))?;

    let mut cache = CACHE.write().await;
    *cache = Some(CacheDeChaves {
        chaves,
        buscado_em: Instant::now(),
    });

    Ok(encontrada)
}

/// Verifica um ID Token do Firebase e devolve os claims.
///
/// `project_id` é o Project ID do Firebase — o token só passa se `aud`
/// e `iss` baterem com ele. Isso impede que um token válido de OUTRO
/// projeto Firebase seja aceito aqui por engano.
pub async fn verificar_id_token(
    token: &str,
    project_id: &str,
) -> Result<FirebaseClaims, TokenError> {
    let header = decode_header(token).map_err(|_| TokenError::Malformado)?;
    let kid = header.kid.ok_or(TokenError::Malformado)?;

    let jwk = obter_chave(&kid).await?;
    let chave_decodificacao = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
        .map_err(|e| TokenError::Invalido(e.to_string()))?;

    let mut validacao = Validation::new(Algorithm::RS256);
    validacao.set_audience(&[project_id]);
    validacao.set_issuer(&[format!("https://securetoken.google.com/{project_id}")]);

    let dados = decode::<FirebaseClaims>(token, &chave_decodificacao, &validacao)
        .map_err(|e| TokenError::Invalido(e.to_string()))?;

    Ok(dados.claims)
}
