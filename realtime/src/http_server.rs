use std::{env, sync::Arc};

use anyhow::Error;
use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::{get, post},
};
use tokio::{
    net::TcpListener,
    sync::{broadcast::Sender, watch},
};
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::services::state_service::StateService;
use crate::source::{Broadcast, ReplayState, Source};

mod connections;
mod current;
mod drivers;
mod health;
mod realtime;
mod replay;

pub struct Context {
    pub state_service: StateService,
    pub tx: Sender<Broadcast>,
    pub source_tx: watch::Sender<Source>,
    pub replay_state: ReplayState,
}

pub async fn start(
    state_service: StateService,
    tx: Sender<Broadcast>,
    source_tx: watch::Sender<Source>,
    replay_state: ReplayState,
) -> Result<(), Error> {
    let addr = env::var("ADDRESS").unwrap_or_else(|_| "0.0.0.0:80".to_string());

    let context = Arc::new(Context {
        state_service,
        tx,
        source_tx,
        replay_state,
    });

    let cors = cors_layer()?;

    let app = Router::new()
        .route("/api/health", get(health::health_check))
        .route("/api/realtime", get(realtime::sse_stream))
        .route("/api/current", get(current::current_state))
        .route("/api/drivers", get(drivers::drivers))
        .route("/api/connections", get(connections::current_connections))
        .route("/api/replay/start", post(replay::start_replay))
        .route("/api/replay/stop", post(replay::stop_replay))
        .route("/api/replay/status", get(replay::status))
        .with_state(context)
        .layer(cors)
        .into_make_service();

    info!(addr, "starting norths http server");

    axum::serve(TcpListener::bind(addr).await?, app).await?;

    Ok(())
}

pub fn cors_layer() -> Result<CorsLayer, Error> {
    let origin = env::var("ORIGIN").unwrap_or_else(|_| "https://f1-dash.com".to_string());

    let origins = origin
        .split(';')
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect::<Vec<HeaderValue>>();

    Ok(CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::CONNECT]))
}
