use crate::models::queue::QueueItemInfo;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct QueueItemProgress {
    pub id: u64,
    pub title: String,
    pub platform: String,
    pub percent: f64,
    pub speed_bytes_per_sec: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub phase: String,
}

pub trait EventEmitter: Send + Sync + Clone + 'static {
    fn emit_queue_state(&self, items: &[QueueItemInfo]);
    fn emit_progress(&self, progress: &QueueItemProgress);
}
