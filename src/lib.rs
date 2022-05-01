//! A dead simple notification daemon.

#![warn(missing_docs, clippy::unwrap_used)]

/// Error handler.
pub mod error;

/// D-Bus handler.
pub mod dbus;

/// X11 handler.
pub mod x11;

/// Notification.
pub mod notification;

use crate::dbus::Dbus;
use crate::error::Result;
use crate::x11::X11;
use std::time::Duration;

/// Runs `runst`.
pub fn run() -> Result<()> {
    let mut x11 = X11::init(None)?;
    x11.create_window()?;
    x11.show_window()?;
    x11.handle_events()?;

    let dbus = Dbus::init()?;
    dbus.register_notification_handler(|notification| {
        println!("{:?}", notification);
    })?;
    dbus.listen(Duration::from_millis(1000))?;
    Ok(())
}
