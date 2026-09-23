//! Fixed-capacity ring buffer.
//!
//! [`RingBuffer`] reserves its capacity up front and never reallocates,
//! dropping the oldest items on overflow. The logical capacity is part of the
//! serialized form so it survives a round trip.

use std::{
	collections::{
		VecDeque,
		vec_deque::{Drain, Iter, IterMut},
	},
	num::NonZeroUsize,
	ops::RangeBounds,
};

use serde::{Deserialize, Deserializer, Serialize};

/// Capacity assumed for buffers written before capacity was serialized.
const LEGACY_DEFAULT_CAPACITY: NonZeroUsize = NonZeroUsize::new(10).unwrap();

/// Fixed-size ring buffer over a [`VecDeque`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RingBuffer<T> {
	/// Held items, front-first.
	#[serde(rename = "items")]
	inner: VecDeque<T>,
	/// Logical capacity, restored on deserialization.
	capacity: NonZeroUsize,
}

impl<T> RingBuffer<T> {
	/// Builds a buffer from parts, trimming overflow and reserving capacity.
	fn from_parts(mut inner: VecDeque<T>, capacity: NonZeroUsize) -> Self {
		let capacity_as_usize = capacity.get();

		for _ in capacity_as_usize..inner.len() {
			inner.pop_front();
		}

		if let Some(extra_space) = capacity_as_usize.checked_sub(inner.len()) {
			inner.reserve_exact(extra_space);
		}

		Self { inner, capacity }
	}

	/// Creates a buffer with the given capacity, reserving it up front.
	#[must_use]
	pub fn new(size: NonZeroUsize) -> Self {
		Self::from_parts(VecDeque::with_capacity(size.into()), size)
	}

	/// Number of items currently held.
	#[must_use]
	pub fn len(&self) -> usize {
		self.inner.len()
	}

	/// Returns [`true`] if no items are held.
	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}

	/// Item at `index`, front-first.
	#[must_use]
	pub fn get(&self, index: usize) -> Option<&T> {
		self.inner.get(index)
	}

	/// Pushes to the back, dropping from the front when full.
	pub fn push(&mut self, value: T) {
		if self.inner.len() == self.inner.capacity() {
			self.inner.pop_front();
		}

		self.inner.push_back(value);
	}

	/// Removes the front item.
	#[must_use]
	pub fn pop(&mut self) -> Option<T> {
		self.inner.pop_front()
	}

	/// Removes the item at `index`.
	#[must_use]
	pub fn remove(&mut self, index: usize) -> Option<T> {
		self.inner.remove(index)
	}

	/// Iterates front-to-back.
	///
	/// For `&RingBuffer` iteration, use [`.iter()`](Self::iter) directly;
	/// no `IntoIterator` impl by design (front-first order is explicit).
	#[must_use]
	pub fn iter(&self) -> Iter<'_, T> {
		self.inner.iter()
	}

	/// Iterates mutably front-to-back.
	///
	/// See [`.iter()`](Self::iter) on the `IntoIterator` choice.
	pub fn iter_mut(&mut self) -> IterMut<'_, T> {
		self.inner.iter_mut()
	}

	/// Drains `range`.
	pub fn drain<R>(&mut self, range: R) -> Drain<'_, T>
	where
		R: RangeBounds<usize>,
	{
		self.inner.drain(range)
	}

	/// Removes all items, keeping the capacity.
	pub fn clear(&mut self) {
		self.inner.clear();
	}

	/// Total number of items the buffer can hold.
	#[must_use]
	pub fn capacity(&self) -> usize {
		self.inner.capacity()
	}

	/// Keeps only items matching `predicate`.
	pub fn retain<F>(&mut self, predicate: F)
	where
		F: FnMut(&T) -> bool,
	{
		self.inner.retain(predicate);
	}
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for RingBuffer<T> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		#[serde(untagged)]
		enum SerializedRingBuffer<T> {
			WithCapacity { items: VecDeque<T>, capacity: NonZeroUsize },
			Legacy(VecDeque<T>),
		}

		match SerializedRingBuffer::deserialize(deserializer)? {
			SerializedRingBuffer::WithCapacity { items, capacity } => {
				Ok(Self::from_parts(items, capacity))
			}
			SerializedRingBuffer::Legacy(items) => {
				let capacity = NonZeroUsize::new(items.len().max(LEGACY_DEFAULT_CAPACITY.get()))
					.expect("legacy capacity is non-zero");
				Ok(Self::from_parts(items, capacity))
			}
		}
	}
}

impl<U> Extend<U> for RingBuffer<U> {
	fn extend<TItem: IntoIterator<Item = U>>(&mut self, iter: TItem) {
		for item in iter {
			self.push(item);
		}
	}
}

#[cfg(test)]
mod tests {
	use std::num::NonZeroUsize;

	use super::RingBuffer;

	#[test]
	fn drops_oldest_on_overflow() {
		let mut buffer = RingBuffer::new(NonZeroUsize::new(5).unwrap());

		for value in 1..=6 {
			buffer.push(value);
		}

		assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), vec![2, 3, 4, 5, 6]);
	}

	#[test]
	fn capacity_survives_serialization_round_trip() {
		let mut buffer = RingBuffer::new(NonZeroUsize::new(3).unwrap());
		buffer.push("1".to_owned());
		buffer.push("2".to_owned());

		let json = serde_json::to_string(&buffer).unwrap();
		assert_eq!(json, r#"{"items":["1","2"],"capacity":3}"#);

		let restored: RingBuffer<String> = serde_json::from_str(&json).unwrap();
		assert_eq!(buffer, restored);
		assert_eq!(restored.capacity(), 3);
	}

	#[test]
	fn legacy_sequence_format_gets_default_capacity() {
		let mut buffer: RingBuffer<i32> = serde_json::from_str("[1,2]").unwrap();

		assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), vec![1, 2]);
		assert_eq!(buffer.capacity(), 10);

		buffer.push(3);
		assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
	}
}
