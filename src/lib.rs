// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Automatic terminal and JSON output for ordinary `tracing` events.
//!
//! Applications call [`init`] once. Libraries continue using `tracing` directly.
//! For existing subscribers and isolated captures, compose [`Console::layer`] instead.
//!
//! ```no_run
//! base_console::init();
//! tracing::info!(connections = 3_u64, "Server started");
//! ```
//!
//! # Composition
//!
//! [`Console`] configures the format, colour, writer, and filter, then builds a composable [`Console::layer`], an isolated [`Console::subscriber`], or a global installation with [`Console::init`] or [`Console::try_init`]. A console's filter applies only to its own output.
//!
//! ```
//! use base_console::{Console, OutputFormat};
//! use tracing_subscriber::{layer::SubscriberExt, EnvFilter};
//!
//! let console = Console::default()
//!     .with_format(OutputFormat::Text)
//!     .with_ansi(false)
//!     .with_filter(EnvFilter::new("info,base_http=debug"));
//!
//! let subscriber = tracing_subscriber::registry().with(console.layer());
//! tracing::subscriber::with_default(subscriber, || {
//!     tracing::info!("Scoped output");
//! });
//! ```
//!
//! For asynchronous scopes, use `tracing::Instrument` and `tracing::instrument::WithSubscriber` to attach spans and subscribers to futures before spawning them. A thread-local default guard must not be held across `.await`.
//!
//! # Output
//!
//! JSON includes `timestamp` (UTC), `elapsed_seconds` (process-local console uptime), `level`, `target`, `name`, `location`, `fields`, and a root-to-leaf `spans` array. Each enabled span has a name and its current fields; spans excluded by the console's filter are omitted. Reserved metadata stays separate from user fields. Typed errors are objects containing `message` and an ordered `causes` array; cause traversal stops after 32 entries and sets `truncated` when more remain.
//!
//! Integer fields, including 128-bit integers, remain JSON numbers. Consumers must support their precision. Non-finite floating-point fields become JSON `null`; `%value` and `?value` remain text representations. Backtraces and captured span traces are text fields. The initial output schema may evolve before a stable release.
//!
//! Writing is synchronous. Each console serializes complete records through its writer and flushes the destination before logging returns; it starts no background threads and requires no runtime. Text output escapes control characters from messages and diagnostic text, while JSON uses standard string escaping.
mod as_console_error;
mod console;
mod console_fields;
mod console_format;
mod console_init_error;
mod failure;
mod output_format;

pub use console::Console;
pub use console_init_error::ConsoleInitError;
pub use failure::Failure;
pub use output_format::OutputFormat;

#[doc(hidden)]
pub use tracing as __tracing;

#[doc(hidden)]
pub use as_console_error::AsConsoleError as __AsConsoleError;

/// Emit a captured [`Failure`] as an error event at the caller's target and source location.
///
/// The error's causes remain typed; captured backtraces and original span context are included as text fields. Ordinary `tracing` subscribers can consume this event too. The failure expression is evaluated once. An explicit `target:` may override the caller's module path.
///
/// ```
/// let failure = base_console::Failure::capture(std::io::Error::other("disconnected"));
/// base_console::failure!(failure, "Request {} failed", 42);
/// ```
#[macro_export]
macro_rules! failure {
    (target: $target:expr, $failure:expr, $($message:tt)+) => {{
        use $crate::__AsConsoleError as _;
        let failure = &$failure;
        $crate::__tracing::error!(
            target: $target,
            error = (*failure.error()).console_error(),
            backtrace = failure.captured_backtrace().map($crate::__tracing::field::display),
            span_trace = failure.span_trace(),
            $($message)+
        );
    }};
    ($failure:expr, $($message:tt)+) => {
        $crate::failure!(target: module_path!(), $failure, $($message)+)
    };
}

/// Install a global console using `RUST_LOG`, defaulting to `info`.
///
/// Output goes to stderr: terminal text with colour when attached to a terminal, otherwise newline-delimited JSON. The process's `log` logger is left unchanged. Libraries should leave installation to their application. Use [`try_init`] to handle setup errors.
///
/// # Panics
///
/// Panics if `RUST_LOG` cannot be read or parsed, or a global subscriber is already installed.
#[track_caller]
pub fn init() {
    try_init().expect("could not initialize the console");
}

/// Try to install a global console using `RUST_LOG`, defaulting to `info`.
///
/// Uses the same output defaults as [`init`]. Returns an error if `RUST_LOG` cannot be read or parsed, or a global subscriber is already installed. The process's `log` logger is left unchanged.
pub fn try_init() -> Result<(), ConsoleInitError> {
    Console::default().with_env_filter()?.try_init()?;
    Ok(())
}
