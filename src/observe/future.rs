//! Futures, streams and iterators that carry an observation.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use pin_project_lite::pin_project;
use serde_json::Value;

use super::{context, EndReason, Observation};

/// Records a future's output on its observation (`#[observe]` passes a closure; manual
/// instrumentation records nothing).
#[doc(hidden)]
pub trait Recorder<T> {
    /// Record `output` on `observation`.
    fn record(self, observation: &Observation, output: &T);
}

/// Records nothing.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NoRecord;

impl<T> Recorder<T> for NoRecord {
    fn record(self, _: &Observation, _: &T) {}
}

/// Records with a closure.
#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct RecordFn<F>(pub F);

impl<T, F: FnOnce(&Observation, &T)> Recorder<T> for RecordFn<F> {
    fn record(self, observation: &Observation, output: &T) {
        (self.0)(observation, output);
    }
}

/// Marks the observation as panicked when a poll unwinds.
struct PanicGuard<'a>(Option<&'a Observation>);

impl Drop for PanicGuard<'_> {
    fn drop(&mut self) {
        if let Some(observation) = self.0 {
            if std::thread::panicking() {
                observation.mark_panicked();
            }
        }
    }
}

pin_project! {
    /// A future that runs with an observation current on every poll, and ends it when it
    /// completes, panics, or is dropped before it completed (cancelled).
    ///
    /// Made by [`Observation::instrument`] and by `#[observe]` on `async fn`.
    #[must_use = "futures do nothing unless polled"]
    pub struct Instrumented<F, R = NoRecord> {
        #[pin]
        inner: F,
        observation: Option<Observation>,
        recorder: Option<R>,
        end_on_ready: bool,
        done: bool,
    }

    impl<F, R> PinnedDrop for Instrumented<F, R> {
        fn drop(this: Pin<&mut Self>) {
            let this = this.project();
            if !*this.done {
                if let Some(observation) = this.observation.as_ref() {
                    if std::thread::panicking() {
                        observation.mark_panicked();
                    }
                    observation.end_with(EndReason::Cancelled);
                }
            }
        }
    }
}

impl<F, R> Instrumented<F, R> {
    pub(crate) fn new(
        observation: Option<Observation>,
        inner: F,
        recorder: R,
        end_on_ready: bool,
    ) -> Self {
        Self {
            inner,
            observation,
            recorder: Some(recorder),
            end_on_ready,
            done: false,
        }
    }
}

impl<F, R> std::fmt::Debug for Instrumented<F, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Instrumented")
            .field("observation", &self.observation)
            .finish_non_exhaustive()
    }
}

impl<F: Future, R: Recorder<F::Output>> Future for Instrumented<F, R> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let this = self.project();
        let _enter = this.observation.as_ref().map(context::enter);
        let _panic = PanicGuard(this.observation.as_ref());
        match this.inner.poll(cx) {
            Poll::Ready(output) => {
                *this.done = true;
                if let Some(observation) = this.observation.as_ref() {
                    if let Some(recorder) = this.recorder.take() {
                        recorder.record(observation, &output);
                    }
                    if *this.end_on_ready {
                        observation.end_with(EndReason::Completed);
                    }
                }
                Poll::Ready(output)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

pin_project! {
    /// A future that runs with an observation current on every poll, without ending it
    /// ([`ObserveFutureExt::in_current_observation`], [`ObserveFutureExt::with_observation`]).
    #[must_use = "futures do nothing unless polled"]
    #[derive(Debug)]
    pub struct WithObservation<F> {
        #[pin]
        inner: F,
        observation: Option<Observation>,
    }
}

impl<F: Future> Future for WithObservation<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let this = self.project();
        let _enter = this.observation.as_ref().map(context::enter);
        this.inner.poll(cx)
    }
}

/// Carry observations into futures (for example spawned tasks).
pub trait ObserveFutureExt: Future + Sized {
    /// Run this future with the observation that is current now (if any) as its current
    /// observation, for example before `tokio::spawn` or `spawn_local`, so that observations
    /// started in the task are children of it.
    ///
    /// ```
    /// use langfuse_ergonomic::observe::ObserveFutureExt;
    ///
    /// # async fn example() {
    /// let task = async { /* #[observe] functions called here nest under the caller */ };
    /// let task = task.in_current_observation();
    /// # task.await;
    /// # }
    /// ```
    fn in_current_observation(self) -> WithObservation<Self> {
        WithObservation {
            inner: self,
            observation: context::top(),
        }
    }

    /// Run this future with `observation` as its current observation (it is not ended).
    fn with_observation(self, observation: Observation) -> WithObservation<Self> {
        WithObservation {
            inner: self,
            observation: Some(observation),
        }
    }
}

impl<F: Future> ObserveFutureExt for F {}

/// What `#[observe(stream)]` records per item: its value and, for an `Err`, a message.
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct ItemCapture {
    /// The captured item.
    pub value: Option<Value>,
    /// The error message of an `Err` item.
    pub error: Option<String>,
}

/// The items seen so far, and the end of the observation.
struct ItemLog {
    items: Vec<Value>,
    capture: bool,
    first: bool,
}

impl ItemLog {
    fn new(capture: bool) -> Self {
        Self {
            items: Vec::new(),
            capture,
            first: true,
        }
    }

    fn push(&mut self, observation: &Observation, capture: ItemCapture) {
        if self.first {
            self.first = false;
            if observation.kind() == Some(super::ObservationType::Generation) {
                observation.set_completion_start_time(crate::time::now_utc());
            }
        }
        if let Some(error) = capture.error {
            observation.set_error(error);
        }
        if self.capture {
            if let Some(value) = capture.value {
                self.items.push(value);
            }
        }
    }

    fn finish(&mut self, observation: &Observation, reason: EndReason) {
        if self.capture {
            observation.set_output_value(Some(Value::Array(std::mem::take(&mut self.items))));
        }
        observation.end_with(reason);
    }
}

pin_project! {
    /// A stream whose observation stays open until it ends (or is dropped: level `WARNING`,
    /// `cancelled`), recording its items as the output. Made by `#[observe(stream)]`.
    #[must_use = "streams do nothing unless polled"]
    pub struct ObservedStream<S, C> {
        #[pin]
        inner: S,
        observation: Option<Observation>,
        capture: C,
        log: ItemLog,
        done: bool,
    }

    impl<S, C> PinnedDrop for ObservedStream<S, C> {
        fn drop(this: Pin<&mut Self>) {
            let this = this.project();
            if !*this.done {
                if let Some(observation) = this.observation.as_ref() {
                    if std::thread::panicking() {
                        observation.mark_panicked();
                    }
                    this.log.finish(observation, EndReason::Cancelled);
                }
            }
        }
    }
}

impl<S, C> ObservedStream<S, C> {
    #[doc(hidden)]
    pub fn new(
        observation: Option<Observation>,
        inner: S,
        capture: C,
        capture_output: bool,
    ) -> Self {
        Self {
            inner,
            observation,
            capture,
            log: ItemLog::new(capture_output),
            done: false,
        }
    }
}

impl<S, C> std::fmt::Debug for ObservedStream<S, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObservedStream")
            .field("observation", &self.observation)
            .finish_non_exhaustive()
    }
}

impl<S, C> Stream for ObservedStream<S, C>
where
    S: Stream,
    C: FnMut(&S::Item) -> ItemCapture,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        let this = self.project();
        if *this.done {
            return Poll::Ready(None);
        }
        let _enter = this.observation.as_ref().map(context::enter);
        let _panic = PanicGuard(this.observation.as_ref());
        match this.inner.poll_next(cx) {
            Poll::Ready(Some(item)) => {
                if let Some(observation) = this.observation.as_ref() {
                    this.log.push(observation, (this.capture)(&item));
                }
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) => {
                *this.done = true;
                if let Some(observation) = this.observation.as_ref() {
                    this.log.finish(observation, EndReason::Completed);
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// An iterator whose observation stays open until it is exhausted (or dropped: level
/// `WARNING`, `cancelled`), recording its items as the output. Made by
/// `#[observe(stream = "iterator")]`.
pub struct ObservedIter<I, C> {
    inner: I,
    observation: Option<Observation>,
    capture: C,
    log: ItemLog,
    done: bool,
}

impl<I, C> ObservedIter<I, C> {
    #[doc(hidden)]
    pub fn new(
        observation: Option<Observation>,
        inner: I,
        capture: C,
        capture_output: bool,
    ) -> Self {
        Self {
            inner,
            observation,
            capture,
            log: ItemLog::new(capture_output),
            done: false,
        }
    }
}

impl<I, C> std::fmt::Debug for ObservedIter<I, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObservedIter")
            .field("observation", &self.observation)
            .finish_non_exhaustive()
    }
}

impl<I, C> Iterator for ObservedIter<I, C>
where
    I: Iterator,
    C: FnMut(&I::Item) -> ItemCapture,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        if self.done {
            return None;
        }
        let _enter = self.observation.as_ref().map(context::enter);
        let _panic = PanicGuard(self.observation.as_ref());
        match self.inner.next() {
            Some(item) => {
                if let Some(observation) = self.observation.as_ref() {
                    self.log.push(observation, (self.capture)(&item));
                }
                Some(item)
            }
            None => {
                self.done = true;
                if let Some(observation) = self.observation.as_ref() {
                    self.log.finish(observation, EndReason::Completed);
                }
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I, C> Drop for ObservedIter<I, C> {
    fn drop(&mut self) {
        if !self.done {
            if let Some(observation) = self.observation.as_ref() {
                if std::thread::panicking() {
                    observation.mark_panicked();
                }
                self.log.finish(observation, EndReason::Cancelled);
            }
        }
    }
}
