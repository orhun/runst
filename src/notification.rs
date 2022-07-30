use serde::Serialize;
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
    /// Name of the application that sends the notification.
    pub app_name: String,
    /// The optional notification ID.
    pub replaces_id: u32,
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
    Close,
}
