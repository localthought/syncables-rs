//! The crate's error type.

use thiserror::Error;

/// Anything that can go wrong loading a document or talking to a server.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Reading a document from disk failed.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// A document could not be parsed as YAML or JSON.
    #[error("could not parse document: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    /// A document did not fit the OpenAPI type surface this crate expects.
    #[error("could not read document: {0}")]
    Json(#[from] serde_json::Error),

    /// An overlay used a JSONPath target outside the supported subset.
    #[error(
        "unsupported overlay target \"{0}\": only \"$\" or simple dot-paths like \
         \"$.components...\" are supported"
    )]
    UnsupportedOverlayTarget(String),

    /// An overlay target segment resolved to something that isn't an object.
    #[error("overlay target segment \"{0}\" does not resolve to an object")]
    OverlayTargetNotAnObject(String),

    /// An overlay tried to `remove` the document root.
    #[error("overlay cannot remove the document root")]
    OverlayRemovesRoot,

    /// The client could not reach the server, or the server rejected the request.
    #[error("http error: {0}")]
    Http(String),

    /// A resource path was asked for that the document does not declare.
    #[error("unknown resource \"{0}\"")]
    UnknownResource(String),
}

/// `Result` specialized to this crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
