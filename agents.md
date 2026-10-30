# Agent instructions

Run `cargo bake agent:context:install` to install dependency context and skills, and use `cargo bake agent:context:list` and `cargo bake agent:context:show` to discover guidance. Follow the Base conventions and relevant installed skills under `.agents/`.

Before planning or changing behavior, discover and read applicable `design/*.md`. Use `base-design-review` to assess plans and changes, and `base-design-author` when user requirements are introduced, changed, or removed. Review design compliance before opening or updating a pull request.

Keep runtime dependencies independent of concrete schedulers. Libraries use ordinary `tracing` events; application setup, formatting, and captured failure diagnostics belong here. Development tooling belongs in the private `bake/` package. CI requires complete source-region coverage across all targets: run `cargo bake test:coverage --all-targets true`.
