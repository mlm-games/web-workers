#![cfg(target_family = "wasm")]

#[cfg(all(target_family = "wasm", wasm_bindgen_unstable_test_coverage))]
use minicov as _;

mod basic_fail;
mod basic_fail_async;
#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
mod supported_spawn_fail;
#[cfg(any(not(target_feature = "atomics"), unsupported_spawn))]
mod unsupported_spawn;
// Some browsers don't support blocking in shared workers.
// See <https://bugzilla.mozilla.org/show_bug.cgi?id=1359745>.
#[cfg(unsupported_shared_block)]
mod unsupported_block;
mod util;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_shared_worker);
