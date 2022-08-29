//! A dead simple notification daemon.

#![warn(missing_docs, clippy::unwrap_used)]

/// Error handler.
pub mod error;

/// D-Bus handler.
pub mod dbus;

/// X11 handler.
pub mod x11;

/// Configuration.
pub mod config;

/// Notification manager.
pub mod notification;

use crate::config::Config;
use crate::dbus::{DbusClient, DbusServer};
use crate::error::Result;
use crate::notification::Action;
use crate::x11::X11;
use notification::Manager;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Runs `runst`.
pub fn run() -> Result<()> {
    let config = Arc::new(Config::parse()?);

    let mut x11 = X11::init(None)?;
    let window = x11.create_window(&config.global)?;
    let dbus_server = DbusServer::init()?;
    let dbus_client = Arc::new(DbusClient::init()?);
    let timeout = Duration::from_millis(1000);

    let x11 = Arc::new(x11);
    let window = Arc::new(window);
    let notifications = Manager::init();

    let x11_cloned = Arc::clone(&x11);
    let window_cloned = Arc::clone(&window);
    let dbus_client_cloned = Arc::clone(&dbus_client);
    let config_cloned = Arc::clone(&config);
    let notifications_cloned = notifications.clone();
    thread::spawn(move || {
        x11_cloned
            .handle_events(
                window_cloned,
                notifications_cloned,
                config_cloned,
                |notification| {
                    dbus_client_cloned
                        .close_notification(notification.id, timeout)
                        .expect("failed to close notification");
                },
            )
            .expect("failed to handle X11 events");
    });

    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        dbus_server
            .register_notification_handler(sender, timeout)
            .expect("failed to register D-Bus notification handler");
    });

    let x11_cloned = Arc::clone(&x11);
    loop {
        match receiver.recv()? {
            Action::Show(notification) => {
                let timeout = notification.expire_timeout.unwrap_or_else(|| {
                    Duration::from_secs(
                        config
                            .get_urgency_config(&notification.urgency)
                            .timeout
                            .into(),
                    )
                });
                if !timeout.is_zero() {
                    let dbus_client_cloned = Arc::clone(&dbus_client);
                    thread::spawn(move || {
                        thread::sleep(timeout);
                        dbus_client_cloned
                            .close_notification(notification.id, timeout)
                            .expect("failed to close notification");
                    });
                }
                notifications.add(notification);
                x11_cloned.hide_window(&window)?;
                x11_cloned.show_window(&window)?;
            }
            Action::Close(id) => {
                notifications.mark_as_read(id);
                x11_cloned.hide_window(&window)?;
                if notifications.get_unread_len() >= 1 {
                    x11_cloned.show_window(&window)?;
                }
            }
        }
    }
}
