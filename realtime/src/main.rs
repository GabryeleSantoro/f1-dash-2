use anyhow::Error;
use shared::tracing_subscriber;
use tokio::sync::{broadcast, watch};
use tracing::{info, warn};

use crate::services::state_service::StateService;
use crate::source::{Broadcast, ReplayState, Source};

mod archive;
mod f1;
mod http_server;
mod source;
mod services {
    pub mod state_service;
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber();

    let state_service = StateService::new();

    let (sender, _) = broadcast::channel::<Broadcast>(64);
    let (source_tx, _) = watch::channel::<Source>(Source::Live);
    let replay_state = ReplayState::new();

    {
        let state_service = state_service.clone();
        let sender = sender.clone();
        let source_tx = source_tx.clone();
        let replay_state = replay_state.clone();

        tokio::spawn(async move {
            let mut iteration: u64 = 0;
            loop {
                let starting = source_tx.borrow().clone();

                if iteration > 0 {
                    let _ = sender.send(Broadcast::Reset);
                    replay_state.reset();
                }
                iteration += 1;

                let rx = source_tx.subscribe();
                let result = match starting.clone() {
                    Source::Live => {
                        info!("supervisor: starting live ingest");
                        f1::ingest_f1(state_service.clone(), sender.clone(), rx).await
                    }
                    Source::Archive {
                        path,
                        speed,
                        start_offset_ms,
                    } => {
                        info!(%path, speed, start_offset_ms, "supervisor: starting archive ingest");
                        archive::ingest_archive(
                            path,
                            speed,
                            start_offset_ms,
                            state_service.clone(),
                            sender.clone(),
                            replay_state.clone(),
                            rx,
                        )
                        .await
                    }
                };

                if let Err(err) = result {
                    warn!(?err, "ingest task returned error");
                }

                let after = source_tx.borrow().clone();
                if after == starting {
                    match starting {
                        Source::Archive { .. } => {
                            info!("archive playback finished, awaiting source change");
                            let mut wait_rx = source_tx.subscribe();
                            let _ = wait_rx.changed().await;
                        }
                        Source::Live => {
                            warn!("live ingest returned, restarting in 2s");
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        }
                    }
                }
            }
        });
    }

    http_server::start(state_service, sender, source_tx, replay_state).await?;

    Ok(())
}
