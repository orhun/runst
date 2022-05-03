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
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

/// Runs `runst`.
pub fn run() -> Result<()> {
    let mut x11 = X11::init(None)?;
    let window = x11.create_window()?;
    let dbus = Dbus::init()?;

    let x11 = Arc::new(x11);
    let window = Arc::new(RwLock::new(window));

    let x11_cloned = Arc::clone(&x11);
    let window_cloned = Arc::clone(&window);
    thread::spawn(move || {
        x11_cloned
            .handle_events(window_cloned)
            .expect("failed to handle X11 events");
    });

    let x11_cloned = Arc::clone(&x11);
    let window_cloned = Arc::clone(&window);
    dbus.register_notification_handler(move |notification| {
        println!("{:?}", notification);
        let mut window = window_cloned.write().expect("failed to retrieve window");
        window.content = Some(notification);
        x11_cloned.show_window(&window)?;
        Ok(())
    })?;

    dbus.listen(Duration::from_millis(1000))?;
    Ok(())
}
