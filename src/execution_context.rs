use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::mpsc;

use crate::prelude::*;

#[derive(Debug)]
pub enum ProgressEvent {
    StageStart { id: ID, parent: Option<ID>, name: &'static str },
    StageDone { id: ID },
    Event {
        stage: ID,
        event_name: &'static str,
        tokens: u64,
        cost: f64,
    },
}

#[derive(Clone)]
pub struct StageContext {
    execution_context: Arc<ExecutionContext>,
    parent: Option<Arc<StageContext>>,
    id: ID,
    name: &'static str,
}

impl StageContext {
    pub fn record_events(&self, event_name: &'static str, tokens: u64, cost: f64) {
        self.execution_context
            .total_tokens
            .fetch_add(tokens, Ordering::Relaxed);
        self.execution_context
            .total_cost
            .fetch_add((cost * 1_000_000_000.0).round() as u64, Ordering::Relaxed);

        if let Some(tx) = &self.execution_context.progress_tx {
            let _ = tx.send(ProgressEvent::Event {
                stage: self.id.clone(),
                event_name,
                tokens,
                cost,
            });
        }
    }

    pub fn finish(self) {
        if let Some(tx) = &self.execution_context.progress_tx {
            let _ = tx.send(ProgressEvent::StageDone {
                id: self.id.clone()
            });
        }
    }

    pub fn enter_stage(&self, name: &'static str) -> StageContext {
        StageContext {
            execution_context: Arc::clone(&self.execution_context),
            id: ID::new(),
            parent: Some(Arc::new(self.clone())),
            name,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExecutionSummary {
    pub total_tokens: u64,
    pub total_cost: f64, // USD
}

#[derive(Debug)]
pub struct ExecutionContext {
    pub total_tokens: AtomicU64,
    pub total_cost: AtomicU64, // USD * 1_000_000_000
    pub progress_tx: Option<mpsc::UnboundedSender<ProgressEvent>>,
}

impl ExecutionContext {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            total_tokens: AtomicU64::new(0),
            total_cost: AtomicU64::new(0),
            progress_tx: None,
        })
    }

    pub fn enter_stage(self: &Arc<Self>, name: &'static str) -> StageContext {
        let stage_id = ID::new();

        if let Some(tx) = &self.progress_tx {
            let _ = tx.send(ProgressEvent::StageStart {
                id: stage_id.clone(),
                parent: None,
                name,
            });
        }

        StageContext {
            execution_context: Arc::clone(&self),
            parent: None,
            id: stage_id,
            name,
        }
    }

    pub fn with_progress(tx: mpsc::UnboundedSender<ProgressEvent>) -> Arc<Self> {
        Arc::new(Self {
            total_tokens: AtomicU64::new(0),
            total_cost: AtomicU64::new(0),
            progress_tx: Some(tx),
        })
    }

    pub fn summary(&self) -> ExecutionSummary {
        ExecutionSummary {
            total_tokens: self.total_tokens.load(Ordering::Relaxed),
            total_cost: self.total_cost.load(Ordering::Relaxed) as f64 / 1_000_000_000.0,
        }
    }
}
