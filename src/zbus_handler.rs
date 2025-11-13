use crate::notification::{Action, Notification, Urgency};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zbus::object_server::SignalEmitter;
use zbus::{fdo, interface};

const NOTIFICATION_SPEC_VERSION: &str = "1.2";

pub struct Notifications {
    /// counter for generating unique notification IDs
    next_id: std::sync::Arc<std::sync::Mutex<u32>>,
    /// channel sender to communicate with the main notification event loop
    sender: Sender<Action>,
}

impl Notifications {
    /// creates a new instance of the notification interface
    pub fn new(sender: Sender<Action>) -> Self {
        Self {
            next_id: std::sync::Arc::new(std::sync::Mutex::new(0)),
            sender,
        }
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl Notifications {
    async fn get_server_information(&self) -> fdo::Result<(String, String, String, String)> {
        Ok((
            env!("CARGO_PKG_NAME").to_string(),    // application name
            env!("CARGO_PKG_AUTHORS").to_string(), // author/Vendor
            env!("CARGO_PKG_VERSION").to_string(), // version
            NOTIFICATION_SPEC_VERSION.to_string(), // notification spec version
        ))
    }

    /// returns the server's capabilities
    async fn get_capabilities(&self) -> fdo::Result<Vec<String>> {
        Ok(vec!["body".to_string(), "body-markup".to_string()])
    }

    /// called when an external program sends a notification request
    async fn notify(
        &self,
        app_name: String,  // name of the app sending the notification
        replaces_id: u32,  // iD of notification to replace, if any
        _app_icon: String, // icon field
        summary: String,   // title of the notification
        body: String,      // body text
        _actions: Vec<String>,
        hints: HashMap<String, zbus::zvariant::Value<'_>>, // extra metadata
        expire_timeout: i32,                               // time before it disappear
    ) -> fdo::Result<u32> {
        // generate or reuse a notification ID
        let id = if replaces_id > 0 {
            replaces_id
        } else {
            let mut next_id = self.next_id.lock().unwrap();
            *next_id += 1;
            *next_id
        };

        // parse the urgency
        let urgency = hints
            .get("urgency")
            .and_then(|v| v.try_into().ok())
            .map(|v: u8| Urgency::from(v as u64))
            .unwrap_or_default();

        // convert timeout
        let expire_timeout = if expire_timeout > 0 {
            Some(Duration::from_millis(expire_timeout as u64))
        } else {
            None
        };

        // record the current timestamp for when the notification is received
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // build the notification struct used internally
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

        // send the notification to the main thread for display
        self.sender
            .send(Action::Show(notification))
            .map_err(|e| fdo::Error::Failed(format!("Send failed: {}", e)))?;

        Ok(id)
    }

    /// closes a notification by ID
    async fn close_notification(&self, id: u32) -> fdo::Result<()> {
        self.sender
            .send(Action::Close(Some(id)))
            .map_err(|e| fdo::Error::Failed(format!("Close failed: {}", e)))?;
        Ok(())
    }

    /// signal emitted when a notification is closed
    #[zbus(signal)]
    async fn notification_closed(
        signal_emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    /// signal emitted when a user invokes an action button
    #[zbus(signal)]
    async fn action_invoked(
        signal_emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: String,
    ) -> zbus::Result<()>;
}

pub struct NotificationControl {
    sender: Sender<Action>,
}

impl NotificationControl {
    pub fn new(sender: Sender<Action>) -> Self {
        Self { sender }
    }
}

#[interface(name = "org.freedesktop.NotificationControl")]
impl NotificationControl {
    async fn history(&self) -> fdo::Result<()> {
        self.sender
            .send(Action::ShowLast)
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        Ok(())
    }

    async fn close(&self) -> fdo::Result<()> {
        self.sender
            .send(Action::Close(None))
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        Ok(())
    }

    async fn close_all(&self) -> fdo::Result<()> {
        self.sender
            .send(Action::CloseAll)
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        Ok(())
    }
}
