# surgeist-window

Window, event, and platform host contracts for Surgeist native application surfaces.

## Model

`surgeist-window` separates app-authored window requests, normalized host command
plans, observed runtime snapshots, and backend capabilities. The fake test host
and the native `winit` runner share command planning and state transition
helpers so tests exercise the same semantics as production paths.

## API Artifact

The committed API coordination artifact lives at `api/public-api.txt`.

Refresh it explicitly with:

```sh
cargo run --manifest-path api/generator/Cargo.toml
```

API refresh tooling is command-only and must not run as part of normal `cargo test`.
