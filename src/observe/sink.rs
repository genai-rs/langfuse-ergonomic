//! Where finished observations go: a client or a batcher, installed globally or given explicitly.

use std::sync::atomic::{AtomicU64, Ordering};

use langfuse_client_base::models::IngestionEvent;

use crate::{Batcher, LangfuseClient};

/// A shared pointer: `Arc` natively, `Rc` on WebAssembly (where the client is not `Send`).
#[cfg(not(target_family = "wasm"))]
pub(crate) type Shared<T> = std::sync::Arc<T>;
/// A shared pointer: `Arc` natively, `Rc` on WebAssembly (where the client is not `Send`).
#[cfg(target_family = "wasm")]
pub(crate) type Shared<T> = std::rc::Rc<T>;

/// Where an observation's events are sent when it ends.
#[derive(Clone)]
pub enum Sink {
    /// Send each finished observation with its own request (in a background task: natively the
    /// current tokio runtime, on WebAssembly `spawn_local`). Fine for scripts and tests; prefer a
    /// [`Batcher`] in services.
    Client(LangfuseClient),
    /// Queue events on a batcher with [`Batcher::try_add`] (never blocks).
    Batcher(Batcher),
}

impl std::fmt::Debug for Sink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Client(_) => f.write_str("Sink::Client(..)"),
            Self::Batcher(_) => f.write_str("Sink::Batcher(..)"),
        }
    }
}

impl From<LangfuseClient> for Sink {
    fn from(client: LangfuseClient) -> Self {
        Self::Client(client)
    }
}

impl From<Batcher> for Sink {
    fn from(batcher: Batcher) -> Self {
        Self::Batcher(batcher)
    }
}

impl Sink {
    /// The client that sends (the batcher's client for a batcher).
    pub(crate) fn client(&self) -> &LangfuseClient {
        match self {
            Self::Client(client) => client,
            Self::Batcher(batcher) => batcher.client(),
        }
    }

    /// Send events without waiting. Errors are logged, never returned or panicked on.
    pub(crate) fn send(&self, events: Vec<IngestionEvent>) {
        match self {
            Self::Batcher(batcher) => {
                for event in events {
                    if let Err(error) = batcher.try_add(event) {
                        tracing::warn!(%error, "dropping a Langfuse observation event");
                    }
                }
            }
            Self::Client(client) => spawn_send(client.clone(), events),
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn spawn_send(client: LangfuseClient, events: Vec<IngestionEvent>) {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            handle.spawn(async move {
                if let Err(error) = client.ingest_events(events).await {
                    tracing::warn!(%error, "sending a Langfuse observation failed");
                }
            });
        }
        Err(_) => tracing::warn!(
            "no tokio runtime to send a Langfuse observation from; install a Batcher with \
             observe::install_batcher to observe code outside a runtime"
        ),
    }
}

#[cfg(target_family = "wasm")]
fn spawn_send(client: LangfuseClient, events: Vec<IngestionEvent>) {
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(error) = client.ingest_events(events).await {
            tracing::warn!(%error, "sending a Langfuse observation failed");
        }
    });
}

// ----- The global sink -----

#[cfg(not(target_family = "wasm"))]
static GLOBAL: std::sync::RwLock<Option<Shared<Sink>>> = std::sync::RwLock::new(None);

#[cfg(target_family = "wasm")]
thread_local! {
    static GLOBAL: std::cell::RefCell<Option<Shared<Sink>>> = const { std::cell::RefCell::new(None) };
}

/// `1.0` as bits: everything is sampled by default.
static SAMPLE_RATE: AtomicU64 = AtomicU64::new(0x3FF0_0000_0000_0000);

pub(crate) fn global() -> Option<Shared<Sink>> {
    #[cfg(not(target_family = "wasm"))]
    {
        GLOBAL
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
    #[cfg(target_family = "wasm")]
    {
        GLOBAL.with(|global| global.borrow().clone())
    }
}

pub(crate) fn set_global(sink: Option<Sink>) -> Option<Sink> {
    let sink = sink.map(Shared::new);
    #[cfg(not(target_family = "wasm"))]
    let previous = std::mem::replace(
        &mut *GLOBAL
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
        sink,
    );
    #[cfg(target_family = "wasm")]
    let previous = GLOBAL.with(|global| global.replace(sink));
    previous.map(|previous| Shared::try_unwrap(previous).unwrap_or_else(|shared| (*shared).clone()))
}

pub(crate) fn sample_rate() -> f64 {
    f64::from_bits(SAMPLE_RATE.load(Ordering::Relaxed))
}

pub(crate) fn set_sample_rate(rate: f64) {
    let rate = if rate.is_nan() {
        1.0
    } else {
        rate.clamp(0.0, 1.0)
    };
    SAMPLE_RATE.store(rate.to_bits(), Ordering::Relaxed);
}

/// Decide whether a new trace is recorded.
pub(crate) fn sample() -> bool {
    let rate = sample_rate();
    if rate >= 1.0 {
        return true;
    }
    if rate <= 0.0 {
        return false;
    }
    use rand::RngExt;
    rand::rng().random::<f64>() < rate
}
