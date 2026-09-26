use serde::Deserialize;
use serde_json::Value;

use crate::error::AppError;
use crate::models::startup::Startup;

#[derive(Debug)]
pub struct ResultadoBusca {
    pub trecho: String,
    pub fonte: String,
}

#[derive(Debug, Deserialize)]
struct SerpApiResponse {
    #[serde(default)]
    organic_results: Vec<OrganicResult>,
}

#[derive(Debug, Deserialize)]
struct OrganicResult {
    #[serde(default)]
    snippet: String,
    #[serde(default)]
    link: String,
}

async fn buscar(query: &str, api_key: &str) -> Result<Vec<ResultadoBusca>, AppError> {
    let url = "https://serpapi.com/search.json";

    let resp = reqwest::Client::new()
        .get(url)
        .query(&[("q", query), ("api_key", api_key), ("num", "5")])
        .send()
        .await
        .map_err(|e| AppError::ExternalService(format!("SerpAPI: {e}")))?;

    let corpo: Value = resp
        .json()
        .await
        .map_err(|e| AppError::ExternalService(format!("SerpAPI (parse): {e}")))?;

    let parsed: SerpApiResponse = serde_json::from_value(corpo)
        .map_err(|e| AppError::ExternalService(format!("SerpAPI (formato): {e}")))?;

    Ok(parsed
        .organic_results
        .into_iter()
        .filter(|r| !r.snippet.is_empty())
        .map(|r| ResultadoBusca {
            trecho: r.snippet,
            fonte: r.link,
        })
        .collect())
}

/// Coleta dados de mercado sobre a startup fazendo VÁRIAS buscas em
/// paralelo — isso é exatamente o cenário de I/O concorrente que
/// conversamos: cada busca é uma chamada de rede independente, então
/// `tokio::join!` dispara as três ao mesmo tempo em vez de uma
/// esperar a outra terminar.
pub async fn coletar_dados_mercado(
    startup: &Startup,
    api_key: &str,
) -> Result<Vec<ResultadoBusca>, AppError> {
    let query_geral = format!("{} startup Brasil", startup.fantasy_name);
    let query_concorrentes = format!("concorrentes de {}", startup.fantasy_name);
    let query_avaliacoes = format!("{} avaliação clientes", startup.fantasy_name);

    let (geral, concorrentes, avaliacoes) = tokio::join!(
        buscar(&query_geral, api_key),
        buscar(&query_concorrentes, api_key),
        buscar(&query_avaliacoes, api_key),
    );

    let mut resultados = geral?;
    resultados.extend(concorrentes?);
    resultados.extend(avaliacoes?);

    Ok(resultados)
}
