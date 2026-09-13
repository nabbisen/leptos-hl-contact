//! Captures every tracing event and span field emitted while a test runs.
//!
//! One subscriber is installed globally for the whole test binary, once, and
//! each test collects into a buffer that is active only on its own thread
//! while its guard lives.  A per-thread `set_default` subscriber is not
//! enough here: tests run in parallel, and a callsite first reached from a
//! thread with no subscriber can be cached as uninteresting, so its events
//! would never reach the capture.

use std::{
    cell::RefCell,
    fmt::Write as _,
    sync::{Arc, Mutex, Once},
};

use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};
use tracing_subscriber::{
    Registry,
    layer::{Context, Layer, SubscriberExt},
    registry::LookupSpan,
};

/// The captured lines: one per event, span creation, or span record, each
/// with its message and every field formatted.
#[derive(Clone, Default)]
pub struct Captured(Arc<Mutex<Vec<String>>>);

impl Captured {
    pub fn lines(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }

    pub fn any_contains(&self, needle: &str) -> bool {
        self.lines().iter().any(|line| line.contains(needle))
    }

    fn push(&self, line: String) {
        self.0.lock().unwrap().push(line);
    }
}

thread_local! {
    static ACTIVE: RefCell<Option<Captured>> = const { RefCell::new(None) };
}

/// Stops the capture on this thread when dropped.
pub struct CaptureGuard;

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        ACTIVE.with(|active| *active.borrow_mut() = None);
    }
}

/// Start capturing on the current thread.  Tests run on Tokio's
/// current-thread runtime, so the request futures emit on this thread.
pub fn capture_logs() -> (Captured, CaptureGuard) {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        tracing::subscriber::set_global_default(Registry::default().with(CaptureLayer))
            .expect("no other global subscriber in the server suite");
    });
    let captured = Captured::default();
    ACTIVE.with(|active| *active.borrow_mut() = Some(captured.clone()));
    (captured, CaptureGuard)
}

fn record(line: impl FnOnce() -> String) {
    ACTIVE.with(|active| {
        if let Some(captured) = active.borrow().as_ref() {
            captured.push(line());
        }
    });
}

struct CaptureLayer;

impl<S> Layer<S> for CaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        record(|| {
            let mut text = FieldText::new(event.metadata().target());
            event.record(&mut text);
            text.0
        });
    }

    fn on_new_span(&self, attrs: &Attributes<'_>, _id: &Id, _ctx: Context<'_, S>) {
        record(|| {
            let mut text = FieldText::new(attrs.metadata().name());
            attrs.record(&mut text);
            text.0
        });
    }

    fn on_record(&self, _id: &Id, values: &Record<'_>, _ctx: Context<'_, S>) {
        record(|| {
            let mut text = FieldText::new("record");
            values.record(&mut text);
            text.0
        });
    }
}

struct FieldText(String);

impl FieldText {
    fn new(prefix: &str) -> Self {
        Self(prefix.to_owned())
    }
}

impl Visit for FieldText {
    fn record_str(&mut self, field: &Field, value: &str) {
        let _ = write!(self.0, " {}={}", field.name(), value);
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let _ = write!(self.0, " {}={:?}", field.name(), value);
    }
}
