#![cfg(target_family = "wasm")]

#[cfg(all(target_family = "wasm", wasm_bindgen_unstable_test_coverage))]
use minicov as _;

mod basic_fail;
mod basic_fail_async;
#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
mod supported_spawn_fail;
#[cfg(any(not(target_feature = "atomics"), unsupported_spawn))]
mod unsupported_spawn;
mod util;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);
