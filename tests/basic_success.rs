#[cfg(target_family = "wasm")]
use minicov as _;

#[cfg(target_family = "wasm")]
use {wasm_bindgen_test::wasm_bindgen_test, web_workers::web};

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn available_parallelism() {
	web_workers::available_parallelism().unwrap();
}

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn thread() {
	let thread = web_workers::current();
	let _ = thread.id();
	let _ = thread.name();
}

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn panicking() {
	assert!(!web_workers::panicking());
}

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn scope() {
	let mut test = 0;

	web_workers::scope(|_| test = 1);

	assert_eq!(test, 1);
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
async fn scope_async() {
	let mut test = 0;

	web::scope_async(|_| async { test = 1 }).await;

	assert_eq!(test, 1);
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
async fn scope_async_wait() {
	let mut test = 0;

	let _future = web::scope_async(|_| async { test = 1 }).into_wait().await;

	assert_eq!(test, 1);
}
