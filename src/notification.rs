use crate::dbus::NOTIFICATION_INTERFACE;
use crate::error::{Error, Result};
use dbus::arg::messageitem::MessageItem;
use dbus::message::{MatchRule, Message};
use dbus::strings::Member;
use std::result::Result as StdResult;

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

impl<'a> TryFrom<&'a Message> for Notification {
    type Error = Error;
    fn try_from(message: &'a Message) -> StdResult<Self, Self::Error> {
        let mut notification = Notification::default();
        let arguments = message.get_items();
        notification.app_name = match arguments.get(0) {
            Some(MessageItem::Str(app_name)) => Ok(app_name.to_string()),
            _ => Err(Error::DbusArgument(String::from(
                "app_name is missing from notification",
            ))),
        }?;
        notification.summary = match arguments.get(3) {
            Some(MessageItem::Str(summary)) => Ok(summary.to_string()),
            _ => Err(Error::DbusArgument(String::from(
                "summary is missing from notification",
            ))),
        }?;
        Ok(notification)
    }
}

impl Notification {
    /// Returns the corresponding D-Bus match rule for incoming notifications.
    pub fn get_dbus_match_rule() -> Result<MatchRule<'static>> {
        Ok(MatchRule::new_method_call()
            .with_interface(NOTIFICATION_INTERFACE)
            .with_member(Member::new("Notify").map_err(Error::DbusString)?)
            .with_eavesdrop())
    }
}
