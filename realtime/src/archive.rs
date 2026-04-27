use std::sync::atomic::Ordering;

use anyhow::Error;
use serde_json::{Value, json};
use tokio::sync::{broadcast::Sender, watch};
use tracing::{debug, error, info, trace, warn};

use crate::f1::TOPICS;
use crate::services::state_service::StateService;
use crate::source::{Broadcast, ReplayState, Source};

const F1_STATIC: &str = "https://livetiming.formula1.com/static";

async fn fetch_text(url: &str) -> Result<String, Error> {
    let body = reqwest::get(url).await?.error_for_status()?.text().await?;
    Ok(body.trim_start_matches('\u{feff}').to_string())
}

fn parse_timestamp(prefix: &str) -> Option<u64> {
    // expected format: HH:MM:SS.mmm
    let bytes = prefix.as_bytes();
    if bytes.len() != 12
        || bytes[2] != b':'
        || bytes[5] != b':'
        || bytes[8] != b'.'
    {
        return None;
    }
    let h: u64 = prefix[0..2].parse().ok()?;
    let m: u64 = prefix[3..5].parse().ok()?;
    let s: u64 = prefix[6..8].parse().ok()?;
    let ms: u64 = prefix[9..12].parse().ok()?;
    Some(((h * 3600 + m * 60 + s) * 1000) + ms)
}

#[derive(Debug)]
struct TimelineEntry {
    timestamp_ms: u64,
    topic: String,
    payload: Value,
}

async fn fetch_topic_stream(path: &str, topic: &str) -> Result<Vec<TimelineEntry>, Error> {
    let url = format!("{F1_STATIC}/{path}{topic}.jsonStream");
    let body = fetch_text(&url).await?;

    let mut entries = Vec::new();
    for raw_line in body.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.len() < 12 {
            continue;
        }
        let (prefix, json_str) = line.split_at(12);
        let Some(ts) = parse_timestamp(prefix) else {
            continue;
        };
        let payload: Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(err) => {
                warn!(?err, topic, "failed to parse stream line json");
                continue;
            }
        };
        entries.push(TimelineEntry {
            timestamp_ms: ts,
            topic: topic.to_string(),
            payload,
        });
    }

    Ok(entries)
}

pub async fn ingest_archive(
    path: String,
    speed: f32,
    start_offset_ms: u64,
    state_service: StateService,
    sender: Sender<Broadcast>,
    replay_state: ReplayState,
    mut source_rx: watch::Receiver<Source>,
) -> Result<(), Error> {
    let speed = if speed <= 0.0 { 1.0 } else { speed };

    info!(%path, speed, start_offset_ms, "starting archive ingest");

    // start clean
    state_service.set_state(Value::Object(serde_json::Map::new())).await?;
    replay_state.reset();

    // fetch all topic streams in parallel
    let mut tasks = Vec::new();
    for topic in TOPICS.iter() {
        let path = path.clone();
        let topic = topic.to_string();
        tasks.push(tokio::spawn(async move {
            (topic.clone(), fetch_topic_stream(&path, &topic).await)
        }));
    }

    let mut timeline: Vec<TimelineEntry> = Vec::new();
    for task in tasks {
        match task.await {
            Ok((topic, Ok(entries))) => {
                debug!(topic, count = entries.len(), "fetched topic stream");
                timeline.extend(entries);
            }
            Ok((topic, Err(err))) => {
                warn!(?err, topic, "failed to fetch topic stream (skipping)");
            }
            Err(err) => {
                warn!(?err, "topic fetch task panicked");
            }
        }
    }

    if timeline.is_empty() {
        return Err(anyhow::anyhow!("no timeline entries fetched for path {path}"));
    }

    timeline.sort_by_key(|e| e.timestamp_ms);

    let total_ms = timeline.last().map(|e| e.timestamp_ms).unwrap_or(0);
    replay_state.total_ms.store(total_ms, Ordering::Relaxed);
    info!(entries = timeline.len(), total_ms, "archive timeline ready");

    // fast-forward: silently merge entries before offset to build initial state
    let split_idx = timeline.partition_point(|e| e.timestamp_ms < start_offset_ms);
    if split_idx > 0 {
        for entry in &timeline[..split_idx] {
            let mut map = serde_json::Map::new();
            map.insert(entry.topic.clone(), entry.payload.clone());
            if let Err(err) = state_service.update_state(Value::Object(map)).await {
                error!(?err, "failed to merge archive prefill");
            }
        }
        info!(prefilled = split_idx, "archive prefill complete");
    }

    // emit initial state at offset
    let initial = state_service.get_state_string().await?;
    let _ = sender.send(Broadcast::Initial(initial));
    replay_state
        .position_ms
        .store(start_offset_ms, Ordering::Relaxed);

    let mut prev_ts: u64 = start_offset_ms;
    for entry in timeline.into_iter().skip(split_idx) {
        let dt_ms = entry.timestamp_ms.saturating_sub(prev_ts);
        let sleep_ms = (dt_ms as f32 / speed).round() as u64;

        if sleep_ms > 0 {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)) => {},
                _ = source_rx.changed() => {
                    info!("archive ingest cancelled (source changed)");
                    return Ok(());
                }
            }
        } else if source_rx.has_changed().unwrap_or(false) {
            info!("archive ingest cancelled (source changed)");
            return Ok(());
        }

        prev_ts = entry.timestamp_ms;
        replay_state
            .position_ms
            .store(entry.timestamp_ms, Ordering::Relaxed);

        let wrapped = json!({ entry.topic: entry.payload });
        if let Err(err) = state_service.update_state(wrapped.clone()).await {
            error!(?err, "failed to merge archive update");
            continue;
        }

        match sender.send(Broadcast::Update(wrapped.to_string())) {
            Ok(_) => trace!("sent archive update"),
            Err(_) => trace!("no subscribers for archive update"),
        }
    }

    info!("archive ingest finished playback");
    Ok(())
}
