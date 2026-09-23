//! Failure backoff cache.
//!
//! [`FailuresCache`] suppresses retries of operations that fail repeatedly:
//! each re-insert extends the entry TTL with exponential backoff, clamped to
//! a maximum delay. Entries stay until explicitly removed or expired.

use std::{borrow::Borrow, collections::HashMap, hash::Hash, sync::Arc, time::Duration};

use web_time::Instant;

use crate::sync::rwlock::RwLock;

/// Longest TTL any [`FailuresCache`] entry may hold.
const MAX_DELAY_SECS: u64 = 15 * 60;
/// Base seconds multiplied by `2 ^ failure_count` for backoff.
const MULTIPLIER: u64 = 15;

/// TTL cache where re-inserted items back off exponentially instead of being
/// discarded.
#[derive(Clone, Debug)]
pub struct FailuresCache<T: Eq + Hash> {
	/// Shared backoff state.
	inner: Arc<InnerCache<T>>,
}

/// Backoff schedule configuration.
#[derive(Debug)]
struct InnerCache<T: Eq + Hash> {
	/// Longest TTL any entry may hold.
	max_delay: Duration,
	/// Base seconds multiplied by `2 ^ failure_count`.
	backoff_multiplier: u64,
	/// Failure records by key.
	items: RwLock<HashMap<T, FailuresItem>>,
}

impl<T: Eq + Hash> Default for InnerCache<T> {
	fn default() -> Self {
		Self {
			max_delay: Duration::from_secs(MAX_DELAY_SECS),
			backoff_multiplier: MULTIPLIER,
			items: RwLock::new(HashMap::new()),
		}
	}
}

/// One backoff-tracked entry.
#[derive(Debug, Clone, Copy)]
struct FailuresItem {
	/// When the current backoff window started.
	insertion_time: Instant,
	/// Length of the current backoff window.
	duration: Duration,
	/// Failures recorded since first insert.
	failure_count: u8,
}

impl FailuresItem {
	/// Returns [`true`] once the backoff window has passed.
	fn expired(&self) -> bool {
		self.insertion_time.elapsed() >= self.duration
	}

	/// Ends the backoff window immediately, keeping the failure count.
	const fn expire(&mut self) {
		self.duration = Duration::from_secs(0);
	}
}

impl<T> FailuresCache<T>
where
	T: Eq + Hash,
{
	/// Creates a cache with default backoff settings.
	#[must_use]
	pub fn new() -> Self {
		Self { inner: Arc::new(InnerCache::default()) }
	}

	/// Creates a cache with custom maximum delay and backoff multiplier.
	#[must_use]
	pub fn with_settings(max_delay: Duration, multiplier: u8) -> Self {
		Self {
			inner: Arc::new(InnerCache {
				max_delay,
				backoff_multiplier: u64::from(multiplier),
				items: RwLock::new(HashMap::new()),
			}),
		}
	}

	/// Returns [`true`] if `key` is present and not expired.
	pub fn contains<Q>(&self, key: &Q) -> bool
	where
		T: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.inner.items.lock_sync_read().get(key).is_some_and(|item| !item.expired())
	}

	/// Number of recorded failures for `key`, or [`None`] if absent.
	#[must_use]
	pub fn failure_count<Q>(&self, key: &Q) -> Option<u8>
	where
		T: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.inner.items.lock_sync_read().get(key).map(|item| item.failure_count)
	}

	/// Backoff delay for `failure_count`, clamped to `[1s, max_delay]`.
	fn calculate_delay(&self, failure_count: u8) -> Duration {
		let exponential_backoff = 2_u64.saturating_pow(u32::from(failure_count));
		let delay = exponential_backoff.saturating_mul(self.inner.backoff_multiplier);

		Duration::from_secs(delay).clamp(Duration::from_secs(1), self.inner.max_delay)
	}

	/// Records a failure for `item`, extending its TTL with backoff.
	pub fn insert(&self, item: T) {
		self.extend([item]);
	}

	/// Records failures for an iterator of items.
	pub fn extend<TItems>(&self, iterator: TItems)
	where
		TItems: IntoIterator<Item = T>,
	{
		let mut lock = self.inner.items.lock_sync_write();
		let now = Instant::now();

		for key in iterator {
			let failure_count = lock
				.get(&key)
				.map_or(0, |value| value.failure_count.saturating_add(1));
			let delay = self.calculate_delay(failure_count);
			lock.insert(key, FailuresItem { insertion_time: now, duration: delay, failure_count });
		}
	}

	/// Removes the given items from the cache.
	pub fn remove<'item, Q, I>(&'item self, iterator: I)
	where
		I: Iterator<Item = &'item Q> + 'item,
		T: Borrow<Q>,
		Q: Hash + Eq + 'item + ?Sized,
	{
		let mut lock = self.inner.items.lock_sync_write();

		for item in iterator {
			lock.remove(item);
		}
	}

	/// Marks an item retryable immediately, keeping its failure count.
	#[doc(hidden)]
	pub fn expire(&self, item: &T) {
		if let Some(entry) = self.inner.items.lock_sync_write().get_mut(item) {
			entry.expire();
		}
	}
}

impl<T: Eq + Hash> Default for FailuresCache<T> {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use core::iter::once;

	use super::FailuresCache;

	#[test]
	fn records_and_expires_entries() {
		let cache = FailuresCache::new();

		assert!(!cache.contains(&1));
		cache.extend(once(1_u8));
		assert!(cache.contains(&1));

		cache.inner.items.lock_sync_write().get_mut(&1).unwrap().duration = Duration::from_secs(0);
		assert!(!cache.contains(&1));

		cache.remove(once(&1_u8));
		assert!(cache.inner.items.lock_sync_read().get(&1).is_none());
	}

	#[test]
	fn backoff_sequence_is_clamped() {
		let cache: FailuresCache<u8> = FailuresCache::new();

		assert_eq!(cache.calculate_delay(0).as_secs(), 15);
		assert_eq!(cache.calculate_delay(1).as_secs(), 30);
		assert_eq!(cache.calculate_delay(2).as_secs(), 60);
		assert_eq!(cache.calculate_delay(3).as_secs(), 120);
		assert_eq!(cache.calculate_delay(4).as_secs(), 240);
		assert_eq!(cache.calculate_delay(5).as_secs(), 480);
		assert_eq!(cache.calculate_delay(6).as_secs(), 900);
		assert_eq!(cache.calculate_delay(7).as_secs(), 900);
	}
}
