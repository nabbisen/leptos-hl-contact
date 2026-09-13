//! The integrator's server, built as the documentation instructs.

use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Router,
    body::Body,
    http::{Request, header},
};
use http_body_util::BodyExt;
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use leptos_hl_contact::{
    ChallengeContext, ContactDeliveryContext, ContactFilterContext, ContactForm,
    ContactServerPolicy, ContactSuccessRedirect,
    axum_helpers::{
        FormTokenCookie, provide_form_token_binding, provide_form_token_issuer,
        provide_form_token_with_cookie,
    },
    form_token::{Binding, FormTokenConfig, FormTokenContext, issue_form_token},
};
use leptos_router::{
    components::{Route, Router as LeptosRouter, Routes},
    path,
};
use tower::ServiceExt;

use super::{
    doubles::{FailingDelivery, RecordingDelivery},
    http::{Fields, Page, Reply},
};

/// The form-token secret every test uses.  Not a real secret.
pub const TEST_SECRET: &str = "integration-test-secret-0123456789";

/// The page a no-JavaScript submission comes from.
const REFERER: &str = "http://localhost/contact";

/// How the form token is configured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TokenMode {
    /// `FormTokenContext` and a `FormToken` per render, without binding.
    #[default]
    Plain,
    /// `Binding::Cookie` with the three Axum helpers, at cookie defaults.
    Bound,
    /// No `FormTokenContext` at all.
    Absent,
}

/// Which context values the one closure provides.
pub struct Setup {
    pub delivery: bool,
    /// Provide `FailingDelivery` instead of `RecordingDelivery`.
    pub failing_delivery: bool,
    /// Provide this delivery context instead of either double.
    pub delivery_context: Option<ContactDeliveryContext>,
    pub token: TokenMode,
    pub min_age_secs: u64,
    pub success_page: bool,
    pub policy: Option<ContactServerPolicy>,
    pub challenge: Option<ChallengeContext>,
    pub filter: Option<ContactFilterContext>,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            delivery: true,
            failing_delivery: false,
            delivery_context: None,
            token: TokenMode::Plain,
            min_age_secs: 0,
            success_page: false,
            policy: None,
            challenge: None,
            filter: None,
        }
    }
}

#[component]
fn App() -> impl IntoView {
    view! {
        <LeptosRouter>
            <Routes fallback=|| "not found">
                <Route path=path!("/contact") view=|| view! { <ContactForm /> } />
                <Route path=path!("/thanks") view=|| view! { <h1>"Thank you"</h1> } />
            </Routes>
        </LeptosRouter>
    }
}

fn shell(_options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html>
            <head></head>
            <body><App /></body>
        </html>
    }
}

pub struct Harness {
    app: Router,
    pub delivery: Arc<RecordingDelivery>,
    pub failing: Arc<FailingDelivery>,
    pub redirects: Arc<Mutex<Vec<String>>>,
    token_config: FormTokenContext,
}

impl Harness {
    pub fn new(setup: Setup) -> Self {
        let options = LeptosOptions::builder().output_name("contact-test").build();
        let delivery = Arc::new(RecordingDelivery::default());
        let failing = Arc::new(FailingDelivery::default());
        let redirects: Arc<Mutex<Vec<String>>> = Arc::default();

        let mut config =
            FormTokenConfig::new(TEST_SECRET.as_bytes().to_vec()).with_min_age(setup.min_age_secs);
        if setup.token == TokenMode::Bound {
            config = config.with_binding(Binding::Cookie);
        }
        let token_config: FormTokenContext = Arc::new(config);

        let success = setup.success_page.then(|| {
            let redirects = Arc::clone(&redirects);
            ContactSuccessRedirect::new("/thanks", move |path| {
                redirects.lock().unwrap().push(path.to_owned());
                leptos_axum::redirect(path);
            })
            .expect("site-relative path")
        });

        // The one context closure (RFC 007).
        let context = {
            let delivery = Arc::clone(&delivery);
            let failing = Arc::clone(&failing);
            let failing_delivery = setup.failing_delivery;
            let delivery_context = setup.delivery_context;
            let token_config = Arc::clone(&token_config);
            let cookie = FormTokenCookie::default();
            let token_mode = setup.token;
            let deliver = setup.delivery;
            let policy = setup.policy;
            let challenge = setup.challenge;
            let filter = setup.filter;
            move || {
                if deliver {
                    let delivery: ContactDeliveryContext = if let Some(context) = &delivery_context
                    {
                        Arc::clone(context)
                    } else if failing_delivery {
                        failing.clone() as ContactDeliveryContext
                    } else {
                        delivery.clone() as ContactDeliveryContext
                    };
                    provide_context(delivery);
                }
                match token_mode {
                    TokenMode::Plain => {
                        provide_context::<FormTokenContext>(Arc::clone(&token_config));
                        provide_context(issue_form_token(&token_config));
                    }
                    TokenMode::Bound => {
                        provide_context::<FormTokenContext>(Arc::clone(&token_config));
                        provide_form_token_with_cookie(&token_config, &cookie);
                        provide_form_token_binding(&cookie);
                        provide_form_token_issuer(&cookie);
                    }
                    TokenMode::Absent => {}
                }
                if let Some(redirect) = &success {
                    provide_context(redirect.clone());
                }
                if let Some(policy) = &policy {
                    provide_context(policy.clone());
                }
                if let Some(challenge) = &challenge {
                    provide_context(challenge.clone());
                }
                if let Some(filter) = &filter {
                    provide_context(Arc::clone(filter));
                }
            }
        };

        let routes = generate_route_list(App);
        let app = Router::new()
            .leptos_routes_with_context(&options, routes, context, {
                let options = options.clone();
                move || shell(options.clone())
            })
            .with_state(options);

        Self {
            app,
            delivery,
            failing,
            redirects,
            token_config,
        }
    }

    /// A valid token for an unbound configuration.
    pub fn token(&self) -> String {
        issue_form_token(&self.token_config).0
    }

    /// A valid submission with a fresh token.
    pub fn fields(&self) -> Fields {
        Fields::valid(&self.token())
    }

    pub fn deliveries(&self) -> usize {
        self.delivery.count()
    }

    /// `GET path`, as a browser navigation.
    pub async fn render(&self, path: &str, cookie: Option<&str>) -> Page {
        let mut request = Request::get(path).header(header::ACCEPT, "text/html");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let reply = self.send(request.body(Body::empty()).unwrap()).await;
        Page {
            status: reply.status,
            set_cookies: reply.set_cookies(),
            html: reply.body,
        }
    }

    /// A plain HTML form post: `Accept: text/html` and a `Referer`.
    pub async fn submit_nojs(&self, fields: &Fields) -> Reply {
        self.post(
            "/api/submit_contact",
            fields,
            "text/html,application/xhtml+xml",
            Some(REFERER),
        )
        .await
    }

    /// The request `ActionForm` makes with JavaScript.
    pub async fn submit_fetch(&self, fields: &Fields) -> Reply {
        self.post("/api/submit_contact", fields, "application/json", None)
            .await
    }

    /// `POST /api/form_token`, as `ContactForm` does in the browser.
    pub async fn fetch_token(&self, cookie: Option<&str>) -> Reply {
        let fields = Fields {
            pairs: Vec::new(),
            cookie: cookie.map(str::to_owned),
        };
        self.post("/api/form_token", &fields, "application/json", None)
            .await
    }

    /// Follow a no-JavaScript redirect and render the page it lands on.
    pub async fn follow(&self, reply: &Reply) -> Page {
        let location = reply.location().expect("a redirect");
        let path = match location.split_once("://") {
            Some((_, rest)) => rest.find('/').map_or("/", |i| &rest[i..]).to_owned(),
            None => location,
        };
        self.render(&path, None).await
    }

    async fn post(
        &self,
        path: &str,
        fields: &Fields,
        accept: &str,
        referer: Option<&str>,
    ) -> Reply {
        let mut request = Request::post(path)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(header::ACCEPT, accept);
        if let Some(referer) = referer {
            request = request.header(header::REFERER, referer);
        }
        if let Some(cookie) = &fields.cookie {
            request = request.header(header::COOKIE, cookie);
        }
        self.send(request.body(Body::from(fields.encode())).unwrap())
            .await
    }

    async fn send(&self, request: Request<Body>) -> Reply {
        let response = self
            .app
            .clone()
            .oneshot(request)
            .await
            .expect("the router answers");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            headers,
            body: String::from_utf8_lossy(&bytes).into_owned(),
        }
    }
}

/// The nonce, the token's middle segment.
pub fn nonce_of(token: &str) -> &str {
    token.split('|').nth(1).expect("three segments")
}

/// A token in the documented format `{unix_seconds}|{nonce_hex}|{hmac_hex}`,
/// signed with `TEST_SECRET`, issued `age_secs` ago.
pub fn signed_token(age_secs: u64, nonce_hex: &str) -> String {
    use hmac::{Hmac, Mac, digest::KeyInit};
    use sha2::Sha256;

    let issued = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - age_secs;
    let payload = format!("{issued}|{nonce_hex}");
    let mut mac = Hmac::<Sha256>::new_from_slice(TEST_SECRET.as_bytes()).unwrap();
    mac.update(payload.as_bytes());
    let signature: String = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("{payload}|{signature}")
}
