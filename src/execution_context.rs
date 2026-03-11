use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum ProgressEvent {
    StageStart(&'static str),
    StageDone(&'static str),
    Event {
        stage: &'static str,
        event_name: &'static str,
        tokens: u64,
    },
}

pub struct StageContext<'a> {
    parent: &'a ExecutionContext,
    stage: &'static str,
}

impl<'a> StageContext<'a> {
    pub fn record_events(&self, event_name: &'static str, tokens: u64) {
        self.parent.total_tokens.fetch_add(tokens, Ordering::Relaxed);

        if let Some(tx) = &self.parent.progress_tx {
            let _ = tx.send(ProgressEvent::Event {
                stage: self.stage,
                event_name,
                tokens,
            });
        }
    }

    pub fn finish(self) {
        if let Some(tx) = &self.parent.progress_tx {
            let _ = tx.send(ProgressEvent::StageDone(self.stage));
        }
    }
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

    pub fn enter_stage(&self, stage: &'static str) -> StageContext<'_> {
        if let Some(tx) = &self.progress_tx {
            let _ = tx.send(ProgressEvent::StageStart(stage));
        }

        StageContext {
            parent: self,
            stage,
        }
    }

    pub fn with_progress(tx: mpsc::UnboundedSender<ProgressEvent>) -> Arc<Self> {
        Arc::new(Self {
            total_tokens: AtomicU64::new(0),
            progress_tx: Some(tx),
        })
    }
}
