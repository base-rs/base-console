// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_console::{Console, Failure, OutputFormat};
use serde_json::Value;
use std::{
    backtrace::{Backtrace, BacktraceStatus},
    error::Error,
    fmt,
    future::Future,
    io::{self, Write},
    pin::Pin,
    process::Command,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};
use tracing::{Instrument, instrument::WithSubscriber};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt};

#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);

impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Buffer {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
    fn records(&self) -> Vec<Value> {
        self.text()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
    fn console(&self) -> Console<Self> {
        Console::default().with_writer(self.clone())
    }
}

#[derive(Debug)]
struct RequestError(io::Error);

impl fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("request failed")
    }
}

impl Error for RequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[test]
fn json_preserves_typed_fields_and_call_site_metadata() {
    let buffer = Buffer::default();
    tracing::subscriber::with_default(buffer.console().subscriber(), || {
        tracing::info!(
            name: "connected",
            target: "client",
            count = u64::MAX,
            debt = -3_i64,
            large = u128::MAX,
            negative = i128::MIN,
            ratio = 0.5_f64,
            non_finite = f64::INFINITY,
            reused = true,
            peer = "localhost",
            display = %42,
            debug = ?vec![1, 2],
            "Connected\nwith {} bytes", 10
        );
    });
    let records = buffer.records();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record["level"], "INFO");
    assert_eq!(record["target"], "client");
    assert_eq!(record["name"], "connected");
    assert_eq!(record["location"]["module"], module_path!());
    assert!(
        record["location"]["file"]
            .as_str()
            .unwrap()
            .ends_with("tests/console.rs")
    );
    assert!(record["location"]["line"].is_number());
    assert!(record["timestamp"].as_str().unwrap().ends_with('Z'));
    assert!(record["elapsed_seconds"].as_f64().unwrap() >= 0.0);
    let fields = &record["fields"];
    assert_eq!(fields["count"].as_u64(), Some(u64::MAX));
    assert_eq!(fields["debt"], -3);
    assert_eq!(
        fields["large"].as_number().unwrap().as_u128(),
        Some(u128::MAX)
    );
    assert_eq!(
        fields["negative"].as_number().unwrap().as_i128(),
        Some(i128::MIN)
    );
    assert_eq!(fields["ratio"], 0.5);
    assert!(fields["non_finite"].is_null());
    assert_eq!(fields["reused"], true);
    assert_eq!(fields["peer"], "localhost");
    assert_eq!(fields["display"], "42");
    assert_eq!(fields["debug"], "[1, 2]");
    assert_eq!(fields["message"], "Connected\nwith 10 bytes");
    assert!(record["spans"].as_array().unwrap().is_empty());
}

#[test]
fn nested_and_explicit_parent_spans_include_recorded_updates() {
    let buffer = Buffer::default();
    tracing::subscriber::with_default(buffer.console().subscriber(), || {
        let request = tracing::info_span!("request", id = 42_u64, status = tracing::field::Empty);
        request.in_scope(|| {
            let work = tracing::info_span!("work", attempt = 1_u64);
            work.record("attempt", 2_u64);
            work.in_scope(|| tracing::info!("Working"));
        });
        request.record("status", 200_u64);
        tracing::info!(parent: &request, "Complete");
    });
    let records = buffer.records();
    assert_eq!(records[0]["spans"][0]["name"], "request");
    assert_eq!(records[0]["spans"][1]["fields"]["attempt"], 2);
    assert_eq!(records[1]["spans"].as_array().unwrap().len(), 1);
    assert_eq!(records[1]["spans"][0]["fields"]["status"], 200);
}

#[test]
fn spans_filtered_from_console_are_omitted_from_its_context() {
    let buffer = Buffer::default();
    let subscriber = tracing_subscriber::registry()
        .with(buffer.console().with_filter(EnvFilter::new("warn")).layer())
        .with(tracing_subscriber::fmt::layer().with_writer(io::sink));
    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("other_layer", id = 42_u64);
        span.in_scope(|| tracing::warn!("warning"));
    });
    let records = buffer.records();
    assert_eq!(records[0]["spans"], serde_json::json!([]));
}

#[test]
fn text_can_render_numeric_messages_and_events_without_messages() {
    let buffer = Buffer::default();
    tracing::subscriber::with_default(
        buffer
            .console()
            .with_format(OutputFormat::Text)
            .subscriber(),
        || {
            tracing::info!(message = 42_u64);
            tracing::info!(answer = 42_u64);
        },
    );
    assert!(buffer.text().contains("  | 42\n"));
    assert!(buffer.text().contains("  | answer=42\n"));
}

#[test]
fn typed_errors_preserve_ordered_causes() {
    let buffer = Buffer::default();
    tracing::subscriber::with_default(buffer.console().subscriber(), || {
        let error = RequestError(io::Error::other("connection lost"));
        tracing::error!(error = &error as &dyn Error, "Could not finish");
        tracing::error!(error = &io::Error::other("standalone") as &dyn Error);
    });
    let records = buffer.records();
    assert_eq!(records[0]["fields"]["error"]["message"], "request failed");
    assert_eq!(
        records[0]["fields"]["error"]["causes"],
        serde_json::json!(["connection lost"])
    );
    assert_eq!(
        records[1]["fields"]["error"]["causes"],
        serde_json::json!([])
    );
}

#[test]
fn cyclic_causes_are_bounded() {
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
    let buffer = Buffer::default();
    tracing::subscriber::with_default(buffer.console().subscriber(), || {
        tracing::error!(error = &Cycle as &dyn Error);
    });
    let records = buffer.records();
    assert_eq!(
        records[0]["fields"]["error"]["causes"]
            .as_array()
            .unwrap()
            .len(),
        32
    );
    assert_eq!(records[0]["fields"]["error"]["truncated"], true);
}

#[test]
fn text_formats_multiline_errors_and_diagnostics_without_colour() {
    let buffer = Buffer::default();
    tracing::subscriber::with_default(
        buffer
            .console()
            .with_format(OutputFormat::Text)
            .subscriber(),
        || {
            let request = tracing::info_span!("request", id = 42_u64);
            request.in_scope(|| {
                let error = RequestError(io::Error::other("connection lost"));
                tracing::error!(
                    error = &error as &dyn Error,
                    backtrace = "first frame\nsecond frame",
                    detail = true,
                    "First line\nSecond line\x1b[31m"
                );
            });
        },
    );
    let text = buffer.text();
    assert!(!text.contains('\x1b'));
    assert!(text.contains("error console / request id=42"));
    assert!(text.contains("  | First line\n  | Second line\\u{1b}[31m\n"));
    assert!(text.contains("  | error: request failed\n  |   caused by: connection lost\n"));
    assert!(text.contains("  | backtrace:\n  |   first frame\n  |   second frame\n"));
    assert!(text.contains("  | detail=true\n"));
}

#[test]
fn ansi_is_explicit_for_custom_text_writers_and_never_styles_json() {
    let buffer = Buffer::default();
    tracing::subscriber::with_default(
        buffer
            .console()
            .with_format(OutputFormat::Text)
            .with_ansi(true)
            .with_filter(EnvFilter::new("trace"))
            .subscriber(),
        || {
            tracing::error!("error");
            tracing::warn!("warning");
            tracing::info!("info");
            tracing::debug!("debug");
            tracing::trace!("trace");
        },
    );
    let text = buffer.text();
    for colour in [31, 33, 32, 36, 90] {
        assert!(text.contains(&format!("\x1b[{colour}m")));
    }
    let json = Buffer::default();
    tracing::subscriber::with_default(json.console().with_ansi(true).subscriber(), || {
        tracing::info!("message")
    });
    assert!(!json.text().contains('\x1b'));
    assert_eq!(json.records()[0]["fields"]["message"], "message");
}

#[test]
fn filters_are_local_to_each_layer() {
    let first = Buffer::default();
    let second = Buffer::default();
    let subscriber = tracing_subscriber::registry()
        .with(first.console().with_filter(EnvFilter::new("warn")).layer())
        .with(second.console().with_filter(EnvFilter::new("info")).layer());
    tracing::subscriber::with_default(subscriber, || {
        tracing::debug!("debug");
        tracing::info!("info");
        tracing::warn!("warning");
    });
    assert_eq!(first.records().len(), 1);
    assert_eq!(second.records().len(), 2);
}

#[test]
fn standalone_subscriber_rejects_disabled_events_before_evaluating_fields() {
    let buffer = Buffer::default();
    tracing::subscriber::with_default(buffer.console().subscriber(), || {
        tracing::debug!(expensive = expensive_field());
        tracing::info!("visible");
    });
    assert_eq!(buffer.records().len(), 1);
}

fn expensive_field() -> u64 {
    panic!("disabled fields must not be evaluated")
}

#[test]
fn captured_failure_keeps_origin_after_span_updates_and_uses_the_callers_target() {
    let buffer = Buffer::default();
    tracing::subscriber::with_default(
        buffer
            .console()
            .with_filter(EnvFilter::new("console=info"))
            .subscriber(),
        || {
            let request = tracing::info_span!("original_request", status = 100_u64);
            request.record("status", 200_u64);
            let failure = request.in_scope(|| {
                Failure::with_backtrace(
                    RequestError(io::Error::other("peer reset")),
                    Backtrace::disabled(),
                )
            });
            request.record("status", 500_u64);
            assert!(failure.span_trace().unwrap().contains("\"status\":200"));
            assert!(!failure.span_trace().unwrap().contains("\"status\":500"));
            assert_eq!(failure.backtrace().status(), BacktraceStatus::Disabled);
            assert_eq!(failure.error().to_string(), "request failed");
            assert_eq!(failure.to_string(), "request failed");
            assert_eq!(failure.source().unwrap().to_string(), "peer reset");
            base_console::failure!(failure, "Request {} failed", 42);
            assert_eq!(failure.into_inner().0.to_string(), "peer reset");
        },
    );
    let records = buffer.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["target"], module_path!());
    assert_eq!(records[0]["fields"]["message"], "Request 42 failed");
    assert!(
        !records[0]["fields"]
            .as_object()
            .unwrap()
            .contains_key("backtrace")
    );
    assert!(
        records[0]["fields"]["span_trace"]
            .as_str()
            .unwrap()
            .contains("original_request")
    );
    assert!(records[0]["spans"].as_array().unwrap().is_empty());
}

#[test]
fn supplied_backtrace_is_reported_and_capture_works_without_a_subscriber() {
    let failure = Failure::capture(io::Error::other("no subscriber"));
    assert!(failure.span_trace().is_none());
    assert!(failure.source().is_none());
    let buffer = Buffer::default();
    let failure = Failure::with_backtrace(io::Error::other("captured"), Backtrace::force_capture());
    tracing::subscriber::with_default(buffer.console().subscriber(), || {
        base_console::failure!(target: "explicit", failure, "Failure");
    });
    let records = buffer.records();
    assert_eq!(records[0]["target"], "explicit");
    assert!(
        !records[0]["fields"]["backtrace"]
            .as_str()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn failure_reports_boxed_and_borrowed_error_trait_objects() {
    let buffer = Buffer::default();
    tracing::subscriber::with_default(buffer.console().subscriber(), || {
        let error: Box<dyn Error> = Box::new(RequestError(io::Error::other("boxed")));
        base_console::failure!(Failure::capture(error), "boxed error");
        let error: Box<dyn Error + Send> = Box::new(io::Error::other("send"));
        base_console::failure!(Failure::capture(error), "send error");
        let error: Box<dyn Error + Sync> = Box::new(io::Error::other("sync"));
        base_console::failure!(Failure::capture(error), "sync error");
        let error: Box<dyn Error + Send + Sync> = Box::new(io::Error::other("send and sync"));
        base_console::failure!(Failure::capture(error), "send sync error");
        let error = io::Error::other("borrowed");
        let error: &dyn Error = &error;
        base_console::failure!(Failure::capture(error), "borrowed error");
    });
    let records = buffer.records();
    assert_eq!(records.len(), 5);
    assert_eq!(records[0]["fields"]["error"]["causes"][0], "boxed");
    assert_eq!(records[4]["fields"]["error"]["message"], "borrowed");
}

type Task = Pin<Box<dyn Future<Output = ()> + Send>>;

fn poll_on_thread(mut future: Task) -> (Task, Poll<()>) {
    std::thread::spawn(move || {
        let result = future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()));
        assert!(tracing::Span::current().is_disabled());
        (future, result)
    })
    .join()
    .unwrap()
}

#[test]
fn futures_keep_their_own_subscriber_and_span_across_threads() {
    let first = Buffer::default();
    let second = Buffer::default();
    let make_task = |buffer: &Buffer, id| -> Task {
        let dispatch = tracing::Dispatch::new(buffer.console().subscriber());
        let span =
            tracing::dispatcher::with_default(&dispatch, || tracing::info_span!("request", id));
        Box::pin(
            async {
                tracing::info!("before suspension");
                futures_lite::future::yield_now().await;
                tracing::info!("after suspension");
            }
            .instrument(span)
            .with_subscriber(dispatch),
        )
    };
    let (first_task, pending) = poll_on_thread(make_task(&first, 1_u64));
    assert!(pending.is_pending());
    let (second_task, pending) = poll_on_thread(make_task(&second, 2_u64));
    assert!(pending.is_pending());
    assert!(poll_on_thread(first_task).1.is_ready());
    assert!(poll_on_thread(second_task).1.is_ready());
    for (buffer, id) in [(&first, 1), (&second, 2)] {
        let records = buffer.records();
        assert_eq!(records.len(), 2);
        for record in records {
            assert_eq!(record["spans"][0]["fields"]["id"], id);
        }
    }
}

#[test]
fn concurrent_records_remain_complete_json_lines() {
    let buffer = Buffer::default();
    let dispatch = tracing::Dispatch::new(buffer.console().subscriber());
    std::thread::scope(|scope| {
        for worker in 0..4 {
            let dispatch = &dispatch;
            scope.spawn(move || {
                tracing::dispatcher::with_default(dispatch, || {
                    for sequence in 0..20 {
                        tracing::info!(worker, sequence, "{}", "x".repeat(1024));
                    }
                })
            });
        }
    });
    let records = buffer.records();
    assert_eq!(records.len(), 80);
    assert!(
        records
            .iter()
            .all(|record| record["fields"]["message"].as_str().unwrap().len() == 1024)
    );
}

#[test]
fn buffered_destinations_are_flushed_before_logging_returns() {
    let buffer = Buffer::default();
    let writer = io::BufWriter::new(buffer.clone());
    tracing::subscriber::with_default(Console::default().with_writer(writer).subscriber(), || {
        tracing::info!("small buffered record");
        assert_eq!(
            buffer.records()[0]["fields"]["message"],
            "small buffered record"
        );
    });
}

#[test]
fn process_probe() {
    let Ok(mode) = std::env::var("CORE_CONSOLE_TEST_PROBE") else {
        return;
    };
    match mode.as_str() {
        "emit" => {
            base_console::init();
            tracing::info!("visible info");
            tracing::warn!("visible warning");
            tracing::debug!("hidden debug");
        }
        "duplicate" => {
            base_console::try_init().unwrap();
            let error = base_console::try_init().unwrap_err();
            assert!(matches!(
                error,
                base_console::ConsoleInitError::Subscriber(_)
            ));
            assert!(error.to_string().contains("install"));
            assert!(error.source().is_some());
            assert!(std::panic::catch_unwind(base_console::init).is_err());
        }
        "invalid" => {
            let error = base_console::try_init().unwrap_err();
            assert!(matches!(error, base_console::ConsoleInitError::Filter(_)));
            assert!(error.to_string().contains("filter"));
            assert!(error.source().is_some());
            assert!(std::panic::catch_unwind(base_console::init).is_err());
        }
        "configured" => {
            let buffer = Buffer::default();
            Console::default().with_writer(buffer.clone()).init();
            tracing::info!("configured console");
            assert_eq!(
                buffer.records()[0]["fields"]["message"],
                "configured console"
            );
            assert!(Console::default().try_init().is_err());
            assert!(std::panic::catch_unwind(|| Console::default().init()).is_err());
        }
        "existing-logger" => {
            struct Logger;
            impl log::Log for Logger {
                fn enabled(&self, _: &log::Metadata<'_>) -> bool {
                    true
                }
                fn log(&self, _: &log::Record<'_>) {}
                fn flush(&self) {}
            }
            static LOGGER: Logger = Logger;
            log::set_logger(&LOGGER).unwrap();
            base_console::init();
            assert!(std::ptr::eq(log::logger(), &LOGGER as &dyn log::Log));
        }
        "bridge" => {
            tracing_log::LogTracer::init().unwrap();
            base_console::init();
            log::info!(target: "legacy_client", "bridged message");
        }
        _ => panic!("unknown probe"),
    }
}

fn probe(mode: &str, filter: Option<&str>) -> std::process::Output {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "process_probe", "--nocapture"])
        .env("CORE_CONSOLE_TEST_PROBE", mode)
        .env_remove("RUST_LOG");
    if let Some(filter) = filter {
        command.env("RUST_LOG", filter);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn global_initialization_and_environment_filters_work_in_isolated_processes() {
    let output = probe("emit", None);
    let records: Vec<Value> = String::from_utf8(output.stderr)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(records.len(), 2);
    let output = probe("emit", Some("warn"));
    let record: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(record["level"], "WARN");
    probe("duplicate", None);
    probe("invalid", Some("[broken"));
    probe("configured", None);
    probe("existing-logger", None);
    let output = probe("bridge", None);
    let record: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(record["target"], "legacy_client");
    assert!(
        record["location"]["file"]
            .as_str()
            .unwrap()
            .ends_with("console.rs")
    );
    assert!(
        !record["fields"]
            .as_object()
            .unwrap()
            .keys()
            .any(|name| name.starts_with("log."))
    );
}
