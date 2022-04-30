#![allow(missing_docs)]

thiserror_lite::err_enum! {
    #[derive(Debug)]
    pub enum Error {
        #[error("D-Bus error: `{}`")]
        Dbus(#[from] dbus::Error),
        #[error("D-Bus string error: `{}`")]
        DbusString(String),
        #[error("D-Bus argument error: `{}`")]
        DbusArgument(String),
    }
}

/// Type alias for the standard [`Result`] type.
pub type Result<T> = core::result::Result<T, Error>;
