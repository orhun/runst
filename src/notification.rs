use crate::error::{Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tera::{Context as TeraContext, Tera};

/// Name of the template for rendering the notification message.
pub const NOTIFICATION_MESSAGE_TEMPLATE: &str = "notification_message_template";

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
    /// Timestamp that the notification is created.
    pub timestamp: u64,
}

impl Notification {
    /// Converts [`Notification`] into [`TeraContext`].
    pub fn into_context<'a>(
        &'a self,
        urgency_text: &'a str,
        unread_count: usize,
    ) -> Result<TeraContext> {
        Ok(TeraContext::from_serialize(Context {
            app_name: &self.app_name,
            summary: &self.summary,
            body: &self.body,
            urgency_text,
            unread_count,
            timestamp: self.timestamp,
        })?)
    }

    /// Renders the notification message using the given template.
    pub fn render_message<'a>(
        &self,
        template: &'a Tera,
        urgency_text: &'a str,
        unread_count: usize,
    ) -> Result<String> {
        match template.render(
            NOTIFICATION_MESSAGE_TEMPLATE,
            &self.into_context(urgency_text, unread_count)?,
        ) {
            Ok(v) => Ok::<String, Error>(v),
            Err(e) => {
                if let Some(error_source) = e.source() {
                    Err(Error::TemplateRender(error_source.to_string()))
                } else {
                    Err(Error::Template(e))
                }
            }
        }
    }

    /// Returns true if the given filter matches the notification message.
    pub fn matches_filter(&self, filter: &NotificationFilter) -> bool {
        macro_rules! check_filter {
            ($field: ident) => {
                if let Some($field) = &filter.$field {
                    if !$field.is_match(&self.$field) {
                        return false;
                    }
                }
            };
        }
        check_filter!(app_name);
        check_filter!(summary);
        check_filter!(body);
        true
    }
}

/// Notification message filter.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NotificationFilter {
    /// Name of the application.
    #[serde(with = "serde_regex", default)]
    pub app_name: Option<Regex>,
    /// Summary text.
    #[serde(with = "serde_regex", default)]
    pub summary: Option<Regex>,
    /// Body.
    #[serde(with = "serde_regex", default)]
    pub body: Option<Regex>,
}

/// Template context for the notification.
#[derive(Clone, Debug, Default, Serialize)]
struct Context<'a> {
    /// Name of the application that sends the notification.
    pub app_name: &'a str,
    /// Summary text.
    pub summary: &'a str,
    /// Body.
    pub body: &'a str,
    /// Urgency.
    #[serde(rename = "urgency")]
    pub urgency_text: &'a str,
    /// Count of unread notifications.
    pub unread_count: usize,
    /// Timestamp of the notification.
    pub timestamp: u64,
}

/// Possible actions for a notification.
#[derive(Debug)]
pub enum Action {
    /// Show a notification.
    Show(Notification),
    /// Show the last notification.
    ShowLast,
    /// Close a notification.
    Close(Option<u32>),
    /// Close all the notifications.
    CloseAll,
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

    /// Returns the number of notifications.
    pub fn count(&self) -> usize {
        self.inner
            .read()
            .expect("failed to retrieve notifications")
            .len()
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

    /// Marks the next notification as unread starting from the first one.
    ///
    /// Returns true if there is an unread notification remaining.
    pub fn mark_next_as_unread(&self) -> bool {
        let mut notifications = self
            .inner
            .write()
            .expect("failed to retrieve notifications");
        let last_unread_index = notifications.iter_mut().position(|v| !v.is_read);
        if last_unread_index.is_none() {
            let len = notifications.len();
            notifications[len - 1].is_read = false;
        }
        if let Some(index) = last_unread_index {
            notifications[index].is_read = true;
            if index > 0 {
                notifications[index - 1].is_read = false;
            } else {
                return false;
            }
        }
        true
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

    /// Marks all the notifications as read.
    pub fn mark_all_as_read(&self) {
        let mut notifications = self
            .inner
            .write()
            .expect("failed to retrieve notifications");
        notifications.iter_mut().for_each(|v| v.is_read = true);
    }

    /// Returns the number of unread notifications.
    pub fn get_unread_count(&self) -> usize {
        let notifications = self.inner.read().expect("failed to retrieve notifications");
        notifications.iter().filter(|v| !v.is_read).count()
    }

    /// Returns true if the notification is unread.
    pub fn is_unread(&self, id: u32) -> bool {
        let notifications = self.inner.read().expect("failed to retrieve notifications");
        notifications
            .iter()
            .find(|notification| notification.id == id)
            .map(|v| !v.is_read)
            .unwrap_or_default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_notification_filter() {
        let notification = Notification {
            app_name: String::from("app"),
            summary: String::from("test"),
            body: String::from("this is a test notification"),
            ..Default::default()
        };
        assert!(notification.matches_filter(&NotificationFilter {
            app_name: Regex::new("app").ok(),
            summary: None,
            body: None,
        }));
        assert!(notification.matches_filter(&NotificationFilter {
            app_name: None,
            summary: Regex::new("te*").ok(),
            body: None,
        }));
        assert!(notification.matches_filter(&NotificationFilter {
            app_name: None,
            summary: None,
            body: Regex::new("notification").ok(),
        }));
        assert!(notification.matches_filter(&NotificationFilter {
            app_name: Regex::new("app").ok(),
            summary: Regex::new("test").ok(),
            body: Regex::new("notification").ok(),
        }));
        assert!(notification.matches_filter(&NotificationFilter {
            app_name: None,
            summary: None,
            body: None,
        }));
        assert!(!notification.matches_filter(&NotificationFilter {
            app_name: Regex::new("xxx").ok(),
            summary: None,
            body: Regex::new("yyy").ok(),
        }));
        assert!(!notification.matches_filter(&NotificationFilter {
            app_name: Regex::new("xxx").ok(),
            summary: Regex::new("aaa").ok(),
            body: Regex::new("yyy").ok(),
        }));
        assert!(!notification.matches_filter(&NotificationFilter {
            app_name: Regex::new("app").ok(),
            summary: Regex::new("invalid").ok(),
            body: Regex::new("regex").ok(),
        }));
    }
}
