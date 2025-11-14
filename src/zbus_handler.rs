#![allow(missing_docs, clippy::too_many_arguments)]

use crate::notification::{Action, Notification, Urgency};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zbus::object_server::SignalEmitter;
use zbus::{fdo, interface};

const NOTIFICATION_SPEC_VERSION: &str = "1.2";

/// Notification interface exposed over D-Bus.
pub struct Notifications {
    /// Counter for generating unique notification IDs.
    next_id: std::sync::Arc<std::sync::Mutex<u32>>,
    /// Channel sender to communicate with the main notification event loop.
    sender: Sender<Action>,
}

impl Notifications {
    /// Creates a new instance of the notification interface.
    pub fn new(sender: Sender<Action>) -> Self {
        Self {
            next_id: std::sync::Arc::new(std::sync::Mutex::new(0)),
            sender,
        }
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl Notifications {
    /// Returns basic information about the notification server.
    async fn get_server_information(&self) -> fdo::Result<(String, String, String, String)> {
        Ok((
            env!("CARGO_PKG_NAME").to_string(),    // Application name
            env!("CARGO_PKG_AUTHORS").to_string(), // Author/vendor
            env!("CARGO_PKG_VERSION").to_string(), // Version
            NOTIFICATION_SPEC_VERSION.to_string(), // Notification spec version
        ))
    }

    /// Returns the server's capabilities.
    async fn get_capabilities(&self) -> fdo::Result<Vec<String>> {
        Ok(vec!["body".to_string(), "body-markup".to_string()])
    }

    /// Called when an external program sends a notification request.
    async fn notify(
        &self,
        app_name: String,  // Name of the app sending the notification
        replaces_id: u32,  // ID of notification to replace, if any
        _app_icon: String, // Icon field
        summary: String,   // Title of the notification
        body: String,      // Body text
        _actions: Vec<String>,
        hints: HashMap<String, zbus::zvariant::Value<'_>>, // Extra metadata
        expire_timeout: i32,                               // Time before it disappears
    ) -> fdo::Result<u32> {
        // Generate or reuse a notification ID.
        let id = if replaces_id > 0 {
            replaces_id
        } else {
            let mut next_id = self
                .next_id
                .lock()
                .map_err(|e| fdo::Error::Failed(format!("Lock poisoned: {}", e)))?;
            *next_id += 1;
            *next_id
        };

        // Parse the urgency.
        let urgency = hints
            .get("urgency")
            .and_then(|v| v.try_into().ok())
            .map(|v: u8| Urgency::from(v as u64))
            .unwrap_or_default();

        // Convert timeout.
        let expire_timeout = if expire_timeout > 0 {
            Some(Duration::from_millis(expire_timeout as u64))
        } else {
            None
        };

        // Record the current timestamp for when the notification is received.
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| fdo::Error::Failed(format!("System time error: {}", e)))?
            .as_secs();

        // Build the notification struct used internally.
        let notification = Notification {
            id,
            app_name,
            summary,
            body,
            expire_timeout,
            urgency,
            is_read: false,
            timestamp,
        };

        // Send the notification to the main thread for display.
        self.sender
            .send(Action::Show(notification))
            .map_err(|e| fdo::Error::Failed(format!("Send failed: {}", e)))?;

        Ok(id)
    }

    /// Closes a notification by ID.
    async fn close_notification(&self, id: u32) -> fdo::Result<()> {
        self.sender
            .send(Action::Close(Some(id)))
            .map_err(|e| fdo::Error::Failed(format!("Close failed: {}", e)))?;
        Ok(())
    }

    /// Signal emitted when a notification is closed.
    #[zbus(signal)]
    async fn notification_closed(
        signal_emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    /// Signal emitted when a user invokes an action button.
    #[zbus(signal)]
    async fn action_invoked(
        signal_emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: String,
    ) -> zbus::Result<()>;
}

/// Control interface for managing notifications.
pub struct NotificationControl {
    sender: Sender<Action>,
}

impl NotificationControl {
    /// Creates a new notification control handle.
    pub fn new(sender: Sender<Action>) -> Self {
        Self { sender }
    }
}

#[interface(name = "org.freedesktop.NotificationControl")]
impl NotificationControl {
    /// Shows the most recent notification entry.
    async fn history(&self) -> fdo::Result<()> {
        self.sender
            .send(Action::ShowLast)
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        Ok(())
    }

    /// Closes the most recently shown notification.
    async fn close(&self) -> fdo::Result<()> {
        self.sender
            .send(Action::Close(None))
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        Ok(())
    }

    /// Closes all currently displayed notifications.
    async fn close_all(&self) -> fdo::Result<()> {
        self.sender
            .send(Action::CloseAll)
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        Ok(())
    }
}
