#![allow(missing_docs)]

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("IO error: `{0}`")]
    Io(#[from] std::io::Error),
    #[error("D-Bus error: `{0}`")]
    Dbus(#[from] dbus::Error),
    #[error("D-Bus string error: `{0}`")]
    DbusString(String),
    #[error("D-Bus argument error: `{0}`")]
    DbusArgument(String),
    #[error("X11 connect error: `{0}`")]
    X11Connect(#[from] x11rb::errors::ConnectError),
    #[error("X11 connection error: `{0}`")]
    X11Connection(#[from] x11rb::errors::ConnectionError),
    #[error("X11 ID error: `{0}`")]
    X11Id(#[from] x11rb::errors::ReplyOrIdError),
    #[error("X11 error: `{0}`")]
    X11Other(String),
    #[error("Cairo error: `{0}`")]
    Cairo(#[from] cairo::Error),
    #[error("Pango error: `{0}`")]
    PangoOther(String),
    #[error("Receiver error: `{0}`")]
    Receiver(#[from] std::sync::mpsc::RecvError),
    #[error("TOML parsing error: `{0}`")]
    Toml(#[from] toml::de::Error),
    #[error("Scan error: `{0}`")]
    Scanf(String),
    #[error("Integer conversion error: `{0}`")]
    IntegerConversion(#[from] std::num::TryFromIntError),
    #[error("Template error: `{0}`")]
    Template(#[from] tera::Error),
    #[error("Template parse error:\n{0}")]
    TemplateParse(String),
    #[error("Template render error:\n{0}")]
    TemplateRender(String),
    #[error("Config error: `{0}`")]
    Config(String),
}

/// Type alias for the standard [`Result`] type.
pub type Result<T> = std::result::Result<T, Error>;
