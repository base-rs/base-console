# `core-console`

Automatic terminal and JSON diagnostics for Rust applications using `tracing`.

Console chooses coloured, human-readable output when stderr is a terminal and newline-delimited JSON when it is redirected. Both formats preserve event fields, nested span context, and typed errors with their causes. Libraries continue using `tracing` directly; the application chooses its subscriber.

## Usage

Add Console and `tracing` to an application:

```sh
cargo add core-console
cargo add tracing
```

```rust
fn main() {
    core_console::init();
    tracing::info!(connections = 3_u64, "Server started");
}
```

`init` reads `RUST_LOG`, defaulting to `info`, and panics if setup fails. Use `core_console::try_init()?` to handle invalid filters or an already installed global subscriber. Both leave the `log` logger unchanged; applications may explicitly install `tracing_log::LogTracer` to include dependencies using `log`.

Try the demonstration:

```sh
cargo run --example failure
cargo run --example failure -- --text
cargo run --example failure -- --json --backtrace
```

### Failures

Ordinary error fields preserve their cause chains when recorded as `&dyn Error`:

```rust
let error = std::io::Error::other("connection closed");
tracing::error!(error = &error as &dyn std::error::Error, "Request failed");
```

Use `Failure` to capture diagnostics at the point an error is wrapped, then retain or return it for later reporting:

```rust
let failure = core_console::Failure::capture(std::io::Error::other("connection closed"));
core_console::failure!(failure, "Request {} failed", 42);
```

The macro preserves the caller's target and source location. The wrapper captures a logical span trace when Console's layer is active, and an OS backtrace when `RUST_BACKTRACE` or `RUST_LIB_BACKTRACE` enables it. `Failure::with_backtrace` accepts a previously captured backtrace. Capture before leaving the failing operation: wrapping an error later cannot reconstruct its earlier stack. Logging the wrapper directly as `&dyn Error` preserves its causes; use `failure!` to include its captured traces as well.

### Composition and scoped output

```rust
use core_console::{Console, OutputFormat};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

let console = Console::default()
    .with_format(OutputFormat::Text)
    .with_ansi(false)
    .with_filter(EnvFilter::new("info,core_http=debug"));

let subscriber = tracing_subscriber::registry().with(console.layer());
tracing::subscriber::with_default(subscriber, || {
    tracing::info!("Scoped output");
});
```

`Console::subscriber()` also builds an isolated subscriber directly. Use `Console::init()` or its fallible counterpart `Console::try_init()` to install a configured console globally. `with_writer` accepts a `Write + Send + 'static` destination; automatic output treats custom writers as redirected streams. `with_env_filter` explicitly reads `RUST_LOG` for a configured console. A layer's filter applies only to that layer.

For asynchronous scopes, use `tracing::Instrument` and `tracing::instrument::WithSubscriber` to attach spans and subscribers to futures before spawning them. A thread-local default guard must not be held across `.await`.

### Output contract

JSON includes `timestamp` (UTC), `elapsed_seconds` (process-local console uptime), `level`, `target`, `name`, `location`, `fields`, and a root-to-leaf `spans` array. Each enabled span has a name and its current fields; spans excluded by the console's filter are omitted. Reserved metadata stays separate from user fields. Typed errors are objects containing `message` and an ordered `causes` array; cause traversal stops after 32 entries and sets `truncated` when more remain.

Integer fields, including 128-bit integers, remain JSON numbers. Consumers must support their precision. Non-finite floating-point fields become JSON `null`; `%value` and `?value` remain text representations. Backtraces and captured span traces are text fields. The initial output schema may evolve before a stable release.

Writing is synchronous. Each console serializes complete records through its writer and flushes the destination before logging returns; it starts no background threads and requires no runtime. Text output escapes control characters from messages and diagnostic text, while JSON uses standard string escaping.

## Development

The private `bake/` package depends on `core-project` for shared tooling:

```sh
cargo bake test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo bake test:coverage --all-targets true
```

## Releasing

Prepare a release with `cargo bake cargo:version:patch` (or `minor`, `major`, or `bump --version X.Y.Z`), commit it with a `Release: vVERSION` trailer, then run `cargo bake cargo:release` and open a pull request. After review and merge, GitHub Actions publishes the release when the configured `crates-io` environment approves it, then creates or updates the matching GitHub Release from `releases.md`. Follow the shared [Releasing skill](https://github.com/core-rs/core-project/blob/main/context/releasing.md) for the standard process.

## Releases

<!-- bake-readme:releases:start -->
See [releases.md](releases.md) for the full release history.

### v0.1.0

- Choose terminal text or newline-delimited JSON automatically for `tracing` events.
- Preserve typed fields, span context, and error causes through composable or scoped subscribers.
- Capture failures with optional OS backtraces and logical span traces for later reporting.
- Follow Rust logging conventions with panicking `init` and fallible `try_init` functions and builder methods.
<!-- bake-readme:releases:end -->

## See Also

- [`tracing`](https://docs.rs/tracing) provides structured events and spans.
- [`tracing-subscriber`](https://docs.rs/tracing-subscriber) provides subscriber composition and filtering.

## Contributing

Please open an issue or pull request on [GitHub](https://github.com/core-rs/core-console).

Run `cargo bake agent:context:install` to install shared context and skills for development. Use `cargo bake agent:context:list` and `cargo bake agent:context:show` to discover guidance. Follow `agents.md` and the installed skills under `.agents/skills/`.
