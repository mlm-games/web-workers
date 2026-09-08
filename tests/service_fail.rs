#![cfg(target_family = "wasm")]

#[cfg(all(target_family = "wasm", wasm_bindgen_unstable_test_coverage))]
use minicov as _;

mod basic_fail;
mod basic_fail_async;
// Service workers can neither spawn workers (`Worker` is unavailable, so
// `has_spawn_support()` is always `false`) nor block, so the `unsupported_*`
// panic paths always apply here, even with the atomics target feature and
// cross-origin isolation.
mod unsupported_block;
mod unsupported_spawn;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_service_worker);
