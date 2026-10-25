// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Automatic terminal and JSON output for ordinary `tracing` events.
//!
//! Applications call [`init`] once. Libraries continue using `tracing` directly.
//! For existing subscribers and isolated captures, compose [`Console::layer`] instead.
//!
//! ```no_run
//! core_console::init();
//! tracing::info!(connections = 3_u64, "Server started");
//! ```

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
/// let failure = core_console::Failure::capture(std::io::Error::other("disconnected"));
/// core_console::failure!(failure, "Request {} failed", 42);
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
