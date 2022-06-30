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

use crate::config::{Config, DEFAULT_CONFIG};
use crate::dbus::{DbusClient, DbusServer, NotificationAction};
use crate::error::Result;
use crate::x11::X11;
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

/// Runs `runst`.
pub fn run() -> Result<()> {
    let config = Arc::new(Config::parse(DEFAULT_CONFIG)?);

    let mut x11 = X11::init(None)?;
    let window = x11.create_window(&config.global)?;
    let dbus_server = DbusServer::init()?;
    let dbus_client = Arc::new(DbusClient::init()?);
    let timeout = Duration::from_millis(1000);

    let x11 = Arc::new(x11);
    let window = Arc::new(window);
    let notifications = Arc::new(RwLock::new(Vec::new()));

    let x11_cloned = Arc::clone(&x11);
    let window_cloned = Arc::clone(&window);
    let dbus_client_cloned = Arc::clone(&dbus_client);
    let config_cloned = Arc::clone(&config);
    let notifications_cloned = Arc::clone(&notifications);
    thread::spawn(move || {
        x11_cloned
            .handle_events(
                window_cloned,
                notifications_cloned,
                config_cloned,
                |notification| {
                    dbus_client_cloned
                        .close_notification(notification.replaces_id, timeout)
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
            NotificationAction::Show(notification) => {
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
                            .close_notification(notification.replaces_id, timeout)
                            .expect("failed to close notification");
                    });
                }
                notifications
                    .write()
                    .expect("failed to retrieve notifications")
                    .push(notification);
                x11_cloned.show_window(&window)?;
            }
            NotificationAction::Close => {
                let notifications = notifications
                    .read()
                    .expect("failed to retrieve notifications");
                x11_cloned.hide_window(&window)?;
                if notifications.iter().filter(|v| !v.is_read).count() >= 1 {
                    x11_cloned.show_window(&window)?;
                }
            }
        }
    }
}
