use std::env;

/// Configuração de acesso ao Firestore (Firebase).
///
/// NOTA SOBRE AUTENTICAÇÃO: a API REST do Firestore exige um token
/// OAuth2 (Bearer) no header Authorization. Num app de produção,
/// o jeito certo é ler o JSON de uma service account e gerar tokens
/// de curta duração automaticamente (a crate `gcp_auth` faz isso
/// prontinho, renovando o token sozinho quando expira).
///
/// Pra manter este protótipo simples e sem dependências pesadas de
/// autenticação Google, carregamos o token direto do .env — você
/// gera um manualmente com `gcloud auth print-access-token` pra
/// testar. Ele expira em ~1h, então é só regenerar quando for testar
/// de novo.
#[derive(Debug, Clone)]
pub struct FirebaseConfig {
    pub project_id: String,
    pub access_token: String,
}

impl FirebaseConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let project_id = env::var("FIREBASE_PROJECT_ID")
            .expect("FIREBASE_PROJECT_ID não definido (veja .env.example)");
        let access_token = env::var("FIREBASE_ACCESS_TOKEN")
            .expect("FIREBASE_ACCESS_TOKEN não definido (veja .env.example)");

        Self {
            project_id,
            access_token,
        }
    }

    pub fn firestore_base_url(&self) -> String {
        format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents",
            self.project_id
        )
    }
}

/// Configuração do "diferencial": geração de relatório de mercado via IA.
/// Separada da FirebaseConfig porque é uma responsabilidade totalmente
/// diferente (chaves de terceiros, não infra do Firebase).
#[derive(Debug, Clone)]
pub struct ReportConfig {
    /// Chave da SerpAPI (busca de dados de mercado sobre a startup)
    pub serpapi_key: String,
    /// Chave da API de IA usada pra sintetizar o relatório
    pub ai_api_key: String,
    /// Quantas horas um relatório em cache é considerado "fresco"
    /// antes de precisar regenerar — isso é o que dá a sensação de
    /// "tempo real" sem chamar a IA a cada request (caro e lento).
    pub cache_ttl_hours: i64,
}

impl ReportConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let serpapi_key =
            env::var("SERPAPI_KEY").expect("SERPAPI_KEY não definido (veja .env.example)");
        let ai_api_key =
            env::var("AI_API_KEY").expect("AI_API_KEY não definido (veja .env.example)");
        let cache_ttl_hours = env::var("REPORT_CACHE_TTL_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(6); // padrão: relatório "vence" a cada 6h

        Self {
            serpapi_key,
            ai_api_key,
            cache_ttl_hours,
        }
    }
}
