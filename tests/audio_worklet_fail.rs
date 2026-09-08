#[cfg(all(target_family = "wasm", wasm_bindgen_unstable_test_coverage))]
use minicov as _;
use wasm_bindgen_test::wasm_bindgen_test;
#[cfg(not(unsupported_headless_audiocontext))]
use web_sys::AudioContext;
use web_sys::OfflineAudioContext;
use web_workers::web::audio_worklet::BaseAudioContextExt;

use super::test_processor::TestProcessor;

#[cfg(not(unsupported_headless_audiocontext))]
#[wasm_bindgen_test]
#[should_panic = "`register_thread()` has to be called on this context first"]
fn node() {
	AudioContext::new()
		.unwrap()
		.audio_worklet_node::<TestProcessor>("test", Box::new(|_| None), None)
		.unwrap();
}

#[wasm_bindgen_test]
#[should_panic = "`register_thread()` has to be called on this context first"]
fn offline_node() {
	OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
		.unwrap()
		.audio_worklet_node::<TestProcessor>("test", Box::new(|_| None), None)
		.unwrap();
}
