//! Native portal networking is unavailable to headless WASM callers.
//!
//! The web editor uses its browser API bridge for these operations.

use serde_json::Value;

use crate::{Args, Automation, Result, ToolError};

fn unavailable() -> ToolError {
    ToolError::Failed("This connected tool requires the native easyPDF CLI; the web editor uses its browser connection".into())
}

impl Automation {
    pub(super) fn ai_document(&mut self, _args: &Args<'_>, _task: &str) -> Result<Value> {
        Err(unavailable())
    }

    pub(super) fn portal_share_document(&mut self, _args: &Args<'_>) -> Result<Value> {
        Err(unavailable())
    }

    pub(super) fn portal_snapshot_list(&self, _key: &str) -> Result<Value> {
        Err(unavailable())
    }

    pub(super) fn portal_shared_open(&self, _args: &Args<'_>) -> Result<Value> {
        Err(unavailable())
    }

    pub(super) fn portal_notification_read(&self, _args: &Args<'_>) -> Result<Value> {
        Err(unavailable())
    }
}
