use thiserror::Error;

/// The error variants Koit can return.
///
/// The concrete source error types are the associated errors types
/// [`Format::Error`](crate::format::Format::Error) and [`Backend::Error`](crate::backend::Backend::Error).
#[derive(Debug, Error)]
pub enum KoitError {
    /// Data failed to be encoded by the formatter.
    #[error("the database failed to serialize")]
    ToFormat(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    /// Data failed to be decoded by the formatter.
    #[error("the database failed to deserialize")]
    FromFormat(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    /// The backend failed to read bytes.
    #[error("failed to read from the backend")]
    BackendRead(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    /// The backend failed to write bytes.
    #[error("failed to write to the backend")]
    BackendWrite(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    /// The backend failed to be created.
    #[error("failed to create backend")]
    BackendCreation(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}
