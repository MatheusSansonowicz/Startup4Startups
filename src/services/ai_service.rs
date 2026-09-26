use serde_json::{json, Value};

use crate::error::AppError;
use crate::models::startup::Startup;
use crate::services::search_service::ResultadoBusca;

/// Monta o prompt combinando os dados estruturados da startup com os
/// trechos coletados na busca de mercado.
fn montar_prompt(startup: &Startup, dados: &[ResultadoBusca]) -> String {
    let contexto = dados
        .iter()
        .map(|d| format!("- {} (fonte: {})", d.trecho, d.fonte))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Você é um analista de mercado. Gere um relatório curto (3-4 parágrafos) \
         sobre a startup abaixo, no estilo de uma pesquisa de mercado objetiva.\n\n\
         Nome fantasia: {}\n\
         Setor/descrição: {}\n\
         Site: {}\n\n\
         Dados coletados na web sobre a startup e concorrentes:\n{}\n\n\
         Escreva o relatório em português, citando concorrentes ou contexto de \
         mercado quando os dados permitirem. Se os dados forem insuficientes, \
         seja honesto sobre isso em vez de inventar informação.",
        startup.fantasy_name,
        startup.description,
        startup.website.as_deref().unwrap_or("não informado"),
        if contexto.is_empty() {
            "(nenhum dado externo encontrado)".to_string()
        } else {
            contexto
        }
    )
}

/// Chama a API da Anthropic pra gerar o texto do relatório.
/// Troque por outro provedor (OpenAI, Gemini) trocando só esta função —
/// o resto do report_service não precisa saber qual IA está por trás.
pub async fn gerar_relatorio_texto(
    startup: &Startup,
    dados: &[ResultadoBusca],
    api_key: &str,
) -> Result<String, AppError> {
    let prompt = montar_prompt(startup, dados);

    let body = json!({
        "model": "claude-sonnet-5",
        "max_tokens": 800,
        "messages": [
            { "role": "user", "content": prompt }
        ]
    });

    let resp = reqwest::Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::ExternalService(format!("API de IA: {e}")))?;

    let corpo: Value = resp
        .json()
        .await
        .map_err(|e| AppError::ExternalService(format!("API de IA (parse): {e}")))?;

    corpo
        .get("content")
        .and_then(Value::as_array)
        .and_then(|blocos| blocos.first())
        .and_then(|bloco| bloco.get("text"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| AppError::ExternalService(format!("resposta inesperada da IA: {corpo}")))
}
