# Releases

## v0.1.1

- Record integers wider than 64 bits as decimal strings by default, and add an optional `arbitrary-precision` feature that records them as JSON numbers.
- Stop enabling `serde_json`'s `arbitrary_precision` by default, because it changes number handling for every `serde_json` user in a build.

## v0.1.0

- Publish as `base-console` in the `base-rs` organization, with package versions starting at 0.1.0.
- Choose terminal text or newline-delimited JSON automatically for `tracing` events.
- Preserve typed fields, span context, and error causes through composable or scoped subscribers.
- Capture failures with optional OS backtraces and logical span traces for later reporting.
- Follow Rust logging conventions with panicking `init` and fallible `try_init` functions and builder methods.
