//! A dead simple notification daemon.

#![warn(missing_docs, clippy::unwrap_used)]

/// Error handler.
pub mod error;

use crate::error::Result;

/// Runs `runst`.
pub fn run() -> Result<()> {
    Ok(())
}
