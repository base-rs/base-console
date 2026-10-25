// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use std::{error::Error, fmt};
use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::filter::FromEnvError;

/// Console setup failed before normal application logging began.
#[derive(Debug)]
pub enum ConsoleInitError {
    /// `RUST_LOG` could not be read or parsed.
    Filter(FromEnvError),
    /// A global subscriber could not be installed.
    Subscriber(SetGlobalDefaultError),
}

impl fmt::Display for ConsoleInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Filter(_) => formatter.write_str("could not configure the console filter"),
            Self::Subscriber(_) => formatter.write_str("could not install the console subscriber"),
        }
    }
}

impl Error for ConsoleInitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::Filter(error) => error,
            Self::Subscriber(error) => error,
        })
    }
}

impl From<FromEnvError> for ConsoleInitError {
    fn from(error: FromEnvError) -> Self {
        Self::Filter(error)
    }
}

impl From<SetGlobalDefaultError> for ConsoleInitError {
    fn from(error: SetGlobalDefaultError) -> Self {
        Self::Subscriber(error)
    }
}
