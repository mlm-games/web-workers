//! Timestamped cache value.
//!
//! [`TtlValue`] wraps data with its fetch timestamp so caches can tell stale
//! from fresh without extra bookkeeping. Missing timestamps deserialize to
//! expired, so values persisted before expiry was added are treated as stale.

use serde::{Deserialize, Serialize};

/// Value that expires [`TtlValue::STALE_THRESHOLD`] after fetching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtlValue<T> {
	/// The cached data.
	#[serde(flatten)]
	data: T,
	/// Last fetch time, milliseconds since the Unix epoch. [`None`] never
	/// expires; a missing field deserializes to expired.
	#[serde(default = "default_timestamp")]
	last_fetch_ts: Option<f64>,
}

impl<T> TtlValue<T> {
	/// Staleness threshold in milliseconds (1 day).
	pub const STALE_THRESHOLD: f64 = 1000.0 * 60.0 * 60.0 * 24.0;

	/// Wraps `data`, stamped now.
	#[must_use]
	pub fn new(data: T) -> Self {
		Self { data, last_fetch_ts: Some(now_timestamp_ms()) }
	}

	/// Wraps `data` so it never expires.
	#[must_use]
	pub const fn without_expiry(data: T) -> Self {
		Self { data, last_fetch_ts: None }
	}

	/// Borrows the data as `TtlValue<&T>`.
	#[must_use]
	pub const fn as_ref(&self) -> TtlValue<&T> {
		TtlValue { data: &self.data, last_fetch_ts: self.last_fetch_ts }
	}

	/// Transforms the data, keeping the timestamp.
	#[must_use]
	pub fn map<Output, Map>(self, map: Map) -> TtlValue<Output>
	where
		Map: FnOnce(T) -> Output,
	{
		TtlValue { data: map(self.data), last_fetch_ts: self.last_fetch_ts }
	}

	/// Returns [`true`] if the value is stale.
	#[must_use]
	pub fn has_expired(&self) -> bool {
		self.last_fetch_ts.is_some_and(|ts| now_timestamp_ms() - ts >= Self::STALE_THRESHOLD)
	}

	/// Marks the value expired.
	pub const fn expire(&mut self) {
		self.last_fetch_ts = Some(0.0);
	}

	/// Borrows the data.
	#[must_use]
	pub const fn data(&self) -> &T {
		&self.data
	}

	/// Unwraps the data.
	#[must_use]
	pub fn into_data(self) -> T {
		self.data
	}
}

/// Current time as milliseconds since the Unix epoch.
fn now_timestamp_ms() -> f64 {
	web_time::SystemTime::now()
		.duration_since(web_time::SystemTime::UNIX_EPOCH)
		.expect("system clock was before 1970")
		.as_secs_f64()
		* 1000.0
}

/// Default timestamp for missing fields: expired by construction.
#[expect(
	clippy::unnecessary_wraps,
	clippy::missing_const_for_fn,
	reason = "serde default fn must return Option<f64> and cannot be const"
)]
fn default_timestamp() -> Option<f64> {
	Some(0.0)
}

#[cfg(test)]
mod tests {
	use serde::Deserialize;

	use super::TtlValue;

	#[derive(Deserialize)]
	struct Empty;

	#[test]
	fn expiry_states() {
		assert!(!TtlValue::new(()).has_expired());
		assert!(!TtlValue::without_expiry(()).has_expired());

		let mut value = TtlValue::new(());
		value.expire();
		assert!(value.has_expired());
	}

	#[test]
	fn missing_timestamp_deserializes_to_expired() {
		let value: TtlValue<Empty> = serde_json::from_str("{}").unwrap();
		assert!(value.has_expired());
	}
}
