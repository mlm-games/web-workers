#[cfg(all(target_family = "wasm", wasm_bindgen_unstable_test_coverage))]
use minicov as _;
use web_workers::web::{self, YieldTime};

#[wasm_bindgen_test::wasm_bindgen_test]
#[should_panic = "`YieldNowFuture` polled after completion"]
async fn yield_now() {
	let mut future = web::yield_now_async(YieldTime::UserBlocking);
	(&mut future).await;
	future.await;
}
