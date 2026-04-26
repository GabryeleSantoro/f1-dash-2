use anyhow::Error;
use serde_json::{Value, json};
use tokio::sync::{broadcast::Sender, watch};
use tokio_stream::StreamExt;
use tracing::{error, info, trace, warn};

use crate::services::state_service::StateService;
use crate::source::{Broadcast, Source};

const URL: &str = "livetiming.formula1.com/signalr";
const HUB: &str = "Streaming";

pub const TOPICS: [&str; 17] = [
    "Heartbeat",
    "CarData.z",
    "Position.z",
    "ExtrapolatedClock",
    "TimingStats",
    "TimingAppData",
    "WeatherData",
    "TrackStatus",
    "SessionStatus",
    "DriverList",
    "RaceControlMessages",
    "SessionInfo",
    "SessionData",
    "LapCount",
    "TimingData",
    "TeamRadio",
    "ChampionshipPrediction",
];

pub async fn ingest_f1(
    state_service: StateService,
    update_sender: Sender<Broadcast>,
    mut source_rx: watch::Receiver<Source>,
) -> Result<(), Error> {
    let mut signalr_client = signalr::create_client(URL, HUB).await?;

    let initial = signalr::subscribe(&mut signalr_client, &TOPICS).await?;
    handle_initial(&update_sender, &state_service, initial).await?;

    let mut stream = signalr::listen(signalr_client);

    loop {
        tokio::select! {
            biased;
            _ = source_rx.changed() => {
                info!("live ingest cancelled (source changed)");
                return Ok(());
            }
            next = stream.next() => {
                let Some(items) = next else { break };
                for update in items {
                    trace!(?update.topic, "Received data for topic");

                    if update.topic == "SessionInfo" && update.data.pointer("/Name").is_some() {
                        warn!("received SessionInfo event, restarting...");
                        return Ok(());
                    }

                    match handle_update(&update_sender, &state_service, update.topic, update.data).await {
                        Ok(_) => trace!("handled update"),
                        Err(err) => error!(?err, "failed to handle update"),
                    };
                }
            }
        }
    }

    Ok(())
}

async fn handle_update(
    sender: &Sender<Broadcast>,
    state_service: &StateService,
    topic: String,
    update: Value,
) -> Result<(), Error> {
    let update = json!({ topic: update });

    match sender.send(Broadcast::Update(update.to_string())) {
        Ok(_) => trace!("sent update to realtime channel"),
        Err(_) => trace!("no subscribers for live update"),
    };

    state_service.update_state(update).await?;

    Ok(())
}

async fn handle_initial(
    sender: &Sender<Broadcast>,
    state_service: &StateService,
    initial: Value,
) -> Result<(), Error> {
    trace!("handling initial state");
    state_service.set_state(initial.clone()).await?;
    let _ = sender.send(Broadcast::Initial(initial.to_string()));
    Ok(())
}
