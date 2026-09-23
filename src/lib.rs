//! TODO:
//! - Re-factor builder implementation.
//! - Add `MessageSend` macro.
//! - Add `WorkletBuilder` (or use `Builder`).
//! - Wrap `MessageChannel` into something safe.
//! - Consider passing message into `Builder`.
//! - Add README.
//! - Consider moving some APIs into `web-workers-core/primitives`.
//!
//! Things to note:
//! - Will fail on import when used with the `no-modules` target.
//! - Blocking is not recommended, e.g. blocks events.
//! - Audio worklets are very limited, e.g. should not do any allocation.
//! - Spawning happens on the "main" thread, e.g. if blocked nothing will spawn
//!   (affects some browsers only).
//! - Calling any functions from a thread not spawned by `web-workers` will
//!   cause issues.
//! - Wasm threads/atomics are now broadly supported (Baseline 2025+), but still
//!   require cross-origin isolation for `SharedArrayBuffer`.
//! - `Atomics.waitAsync()` is broadly available (Baseline 2025) and works on
//!   the main thread for non-blocking waits.
//! - Deploying cross-origin isolation can use COOP+COEP headers, or
//!   `Document-Isolation-Policy` in Chromium 137+.
//!
//! Browser bugs:
//! - Browsers don't support `TextEncoder`/`TextDecoder` in audio worklets:
//!   - Chrome: ?
//!   - Firefox: <https://bugzilla.mozilla.org/show_bug.cgi?id=1826432>
//!   - Safari: ?
//! - Older Firefox doesn't support module service workers: <https://bugzilla.mozilla.org/show_bug.cgi?id=1360870>
//!   (fixed in Firefox 147+).
//! - Browsers don't support blocking in shared workers:
//!   - Firefox: <https://bugzilla.mozilla.org/show_bug.cgi?id=1359745>
//!   - Safari: ?
//! - Spec doesn't allow cross-origin isolation in shared and service workers: <https://github.com/w3c/ServiceWorker/pull/1545>.
//! - Browsers don't support spawning and blocking afterwards (e.g.
//!   `spawn(..).join()`):
//!   - Chrome:
//!     - Spawning: <https://issues.chromium.org/issues/40633395>
//!     - `postMessage()`: <https://issues.chromium.org/issues/40687798>
//!   - Safari: ?
//! - Browsers don't properly shutdown audio worklet when state is `closed`:
//!   - Chrome: <https://issues.chromium.org/issues/40072701>
//!   - Firefox: <https://bugzilla.mozilla.org/show_bug.cgi?id=1878516>
//!   - Safari: ?
//! - Headless browsers seem to have some issues when spawning too many threads:
//!   - Firefox: ?
//!   - Safari: ?
//! - Headless Firefox can't run `AudioContext` without a real audio device: <https://bugzilla.mozilla.org/show_bug.cgi?id=1881904>.
//! - Chrome fails to send `WebAssembly.Module` over `AudioWorkletNode.port`: <https://issues.chromium.org/issues/40855462>.

#![doc(test(attr(deny(unused, warnings))))]
#![cfg_attr(
	all(
		target_family = "wasm",
		target_os = "unknown",
		target_feature = "atomics"
	),
	feature(stdarch_wasm_atomic_wait)
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(target_family = "wasm", wasm_bindgen_unstable_test_coverage))]
use minicov as _;

/// Cross-platform sync primitives (Mutex, RwLock, Condvar, mpsc, Spinlock)
/// that work on native and WebAssembly, adapting locking strategy per platform.
pub mod sync;

/// Cross-platform async task handles ([`task::spawn`], [`task::JoinHandle`],
/// [`task::AbortOnDrop`]): tokio-backed on native, `spawn_local` on wasm.
#[cfg(any(not(target_family = "wasm"), all(target_family = "wasm", target_os = "unknown")))]
pub mod task;

/// Async [`time::sleep`] and [`time::timeout`] as non-blocking futures.
pub mod time;

/// Exponential-backoff failure cache ([`failures::FailuresCache`]).
pub mod failures;

/// Fixed-capacity ring buffer ([`ring::RingBuffer`]).
pub mod ring;

/// Timestamped cache value ([`ttl::TtlValue`]).
pub mod ttl;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
mod thread;
#[cfg(any(all(target_family = "wasm", target_os = "unknown"), docsrs))]
#[cfg_attr(docsrs, doc(cfg(Web)))]
pub mod web;

pub use std::thread::*;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
#[allow(deprecated)]
pub use self::thread::{
	Builder, JoinHandle, Scope, ScopedJoinHandle, Thread, ThreadId, available_parallelism, current,
	park, park_timeout, park_timeout_ms, scope, sleep, sleep_ms, spawn, yield_now,
};

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	target_feature = "exception-handling"
))]
compile_error!("this library does not work correctly with the exception handling proposal");

#[cfg(feature = "derive")]
pub use web_workers_derive::MessageSend;

/// Unified spawn for native, Android, and WASM.
pub fn spawn_unified<F>(f: F)
where
	F: FnOnce() + Send + 'static,
{
	#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
	{
		spawn(f);
	}
	#[cfg(all(target_family = "wasm", target_os = "unknown"))]
	{
		if crate::web::has_spawn_support() {
			crate::spawn(f);
		} else {
			wasm_bindgen_futures::spawn_local(async move { f() });
		}
	}
}

/// Async version of [`spawn_unified`].
pub fn spawn_async_unified<F1, F2, T>(f: F1)
where
	F1: FnOnce() -> F2 + Send + 'static,
	F2: Future<Output = T> + 'static,
	T: Send + 'static,
{
	#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
	{
		spawn(move || {
			let _ = pollster::block_on(f());
		});
	}
	#[cfg(all(target_family = "wasm", target_os = "unknown"))]
	{
		if crate::web::has_spawn_support() {
			use crate::web::BuilderExt;
			if let Err(e) = crate::Builder::new().spawn_async(f) {
				eprintln!("[web-workers] spawn_async failed: {e}");
			}
		} else {
			wasm_bindgen_futures::spawn_local(async move {
				let _ = f().await;
			});
		}
	}
}
