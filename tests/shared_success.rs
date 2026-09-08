#![cfg(target_family = "wasm")]

#[cfg(all(target_family = "wasm", wasm_bindgen_unstable_test_coverage))]
use minicov as _;

mod basic_success;
mod basic_success_async;
#[cfg(all(
	target_family = "wasm",
	target_feature = "atomics",
	feature = "message",
	not(unsupported_spawn)
))]
mod message_success;
// Some browsers don't support blocking in shared workers.
// See <https://bugzilla.mozilla.org/show_bug.cgi?id=1359745>.
#[cfg(not(unsupported_shared_block))]
mod supported_block;
#[cfg(all(
	target_family = "wasm",
	target_feature = "atomics",
	not(unsupported_spawn)
))]
mod supported_spawn_success;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_shared_worker);
