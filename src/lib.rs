//! A dead simple notification daemon.

#![warn(missing_docs, clippy::unwrap_used)]

/// Error handler.
pub mod error;

/// D-Bus handler.
pub mod dbus;

/// Notification.
pub mod notification;

use crate::dbus::Dbus;
use crate::error::Result;
use std::time::Duration;

/// Runs `runst`.
pub fn run() -> Result<()> {
    let dbus = Dbus::init()?;
    dbus.register_notification_handler(|notification| {
        println!("{:?}", notification);
    })?;
    dbus.listen(Duration::from_millis(1000))?;
    Ok(())
}
