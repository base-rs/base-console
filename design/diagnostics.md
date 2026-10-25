# Console Diagnostics

## Requirement

Console consumes ordinary `tracing` events and provides automatic terminal text or newline-delimited JSON output. It must remain independent of concrete schedulers and runtimes. Applications choose when and where subscribers are installed; composable layers and scoped subscribers must work without process-wide initialization.

Global initialization follows Rust logging conventions: `init` panics on setup failure, while `try_init` returns an error. This applies to the convenience functions and configured console methods.

Primitive event and span fields retain their types in JSON. Typed error fields preserve the error message and ordered cause chain. Text output presents those causes and supplied diagnostic traces with indentation. A captured failure retains its diagnostic origin when the error is reported later. Diagnostics that were never captured cannot be recovered by the formatter.

## Rationale

The initial implementation follows the October 2026 design discussion: preserve Ruby Console's automatic output choice and rich failure reports while using `tracing` for event propagation and context. Libraries need no console-specific emission interface, and applications must remain free to select their subscriber and execution environment. Initialization follows the user's agreed Rust logging conventions so applications can choose convenient startup or explicit error handling.

## Scope

The public console configuration, output layer, failure helper, runtime dependencies, examples, and public integration tests.

## Validation

Exercise automatic selection with a redirected example process and a real terminal. Parse JSON records to check primitive types, updated span fields, cause order, and distinct event metadata. Check multiline terminal reports and capture-time failure diagnostics. Poll independently instrumented futures on different threads using isolated subscribers to check context propagation and isolation. Inspect dependencies for concrete runtimes and check layer composition with other subscribers.

Exercise successful initialization, invalid environment filters, and repeated global installation in isolated processes. Check that `init` panics and `try_init` returns errors for the failures each API can encounter.

