//! Test doubles for the context values an integrator provides.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use leptos_hl_contact::{
    ChallengeError, ChallengeOutcome, ChallengeVerifier, ContactDelivery, ContactDeliveryError,
    ContactFilter, ContactInput, FilterDecision,
};

/// Counts deliveries and keeps the last input, so "delivered or not" is
/// asserted directly rather than inferred from logs.
#[derive(Default)]
pub struct RecordingDelivery {
    calls: AtomicUsize,
    last: Mutex<Option<ContactInput>>,
}

impl RecordingDelivery {
    pub fn count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    pub fn last(&self) -> Option<ContactInput> {
        self.last.lock().unwrap().clone()
    }
}

impl ContactDelivery for RecordingDelivery {
    fn deliver(
        &self,
        input: ContactInput,
    ) -> Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + Send + '_>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.last.lock().unwrap() = Some(input);
        Box::pin(async { Ok(()) })
    }
}

/// A `ChallengeVerifier` with a scripted answer that records the tokens it
/// was asked about.
pub struct ScriptedVerifier {
    answer: Result<ChallengeOutcome, ChallengeError>,
    seen: Mutex<Vec<String>>,
}

impl ScriptedVerifier {
    pub fn passing() -> Self {
        Self::answering(Ok(ChallengeOutcome {
            passed: true,
            ..Default::default()
        }))
    }

    pub fn failing(error_codes: &[&str]) -> Self {
        Self::answering(Ok(ChallengeOutcome {
            passed: false,
            error_codes: error_codes.iter().map(|c| (*c).to_owned()).collect(),
            ..Default::default()
        }))
    }

    pub fn unavailable() -> Self {
        Self::answering(Err(ChallengeError::Timeout))
    }

    fn answering(answer: Result<ChallengeOutcome, ChallengeError>) -> Self {
        Self {
            answer,
            seen: Mutex::new(Vec::new()),
        }
    }

    pub fn seen(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }
}

impl ChallengeVerifier for ScriptedVerifier {
    fn verify(
        &self,
        token: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ChallengeOutcome, ChallengeError>> + Send + '_>> {
        self.seen.lock().unwrap().push(token.to_owned());
        let answer = match &self.answer {
            Ok(outcome) => Ok(outcome.clone()),
            Err(ChallengeError::Timeout) => Err(ChallengeError::Timeout),
            Err(ChallengeError::Unavailable(s)) => Err(ChallengeError::Unavailable(s.clone())),
            Err(ChallengeError::Misconfigured(s)) => Err(ChallengeError::Misconfigured(s.clone())),
        };
        Box::pin(async move { answer })
    }
}

/// A `ContactFilter` with a fixed decision that counts its calls.
pub struct FixedFilter {
    decision: FilterDecision,
    name: &'static str,
    calls: AtomicUsize,
}

impl FixedFilter {
    pub fn new(decision: FilterDecision, name: &'static str) -> Self {
        Self {
            decision,
            name,
            calls: AtomicUsize::new(0),
        }
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl ContactFilter for FixedFilter {
    fn filter(
        &self,
        _input: &ContactInput,
    ) -> Pin<Box<dyn Future<Output = FilterDecision> + Send + '_>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let decision = self.decision;
        Box::pin(async move { decision })
    }

    fn name(&self) -> &'static str {
        self.name
    }
}
