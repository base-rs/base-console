// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use super::*;
use std::error::Error;

/// Check error propagation at every write boundary using a real subscriber context.
struct CheckWrites(ConsoleFormat);

struct RejectWrite {
    remaining: usize,
}

impl fmt::Write for RejectWrite {
    fn write_str(&mut self, _: &str) -> fmt::Result {
        if self.remaining == 0 {
            return Err(fmt::Error);
        }
        self.remaining -= 1;
        Ok(())
    }
}

impl<S> FormatEvent<S, ConsoleFields> for CheckWrites
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn format_event(
        &self,
        context: &FmtContext<'_, S, ConsoleFields>,
        _: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        for remaining in 0..1024 {
            let mut output = RejectWrite { remaining };
            let writer = Writer::new(&mut output);
            match self.0.format_event(context, writer, event) {
                Ok(()) => return Ok(()),
                Err(_) => assert_eq!(
                    output.remaining, 0,
                    "formatting failed before the destination rejected a write"
                ),
            }
        }
        panic!("event should need fewer than 1024 writes");
    }
}

#[derive(Debug)]
struct Cycle;
impl fmt::Display for Cycle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("cycle")
    }
}
impl Error for Cycle {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

#[test]
fn formatter_propagates_destination_errors_for_every_part_of_a_report() {
    use tracing_subscriber::layer::SubscriberExt;
    for format in [OutputFormat::Text, OutputFormat::Json] {
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .fmt_fields(ConsoleFields)
                .event_format(CheckWrites(ConsoleFormat {
                    format,
                    started: Instant::now(),
                    ansi: true,
                }))
                .with_writer(std::io::sink),
        );
        tracing::subscriber::with_default(subscriber, || {
            let request = tracing::info_span!("request", id = 42_u64);
            request.in_scope(|| {
                tracing::error!(
                    error = &Cycle as &dyn Error,
                    backtrace = "first\nsecond",
                    span_trace = "origin",
                    detail = true,
                    "Request failed"
                );
            });
        });
    }
}
