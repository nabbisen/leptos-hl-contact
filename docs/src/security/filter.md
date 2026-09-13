# Filter

A filter is your site's own rule about **content**: the question "does this
site want this message?", asked once a submission has passed every other
check.  It is not a structural limit — lengths and required fields belong to
[server policy](../guides/customization.md#contactserverpolicy), which
answers with a field error — and it is not a test of whether the sender is
human, which is the [challenge](./challenge.md)'s job.  A filter only ever
sees validated input, never produces a field error, and the crate ships no
rules of its own: the hook is yours to fill.

## The API

```rust,ignore
pub enum FilterDecision { Accept, Reject, SilentDrop }

pub trait ContactFilter: Send + Sync + 'static {
    /// Called after validation, policy, and challenge; before delivery.
    fn filter(&self, input: &ContactInput) -> Pin<Box<dyn Future<Output = FilterDecision> + Send + '_>>;
    /// Short name for logs; default = type name.
    fn name(&self) -> &'static str { std::any::type_name::<Self>() }
}

pub type ContactFilterContext = Arc<dyn ContactFilter>;

/// Runs filters in order; the first non-Accept decision wins.
pub struct FilterChain(/* … */);
impl FilterChain { pub fn new(filters: Vec<Arc<dyn ContactFilter>>) -> Self; }
```

Provide the filter in the context closure passed to
`leptos_routes_with_context`, like every other server-side value:

```rust,ignore
provide_context::<ContactFilterContext>(Arc::new(MaxLinks { max: 2 }));
```

| Decision | The visitor sees | Delivered | Logged at `warn` |
|----------|------------------|-----------|------------------|
| `Accept` | success | yes | — |
| `Reject` | `labels.errors.rejected`: "Your message could not be accepted." | no | `submission rejected by filter`, with the filter's name |
| `SilentDrop` | success, exactly as if it had been delivered | no | `submission silently dropped by filter`, with the filter's name |

`SilentDrop` behaves like the honeypot: the sender learns nothing.  Use
`Reject` when a genuine visitor could have written the message and deserves
to know it did not arrive.

## Example: at most two links

This is compiled and run as a doctest on `ContactFilter`, so it stays
correct:

```rust,ignore
use std::{future::Future, pin::Pin};
use leptos_hl_contact::{ContactInput, filter::{ContactFilter, FilterDecision}};

/// Reject messages with more than `max` links.
struct MaxLinks {
    max: usize,
}

impl ContactFilter for MaxLinks {
    fn filter(&self, input: &ContactInput) -> Pin<Box<dyn Future<Output = FilterDecision> + Send + '_>> {
        let links = input.message.matches("http://").count()
            + input.message.matches("https://").count();
        Box::pin(async move {
            if links > self.max { FilterDecision::Reject } else { FilterDecision::Accept }
        })
    }

    fn name(&self) -> &'static str {
        "MaxLinks"
    }
}
```

Do the work before `Box::pin` when it needs only the input: the returned
future may borrow `self`, but not `input`.

## Example: blocked email domains

```rust,ignore
use std::collections::HashSet;

/// Refuse addresses at domains you have decided not to hear from.
struct BlockedDomains {
    /// Lower-case domains, such as "example.invalid".
    domains: HashSet<String>,
}

impl ContactFilter for BlockedDomains {
    fn filter(&self, input: &ContactInput) -> Pin<Box<dyn Future<Output = FilterDecision> + Send + '_>> {
        let blocked = input
            .email
            .rsplit_once('@')
            .is_some_and(|(_, domain)| self.domains.contains(&domain.to_ascii_lowercase()));
        Box::pin(async move {
            if blocked { FilterDecision::SilentDrop } else { FilterDecision::Accept }
        })
    }

    fn name(&self) -> &'static str {
        "BlockedDomains"
    }
}
```

## Calling an external service

A filter that asks a classification service must put a time limit on the
call and decide what an unanswered call means.  The trait returns a
decision, not a `Result`, on purpose: only you know which way your
deployment should fail.

```rust,ignore
use std::time::Duration;

struct Classifier {
    client: reqwest::Client,   // built once, reused
    url: String,
}

impl ContactFilter for Classifier {
    fn filter(&self, input: &ContactInput) -> Pin<Box<dyn Future<Output = FilterDecision> + Send + '_>> {
        let message = input.message.clone();
        Box::pin(async move {
            let call = self.client.post(&self.url).body(message).send();
            match tokio::time::timeout(Duration::from_secs(2), call).await {
                Ok(Ok(response)) if response.status().is_success() => {
                    match response.text().await.as_deref() {
                        Ok("reject") => FilterDecision::Reject,
                        _ => FilterDecision::Accept,
                    }
                }
                // Timed out, unreachable, or answered with an error.
                _ => FilterDecision::Accept,
            }
        })
    }

    fn name(&self) -> &'static str {
        "Classifier"
    }
}
```

The last arm is the choice:

- **Fail open** — `Accept` when the service does not answer, as above.  A
  real enquiry is never lost to someone else's outage; unwanted messages get
  through while it lasts.  This is the usual choice for a classifier.
- **Fail closed** — `Reject` instead.  Nothing arrives unchecked, and every
  visitor is turned away while the service is down.  Choose it only when an
  unchecked message is worse than a lost one.

Keep the time limit well under your reverse proxy's request timeout: the
visitor waits for the filter.

## Chaining

```rust,ignore
let filter: ContactFilterContext = Arc::new(FilterChain::new(vec![
    Arc::new(MaxLinks { max: 2 }),
    Arc::new(BlockedDomains { domains }),
    Arc::new(Classifier { client, url }),
]));
provide_context(filter);
```

Filters run in order and the first decision other than `Accept` wins; later
filters are not called.  Put cheap local rules first, so a message that one
of them settles never reaches a service.  An empty chain accepts.  The log
line carries both names — `filter` is the chain, `decided_by` the member
that decided:

```text
WARN contact_filter{filter="leptos_hl_contact::filter::FilterChain" decided_by="KeywordReject"}: submission rejected by filter
```

## Privacy

A filter sees the visitor's name, email address and message.

- **Do not log them.**  The crate's own log events carry no personal data;
  keep yours the same — log the decision and the filter's name, as
  `submit_contact` does.
- **A filter that sends content to a service sends personal data to a third
  party.**  Say so in your privacy notice, and send only what the service
  needs.
