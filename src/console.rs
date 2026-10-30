// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::{OutputFormat, console_fields::ConsoleFields, console_format::ConsoleFormat};
use std::{
    io::{self, IsTerminal, Write},
    sync::Mutex,
    time::Instant,
};
use tracing::{Subscriber, subscriber::SetGlobalDefaultError};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    EnvFilter, Layer, Registry, filter::FromEnvError, layer::SubscriberExt, registry::LookupSpan,
};

/// Configure console output without selecting a runtime or installing global state.
///
/// The default filter is `info`. Use [`Self::with_env_filter`] to read `RUST_LOG`, or [`Self::with_filter`] for explicit configuration. Writing is synchronous; complete records are serialized through the writer's mutex.
pub struct Console<W = io::Stderr> {
    writer: W,
    format: OutputFormat,
    terminal: bool,
    ansi: Option<bool>,
    filter: EnvFilter,
}

impl Default for Console {
    fn default() -> Self {
        let writer = io::stderr();
        let terminal = writer.is_terminal();
        Self {
            writer,
            format: OutputFormat::Auto,
            terminal,
            ansi: None,
            filter: EnvFilter::new("info"),
        }
    }
}

impl<W> Console<W> {
    /// Select text, JSON, or automatic output.
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }

    /// Enable or disable ANSI colour in text output. JSON never includes styling.
    pub fn with_ansi(mut self, enabled: bool) -> Self {
        self.ansi = Some(enabled);
        self
    }

    /// Replace the destination. Building a layer, subscriber, or global console requires a `Write + Send + 'static` writer. Automatic output treats custom writers as redirected streams, selecting JSON without colour. Use [`Self::with_format`] to request text.
    pub fn with_writer<T>(self, writer: T) -> Console<T> {
        Console {
            writer,
            format: self.format,
            terminal: false,
            ansi: self.ansi,
            filter: self.filter,
        }
    }

    /// Set the console's filter without affecting other subscriber layers.
    pub fn with_filter(mut self, filter: EnvFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Read `RUST_LOG`, defaulting to `info` if absent. Invalid values are errors.
    pub fn with_env_filter(mut self) -> Result<Self, FromEnvError> {
        self.filter = EnvFilter::builder()
            .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
            .from_env()?;
        Ok(self)
    }
}

impl<W: Write + Send + 'static> Console<W> {
    /// Build a composable output layer, including support for captured span traces.
    ///
    /// This does not install a subscriber or a `log` bridge. Its filter applies only to this layer. Attach it to a registry, then use a scoped subscriber or install the application's combined subscriber.
    pub fn layer<S>(self) -> impl Layer<S>
    where
        S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    {
        let filter = self.filter.clone();
        self.output_layer().with_filter(filter)
    }

    fn output_layer<S>(self) -> impl Layer<S>
    where
        S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    {
        let format = self.format.resolve(self.terminal);
        let ansi = self.ansi.unwrap_or(self.terminal) && format == OutputFormat::Text;
        let formatter = ConsoleFormat {
            format,
            started: Instant::now(),
            ansi,
        };
        let output = tracing_subscriber::fmt::layer()
            .fmt_fields(ConsoleFields)
            .event_format(formatter)
            .with_ansi(ansi)
            .with_writer(Mutex::new(RecordWriter(self.writer)));
        ErrorLayer::new(ConsoleFields).and_then(output)
    }

    /// Build an isolated subscriber; no global state is installed.
    pub fn subscriber(self) -> impl Subscriber + Send + Sync {
        let filter = self.filter.clone();
        Registry::default().with(filter).with(self.output_layer())
    }

    /// Install this console globally. Use [`Self::try_init`] to handle setup errors.
    ///
    /// This leaves the process's `log` logger unchanged.
    ///
    /// # Panics
    ///
    /// Panics if another global subscriber is already installed.
    #[track_caller]
    pub fn init(self) {
        self.try_init().expect("could not initialize the console");
    }

    /// Try to install this console globally. Returns an error if another subscriber is already installed.
    ///
    /// This leaves the process's `log` logger unchanged. Applications that want to route `log` records into tracing can install `tracing_log::LogTracer` separately.
    pub fn try_init(self) -> Result<(), SetGlobalDefaultError> {
        tracing::subscriber::set_global_default(self.subscriber())
    }
}

/// A global subscriber is never dropped, so buffered destinations must flush while the record is being written.
struct RecordWriter<W>(W);

impl<W: Write> Write for RecordWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.0.write(bytes)?;
        self.0.flush()?;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[cfg(test)]
mod tests;
