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
        #[error("X11 connect error: `{}`")]
        X11Connect(#[from] x11rb::errors::ConnectError),
        #[error("X11 connection error: `{}`")]
        X11Connection(#[from] x11rb::errors::ConnectionError),
        #[error("X11 ID error: `{}`")]
        X11Id(#[from] x11rb::errors::ReplyOrIdError),
    }
}

/// Type alias for the standard [`Result`] type.
pub type Result<T> = core::result::Result<T, Error>;
