use crate::error;
use crate::notification::Notification;
use dbus::blocking::Connection;
use dbus::channel::MatchingReceiver;
use dbus::message::MatchRule;
use dbus_crossroads::Crossroads;
use std::time::Duration;

mod dbus_server {
    #![allow(clippy::too_many_arguments)]
    include!(concat!(env!("OUT_DIR"), "/introspection.rs"));
}

/// D-Bus interface for desktop notifications.
pub const NOTIFICATION_INTERFACE: &str = "org.freedesktop.Notifications";

/// D-Bus path for desktop notifications.
const NOTIFICATION_PATH: &str = "/org/freedesktop/Notifications";

/// D-Bus notification implementation.
///
/// <https://specifications.freedesktop.org/notification-spec/latest/ar01s09.html>
pub struct DbusNotification {}

impl dbus_server::OrgFreedesktopNotifications for DbusNotification {
    fn get_capabilities(&mut self) -> Result<Vec<String>, dbus::MethodErr> {
        Ok(vec![String::from("actions"), String::from("body")])
    }

    fn notify(
        &mut self,
        _app_name: String,
        replaces_id: u32,
        _app_icon: String,
        _summary: String,
        _body: String,
        _actions: Vec<String>,
        _hints: dbus::arg::PropMap,
        _expire_timeout: i32,
    ) -> Result<u32, dbus::MethodErr> {
        Ok(replaces_id)
    }

    fn close_notification(&mut self, _id: u32) -> Result<(), dbus::MethodErr> {
        Ok(())
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
    pub fn register_notification_handler<F>(
        mut self,
        handler: F,
        timeout: Duration,
    ) -> error::Result<()>
    where
        F: Fn(Notification) -> error::Result<()> + Send + Sync + 'static,
    {
        self.connection
            .request_name(NOTIFICATION_INTERFACE, false, true, false)?;
        let token = dbus_server::register_org_freedesktop_notifications(&mut self.crossroads);
        self.crossroads
            .insert(NOTIFICATION_PATH, &[token], DbusNotification {});
        self.connection.start_receive(
            MatchRule::new_method_call(),
            Box::new(move |message, connection| {
                println!("{:?}", message);

                let notification = Notification::try_from(&message);
                self.crossroads
                    .handle_message(message, connection)
                    .expect("failed to handle message");

                match notification {
                    Ok(notification) => {
                        handler(notification).expect("failed to handle notification")
                    }
                    Err(e) => eprintln!("{}", e),
                }
                true
            }),
        );
        loop {
            self.connection.process(timeout)?;
        }
    }
}
