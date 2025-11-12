use zbus::{interface, fdo};
use zbus::object_server::SignalEmitter;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use crate::notification::{Action, Notification, Urgency};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct Notifications {
    next_id: std::sync::Arc<std::sync::Mutex<u32>>,
    sender: Sender<Action>,
}

impl Notifications {
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
            "runst".to_string(),
            "Orhun Parmaksız".to_string(),
            "0.1.7".to_string(),
            "1.2".to_string(),
        ))
    }

    async fn get_capabilities(&self) -> fdo::Result<Vec<String>> {
        Ok(vec!["body".to_string(), "body-markup".to_string()])
    }

    async fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        _app_icon: String,
        summary: String,
        body: String,
        _actions: Vec<String>,
        hints: HashMap<String, zbus::zvariant::Value<'_>>,
        expire_timeout: i32,
    ) -> fdo::Result<u32> {
        let id = if replaces_id > 0 {
            replaces_id
        } else {
            let mut next_id = self.next_id.lock().unwrap();
            *next_id += 1;
            *next_id
        };

        // Fix: Use try_into() instead of downcast_ref()
        let urgency = hints.get("urgency")
            .and_then(|v| v.try_into().ok())
            .map(|v: u8| Urgency::from(v as u64))
            .unwrap_or_default();

        let expire_timeout = if expire_timeout > 0 {
            Some(Duration::from_millis(expire_timeout as u64))
        } else {
            None
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

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

        self.sender
            .send(Action::Show(notification))
            .map_err(|e| fdo::Error::Failed(format!("Send failed: {}", e)))?;

        Ok(id)
    }

    async fn close_notification(&self, id: u32) -> fdo::Result<()> {
        self.sender
            .send(Action::Close(Some(id)))
            .map_err(|e| fdo::Error::Failed(format!("Close failed: {}", e)))?;
        Ok(())
    }

    #[zbus(signal)]
    async fn notification_closed(
        signal_emitter: &SignalEmitter<'_>,  // Should be signal_emitter, not signal_ctxt
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn action_invoked(
        signal_emitter: &SignalEmitter<'_>,  // Should be signal_emitter, not signal_ctxt
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