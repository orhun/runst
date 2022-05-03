use crate::error::Result;
use crate::notification::Notification;
use dbus::blocking::Connection;
use std::time::Duration;

/// D-Bus interface to listen for notifications.
pub const NOTIFICATION_INTERFACE: &str = "org.freedesktop.Notifications";

/// Wrapper for a [`D-Bus connection`].
///
/// [`D-Bus connection`]: Connection
pub struct Dbus {
    /// Connection to D-Bus.
    connection: Connection,
}

impl Dbus {
    /// Initializes the D-Bus controller.
    pub fn init() -> Result<Self> {
        Ok(Self {
            connection: Connection::new_session()?,
        })
    }

    /// Registers a handler for the notifications that are parsed from D-Bus messages.
    pub fn register_notification_handler<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(Notification) -> Result<()> + Send + Sync + 'static,
    {
        self.connection.add_match(
            Notification::get_dbus_match_rule()?,
            move |_: (), _, message| {
                match Notification::try_from(message) {
                    Ok(notification) => {
                        handler(notification).expect("failed to handle notification")
                    }
                    Err(e) => eprintln!("{}", e),
                }
                true
            },
        )?;
        Ok(())
    }

    /// Starts handling the incoming messages.
    ///
    /// See [`Connection::process`]
    pub fn listen(&self, timeout: Duration) -> Result<()> {
        loop {
            self.connection.process(timeout)?;
        }
    }
}
