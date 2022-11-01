use crate::error;
use crate::notification::{Action, Notification};
use dbus::arg::{RefArg, Variant};
use dbus::blocking::{Connection, Proxy};
use dbus::channel::MatchingReceiver;
use dbus::message::MatchRule;
use dbus::MethodErr;
use dbus_crossroads::Crossroads;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// D-Bus server information.
///
/// Specifically, the server name, vendor, version, and spec version.
const SERVER_INFO: [&str; 4] = [
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_AUTHORS"),
    env!("CARGO_PKG_VERSION"),
    "1.2",
];

/// D-Bus server capabilities.
///
/// - `actions`: The server will provide the specified actions to the user.
/// - `body`: Supports body text.
const SERVER_CAPABILITIES: [&str; 2] = ["actions", "body"];

mod dbus_server {
    #![allow(clippy::too_many_arguments)]
    include!(concat!(env!("OUT_DIR"), "/introspection.rs"));
}

/// ID counter for the notification.
static ID_COUNT: AtomicU32 = AtomicU32::new(1);

/// D-Bus interface for desktop notifications.
const NOTIFICATION_INTERFACE: &str = "org.freedesktop.Notifications";

/// D-Bus path for desktop notifications.
const NOTIFICATION_PATH: &str = "/org/freedesktop/Notifications";

/// D-Bus notification implementation.
///
/// <https://specifications.freedesktop.org/notification-spec/latest/ar01s09.html>
pub struct DbusNotification {
    sender: Sender<Action>,
}

impl dbus_server::OrgFreedesktopNotifications for DbusNotification {
    fn get_capabilities(&mut self) -> Result<Vec<String>, dbus::MethodErr> {
        Ok(SERVER_CAPABILITIES.into_iter().map(String::from).collect())
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
        let id = if replaces_id == 0 {
            ID_COUNT.fetch_add(1, Ordering::Relaxed)
        } else {
            replaces_id
        };
        let notification = Notification {
            id,
            app_name,
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
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| dbus::MethodErr::failed(&e))?
                .as_secs(),
        };
        tracing::trace!("{:#?}", notification);
        match self.sender.send(Action::Show(notification)) {
            Ok(_) => Ok(id),
            Err(e) => Err(dbus::MethodErr::failed(&e)),
        }
    }

    fn close_notification(&mut self, id: u32) -> Result<(), dbus::MethodErr> {
        tracing::trace!("received close signal for notification: {}", id);
        match self.sender.send(Action::Close(Some(id))) {
            Ok(_) => Ok(()),
            Err(e) => Err(dbus::MethodErr::failed(&e)),
        }
    }

    fn get_server_information(
        &mut self,
    ) -> Result<(String, String, String, String), dbus::MethodErr> {
        Ok((
            SERVER_INFO[0].to_string(),
            SERVER_INFO[1].to_string(),
            SERVER_INFO[2].to_string(),
            SERVER_INFO[3].to_string(),
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
        tracing::trace!("D-Bus server information: {:#?}", SERVER_INFO);
        tracing::trace!("D-Bus server capabilities: {:?}", SERVER_CAPABILITIES);
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
        sender: Sender<Action>,
        timeout: Duration,
    ) -> error::Result<()> {
        self.connection
            .request_name(NOTIFICATION_INTERFACE, false, true, false)?;
        let token = dbus_server::register_org_freedesktop_notifications(&mut self.crossroads);
        self.crossroads.insert(
            NOTIFICATION_PATH,
            &[token],
            DbusNotification {
                sender: sender.clone(),
            },
        );
        let token = self.crossroads.register(NOTIFICATION_INTERFACE, |builder| {
            let sender_cloned = sender.clone();
            builder.method("History", (), ("reply",), move |_, _, ()| {
                sender_cloned
                    .send(Action::ShowLast)
                    .map_err(|e| MethodErr::failed(&e))?;
                Ok((String::from("history signal sent"),))
            });
            let sender_cloned = sender.clone();
            builder.method("Close", (), ("reply",), move |_, _, (): ()| {
                sender_cloned
                    .send(Action::Close(None))
                    .map_err(|e| MethodErr::failed(&e))?;
                Ok((String::from("close signal sent"),))
            });
            builder.method("CloseAll", (), ("reply",), move |_, _, ()| {
                sender
                    .send(Action::CloseAll)
                    .map_err(|e| MethodErr::failed(&e))?;
                Ok((String::from("close all signal sent"),))
            });
        });
        self.crossroads
            .insert(format!("{}/ctl", NOTIFICATION_PATH), &[token], ());
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

    /// Sends a notification.
    ///
    /// See `org.freedesktop.Notifications.Notify`
    pub fn notify<S: Into<String>>(
        &self,
        app_name: S,
        summary: S,
        body: S,
        expire_timeout: i32,
    ) -> error::Result<()> {
        let proxy = Proxy::new(
            NOTIFICATION_INTERFACE,
            NOTIFICATION_PATH,
            Duration::from_millis(1000),
            &self.connection,
        );
        proxy.method_call(
            NOTIFICATION_INTERFACE,
            "Notify",
            (
                app_name.into(),
                0_u32,
                String::new(),
                summary.into(),
                body.into(),
                Vec::<String>::new(),
                {
                    let mut hints = HashMap::<String, Variant<Box<dyn RefArg + 'static>>>::new();
                    hints.insert(String::from("urgency"), Variant(Box::new(0_u8)));
                    hints
                },
                expire_timeout,
            ),
        )?;
        Ok(())
    }

    /// Closes the notification.
    ///
    /// See `org.freedesktop.Notifications.CloseNotification`
    pub fn close_notification(&self, id: u32, timeout: Duration) -> error::Result<()> {
        tracing::trace!(
            "sending close signal for notification: {} (timeout: {}ms)",
            id,
            timeout.as_millis()
        );
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
