# `base-console`

Automatic terminal and JSON diagnostics for Rust applications using `tracing`.

Console chooses coloured, human-readable output when stderr is a terminal and newline-delimited JSON when it is redirected. Both formats preserve event fields, nested span context, and typed errors with their causes. Libraries continue using `tracing` directly; the application chooses its subscriber.

## Usage

Add Console and `tracing` to an application:

```sh
cargo add base-console
cargo add tracing
```

```rust
fn main() {
    base_console::init();
    tracing::info!(connections = 3_u64, "Server started");
}
```

`init` reads `RUST_LOG`, defaulting to `info`, and panics if setup fails. Use `base_console::try_init()?` to handle invalid filters or an already installed global subscriber. Both leave the `log` logger unchanged; applications may explicitly install `tracing_log::LogTracer` to include dependencies using `log`.

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
let failure = base_console::Failure::capture(std::io::Error::other("connection closed"));
base_console::failure!(failure, "Request {} failed", 42);
```

The macro preserves the caller's target and source location. The wrapper captures a logical span trace when Console's layer is active, and an OS backtrace when `RUST_BACKTRACE` or `RUST_LIB_BACKTRACE` enables it. `Failure::with_backtrace` accepts a previously captured backtrace. Capture before leaving the failing operation: wrapping an error later cannot reconstruct its earlier stack. Logging the wrapper directly as `&dyn Error` preserves its causes; use `failure!` to include its captured traces as well.

### Composition

Configure a `Console` and compose its layer with other subscribers, or use it for scoped output:

```rust
use base_console::{Console, OutputFormat};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

let console = Console::default()
    .with_format(OutputFormat::Text)
    .with_ansi(false)
    .with_filter(EnvFilter::new("info,base_http=debug"));

let subscriber = tracing_subscriber::registry().with(console.layer());
tracing::subscriber::with_default(subscriber, || {
    tracing::info!("Scoped output");
});
```

See the [API documentation](https://docs.rs/base-console) for writers, filters, asynchronous scopes, and the JSON output contract.

## Releasing

Prepare a release with `cargo bake cargo:version:patch` (or `minor`, `major`, or `bump --version X.Y.Z`), commit it with a `Release: vVERSION` trailer, then run `cargo bake cargo:release` and open a pull request. After review and merge, GitHub Actions publishes the release when the configured `crates-io` environment approves it, then creates or updates the matching GitHub Release from `releases.md`. Follow the shared [Releasing skill](https://github.com/base-rs/base-project/blob/main/context/releasing.md) for the standard process.

## Releases

<!-- bake-readme:releases:start -->

See [releases.md](releases.md) for the full release history.

### v0.1.0

- Publish as `base-console` in the `base-rs` organization, with package versions starting at 0.1.0.
- Choose terminal text or newline-delimited JSON automatically for `tracing` events.
- Preserve typed fields, span context, and error causes through composable or scoped subscribers.
- Capture failures with optional OS backtraces and logical span traces for later reporting.
- Follow Rust logging conventions with panicking `init` and fallible `try_init` functions and builder methods.

<!-- bake-readme:releases:end -->

## See Also

- [`tracing`](https://docs.rs/tracing) provides structured events and spans.
- [`tracing-subscriber`](https://docs.rs/tracing-subscriber) provides subscriber composition and filtering.

## Contributing

Please open an issue or pull request on [GitHub](https://github.com/base-rs/base-console).
