# web-workers

Drop-in replacement for `std::thread` (and a focused set of `std::sync` primitives) on **Wasm in browsers**, built on Web Workers / worklets.

[![Crates.io](https://img.shields.io/crates/v/web-workers)](https://crates.io/crates/web-workers)
[![Docs](https://docs.rs/web-workers/badge.svg)](https://docs.rs/web-workers)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

```toml
[dependencies]
web-workers = "0.2"
```

On native targets, thread APIs re-export `std::thread`. On `wasm32-unknown-unknown`, they are implemented with browser workers (and optional atomics / SharedArrayBuffer).

## What you get

### Thread API (Wasm)
- `spawn`, `Builder`, `JoinHandle`, `scope` / `ScopedJoinHandle`
- `current`, `Thread`, `ThreadId`, `available_parallelism`
- `park` / `park_timeout`, `sleep`, `yield_now`
- Same general shape as `std::thread` so existing code can target the web with fewer `#cfg`s

### Web extensions (`web_workers::web`)
- **Capability probes:** `has_spawn_support()`, `has_block_support()`
- **Async joining:** `JoinHandleExt::join_async`, `ScopedJoinHandleExt::join_async`
- **Async scope / spawn:** `scope_async`, `spawn_async`, optional `spawn_with_message`
- **Event-loop yield:** `yield_now_async(YieldTime)` (e.g. user-blocking vs default)
- **Builder / Scope extensions** for web-specific options

### Sync (`web_workers::sync`)
Cross-platform primitives that adapt locking strategy per platform:

- Mutex, RwLock, Condvar  
- mpsc-style channels  
- Spinlock  

Usable from native and Wasm so shared libraries don’t need a second concurrency stack.

### Optional features

| Feature | Purpose |
|---------|---------|
| `audio-worklet` | Register threads on `BaseAudioContext`, `ExtendAudioWorkletProcessor`, `audio_worklet_node`, etc. |
| `message` | Send **structured-clone / transferable** values (`MessageSend`, `TransferableWrapper`, …) across threads and into worklets |

See the [audio worklet example](examples/audio_worklet.rs) for a full volume/piano demo using atomics + worklet processors.

## Requirements & notes

- **Modules target** — fails to import with the `no-modules` wasm-bindgen target.
- **Atomics / multi-thread spawn** — need `+atomics` (and typically `-Zbuild-std` for full std atomics) **and** [cross-origin isolation](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer#security_requirements) (`SharedArrayBuffer`). Deploy with COOP+COEP, or Chromium’s Document-Isolation-Policy where available.
- **Blocking** — not supported on the window (main) thread, service workers, or worklets; dedicated workers (and shared workers on Chromium) can block. Prefer `join_async` / `scope_async` when unsure.
- **Spawn location** — spawning is driven from the library’s notion of the “main” thread; if that thread is blocked, spawn may stall (browser-dependent).
- **Threads must be spawned by this crate** — calling thread APIs from foreign worker contexts can break assumptions.
- **Audio worklets** — very constrained (avoid allocation in `process`); some browsers lack `TextEncoder`/`TextDecoder` in worklets (example ships a small polyfill).
- **Exception-handling proposal** — explicitly unsupported (`compile_error` if enabled).

Capability helpers and async join paths are there so apps can degrade gracefully when isolation or blocking isn’t available.

## Quick usage

```rust
use web_workers::web::JoinHandleExt;

// Same as std::thread::spawn on native; workers on Wasm (when supported).
let mut handle = web_workers::spawn(|| {
    // work...
    42
});

// Prefer async join on the main thread / non-blocking contexts
let value = handle.join_async().await.unwrap();
```

```rust
use web_workers::web::{self, YieldTime};

if web::has_spawn_support() {
    web_workers::spawn(|| heavy_work());
} else {
    // fallback: local async or single-thread path
}

web::yield_now_async(YieldTime::UserBlocking).await;
```

Audio worklet (feature `audio-worklet`):

```rust
use web_sys::AudioContext;
use web_workers::web::audio_worklet::{
    AudioWorkletGlobalScopeExt, BaseAudioContextExt, ExtendAudioWorkletProcessor,
};

let ctx = AudioContext::new().unwrap();
ctx.clone()
    .register_thread(None, || {
        let global: web_sys::AudioWorkletGlobalScope = js_sys::global().unchecked_into();
        global.register_processor_ext::<MyProcessor>("my-processor").unwrap();
    })
    .await
    .unwrap();

// after registration + yield...
let node = ctx.audio_worklet_node::<MyProcessor>("my-processor", my_data, None)?;
```

## Browser caveats (summary)

Documented in crate docs: incomplete support for blocking in shared workers, spawn-then-block quirks, worklet shutdown leaks unless `AudioWorkletHandle::release` is used carefully, headless audio device issues, transferring `WebAssembly.Module` into some worklet ports, older Firefox module service workers, etc. Check [docs.rs](https://docs.rs/web-workers) and the module-level notes in `src/lib.rs` for linked bug trackers.

## Worker bootstrap contract

1. `initSync` on a worker re-runs `#[wasm_bindgen(start)]` hooks. Start hooks **must** no-op on worker threads (e.g. check `web_sys::window().is_some()` or `web_workers::web::is_main_thread()` after the first call on the page thread), or the worker never reaches its `__web_workers_worker_entry` body.
2. Never `spawn(..).join()` / `Atomics.wait` on the page thread in the same turn as spawning workers; Chrome will not start the Worker (see bug links above). Prefer an async ready barrier (e.g. await pool entrance before parallel work).
3. After renaming `#[wasm_bindgen]` exports, rebuild `src/thread/atomics/script/*.min.js` from the `.ts` sources in the same commit. A rename-only Rust change with stale min.js ships a hard worker boot failure. The CI check below catches this.

```bash
# fail if min.js entry name != Rust export name
rg -n '__web_workers_worker_entry' src/thread/atomics/script/worker.min.js
rg -n 'fn __web_workers_worker_entry' src/thread/atomics/spawn/mod.rs
```

Worker load/runtime failures are logged as `[web-workers] worker failed to start:` and the worker is terminated instead of leaking; post-message failures propagate as `io::Error` from `Builder::spawn`.

## Development

```bash
# see notes.md / justfile
just ci                 # fmt, clippy, build, docs, native tests, audit
just test-wasm-chrome   # browser tests (chromedriver)
just test-wasm-atomics-chrome
```

MSRV: **1.85**. Edition 2024.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
