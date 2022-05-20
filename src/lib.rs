//! A dead simple notification daemon.

#![warn(missing_docs, clippy::unwrap_used)]

/// Error handler.
pub mod error;

/// D-Bus handler.
pub mod dbus;

/// X11 handler.
pub mod x11;

use crate::dbus::{DbusClient, DbusServer, NotificationAction};
use crate::error::Result;
use crate::x11::X11;
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

/// Runs `runst`.
pub fn run() -> Result<()> {
    let mut x11 = X11::init(None)?;
    let window = x11.create_window()?;
    let dbus_server = DbusServer::init()?;
    let dbus_client = DbusClient::init()?;

    let x11 = Arc::new(x11);
    let window = Arc::new(RwLock::new(window));

    let x11_cloned = Arc::clone(&x11);
    let window_cloned = Arc::clone(&window);
    thread::spawn(move || {
        // TODO: call CloseNotification
        x11_cloned
            .handle_events(window_cloned)
            .expect("failed to handle X11 events");
    });

    let timeout = Duration::from_millis(1000);
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        dbus_server
            .register_notification_handler(sender, timeout)
            .expect("failed to register D-Bus notification handler");
    });

    let x11_cloned = Arc::clone(&x11);
    let window_cloned = Arc::clone(&window);
    let dbus_client = Arc::new(dbus_client);

    loop {
        match receiver.recv()? {
            NotificationAction::Show(notification) => {
                if let Some(expire_timeout) = notification.expire_timeout {
                    let dbus_client_cloned = Arc::clone(&dbus_client);
                    thread::spawn(move || {
                        thread::sleep(expire_timeout);
                        dbus_client_cloned
                            .close_notification(notification.replaces_id, timeout)
                            .expect("fiailed to close notification");
                    });
                }
                let mut window = window_cloned.write().expect("failed to retrieve window");
                window.content = Some(notification);
                x11_cloned.show_window(&window)?;
            }
            NotificationAction::Close => {
                let window = window_cloned.write().expect("failed to retrieve window");
                x11_cloned.hide_window(&window)?;
            }
        }
    }
}
