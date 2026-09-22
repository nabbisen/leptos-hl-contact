// delivery/body.rs — the plain-text body and the subject line every backend
// sends.
//
// `build_plain_text_body` moved from `delivery/smtp.rs` (RFC 017 D1),
// unchanged: `delivery-resend` (RFC 017 D3) is the second caller.
// `compose_subject` is new, factored out of both backends (handoff 02
// review, C3): each composed it separately, and with the default empty
// `ResendConfig` prefix that left a stray leading space no test caught.

use crate::{model::ContactInput, security::sanitize_header_value};

/// The `Subject` header text: `prefix`, a space, then `subject` when a
/// prefix is set; `subject` alone otherwise.  Both inputs are sanitized
/// against header injection first (defence-in-depth: validation already
/// rejects newlines), and the result is trimmed, so an empty prefix never
/// leaves a leading space.
pub(crate) fn compose_subject(prefix: &str, subject: &str) -> String {
    let prefix = sanitize_header_value(prefix);
    let subject = sanitize_header_value(subject);
    let composed = if prefix.is_empty() {
        subject
    } else {
        format!("{prefix} {subject}")
    };
    composed.trim().to_owned()
}

/// The message body, in plain text.
///
/// The site's own fields (RFC 015 D4) have one block each, in the order the
/// site defined them, after `Subject:` and before `Message:`.  A choice reads
/// `choice label (choice key)`; any other field reads as its value, which for
/// several lines is kept as it was typed.  Only answered fields are present,
/// and with none the body is exactly what it was before site fields existed.
///
/// Site values go into the body only.  No header is ever built from one.
pub(crate) fn build_plain_text_body(input: &ContactInput) -> String {
    let subject = input.subject.as_deref().unwrap_or("(none)");
    let mut body = format!(
        "New contact form submission\n\
         ===========================\n\
         \n\
         Name:\n\
         {}\n\
         \n\
         Email:\n\
         {}\n\
         \n\
         Subject:\n\
         {}\n\
         \n",
        input.name, input.email, subject,
    );
    for field in &input.site_fields {
        let value = match &field.value_label {
            Some(label) => format!("{label} ({})", field.value),
            None => field.value.clone(),
        };
        body.push_str(&format!("{}:\n{value}\n\n", field.label));
    }
    body.push_str(&format!("Message:\n{}\n", input.message));
    body
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
