//! The current observation: a thread-local stack, entered for the duration of a synchronous scope
//! or of each `poll` of an instrumented future (like `tracing::Instrumented`), so it follows a task
//! across `.await` on any executor (tokio, `wasm_bindgen_futures::spawn_local`, `block_on`, ...).

use std::cell::RefCell;
use std::marker::PhantomData;

use super::Observation;

thread_local! {
    static STACK: RefCell<Vec<Observation>> = const { RefCell::new(Vec::new()) };
}

/// The innermost entered observation, recording or not.
pub(crate) fn top() -> Option<Observation> {
    STACK.with(|stack| stack.borrow().last().cloned())
}

/// Makes an observation the current one until it is dropped (then the previous one is current
/// again). Not `Send`: it must be dropped on the thread that created it.
#[must_use = "the observation is current only while the guard lives"]
pub struct EnterGuard {
    #[cfg(feature = "otel-context")]
    _otel: Option<opentelemetry::ContextGuard>,
    _not_send: PhantomData<*const ()>,
}

impl std::fmt::Debug for EnterGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EnterGuard")
    }
}

pub(crate) fn enter(observation: &Observation) -> EnterGuard {
    #[cfg(feature = "otel-context")]
    let otel = super::otel_context::attach(observation);
    STACK.with(|stack| stack.borrow_mut().push(observation.clone()));
    EnterGuard {
        #[cfg(feature = "otel-context")]
        _otel: otel,
        _not_send: PhantomData,
    }
}

impl Drop for EnterGuard {
    fn drop(&mut self) {
        // `try_with`: the stack may already be gone when a thread-local destructor drops a guard.
        let _ = STACK.try_with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}
