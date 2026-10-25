// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use std::{
    backtrace::{Backtrace, BacktraceStatus},
    error::Error,
    fmt,
};
use tracing_error::{SpanTrace, SpanTraceStatus};

/// An error with diagnostics captured where it is wrapped, which can be retained and reported later.
///
/// Capture at the failure boundary to preserve its origin before returning the value across async tasks. [`Backtrace::capture`] honours `RUST_BACKTRACE` and `RUST_LIB_BACKTRACE`; a supplied backtrace may instead be passed to [`Self::with_backtrace`]. Span traces require the error layer included by [`crate::Console::layer`].
#[derive(Debug)]
pub struct Failure<E> {
    error: E,
    backtrace: Backtrace,
    span_trace: Option<String>,
}

impl<E> Failure<E> {
    /// Capture the current backtrace, when enabled, and logical span context.
    pub fn capture(error: E) -> Self {
        Self::with_backtrace(error, Backtrace::capture())
    }

    /// Retain a previously captured backtrace and capture the current span context.
    pub fn with_backtrace(error: E, backtrace: Backtrace) -> Self {
        let trace = SpanTrace::capture();
        let span_trace = (trace.status() == SpanTraceStatus::CAPTURED).then(|| trace.to_string());
        Self {
            error,
            backtrace,
            span_trace,
        }
    }

    /// The underlying error.
    pub fn error(&self) -> &E {
        &self.error
    }

    /// The captured or supplied OS stack backtrace.
    pub fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    /// A text snapshot of the logical span context at capture time, if available.
    pub fn span_trace(&self) -> Option<&str> {
        self.span_trace.as_deref()
    }

    /// Recover the underlying error, discarding the captured diagnostics.
    pub fn into_inner(self) -> E {
        self.error
    }
}

impl<E> Failure<E> {
    /// The backtrace when capture succeeded; disabled or unsupported traces are omitted.
    pub fn captured_backtrace(&self) -> Option<&Backtrace> {
        (self.backtrace.status() == BacktraceStatus::Captured).then_some(&self.backtrace)
    }
}

impl<E: fmt::Display> fmt::Display for Failure<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<E: Error + 'static> Error for Failure<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error.source()
    }
}
