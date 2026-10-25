// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::{
    OutputFormat,
    console_fields::{ConsoleFields, FieldVisitor},
};
use serde_json::{Map, Value, json};
use std::{fmt, time::Instant};
use tracing::{Event, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{
    fmt::{
        FmtContext, FormatEvent, FormattedFields,
        format::Writer,
        time::{FormatTime, SystemTime},
    },
    registry::LookupSpan,
};

pub(crate) struct ConsoleFormat {
    pub(crate) format: OutputFormat,
    pub(crate) started: Instant,
    pub(crate) ansi: bool,
}

impl<S> FormatEvent<S, ConsoleFields> for ConsoleFormat
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn format_event(
        &self,
        context: &FmtContext<'_, S, ConsoleFields>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut fields = FieldVisitor::default();
        event.record(&mut fields);
        let mut spans = Vec::new();
        if let Some(scope) = context.event_scope() {
            for span in scope.from_root() {
                let extensions = span.extensions();
                let fields = extensions.get::<FormattedFields<ConsoleFields>>().expect(
                    "the console records fields for every span visible to its formatting layer",
                );
                let fields: Map<String, Value> = serde_json::from_str(&fields.fields)
                    .expect("console span fields are JSON objects");
                spans.push(json!({"name": span.name(), "fields": fields}));
            }
        }
        let normalized = event.normalized_metadata();
        let metadata = normalized.as_ref().unwrap_or_else(|| event.metadata());
        if normalized.is_some() {
            fields.0.retain(|name, _| !name.starts_with("log."));
        }
        if self.format == OutputFormat::Json {
            let mut timestamp = String::new();
            SystemTime
                .format_time(&mut Writer::new(&mut timestamp))
                .expect("writing a timestamp to a String cannot fail");
            let record = json!({
                "timestamp": timestamp,
                "elapsed_seconds": self.started.elapsed().as_secs_f64(),
                "level": metadata.level().as_str(),
                "target": metadata.target(),
                "name": metadata.name(),
                "location": {"module": metadata.module_path(), "file": metadata.file(), "line": metadata.line()},
                "fields": fields.0,
                "spans": spans,
            });
            return writeln!(writer, "{record}");
        }

        let level = metadata.level().as_str().to_lowercase();
        if self.ansi {
            let colour = match *metadata.level() {
                tracing::Level::ERROR => 31,
                tracing::Level::WARN => 33,
                tracing::Level::INFO => 32,
                tracing::Level::DEBUG => 36,
                tracing::Level::TRACE => 90,
            };
            write!(writer, "\x1b[{colour}m")?;
        }
        write!(
            writer,
            "{:>7.3}s {level:>5}",
            self.started.elapsed().as_secs_f64()
        )?;
        if self.ansi {
            write!(writer, "\x1b[0m")?;
        }
        write!(writer, " {}", terminal_text(metadata.target()))?;
        for span in &spans {
            write!(writer, " / {}", terminal_text(&text(&span["name"])))?;
            for (name, value) in span["fields"].as_object().into_iter().flatten() {
                write!(writer, " {name}={value}")?;
            }
        }
        writeln!(writer)?;
        if let Some(message) = fields.0.remove("message") {
            lines(&mut writer, &text(&message), "  | ")?;
        }
        for (name, value) in fields.0 {
            if let Some(error) = value
                .as_object()
                .filter(|value| value.contains_key("causes"))
            {
                lines(
                    &mut writer,
                    &format!("{name}: {}", text(&error["message"])),
                    "  | ",
                )?;
                for cause in error["causes"].as_array().into_iter().flatten() {
                    lines(&mut writer, &text(cause), "  |   caused by: ")?;
                }
                if error.contains_key("truncated") {
                    writeln!(writer, "  |   (cause chain truncated)")?;
                }
            } else if name == "backtrace" || name == "span_trace" {
                writeln!(writer, "  | {name}:")?;
                lines(&mut writer, &text(&value), "  |   ")?;
            } else {
                lines(&mut writer, &format!("{name}={value}"), "  | ")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

fn text(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

fn lines(writer: &mut Writer<'_>, value: &str, prefix: &str) -> fmt::Result {
    for line in value.lines() {
        writeln!(writer, "{prefix}{}", terminal_text(line))?;
    }
    Ok(())
}

fn terminal_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() && character != '\t' {
            output.extend(character.escape_default());
        } else {
            output.push(character);
        }
    }
    output
}
