//! A dead simple notification daemon.

#![warn(missing_docs, clippy::unwrap_used)]

/// Error handler.
pub mod error;

/// zbus D-Bus handler.
pub mod zbus_notify;

/// X11 handler.
pub mod x11;

/// Configuration.
pub mod config;

/// Notification manager.
pub mod notification;

use crate::config::Config;
use crate::error::Result;
use crate::notification::Action;
use crate::x11::X11;
use estimated_read_time::Options;
use notification::{Manager, Notification, Urgency};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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
    tracing::info!("starting runst with zbus");

    let mut x11 = X11::init(None)?;
    let window = x11.create_window(&config.global)?;

    let x11 = Arc::new(x11);
    let window = Arc::new(window);
    let notifications = Manager::init();

    let (sender, receiver) = mpsc::channel();

    // Spawn X11 event handler thread
    let x11_cloned = Arc::clone(&x11);
    let window_cloned = Arc::clone(&window);
    let config_cloned = Arc::clone(&config);
    let notifications_cloned = notifications.clone();
    let sender_cloned = sender.clone();

    thread::spawn(move || {
        if let Err(e) = x11_cloned.handle_events(
            window_cloned,
            notifications_cloned,
            config_cloned,
            move |notification| {
                tracing::debug!("user input detected");
                sender_cloned
                    .send(Action::Close(Some(notification.id)))
                    .expect("failed to send close action");
            },
        ) {
            eprintln!("Failed to handle X11 events: {e}")
        }
    });

    // Spawn zbus D-Bus server thread
    let sender_for_dbus = sender.clone();
    thread::spawn(move || {
        tracing::debug!("starting zbus D-Bus server thread");
        
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            let notifications = zbus_notify::Notifications::new(sender_for_dbus.clone());
            let control = zbus_notify::NotificationControl::new(sender_for_dbus);

            match zbus::connection::Builder::session() {
                Ok(mut builder) => {
                    // Request the well-known name
                    builder = match builder.name("org.freedesktop.Notifications") {
                        Ok(b) => b,
                        Err(e) => {
                            eprintln!("Failed to request name: {}", e);
                            return;
                        }
                    };
                    
                    // Build the connection
                    match builder.build().await {
                        Ok(connection) => {
                            // Serve the notifications interface
                            if let Err(e) = connection
                                .object_server()
                                .at("/org/freedesktop/Notifications", notifications)
                                .await
                            {
                                eprintln!("Failed to serve notifications interface: {}", e);
                                return;
                            }
                            
                            // Serve the control interface
                            if let Err(e) = connection
                                .object_server()
                                .at("/org/freedesktop/Notifications/ctl", control)
                                .await
                            {
                                eprintln!("Failed to serve control interface: {}", e);
                                return;
                            }
                            
                            tracing::info!("zbus D-Bus server is running");
                            // Keep the connection alive
                            std::future::pending::<()>().await;
                        }
                        Err(e) => {
                            eprintln!("Failed to build zbus connection: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create session builder: {}", e);
                }
            }
        });
    });

    // Small delay to let D-Bus server start
    thread::sleep(Duration::from_millis(100));

    if config.global.startup_notification {
        let startup_notification = Notification {
            id: 0,
            app_name: env!("CARGO_PKG_NAME").to_string(),
            summary: "startup".to_string(),
            body: concat!(env!("CARGO_PKG_NAME"), " is up and running 🦡").to_string(),
            expire_timeout: Some(Duration::from_secs(3)),
            urgency: Urgency::Normal,
            is_read: false,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        sender.send(Action::Show(startup_notification))?;
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
                            .render_message(&window.template, urgency_config.text, 0)
                            .map(|v| estimated_read_time::text(&v, &Options::default()).seconds())
                            .unwrap_or_default()
                    } else {
                        urgency_config.timeout.into()
                    })
                });
                if !timeout.is_zero() {
                    tracing::debug!("notification timeout: {}ms", timeout.as_millis());
                    let sender_cloned = sender.clone();
                    let notifications_cloned = notifications.clone();
                    let notification_id = notification.id;
                    thread::spawn(move || {
                        thread::sleep(timeout);
                        if notifications_cloned.is_unread(notification_id) {
                            sender_cloned
                                .send(Action::Close(Some(notification_id)))
                                .expect("failed to send close action");
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