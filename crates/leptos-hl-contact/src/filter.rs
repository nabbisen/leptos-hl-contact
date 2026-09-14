// filter.rs — The pre-delivery filter hook: the site's own rules about
// content, run on a validated submission just before delivery.
//
// The crate ships the hook, not rules.  See `security/filter.md`.

use std::{future::Future, pin::Pin, sync::Arc};

use crate::model::ContactInput;

/// What a [`ContactFilter`] decided about a submission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterDecision {
    /// Deliver it.
    Accept,
    /// Refuse it.  The visitor sees the generic `rejected` label.
    Reject,
    /// Refuse it without telling the sender: they see success and nothing is
    /// delivered, as with the honeypot.
    SilentDrop,
}

/// The site's own rule about a submission's content.
///
/// A filter runs after the form token, the honeypot, field validation, the
/// server policy and the challenge, and just before delivery.  It therefore
/// only ever sees input that has passed validation, and it never produces a
/// field error: structural limits belong to `ContactServerPolicy`.
///
/// The decision is not a `Result`.  A filter that calls a service decides
/// itself what an outage means — usually [`FilterDecision::Accept`], so a
/// real enquiry is never lost to someone else's downtime.
///
/// Filters see the visitor's name, address and message.  Do not log them.
///
/// # Example
///
/// ```rust
/// use std::{future::Future, pin::Pin};
/// use leptos_hl_contact::{
///     ContactInput,
///     filter::{ContactFilter, FilterDecision},
/// };
///
/// /// Reject messages with more than `max` links.
/// struct MaxLinks {
///     max: usize,
/// }
///
/// impl ContactFilter for MaxLinks {
///     fn filter(
///         &self,
///         input: &ContactInput,
///     ) -> Pin<Box<dyn Future<Output = FilterDecision> + Send + '_>> {
///         let links = input.message.matches("http://").count()
///             + input.message.matches("https://").count();
///         Box::pin(async move {
///             if links > self.max {
///                 FilterDecision::Reject
///             } else {
///                 FilterDecision::Accept
///             }
///         })
///     }
///
///     fn name(&self) -> &'static str {
///         "MaxLinks"
///     }
/// }
///
/// let input = ContactInput::from_raw(
///     "Ada".into(),
///     "ada@example.com".into(),
///     None,
///     "See https://a.example, http://b.example and https://c.example".into(),
///     String::new(),
/// );
/// let decision = tokio::runtime::Runtime::new()
///     .unwrap()
///     .block_on(MaxLinks { max: 2 }.filter(&input));
/// assert_eq!(decision, FilterDecision::Reject);
/// ```
pub trait ContactFilter: Send + Sync + 'static {
    /// Called after validation, policy, and challenge; before delivery.
    fn filter(&self, input: &ContactInput) -> FilterFuture<'_>;

    /// Short name for logs; default = type name.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// The future [`ContactFilter::filter`] returns.
///
/// It is `Send` on every target except a wasm32 server build
/// (`all(target_arch = "wasm32", feature = "ssr")`, such as Cloudflare
/// Workers), where an implementation may await JavaScript futures, which are
/// not `Send`.  The implementing type itself must still be `Send + Sync` on
/// every target: Leptos context requires it.
#[cfg(not(all(target_arch = "wasm32", feature = "ssr")))]
pub type FilterFuture<'a> = Pin<Box<dyn Future<Output = FilterDecision> + Send + 'a>>;

/// The future [`ContactFilter::filter`] returns.
///
/// It is `Send` on every target except a wasm32 server build
/// (`all(target_arch = "wasm32", feature = "ssr")`, such as Cloudflare
/// Workers), where an implementation may await JavaScript futures, which are
/// not `Send`.  The implementing type itself must still be `Send + Sync` on
/// every target: Leptos context requires it.
#[cfg(all(target_arch = "wasm32", feature = "ssr"))]
pub type FilterFuture<'a> = Pin<Box<dyn Future<Output = FilterDecision> + 'a>>;

/// Turns filtering on for `submit_contact`.
///
/// Provide it in the context closure passed to `leptos_routes_with_context`.
/// Several filters go in a [`FilterChain`].
pub type ContactFilterContext = Arc<dyn ContactFilter>;

/// Runs filters in order; the first non-Accept decision wins.
///
/// Later filters are not called once one has decided, so put cheap local
/// rules before anything that calls a service.  An empty chain accepts.
pub struct FilterChain(Vec<Arc<dyn ContactFilter>>);

impl FilterChain {
    /// A chain that runs `filters` in this order.
    pub fn new(filters: Vec<Arc<dyn ContactFilter>>) -> Self {
        Self(filters)
    }
}

impl std::fmt::Debug for FilterChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|filter| filter.name()))
            .finish()
    }
}

impl ContactFilter for FilterChain {
    fn filter(&self, input: &ContactInput) -> FilterFuture<'_> {
        let input = input.clone();
        Box::pin(async move {
            for filter in &self.0 {
                let decision = filter.filter(&input).await;
                if decision != FilterDecision::Accept {
                    // The chain is shared by concurrent requests, so the
                    // deciding filter's name cannot live on `self`.  It goes
                    // on the span `submit_contact` runs the chain in.
                    tracing::Span::current().record(FILTER_FIELD, filter.name());
                    return decision;
                }
            }
            FilterDecision::Accept
        })
    }
}

/// The span `submit_contact` runs a filter in, and the field on it where a
/// chain names the member that decided.  The span also carries `filter`,
/// the name of the filter in context; `decided_by` is separate because the
/// log formatter appends a re-recorded field instead of replacing it.
pub(crate) const FILTER_SPAN: &str = "contact_filter";
pub(crate) const FILTER_FIELD: &str = "decided_by";

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
