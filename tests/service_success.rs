// Older Firefox doesn't support module service workers (fixed in Firefox 147+).
// See <https://bugzilla.mozilla.org/show_bug.cgi?id=1360870>.
//
#![cfg(target_family = "wasm")]

#[cfg(all(target_family = "wasm", wasm_bindgen_unstable_test_coverage))]
use minicov as _;

mod basic_success;
mod basic_success_async;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_service_worker);
