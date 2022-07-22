use crate::error;
use dbus::arg::RefArg;
use dbus::blocking::{Connection, Proxy};
use dbus::channel::MatchingReceiver;
use dbus::message::MatchRule;
use dbus_crossroads::Crossroads;
use serde::Serialize;
use std::sync::mpsc::Sender;
use std::time::Duration;

mod dbus_server {
    #![allow(clippy::too_many_arguments)]
    include!(concat!(env!("OUT_DIR"), "/introspection.rs"));
}

/// D-Bus interface for desktop notifications.
const NOTIFICATION_INTERFACE: &str = "org.freedesktop.Notifications";

/// D-Bus path for desktop notifications.
const NOTIFICATION_PATH: &str = "/org/freedesktop/Notifications";

/// Possible urgency levels for the notification.
#[derive(Clone, Debug, Serialize)]
pub enum NotificationUrgency {
    /// Low urgency.
    Low,
    /// Normal urgency (default).
    Normal,
    /// Critical urgency.
    Critical,
}

impl From<u64> for NotificationUrgency {
    fn from(value: u64) -> Self {
        match value {
            0 => Self::Low,
            1 => Self::Normal,
            2 => Self::Critical,
            _ => Self::default(),
        }
    }
}

impl Default for NotificationUrgency {
    fn default() -> Self {
        Self::Normal
    }
}

/// Representation of a notification.
///
/// See [D-Bus Notify Parameters](https://specifications.freedesktop.org/notification-spec/latest/ar01s09.html)
#[derive(Clone, Debug, Default, Serialize)]
pub struct Notification {
    /// Name of the application that sends the notification.
    pub app_name: String,
    /// The optional notification ID.
    #[serde(skip_serializing)]
    pub replaces_id: u32,
    /// Summary text.
    pub summary: String,
    /// Body.
    pub body: String,
    /// The timeout time in milliseconds.
    #[serde(skip_serializing)]
    pub expire_timeout: Option<Duration>,
    /// Urgency.
    pub urgency: NotificationUrgency,
    /// Whether if the notification is read.
    #[serde(skip_serializing)]
    pub is_read: bool,
}

/// Possible actions for a notification.
#[derive(Debug)]
pub enum NotificationAction {
    /// Show a notification.
    Show(Notification),
    /// Close a notification.
    Close,
}

/// D-Bus notification implementation.
///
/// <https://specifications.freedesktop.org/notification-spec/latest/ar01s09.html>
pub struct DbusNotification {
    sender: Sender<NotificationAction>,
}

impl dbus_server::OrgFreedesktopNotifications for DbusNotification {
    fn get_capabilities(&mut self) -> Result<Vec<String>, dbus::MethodErr> {
        Ok(vec![String::from("actions"), String::from("body")])
    }

    fn notify(
        &mut self,
        app_name: String,
        replaces_id: u32,
        _app_icon: String,
        summary: String,
        body: String,
        _actions: Vec<String>,
        hints: dbus::arg::PropMap,
        expire_timeout: i32,
    ) -> Result<u32, dbus::MethodErr> {
        match self.sender.send(NotificationAction::Show(Notification {
            app_name,
            replaces_id,
            summary,
            body,
            expire_timeout: if expire_timeout != -1 {
                match expire_timeout.try_into() {
                    Ok(v) => Some(Duration::from_millis(v)),
                    Err(_) => None,
                }
            } else {
                None
            },
            urgency: hints
                .get("urgency")
                .and_then(|v| v.as_u64())
                .map(|v| v.into())
                .unwrap_or_default(),
            is_read: false,
        })) {
            Ok(_) => Ok(replaces_id),
            Err(e) => Err(dbus::MethodErr::failed(&e)),
        }
    }

    fn close_notification(&mut self, _id: u32) -> Result<(), dbus::MethodErr> {
        match self.sender.send(NotificationAction::Close) {
            Ok(_) => Ok(()),
            Err(e) => Err(dbus::MethodErr::failed(&e)),
        }
    }

    fn get_server_information(
        &mut self,
    ) -> Result<(String, String, String, String), dbus::MethodErr> {
        Ok((
            env!("CARGO_PKG_NAME").to_string(),
            env!("CARGO_PKG_AUTHORS").to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            "1.2".to_string(),
        ))
    }
}

/// Wrapper for a [`D-Bus connection`] and [`server`] handler.
///
/// [`D-Bus connection`]: Connection
/// [`server`]: Crossroads
pub struct DbusServer {
    /// Connection to D-Bus.
    connection: Connection,
    /// Server handler.
    crossroads: Crossroads,
}

impl DbusServer {
    /// Initializes the D-Bus controller.
    pub fn init() -> error::Result<Self> {
        let connection = Connection::new_session()?;
        let crossroads = Crossroads::new();
        Ok(Self {
            connection,
            crossroads,
        })
    }

    /// Registers a handler for handling D-Bus notifications.
    ///
    /// Handles the incoming messages in a blocking manner.
    pub fn register_notification_handler(
        mut self,
        sender: Sender<NotificationAction>,
        timeout: Duration,
    ) -> error::Result<()> {
        self.connection
            .request_name(NOTIFICATION_INTERFACE, false, true, false)?;
        let token = dbus_server::register_org_freedesktop_notifications(&mut self.crossroads);
        self.crossroads
            .insert(NOTIFICATION_PATH, &[token], DbusNotification { sender });
        self.connection.start_receive(
            MatchRule::new_method_call(),
            Box::new(move |message, connection| {
                self.crossroads
                    .handle_message(message, connection)
                    .expect("failed to handle message");
                true
            }),
        );
        loop {
            self.connection.process(timeout)?;
        }
    }
}

/// Wrapper for a [`D-Bus connection`] without the server part.
///
/// [`D-Bus connection`]: Connection
pub struct DbusClient {
    /// Connection to D-Bus.
    connection: Connection,
}

unsafe impl Send for DbusClient {}
unsafe impl Sync for DbusClient {}

impl DbusClient {
    /// Initializes the D-Bus controller.
    pub fn init() -> error::Result<Self> {
        let connection = Connection::new_session()?;
        Ok(Self { connection })
    }

    /// Closes the notification.
    ///
    /// See `org.freedesktop.Notifications.CloseNotification`
    pub fn close_notification(&self, id: u32, timeout: Duration) -> error::Result<()> {
        let proxy = Proxy::new(
            NOTIFICATION_INTERFACE,
            NOTIFICATION_PATH,
            timeout,
            &self.connection,
        );
        proxy.method_call(NOTIFICATION_INTERFACE, "CloseNotification", (id,))?;
        Ok(())
    }
}
