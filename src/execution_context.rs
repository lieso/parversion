use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum ProgressEvent {
    StageStart(&'static str),
    StageDone(&'static str),
    TokensProcessed(u64),
}

#[derive(Debug)]
pub struct ExecutionContext {
    pub total_tokens: AtomicU64,
    pub progress_tx: Option<mpsc::UnboundedSender<ProgressEvent>>,
}

impl ExecutionContext {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            total_tokens: AtomicU64::new(0),
            progress_tx: None,
        })
    }

    pub fn with_progress(tx: mpsc::UnboundedSender<ProgressEvent>) -> Arc<Self> {
        Arc::new(Self {
            total_tokens: AtomicU64::new(0),
            progress_tx: Some(tx),
        })
    }

    pub fn increment_tokens(&self, n: u64) {
        self.total_tokens.fetch_add(n, Ordering::Relaxed);
        if let Some(tx) = &self.progress_tx {
            let _ = tx.send(ProgressEvent::TokensProcessed(n));
        }
    }

    pub fn stage_start(&self, name: &'static str) {
        if let Some(tx) = &self.progress_tx {
            let _ = tx.send(ProgressEvent::StageStart(name));
        }
    }

    pub fn stage_done(&self, name: &'static str) {
        if let Some(tx) = &self.progress_tx {
            let _ = tx.send(ProgressEvent::StageDone(name));
        }
    }
}
