mod auth;
mod config;
mod dtos;
mod error;
mod firebase;
mod handlers;
mod jobs;
mod models;
mod repository;
mod services;
mod state;

use std::time::Duration;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, services::ServeDir};

use auth::middleware::exigir_autenticacao;
use config::{FirebaseConfig, ReportConfig};
use firebase::client::FirestoreClient;
use handlers::report_handler::{atualizar_relatorio, buscar_relatorio};
use handlers::startup_handler::{
    atualizar_startup, buscar_startup, criar_startup, deletar_startup, listar_startups,
};
use repository::startup_repository::StartupRepository;
use services::report_service::ReportService;
use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let firebase_config = FirebaseConfig::from_env();
    let report_config = ReportConfig::from_env();

    let firestore = FirestoreClient::new(firebase_config.clone());
    let repo = StartupRepository::new(firestore.clone());
    let report_service = ReportService::new(firestore, report_config);

    // Worker de background: verifica e atualiza relatórios vencidos a
    // cada 30 minutos, rodando na mesma runtime tokio da API, mas sem
    // bloquear as requisições HTTP (é uma task independente).
    tokio::spawn(jobs::report_refresher::iniciar(
        repo.clone(),
        report_service.clone(),
        Duration::from_secs(30 * 60),
    ));

    let state = AppState {
        repo,
        report_service,
    };

    let app = Router::new()
        .route("/startups", post(criar_startup).get(listar_startups))
        .route(
            "/startups/:id",
            get(buscar_startup)
                .put(atualizar_startup)
                .delete(deletar_startup),
        )
        .route("/startups/:id/report", get(buscar_relatorio))
        .route("/startups/:id/report/refresh", post(atualizar_relatorio))
        // O middleware roda ANTES de qualquer handler acima — sem
        // token válido, a requisição nem chega no repository/Firestore.
        .layer(middleware::from_fn_with_state(
            firebase_config.clone(),
            exigir_autenticacao,
        ))
        // CorsLayer fica por FORA do middleware de auth (adicionado
        // depois = camada mais externa) — assim o navegador consegue
        // completar o "preflight" (requisição OPTIONS automática antes
        // de POST/PUT/DELETE) sem cair no bloqueio de token ausente.
        // Em produção, troque `permissive()` por `CorsLayer::new()`
        // com `.allow_origin` restrito ao domínio real do frontend.
        .layer(CorsLayer::permissive())
        .with_state(state)
        // IMPORTANTE: fallback_service é adicionado DEPOIS dos .layer()
        // acima — em Axum, .layer() só envolve as rotas que já existem
        // no Router NAQUELE momento. Como as rotas /startups* já foram
        // "seladas" com auth+cors antes deste ponto, o fallback abaixo
        // fica de fora dessas camadas: ele serve os arquivos estáticos
        // de `frontend/` (index.html, etc.) SEM exigir token — faz
        // sentido, já que a própria página de login precisa carregar
        // antes de existir qualquer token pra mandar.
        .fallback_service(ServeDir::new("frontend"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("não foi possível abrir a porta 3000");

    tracing::info!("API + frontend rodando em http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
