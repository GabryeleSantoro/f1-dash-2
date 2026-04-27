use std::sync::{Arc, atomic::Ordering};

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::http_server::Context;
use crate::source::Source;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartBody {
    pub path: String,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default)]
    pub start_offset_ms: u64,
}

fn default_speed() -> f32 {
    1.0
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeekBody {
    pub position_ms: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusBody {
    pub mode: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_ms: Option<u64>,
}

pub async fn start_replay(
    State(ctx): State<Arc<Context>>,
    Json(body): Json<StartBody>,
) -> Result<StatusCode, StatusCode> {
    if body.path.contains("..") || body.path.starts_with('/') || body.path.is_empty() {
        warn!(path = %body.path, "rejected replay start with invalid path");
        return Err(StatusCode::BAD_REQUEST);
    }
    let speed = if body.speed <= 0.0 || body.speed > 16.0 {
        1.0
    } else {
        body.speed
    };
    info!(path = %body.path, speed, start_offset_ms = body.start_offset_ms, "switching to archive replay");
    ctx.source_tx
        .send(Source::Archive {
            path: body.path,
            speed,
            start_offset_ms: body.start_offset_ms,
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn seek_replay(
    State(ctx): State<Arc<Context>>,
    Json(body): Json<SeekBody>,
) -> Result<StatusCode, StatusCode> {
    let current = ctx.source_tx.borrow().clone();
    let Source::Archive { path, speed, .. } = current else {
        return Err(StatusCode::CONFLICT);
    };
    info!(%path, position_ms = body.position_ms, "seeking archive replay");
    ctx.source_tx
        .send(Source::Archive {
            path,
            speed,
            start_offset_ms: body.position_ms,
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn stop_replay(State(ctx): State<Arc<Context>>) -> Result<StatusCode, StatusCode> {
    info!("switching to live source");
    ctx.source_tx
        .send(Source::Live)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn status(State(ctx): State<Arc<Context>>) -> Json<StatusBody> {
    let source = ctx.source_tx.borrow().clone();
    match source {
        Source::Live => Json(StatusBody {
            mode: "live",
            path: None,
            speed: None,
            position_ms: None,
            total_ms: None,
        }),
        Source::Archive { path, speed, .. } => Json(StatusBody {
            mode: "archive",
            path: Some(path),
            speed: Some(speed),
            position_ms: Some(ctx.replay_state.position_ms.load(Ordering::Relaxed)),
            total_ms: Some(ctx.replay_state.total_ms.load(Ordering::Relaxed)),
        }),
    }
}
