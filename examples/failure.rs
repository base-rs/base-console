// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_console::{Console, Failure, OutputFormat};
use std::{backtrace::Backtrace, error::Error, fmt};

#[derive(Debug)]
struct RequestError(std::io::Error);

impl fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("could not complete the request")
    }
}

impl Error for RequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args().collect();
    let format = if arguments.iter().any(|argument| argument == "--text") {
        OutputFormat::Text
    } else if arguments.iter().any(|argument| argument == "--json") {
        OutputFormat::Json
    } else {
        OutputFormat::Auto
    };
    Console::default()
        .with_format(format)
        .with_env_filter()?
        .try_init()?;
    tracing::info!("Server started");
    let request = tracing::info_span!(
        "http.request",
        request_id = 42_u64,
        method = "GET",
        path = "/"
    );
    let failure = request.in_scope(|| {
        tracing::info!(bytes = 128_u64, "Reading request");
        let error = RequestError(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "peer reset the connection",
        ));
        if arguments.iter().any(|argument| argument == "--backtrace") {
            Failure::with_backtrace(error, Backtrace::force_capture())
        } else {
            Failure::capture(error)
        }
    });
    // The request scope has ended; its captured origin is still available.
    base_console::failure!(failure, "Request failed");
    Ok(())
}
