// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use serde_json::{Map, Value, json};
use std::{error::Error, fmt};
use tracing::{
    field::{Field, Visit},
    span,
};
use tracing_subscriber::{
    field::RecordFields,
    fmt::{FormatFields, FormattedFields, format::Writer},
};

/// Span fields are stored as JSON by the formatting layer so later records can update them without losing their types.
pub(crate) struct ConsoleFields;

impl<'writer> FormatFields<'writer> for ConsoleFields {
    fn format_fields<R: RecordFields>(
        &self,
        mut writer: Writer<'writer>,
        fields: R,
    ) -> fmt::Result {
        let mut values = FieldVisitor::default();
        fields.record(&mut values);
        write!(writer, "{}", Value::Object(values.0))
    }

    fn add_fields(
        &self,
        current: &'writer mut FormattedFields<Self>,
        fields: &span::Record<'_>,
    ) -> fmt::Result {
        let mut values = FieldVisitor(
            serde_json::from_str(&current.fields).expect("console span fields are JSON objects"),
        );
        fields.record(&mut values);
        current.fields = Value::Object(values.0).to_string();
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct FieldVisitor(pub(crate) Map<String, Value>);

impl Visit for FieldVisitor {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.insert(field.name().into(), json!(value));
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.insert(field.name().into(), json!(value));
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name().into(), json!(value));
    }
    fn record_i128(&mut self, field: &Field, value: i128) {
        self.0.insert(field.name().into(), json!(value));
    }
    fn record_u128(&mut self, field: &Field, value: u128) {
        self.0.insert(field.name().into(), json!(value));
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0.insert(field.name().into(), json!(value));
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().into(), json!(value));
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0
            .insert(field.name().into(), json!(format!("{value:?}")));
    }

    fn record_error(&mut self, field: &Field, value: &(dyn Error + 'static)) {
        let mut causes = Vec::new();
        let mut source = value.source();
        // User-defined cause chains can cycle. Bound traversal without discarding the reported error.
        while let Some(cause) = source {
            if causes.len() == 32 {
                break;
            }
            causes.push(cause.to_string());
            source = cause.source();
        }
        let mut error = json!({"message": value.to_string(), "causes": causes});
        if source.is_some() {
            error["truncated"] = true.into();
        }
        self.0.insert(field.name().into(), error);
    }
}
