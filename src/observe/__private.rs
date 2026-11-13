//! Support code for the `#[observe]` expansion. Not public API: it may change in any release.
//!
//! The capture traits use autoref specialisation: `(&&&Capture(&value)).__lf_capture()` picks the
//! `Serialize` impl when the type has one, else `Debug`, else nothing. Method resolution happens
//! where the macro expands, with the concrete types, so users need no `serde` of their own.

#![allow(missing_docs, missing_debug_implementations)]

use std::fmt::{Debug, Display};
use std::future::Future;

use serde::Serialize;
use serde_json::Value;

pub use super::future::{ItemCapture, NoRecord, RecordFn, Recorder};
use super::EndReason;
use super::{Instrumented, Observation, ObservedIter, ObservedStream};
pub use super::{ObservationType, ScopeGuard};
pub use langfuse_client_base::models::ObservationLevel;

/// What the attribute fixed at compile time.
#[derive(Debug, Clone, Copy)]
pub struct Meta {
    pub name: &'static str,
    pub kind: ObservationType,
    pub level: Option<ObservationLevel>,
    pub capture_input: bool,
    pub capture_output: bool,
    pub capture_err: bool,
}

/// Start the observation of a call: `None` without a sink.
pub fn start(meta: &Meta) -> Option<Observation> {
    let observation = Observation::start(meta.name.to_owned(), meta.kind)?;
    if let Some(level) = meta.level {
        observation.set_level(level);
    }
    Some(observation)
}

/// Whether `observation` records (so the expansion only evaluates captures when it does).
pub fn recording(observation: &Option<Observation>) -> Option<&Observation> {
    observation.as_ref().filter(|o| o.is_recording())
}

/// Record the captured arguments as the input: an object keyed by argument name.
pub fn set_input(observation: &Observation, args: Vec<(&'static str, Option<Value>)>) {
    let map: serde_json::Map<String, Value> = args
        .into_iter()
        .filter_map(|(name, value)| value.map(|value| (name.to_owned(), value)))
        .collect();
    if !map.is_empty() {
        observation.set_input_value(Some(Value::Object(map)));
    }
}

pub fn set_metadata(observation: &Observation, metadata: Option<Value>) {
    if let Some(metadata) = metadata {
        observation.set_metadata(metadata);
    }
}

pub fn set_tags(observation: &Observation, tags: impl crate::traces::IntoTags) {
    observation.set_tags(tags);
}

// ----- Lossy capture: Serialize, else Debug, else nothing -----

pub struct Capture<'a, T: ?Sized>(pub &'a T);

pub trait CaptureSerialize {
    fn __lf_capture(&self) -> Option<Value>;
}

pub trait CaptureDebug {
    fn __lf_capture(&self) -> Option<Value>;
}

pub trait CaptureNone {
    fn __lf_capture(&self) -> Option<Value>;
}

impl<T: Serialize + ?Sized> CaptureSerialize for &&Capture<'_, T> {
    fn __lf_capture(&self) -> Option<Value> {
        super::to_value(self.0)
    }
}

impl<T: Debug + ?Sized> CaptureDebug for &Capture<'_, T> {
    fn __lf_capture(&self) -> Option<Value> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| format!("{:?}", self.0)))
            .ok()
            .map(Value::String)
    }
}

impl<T: ?Sized> CaptureNone for Capture<'_, T> {
    fn __lf_capture(&self) -> Option<Value> {
        None
    }
}

// ----- Error messages: Display, else Debug, else a fixed text -----

pub struct ErrMsg<'a, T: ?Sized>(pub &'a T);

pub trait ErrMsgDisplay {
    fn __lf_msg(&self) -> String;
}

pub trait ErrMsgDebug {
    fn __lf_msg(&self) -> String;
}

pub trait ErrMsgNone {
    fn __lf_msg(&self) -> String;
}

impl<T: Display + ?Sized> ErrMsgDisplay for &&ErrMsg<'_, T> {
    fn __lf_msg(&self) -> String {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.0.to_string()))
            .unwrap_or_else(|_| "error".to_owned())
    }
}

impl<T: Debug + ?Sized> ErrMsgDebug for &ErrMsg<'_, T> {
    fn __lf_msg(&self) -> String {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| format!("{:?}", self.0)))
            .unwrap_or_else(|_| "error".to_owned())
    }
}

impl<T: ?Sized> ErrMsgNone for ErrMsg<'_, T> {
    fn __lf_msg(&self) -> String {
        "error".to_owned()
    }
}

// ----- Result detection: split a return value into Ok and Err views -----

/// Never constructed: the error view of a value that is not a `Result`.
pub enum NoError {}

impl Display for NoError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl Serialize for NoError {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        match *self {}
    }
}

pub struct Split<'a, T: ?Sized, E: ?Sized> {
    pub ok: Option<&'a T>,
    pub err: Option<&'a E>,
}

pub struct Outcome<'a, T: ?Sized>(pub &'a T);

pub trait OutcomeResult<'a> {
    type Ok: ?Sized;
    type Err: ?Sized;
    fn __lf_split(&self) -> Split<'a, Self::Ok, Self::Err>;
}

pub trait OutcomeAny<'a> {
    type Ok: ?Sized;
    fn __lf_split(&self) -> Split<'a, Self::Ok, NoError>;
}

impl<'a, T, E> OutcomeResult<'a> for &Outcome<'a, Result<T, E>> {
    type Ok = T;
    type Err = E;
    fn __lf_split(&self) -> Split<'a, T, E> {
        match self.0 {
            Ok(ok) => Split {
                ok: Some(ok),
                err: None,
            },
            Err(err) => Split {
                ok: None,
                err: Some(err),
            },
        }
    }
}

impl<'a, T: ?Sized> OutcomeAny<'a> for Outcome<'a, T> {
    type Ok = T;
    fn __lf_split(&self) -> Split<'a, T, NoError> {
        Split {
            ok: Some(self.0),
            err: None,
        }
    }
}

/// Record a split return value: the `Ok` value as output, or for an `Err` level `ERROR` with its
/// message (and the error as output with `err`).
pub fn record(
    observation: &Observation,
    meta: &Meta,
    ok: Option<Value>,
    err: Option<(String, Option<Value>)>,
) {
    if let Some((message, value)) = err {
        observation.set_error(message);
        if meta.capture_err {
            observation.set_output_value(value);
        }
    } else if meta.capture_output {
        if let Some(value) = ok {
            observation.set_output_value(Some(value));
        }
    }
}

/// `#[observe]` on an `async fn`: run the body with the observation current, record the
/// output and end the observation.
pub fn instrument<F, R>(
    observation: Option<Observation>,
    future: F,
    recorder: R,
) -> Instrumented<F, RecordFn<R>>
where
    F: Future,
    R: FnOnce(&Observation, &F::Output),
{
    Instrumented::new(observation, future, RecordFn(recorder), true)
}

/// `#[observe(stream)]` on an `async fn`: like [`instrument`], but the observation stays open
/// once the future completes (the returned stream ends it).
pub fn instrument_open<F: Future>(observation: Option<Observation>, future: F) -> Instrumented<F> {
    Instrumented::new(observation, future, NoRecord, false)
}

pub fn observe_stream<S, C>(
    observation: Option<Observation>,
    stream: S,
    capture: C,
    capture_output: bool,
) -> ObservedStream<S, C>
where
    S: futures_core::Stream,
    C: FnMut(&S::Item) -> ItemCapture,
{
    ObservedStream::new(observation, stream, capture, capture_output)
}

pub fn observe_iter<I, C>(
    observation: Option<Observation>,
    iter: I,
    capture: C,
    capture_output: bool,
) -> ObservedIter<I::IntoIter, C>
where
    I: IntoIterator,
    C: FnMut(&I::Item) -> ItemCapture,
{
    ObservedIter::new(observation, iter.into_iter(), capture, capture_output)
}

/// `#[observe]` on a synchronous function: current for the body, ended on drop.
pub fn scope(observation: Option<Observation>) -> ScopeGuard {
    ScopeGuard::new(observation)
}

/// A synchronous `#[observe(stream = "iterator")]`: current for the body, but not ended (the
/// returned iterator ends it, or the panic guard).
pub struct OpenScope {
    observation: Option<Observation>,
    _enter: Option<super::EnterGuard>,
}

pub fn open_scope(observation: Option<Observation>) -> OpenScope {
    let enter = observation.as_ref().map(super::context::enter);
    OpenScope {
        observation,
        _enter: enter,
    }
}

impl Drop for OpenScope {
    fn drop(&mut self) {
        if std::thread::panicking() {
            if let Some(observation) = &self.observation {
                observation.mark_panicked();
                observation.end_with(EndReason::Panicked);
            }
        }
    }
}
