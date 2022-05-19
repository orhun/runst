use crate::error;
use dbus::blocking::Connection;
use dbus::channel::MatchingReceiver;
use dbus::message::MatchRule;
use dbus_crossroads::Crossroads;
use std::fmt;
use std::sync::mpsc::Sender;
use std::time::Duration;

mod dbus_server {
    #![allow(clippy::too_many_arguments)]
    include!(concat!(env!("OUT_DIR"), "/introspection.rs"));
}

/// D-Bus interface for desktop notifications.
pub const NOTIFICATION_INTERFACE: &str = "org.freedesktop.Notifications";

/// D-Bus path for desktop notifications.
const NOTIFICATION_PATH: &str = "/org/freedesktop/Notifications";

/// Representation of a notification.
///
/// See [D-Bus Notify Parameters](https://specifications.freedesktop.org/notification-spec/latest/ar01s09.html)
#[derive(Debug, Default)]
pub struct Notification {
    /// Name of the application that sends the notification.
    pub app_name: String,
    /// Summary text.
    pub summary: String,
}

impl fmt::Display for Notification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.app_name, self.summary)
    }
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
        _body: String,
        _actions: Vec<String>,
        _hints: dbus::arg::PropMap,
        _expire_timeout: i32,
    ) -> Result<u32, dbus::MethodErr> {
        match self
            .sender
            .send(NotificationAction::Show(Notification { app_name, summary }))
        {
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
pub struct Dbus {
    /// Connection to D-Bus.
    connection: Connection,
    /// Server handler.
    crossroads: Crossroads,
}

impl Dbus {
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
