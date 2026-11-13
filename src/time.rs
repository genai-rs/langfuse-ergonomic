//! Clocks, timers and the background task: tokio's natively; on WebAssembly (`target_family = "wasm"`) the
//! JavaScript host's `setTimeout` (`gloo-timers`), `web-time`'s clocks (`std::time::Instant::now` and
//! `SystemTime::now` panic on `wasm32-unknown-unknown`) and `wasm_bindgen_futures::spawn_local`.
//!
//! The same approach as `timvw-rust-runtime`'s timer, whose types are crate-private.

use std::future::Future;
use std::time::Duration;

/// A monotonic instant: `performance.now()` on WebAssembly.
#[cfg(not(target_family = "wasm"))]
pub(crate) use std::time::Instant;
/// A monotonic instant: `performance.now()` on WebAssembly.
#[cfg(target_family = "wasm")]
pub(crate) use web_time::Instant;

/// The wall clock as a UTC date-time (`Date.now()` on WebAssembly).
pub(crate) fn now_utc() -> chrono::DateTime<chrono::Utc> {
    let since = since_epoch();
    #[allow(clippy::cast_possible_wrap)]
    chrono::DateTime::from_timestamp(since.as_secs() as i64, since.subsec_nanos())
        .unwrap_or_default()
}

/// The wall clock, as the time since the Unix epoch: `Date.now()` on WebAssembly.
pub(crate) fn since_epoch() -> Duration {
    #[cfg(not(target_family = "wasm"))]
    use std::time::{SystemTime, UNIX_EPOCH};
    #[cfg(target_family = "wasm")]
    use web_time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

/// Waits for `duration`.
#[cfg(not(target_family = "wasm"))]
pub(crate) async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

/// Waits for `duration`.
#[cfg(target_family = "wasm")]
pub(crate) async fn sleep(duration: Duration) {
    sleep_until(web_time::Instant::now() + duration).await;
}

/// Waits until `deadline`: at once when it is past.
///
/// `setTimeout` counts whole milliseconds and fires at once for a delay over 2^31 - 1 ms: round up, and wait in
/// steps for a longer one. A timer that fires early (clocks of different precision) is armed again.
#[cfg(target_family = "wasm")]
async fn sleep_until(deadline: web_time::Instant) {
    loop {
        let left = deadline.saturating_duration_since(web_time::Instant::now());
        if left.is_zero() {
            return;
        }
        gloo_timers::future::TimeoutFuture::new(timeout_millis(left)).await;
    }
}

/// A delay for `setTimeout`: whole milliseconds, rounded up, at most 2^31 - 1.
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
fn timeout_millis(delay: Duration) -> u32 {
    const MAX: u128 = i32::MAX as u128;
    #[allow(clippy::cast_possible_truncation)]
    let millis = delay.as_nanos().div_ceil(1_000_000).min(MAX) as u32;
    millis
}

/// `future`'s output, or `None` when it did not complete within `duration`.
#[cfg(not(target_family = "wasm"))]
pub(crate) async fn timeout<F: Future>(duration: Duration, future: F) -> Option<F::Output> {
    tokio::time::timeout(duration, future).await.ok()
}

/// `future`'s output, or `None` when it did not complete within `duration`.
#[cfg(target_family = "wasm")]
pub(crate) async fn timeout<F: Future>(duration: Duration, future: F) -> Option<F::Output> {
    tokio::select! {
        biased;
        output = future => Some(output),
        () = sleep(duration) => None,
    }
}

/// Ticks every `period`, the first tick at once; a tick that is missed (the receiver was busy) is skipped, not
/// made up for (tokio's `MissedTickBehavior::Skip`). `tick` is cancel-safe.
pub(crate) struct Interval {
    #[cfg(not(target_family = "wasm"))]
    inner: tokio::time::Interval,
    #[cfg(target_family = "wasm")]
    period: Duration,
    #[cfg(target_family = "wasm")]
    next: web_time::Instant,
}

impl Interval {
    /// An interval of `period` (which must not be zero), whose first tick completes at once.
    pub(crate) fn new(period: Duration) -> Self {
        #[cfg(not(target_family = "wasm"))]
        {
            let mut inner = tokio::time::interval(period);
            inner.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            Self { inner }
        }
        #[cfg(target_family = "wasm")]
        {
            Self {
                period,
                next: web_time::Instant::now(),
            }
        }
    }

    /// Waits for the next tick.
    pub(crate) async fn tick(&mut self) {
        #[cfg(not(target_family = "wasm"))]
        {
            self.inner.tick().await;
        }
        #[cfg(target_family = "wasm")]
        {
            // Advance only once the tick happened, so that a dropped `tick` future loses nothing.
            sleep_until(self.next).await;
            let behind = web_time::Instant::now().saturating_duration_since(self.next);
            self.next += next_tick_offset(behind, self.period);
        }
    }
}

/// When the tick after the one due at some instant `due` is, as an offset from `due`, at `behind` past `due`: one
/// `period` later, or, when that is past already, the first multiple of `period` after `behind` (missed ticks are
/// skipped).
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
fn next_tick_offset(behind: Duration, period: Duration) -> Duration {
    if behind < period || period.is_zero() {
        return period;
    }
    let periods = behind.as_nanos() / period.as_nanos() + 1;
    // Fits in a u32 for any realistic period: the clamp only guards a pathological one.
    period * u32::try_from(periods).unwrap_or(u32::MAX)
}

/// Runs `future` in the background.
///
/// Natively a tokio task (so a tokio runtime must be running); on WebAssembly a task of the JavaScript host's event
/// loop (`wasm_bindgen_futures::spawn_local`), where the future need not be `Send`.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn spawn<F>(future: F) -> TaskHandle
where
    F: Future<Output = ()> + Send + 'static,
{
    TaskHandle(tokio::spawn(future))
}

/// Runs `future` in the background.
///
/// Natively a tokio task (so a tokio runtime must be running); on WebAssembly a task of the JavaScript host's event
/// loop (`wasm_bindgen_futures::spawn_local`), where the future need not be `Send`.
#[cfg(target_family = "wasm")]
pub(crate) fn spawn<F>(future: F) -> TaskHandle
where
    F: Future<Output = ()> + 'static,
{
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        future.await;
        let _ = done_tx.send(());
    });
    TaskHandle(done_rx)
}

/// A task started with [`spawn`].
#[cfg(not(target_family = "wasm"))]
pub(crate) struct TaskHandle(tokio::task::JoinHandle<()>);

/// A task started with [`spawn`].
#[cfg(target_family = "wasm")]
pub(crate) struct TaskHandle(tokio::sync::oneshot::Receiver<()>);

impl TaskHandle {
    /// Waits for the task to finish; an error when it panicked (or, natively, was cancelled).
    pub(crate) async fn join(self) -> Result<(), String> {
        // On WebAssembly a panic aborts the module (panic=abort), so the sender is only dropped unsent when the
        // task itself was dropped.
        self.0.await.map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeouts_round_up_and_stay_in_range() {
        assert_eq!(timeout_millis(Duration::ZERO), 0);
        assert_eq!(timeout_millis(Duration::from_nanos(1)), 1);
        assert_eq!(timeout_millis(Duration::from_micros(1500)), 2);
        assert_eq!(timeout_millis(Duration::from_secs(2)), 2000);
        assert_eq!(
            timeout_millis(Duration::from_secs(100 * 86_400)),
            i32::MAX as u32
        );
    }

    #[test]
    fn missed_ticks_are_skipped() {
        let period = Duration::from_secs(5);
        assert_eq!(next_tick_offset(Duration::ZERO, period), period);
        assert_eq!(next_tick_offset(Duration::from_secs(4), period), period);
        assert_eq!(
            next_tick_offset(Duration::from_secs(5), period),
            Duration::from_secs(10)
        );
        assert_eq!(
            next_tick_offset(Duration::from_secs(12), period),
            Duration::from_secs(15)
        );
    }

    #[test]
    fn since_epoch_is_after_2020() {
        assert!(since_epoch() > Duration::from_secs(1_577_836_800));
    }
}
