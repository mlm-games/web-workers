#![cfg(all(
	target_family = "wasm",
	target_feature = "atomics",
	not(unsupported_spawn)
))]

#[cfg(target_family = "wasm")]
use minicov as _;

mod supported_spawn_corrupt;
mod util;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);
