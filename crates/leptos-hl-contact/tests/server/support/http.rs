//! Requests and responses: the submission's fields, a reply, a rendered page.

use axum::http::{HeaderMap, StatusCode, header};
use leptos_hl_contact::ContactFieldErrors;

/// A form-encoded submission, and the cookie sent with it.
#[derive(Clone)]
pub struct Fields {
    pub(super) pairs: Vec<(String, String)>,
    pub cookie: Option<String>,
}

impl Fields {
    pub fn valid(token: &str) -> Self {
        let pairs = [
            ("name", "Ada Lovelace"),
            ("email", "ada@example.com"),
            ("subject", "Hello"),
            ("message", "A message typed by a person."),
            ("website", ""),
            ("form_token", token),
        ];
        Self {
            pairs: pairs
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
            cookie: None,
        }
    }

    pub fn set(mut self, key: &str, value: &str) -> Self {
        match self.pairs.iter_mut().find(|(k, _)| k == key) {
            Some(pair) => pair.1 = value.to_owned(),
            None => self.pairs.push((key.to_owned(), value.to_owned())),
        }
        self
    }

    /// Adds a pair without replacing one with the same key, so a test can send
    /// a key twice.
    pub fn push(mut self, key: &str, value: &str) -> Self {
        self.pairs.push((key.to_owned(), value.to_owned()));
        self
    }

    pub fn without(mut self, key: &str) -> Self {
        self.pairs.retain(|(k, _)| k != key);
        self
    }

    pub fn with_cookie(mut self, cookie: &str) -> Self {
        self.cookie = Some(cookie.to_owned());
        self
    }

    pub(super) fn encode(&self) -> String {
        self.pairs
            .iter()
            .map(|(k, v)| format!("{}={}", form_encode(k), form_encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }
}

fn form_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".to_owned(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// A server response.
pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
}

impl Reply {
    fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    }

    pub fn location(&self) -> Option<String> {
        self.header("location")
    }

    pub fn redirect_header(&self) -> Option<String> {
        self.header("serverfnredirect")
    }

    /// A no-JavaScript submission that succeeded: a `302` whose `Location`
    /// carries no `__err`.
    pub fn is_nojs_success(&self) -> bool {
        self.status.as_u16() == 302 && self.location().is_some_and(|l| !l.contains("__err"))
    }

    /// A no-JavaScript submission that failed: a `302` carrying `__err`.
    pub fn is_nojs_error(&self) -> bool {
        self.status.as_u16() == 302 && self.location().is_some_and(|l| l.contains("__err"))
    }

    /// What a sender can compare between two responses: status, body, and
    /// every header as sorted `(name, value)` pairs, `Location` and
    /// `serverfnredirect` included.  No header is excluded: under `oneshot`
    /// nothing adds a value that varies per response, such as `Date`.
    pub fn fingerprint(&self) -> (u16, String, Vec<(String, String)>) {
        let mut headers: Vec<(String, String)> = self
            .headers
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect();
        headers.sort();
        (self.status.as_u16(), self.body.clone(), headers)
    }

    /// The `contact_error:` code in a fetch response body.
    pub fn contact_error(&self) -> Option<String> {
        let at = self.body.find("contact_error:")?;
        Some(
            self.body[at + "contact_error:".len()..]
                .trim()
                .trim_end_matches('"')
                .to_owned(),
        )
    }

    /// Whether the body carries the server-error variant (not `Args`).
    pub fn is_server_error(&self) -> bool {
        self.body.starts_with("ServerError|")
    }

    pub fn field_errors(&self) -> Option<ContactFieldErrors> {
        ContactFieldErrors::from_error_str(&self.body)
    }

    pub fn set_cookies(&self) -> Vec<String> {
        self.headers
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(str::to_owned))
            .collect()
    }
}

/// A rendered page.
pub struct Page {
    pub status: StatusCode,
    pub html: String,
    pub set_cookies: Vec<String>,
}

impl Page {
    /// The hidden `form_token` value.
    pub fn token(&self) -> Option<String> {
        let at = self.html.find("name=\"form_token\"")?;
        let rest = &self.html[at..];
        let value = rest.find("value=\"")? + "value=\"".len();
        let end = rest[value..].find('"')?;
        Some(rest[value..value + end].to_owned())
    }

    /// The text of the assertive banner, if one is rendered.  Attribute
    /// order in server-rendered HTML is not fixed, so find the attribute, then
    /// the end of its tag.
    pub fn banner(&self) -> Option<String> {
        let attribute = self.html.find("aria-live=\"assertive\"")?;
        let text = attribute + self.html[attribute..].find('>')? + 1;
        let end = self.html[text..].find('<')?;
        Some(self.html[text..text + end].to_owned())
    }

    pub fn aria_invalid_count(&self) -> usize {
        self.html.matches("aria-invalid=\"true\"").count()
    }
}
