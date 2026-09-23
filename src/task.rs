//! Cross-platform async task helpers.
//!
//! The thread-level story lives in [`crate::thread`] / [`crate::web`]. This
//! module covers the future-level gap: spawning a future and awaiting it
//! later, on runtimes where `tokio` is unavailable or undesirable (wasm
//! without atomics, single-threaded targets).
//!
//! - [`spawn`] returns a [`JoinHandle`] that resolves to `Ok(T)` on success
//!   or `Err(JoinError)` on abort/panic.
//! - [`AbortOnDrop`] aborts the task when dropped.
//! - [`JoinHandleExt::abort_on_drop`] converts any handle.
//!
//! On non-wasm targets this is a tokio-backed handle; on wasm it is a
//! `wasm_bindgen_futures::spawn_local` task wrapped in
//! `futures_util::future::Abortable`, matching the interface of
//! `tokio::task`.

use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

#[cfg(not(target_family = "wasm"))]
mod sys {
	//! Re-exports of `tokio::task` on native targets.
	pub use tokio::task::{AbortHandle, JoinError, JoinHandle, spawn};
}

#[cfg(target_family = "wasm")]
mod sys {
	//! `spawn_local`-backed task handles for single-threaded wasm targets.
	use core::{
		future::Future,
		pin::Pin,
		task::{Context, Poll},
	};

	pub use futures_util::future::AbortHandle;
	use futures_util::{
		FutureExt,
		future::{Abortable, RemoteHandle},
	};

	/// Wasm counterpart to `tokio::task::JoinError` for single-threaded
	/// executors.
	#[derive(Debug)]
	pub enum JoinError {
		Cancelled,
		Panic,
	}

	impl JoinError {
		/// Returns [`true`] if the task was cancelled via [`JoinHandle::abort`].
		#[must_use]
		pub fn is_cancelled(&self) -> bool {
			matches!(self, JoinError::Cancelled)
		}
	}

	impl core::fmt::Display for JoinError {
		fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			match &self {
				JoinError::Cancelled => write!(fmt, "task was cancelled"),
				JoinError::Panic => write!(fmt, "task panicked"),
			}
		}
	}

	impl std::error::Error for JoinError {}

	/// Wasm counterpart to `tokio::task::JoinHandle`, backed by
	/// `wasm_bindgen_futures::spawn_local`.
	#[derive(Debug)]
	pub struct JoinHandle<T> {
		remote_handle: Option<RemoteHandle<T>>,
		abort_handle: AbortHandle,
	}

	impl<T> JoinHandle<T> {
		/// Prevents the spawned future from being polled again.
		pub fn abort(&self) {
			self.abort_handle.abort();
		}

		/// Handle usable to abort the spawned future.
		#[must_use]
		pub fn abort_handle(&self) -> AbortHandle {
			self.abort_handle.clone()
		}

		/// Returns [`true`] if the spawned future has been aborted.
		#[must_use]
		pub fn is_finished(&self) -> bool {
			self.abort_handle.is_aborted()
		}
	}

	impl<T> Drop for JoinHandle<T> {
		fn drop(&mut self) {
			if let Some(handle) = self.remote_handle.take() {
				handle.forget();
			}
		}
	}

	impl<T: 'static> Future for JoinHandle<T> {
		type Output = Result<T, JoinError>;

		fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
			if self.abort_handle.is_aborted() {
				Poll::Ready(Err(JoinError::Cancelled))
			} else if let Some(handle) = self.remote_handle.as_mut() {
				Pin::new(handle).poll(context).map(Ok)
			} else {
				Poll::Ready(Err(JoinError::Panic))
			}
		}
	}

	/// Wasm counterpart to `tokio::task::spawn`, via `spawn_local`.
	pub fn spawn<F, T>(future: F) -> JoinHandle<T>
	where
		F: Future<Output = T> + 'static,
	{
		let (future, remote_handle) = future.remote_handle();
		let (abort_handle, abort_registration) = AbortHandle::new_pair();
		let future = Abortable::new(future, abort_registration);

		wasm_bindgen_futures::spawn_local(async move {
			let _ = future.await;
		});

		JoinHandle { remote_handle: Some(remote_handle), abort_handle }
	}
}

pub use sys::*;

/// Aborts the wrapped task on drop.
#[derive(Debug)]
pub struct AbortOnDrop<T>(JoinHandle<T>);

impl<T> AbortOnDrop<T> {
	/// Wraps a [`JoinHandle`] so dropping it aborts the task.
	#[must_use]
	pub const fn new(join_handle: JoinHandle<T>) -> Self {
		Self(join_handle)
	}
}

impl<T> Drop for AbortOnDrop<T> {
	fn drop(&mut self) {
		self.0.abort();
	}
}

impl<T: 'static> Future for AbortOnDrop<T> {
	type Output = Result<T, JoinError>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.0).poll(cx)
	}
}

/// Creates an [`AbortOnDrop`] from a [`JoinHandle`].
pub trait JoinHandleExt<T> {
	/// Aborts the task when the returned handle is dropped.
	fn abort_on_drop(self) -> AbortOnDrop<T>;
}

impl<T> JoinHandleExt<T> for JoinHandle<T> {
	fn abort_on_drop(self) -> AbortOnDrop<T> {
		AbortOnDrop::new(self)
	}
}
