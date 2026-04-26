use std::sync::{Arc, atomic::AtomicU64};

#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Live,
    Archive { path: String, speed: f32 },
}

#[derive(Clone, Debug)]
pub enum Broadcast {
    Reset,
    Initial(String),
    Update(String),
}

#[derive(Clone, Default)]
pub struct ReplayState {
    pub position_ms: Arc<AtomicU64>,
    pub total_ms: Arc<AtomicU64>,
}

impl ReplayState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&self) {
        self.position_ms
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.total_ms
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }
}
