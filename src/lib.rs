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
use estimated_read_time::Options;
use notification::Manager;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

/// Runs `runst`.
pub fn run() -> Result<()> {
    let config = Arc::new(Config::parse()?);

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(config.global.log_verbosity.into())
                .from_env_lossy(),
        )
        .init();
    tracing::trace!("{:#?}", config);
    tracing::info!("starting");

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
        if let Err(e) = x11_cloned.handle_events(
            window_cloned,
            notifications_cloned,
            config_cloned,
            |notification| {
                tracing::debug!("user input detected");
                dbus_client_cloned
                    .close_notification(notification.id, timeout)
                    .expect("failed to close notification");
            },
        ) {
            eprintln!("Failed to handle X11 events: {}", e)
        }
    });

    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        tracing::debug!("registering D-Bus handler");
        dbus_server
            .register_notification_handler(sender, timeout)
            .expect("failed to register D-Bus notification handler");
    });

    if config.global.startup_notification {
        dbus_client.notify(
            env!("CARGO_PKG_NAME"),
            "startup",
            &format!("{} is up and running", env!("CARGO_PKG_NAME")),
            -1,
        )?;
    }

    let x11_cloned = Arc::clone(&x11);
    loop {
        match receiver.recv()? {
            Action::Show(notification) => {
                tracing::debug!("received notification: {}", notification.id);
                let timeout = notification.expire_timeout.unwrap_or_else(|| {
                    let urgency_config = config.get_urgency_config(&notification.urgency);
                    Duration::from_secs(if urgency_config.auto_clear.unwrap_or(false) {
                        notification
                            .render_message(&window.template, &urgency_config.text, 0)
                            .map(|v| estimated_read_time::text(&v, &Options::default()).seconds())
                            .unwrap_or_default()
                    } else {
                        urgency_config.timeout.into()
                    })
                });
                if !timeout.is_zero() {
                    tracing::debug!("notification timeout: {}ms", timeout.as_millis());
                    let dbus_client_cloned = Arc::clone(&dbus_client);
                    let notifications_cloned = notifications.clone();
                    thread::spawn(move || {
                        thread::sleep(timeout);
                        if notifications_cloned.is_unread(notification.id) {
                            dbus_client_cloned
                                .close_notification(notification.id, timeout)
                                .expect("failed to close notification");
                        }
                    });
                }
                notifications.add(notification);
                x11_cloned.hide_window(&window)?;
                x11_cloned.show_window(&window)?;
            }
            Action::ShowLast => {
                tracing::debug!("showing the last notification");
                if notifications.count() == 0 {
                    continue;
                } else if notifications.mark_next_as_unread() {
                    x11_cloned.hide_window(&window)?;
                    x11_cloned.show_window(&window)?;
                } else {
                    x11_cloned.hide_window(&window)?;
                }
            }
            Action::Close(id) => {
                if let Some(id) = id {
                    tracing::debug!("closing notification: {}", id);
                    notifications.mark_as_read(id);
                } else {
                    tracing::debug!("closing the last notification");
                    notifications.mark_last_as_read();
                }
                x11_cloned.hide_window(&window)?;
                if notifications.get_unread_count() >= 1 {
                    x11_cloned.show_window(&window)?;
                }
            }
            Action::CloseAll => {
                tracing::debug!("closing all notifications");
                notifications.mark_all_as_read();
                x11_cloned.hide_window(&window)?;
            }
        }
    }
}
