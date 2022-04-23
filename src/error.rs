#![allow(missing_docs)]

thiserror_lite::err_enum! {
    #[derive(Debug)]
    pub enum Error {
        #[error("unknown error")]
        Unknown,
    }
}

/// Type alias for the standard [`Result`] type.
pub type Result<T> = core::result::Result<T, Error>;
