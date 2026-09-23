//! Cross-platform async sleep and timeout as futures.
//!
//! Blocking [`crate::sleep`] parks the thread, which is forbidden on the wasm
//! main thread. These await without blocking: backed by `tokio::time` on
//! native and `gloo_timers::future::TimeoutFuture` on wasm.

use std::{error::Error, fmt, future::Future, time::Duration};

#[cfg(not(target_family = "wasm"))]
use tokio::time::{sleep as tokio_sleep, timeout as tokio_timeout};
#[cfg(target_family = "wasm")]
use futures_util::future::{Either, select};
#[cfg(target_family = "wasm")]
use gloo_timers::future::TimeoutFuture;

/// Sleeps for `duration` without blocking the thread.
pub async fn sleep(duration: Duration) {
	#[cfg(not(target_family = "wasm"))]
	tokio_sleep(duration).await;

	#[cfg(target_family = "wasm")]
	TimeoutFuture::new(duration.as_millis().try_into().unwrap_or(u32::MAX)).await;
}

/// Error returned when [`timeout`] elapses before the future completes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElapsedError;

impl fmt::Display for ElapsedError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "time waiting for future has elapsed!")
	}
}

impl Error for ElapsedError {}

/// Waits for `future` to complete, returning [`ElapsedError`] if `duration`
/// elapses first.
///
/// # Errors
///
/// Returns [`ElapsedError`] if `duration` elapses before `future` completes.
pub async fn timeout<F, T>(future: F, duration: Duration) -> Result<T, ElapsedError>
where
	F: Future<Output = T>,
{
	#[cfg(not(target_family = "wasm"))]
	return tokio_timeout(duration, future).await.map_err(|_| ElapsedError);

	#[cfg(target_family = "wasm")]
	{
		let timeout_future =
			TimeoutFuture::new(duration.as_millis().try_into().expect("overlong duration"));

		match select(core::pin::pin!(future), timeout_future).await {
			Either::Left((result, _)) => Ok(result),
			Either::Right((_, _)) => Err(ElapsedError),
		}
	}
}
