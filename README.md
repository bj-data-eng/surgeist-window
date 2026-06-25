# surgeist-window

Window, event, and platform host contracts for Surgeist native application surfaces.

## Model

`surgeist-window` separates app-authored window requests, normalized host command
plans, observed runtime snapshots, and backend capabilities. The fake test host
and the native `winit` runner share command planning and state transition
helpers so tests exercise the same semantics as production paths.

## Cross-Thread Proxy

`Context::proxy()` returns a cloneable handle for waking the native event loop
from external work. Move it to a worker thread to enqueue typed window commands
and draw or exit actions without sharing the handler context itself.

```rust
if let Some(proxy) = cx.proxy() {
    std::thread::spawn(move || {
        let _ = proxy.draw(window_id);
    });
}
```

## API Artifact

The committed API coordination artifact lives at `api/public-api.txt`.

Refresh it explicitly with:

```sh
cargo run --manifest-path api/generator/Cargo.toml
```

API refresh tooling is command-only and must not run as part of normal `cargo test`.
