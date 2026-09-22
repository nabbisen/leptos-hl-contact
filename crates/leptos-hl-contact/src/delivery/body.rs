// delivery/body.rs — the plain-text body every backend sends.
//
// Moved from `delivery/smtp.rs` (RFC 017 D1), unchanged: `delivery-resend`
// (RFC 017 D3) is the second caller.

use crate::model::ContactInput;

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
