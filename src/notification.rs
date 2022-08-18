use serde::Serialize;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Possible urgency levels for the notification.
#[derive(Clone, Debug, Serialize)]
pub enum Urgency {
    /// Low urgency.
    Low,
    /// Normal urgency (default).
    Normal,
    /// Critical urgency.
    Critical,
}

impl From<u64> for Urgency {
    fn from(value: u64) -> Self {
        match value {
            0 => Self::Low,
            1 => Self::Normal,
            2 => Self::Critical,
            _ => Self::default(),
        }
    }
}

impl Default for Urgency {
    fn default() -> Self {
        Self::Normal
    }
}

/// Representation of a notification.
///
/// See [D-Bus Notify Parameters](https://specifications.freedesktop.org/notification-spec/latest/ar01s09.html)
#[derive(Clone, Debug, Default)]
pub struct Notification {
    /// The optional notification ID.
    pub id: u32,
    /// Name of the application that sends the notification.
    pub app_name: String,
    /// Summary text.
    pub summary: String,
    /// Body.
    pub body: String,
    /// The timeout time in milliseconds.
    pub expire_timeout: Option<Duration>,
    /// Urgency.
    pub urgency: Urgency,
    /// Whether if the notification is read.
    pub is_read: bool,
}

impl Notification {
    /// Converts [`Notification`] into [`Context`].
    pub fn into_context<'a>(&'a self, urgency_text: &'a str) -> Context {
        Context {
            app_name: &self.app_name,
            summary: &self.summary,
            body: &self.body,
            urgency: urgency_text,
        }
    }
}

/// Template context for the notification.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Context<'a> {
    /// Name of the application that sends the notification.
    pub app_name: &'a str,
    /// Summary text.
    pub summary: &'a str,
    /// Body.
    pub body: &'a str,
    /// Urgency.
    pub urgency: &'a str,
}

/// Possible actions for a notification.
#[derive(Debug)]
pub enum Action {
    /// Show a notification.
    Show(Notification),
    /// Close a notification.
    Close(u32),
}

/// Notification manager.
#[derive(Debug)]
pub struct Manager {
    /// Inner type that holds the notifications in thread-safe way.
    inner: Arc<RwLock<Vec<Notification>>>,
}

impl Clone for Manager {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Manager {
    /// Initializes the notification manager.
    pub fn init() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Adds a new notifications to manage.
    pub fn add(&self, notification: Notification) {
        self.inner
            .write()
            .expect("failed to retrieve notifications")
            .push(notification);
    }

    /// Returns the last unread notification.
    pub fn get_last_unread(&self) -> Notification {
        let notifications = self.inner.read().expect("failed to retrieve notifications");
        let notifications = notifications
            .iter()
            .filter(|v| !v.is_read)
            .collect::<Vec<&Notification>>();
        notifications[notifications.len() - 1].clone()
    }

    /// Marks the last notification as read.
    pub fn mark_last_as_read(&self) {
        let mut notifications = self
            .inner
            .write()
            .expect("failed to retrieve notifications");
        if let Some(notification) = notifications.iter_mut().filter(|v| !v.is_read).last() {
            notification.is_read = true;
        }
    }

    /// Marks the given notification as read.
    pub fn mark_as_read(&self, id: u32) {
        let mut notifications = self
            .inner
            .write()
            .expect("failed to retrieve notifications");
        if let Some(notification) = notifications
            .iter_mut()
            .find(|notification| notification.id == id)
        {
            notification.is_read = true;
        }
    }

    /// Returns the number of unread notifications.
    pub fn get_unread_len(&self) -> usize {
        let notifications = self.inner.read().expect("failed to retrieve notifications");
        notifications.iter().filter(|v| !v.is_read).count()
    }
}
