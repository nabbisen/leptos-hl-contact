// model.rs — Core data model for contact form submissions.
//
// [`ContactInput`] is an internal model produced on the server after the raw
// server-function arguments have been normalised and validated.  It is never
// serialised to the client.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    config::{SiteFieldKind, SiteFields},
    error::{ContactValidationError, FieldError, FieldErrorCode},
};

// ---------------------------------------------------------------------------
// Limits
// ---------------------------------------------------------------------------

/// Hard ceiling for `message`, in characters (Unicode scalar values).
///
/// UI options and server policy are clamped to it.
pub const MESSAGE_MAX_LEN: usize = 4000;

/// [`MESSAGE_MAX_LEN`] as the `u64` that `validator_derive` 0.20 requires in
/// `length(max = …)`.  Derived, never written out, so the ceiling has exactly
/// one definition.
const MESSAGE_MAX_LEN_U64: u64 = MESSAGE_MAX_LEN as u64;

// ---------------------------------------------------------------------------
// Custom validators
// ---------------------------------------------------------------------------

/// Rejects any string that contains a CR (`\r`) or LF (`\n`) character.
///
/// Used for `name` and `subject` to prevent email header injection.
fn no_newlines(value: &str) -> Result<(), validator::ValidationError> {
    if value.contains('\n') || value.contains('\r') {
        Err(validator::ValidationError::new("no_newlines"))
    } else {
        Ok(())
    }
}

/// Validator for optional string fields: rejects newlines when the value is present.
///
/// The `validator` crate passes the inner `&String` reference for `Option<String>` fields
/// when using a `custom` validator.
fn optional_no_newlines(value: &str) -> Result<(), validator::ValidationError> {
    no_newlines(value)
}

/// Rejects an email domain that no public mailbox could reply to: an address
/// literal (`a@[127.0.0.1]`), a single-label name (`abc@bar`), or an empty
/// label (`a@example.`, `a@.com`, `a@example..com`).
///
/// The domain is everything after the last `@`.  A value without `@` is left
/// to the `email` syntax check, so it is not reported twice.
fn reply_to_domain(value: &str) -> Result<(), validator::ValidationError> {
    let Some((_, domain)) = value.rsplit_once('@') else {
        return Ok(());
    };
    let labels: Vec<&str> = domain.split('.').collect();
    if domain.starts_with('[') || labels.len() < 2 || labels.iter().any(|l| l.is_empty()) {
        Err(validator::ValidationError::new("email"))
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ContactInput
// ---------------------------------------------------------------------------

/// Validated, normalised contact form submission.
///
/// This type lives entirely on the server.  SMTP credentials, the recipient
/// address, and any other secrets never appear here.
///
/// # Security
///
/// `name` and `subject` are validated to contain no newline characters to
/// prevent email header injection.  `email` is used as the `Reply-To` header
/// only — never as the `From` address.
///
/// # Feature flags
///
/// This type is available under both `ssr` and `hydrate` features so that
/// client-side code can reference its shape (e.g. for type-checking form
/// field names) without pulling in server-only dependencies.
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct ContactInput {
    /// Display name of the person submitting the form.
    ///
    /// Constraints: 1–80 characters, no newline characters.
    #[validate(
        length(min = 1, max = 80),
        custom(function = "no_newlines", code = "no_newlines")
    )]
    pub name: String,

    /// Email address used as the `Reply-To` header.
    ///
    /// Constraints: a syntactically valid address (the `validator` crate), at
    /// most 254 characters, whose domain is a name of at least two non-empty
    /// labels; address literals such as `[127.0.0.1]` are rejected.
    #[validate(
        email,
        length(max = 254),
        custom(function = "reply_to_domain", code = "email")
    )]
    pub email: String,

    /// Optional subject line for the enquiry.
    ///
    /// Constraints: 0–120 characters, no newline characters.
    #[validate(
        length(max = 120),
        custom(function = "optional_no_newlines", code = "no_newlines")
    )]
    pub subject: Option<String>,

    /// Body of the enquiry in plain text.
    ///
    /// Constraints: 1 to [`MESSAGE_MAX_LEN`] characters.
    #[validate(length(min = 1, max = MESSAGE_MAX_LEN_U64))]
    pub message: String,

    /// Honeypot field — must be empty.
    ///
    /// This field is invisible to genuine users.  A non-empty value indicates
    /// an automated submission.  The server silently succeeds to avoid leaking
    /// detection logic to bots.
    #[serde(default)]
    pub website: String,
}

impl ContactInput {
    /// Construct a new [`ContactInput`] from raw server-function arguments.
    ///
    /// Performs trimming (whitespace normalisation) before storing values.
    /// Call [`ContactInput::check_honeypot`] and then [`Validate::validate`]
    /// after construction to complete the security checks.
    pub fn from_raw(
        name: String,
        email: String,
        subject: Option<String>,
        message: String,
        website: String,
    ) -> Self {
        Self {
            name: name.trim().to_owned(),
            email: email.trim().to_owned(),
            subject: subject
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty()),
            message: message.trim().to_owned(),
            website,
        }
    }

    /// Return `Err(ContactValidationError::HoneypotTriggered)` if the honeypot
    /// field is non-empty.
    ///
    /// This must be checked **before** returning any validation errors to the
    /// caller, so that bots cannot distinguish honeypot failures from genuine
    /// validation failures.
    pub fn check_honeypot(&self) -> Result<(), ContactValidationError> {
        if !self.website.trim().is_empty() {
            tracing::warn!("contact form honeypot triggered");
            return Err(ContactValidationError::HoneypotTriggered);
        }
        Ok(())
    }

    /// Run server-side validation via the [`Validate`] derive.
    ///
    /// Returns a structured error string suitable for logging (not for display
    /// to end-users).
    pub fn validate_input(&self) -> Result<(), ContactValidationError> {
        self.validate()
            .map_err(|e| ContactValidationError::InvalidInput(e.to_string()))
    }

    /// Run server-side validation and return per-field errors safe for client display.
    ///
    /// Unlike [`validate_input`](Self::validate_input), which returns an opaque
    /// server-internal message, this method returns a
    /// [`ContactFieldErrors`](crate::error::ContactFieldErrors) value with a
    /// generic human-readable message for each failed field.
    pub fn validate_fields(&self) -> crate::error::ContactFieldErrors {
        use crate::error::{ContactField, FieldError};
        use validator::Validate as _;

        let mut out = crate::error::ContactFieldErrors::default();

        if let Err(ve) = self.validate() {
            for (name, errors) in ve.field_errors() {
                // A length error is reported before any other on the same field,
                // so a visitor who pasted something far too long is told why
                // (RFC 010 D2), whatever order `validator` lists the errors in.
                let chosen = errors
                    .iter()
                    .find(|e| e.code == "length")
                    .or_else(|| errors.first());
                let (Some(field), Some(first)) = (contact_field(name.as_ref()), chosen) else {
                    continue;
                };

                // `validator` reports a blank required field as `length` with
                // `min: 1`, indistinguishable from a value that is merely too
                // short.  The two need different sentences, so the emptiness
                // is taken from the input itself.
                let blank = match field {
                    ContactField::Name => self.name.is_empty(),
                    ContactField::Message => self.message.is_empty(),
                    // `subject` is `None` when blank, so it never reaches a
                    // length violation empty; `email` has no minimum.
                    ContactField::Email | ContactField::Subject => false,
                };
                let code = if blank && first.code == "length" {
                    crate::error::FieldErrorCode::Required
                } else {
                    field_error_code(first)
                };

                let err = Some(FieldError::Code(code));
                match field {
                    ContactField::Name => out.name = err,
                    ContactField::Email => out.email = err,
                    ContactField::Subject => out.subject = err,
                    ContactField::Message => out.message = err,
                }
            }
        }
        out
    }

    /// Resolve the effective subject line, falling back to a default when the
    /// caller did not supply one or it was blank after trimming.
    pub fn effective_subject(&self, fallback: &str) -> String {
        self.subject
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(fallback)
            .to_owned()
    }
}

/// Resolve a `validator` field name to a [`ContactField`](crate::error::ContactField).
///
/// Naming the field once is what lets the two matches in
/// [`validate_fields`](ContactInput::validate_fields) be exhaustive over the
/// enum, so a field added to [`ContactInput`] is a compile error there rather
/// than a silently missing error message.
fn contact_field(name: &str) -> Option<crate::error::ContactField> {
    use crate::error::ContactField;
    match name {
        "name" => Some(ContactField::Name),
        "email" => Some(ContactField::Email),
        "subject" => Some(ContactField::Subject),
        "message" => Some(ContactField::Message),
        _ => None,
    }
}

/// Map one `validator` error to a [`FieldErrorCode`](crate::error::FieldErrorCode).
///
/// `validator` reports an empty required string as `length` with `min: 1`, so
/// `Required` is never produced here; the server policy emits it.  A `length`
/// error without `max` is rendered with `usize::MAX`, which no rule in this
/// crate produces.
fn field_error_code(e: &validator::ValidationError) -> crate::error::FieldErrorCode {
    use crate::error::FieldErrorCode;

    let param = |k: &str| e.params.get(k).and_then(|v| v.as_u64()).map(|n| n as usize);

    match e.code.as_ref() {
        "length" => FieldErrorCode::Length {
            min: param("min").unwrap_or(0),
            max: param("max").unwrap_or(usize::MAX),
        },
        "email" => FieldErrorCode::Format,
        "no_newlines" => FieldErrorCode::LineBreaks,
        _ => FieldErrorCode::Format,
    }
}

// ---------------------------------------------------------------------------
// Site-defined fields (RFC 015)
// ---------------------------------------------------------------------------

/// One answered site-defined field, as delivery receives it.
///
/// `label` comes from the site's [`SiteFields`] definition, never from the
/// request, so a sender cannot choose what the delivered message says a field
/// is called.
///
/// Like `message`, `value` is personal data: never log it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteFieldValue {
    /// The field's key from the definition.
    pub key: String,
    /// The field's label from the definition.
    pub label: String,
    /// The trimmed value.  For a `Choice`, the choice's key.
    pub value: String,
    /// For a `Choice`, the chosen option's label; `None` for other kinds.
    pub value_label: Option<String>,
}

/// What [`validate_site_fields`] decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiteFieldsOutcome {
    /// Every value is valid: the answered fields, in definition order.  A
    /// blank optional field is left out.
    Valid(Vec<SiteFieldValue>),
    /// Errors per field, keyed by the definition's key.
    Invalid(BTreeMap<String, FieldError>),
    /// The request carried more keys than the definition allows, or a key the
    /// definition does not have.  There is no field to attach an error to,
    /// and saying which key was wrong would teach a crafted request the
    /// definition, so the submission is refused as a whole.
    Refused,
}

/// Validate the raw `fields[…]` map of a submission against the site's
/// definition.
///
/// In order:
/// 1. **Too many keys** (more than [`SiteFields::MAX`]): [`Refused`](SiteFieldsOutcome::Refused).
/// 2. **A key the definition lacks**: `Refused`.  Only defined keys pass, so
///    a crafted key of any shape is refused here, whatever characters it has.
/// 3. **Each defined field, in definition order**: the value is trimmed and a
///    blank one is absent.  Absent and required is `Required`; absent and
///    optional is left out.  Otherwise `Line` fails on a CR or LF
///    (`LineBreaks`), `Line` and `Text` on more than `max_len` characters
///    (`Length`, which is checked first), and `Choice` on a value that is not
///    a listed key (`Format`).
///
/// A refusal is logged at `warn` with a **count** only.  A request's keys and
/// values are attacker text and personal data, so they are never logged.
pub fn validate_site_fields(
    def: &SiteFields,
    raw: Option<&BTreeMap<String, String>>,
) -> SiteFieldsOutcome {
    let empty = BTreeMap::new();
    let raw = raw.unwrap_or(&empty);

    if raw.len() > SiteFields::MAX {
        tracing::warn!(count = raw.len(), "site fields refused: too many keys");
        return SiteFieldsOutcome::Refused;
    }
    let unknown = raw.keys().filter(|key| def.get(key).is_none()).count();
    if unknown > 0 {
        tracing::warn!(unknown_keys = unknown, "site fields refused: unknown keys");
        return SiteFieldsOutcome::Refused;
    }

    let mut values = Vec::new();
    let mut errors = BTreeMap::new();
    for field in def.iter() {
        let value = raw
            .get(&field.key)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());
        let Some(value) = value else {
            if field.required {
                errors.insert(
                    field.key.clone(),
                    FieldError::Code(FieldErrorCode::Required),
                );
            }
            continue;
        };

        let length = FieldErrorCode::Length {
            min: usize::from(field.required),
            max: field.max_len,
        };
        let mut value_label = None;
        let error = match &field.kind {
            SiteFieldKind::Line if value.chars().count() > field.max_len => Some(length),
            SiteFieldKind::Line if value.contains(['\r', '\n']) => Some(FieldErrorCode::LineBreaks),
            SiteFieldKind::Text if value.chars().count() > field.max_len => Some(length),
            SiteFieldKind::Choice(choices) => {
                match choices.iter().find(|choice| choice.key == value) {
                    Some(choice) => {
                        value_label = Some(choice.label.clone());
                        None
                    }
                    None => Some(FieldErrorCode::Format),
                }
            }
            _ => None,
        };
        match error {
            Some(code) => {
                errors.insert(field.key.clone(), FieldError::Code(code));
            }
            None => values.push(SiteFieldValue {
                key: field.key.clone(),
                label: field.label.clone(),
                value: value.to_owned(),
                value_label,
            }),
        }
    }

    if errors.is_empty() {
        SiteFieldsOutcome::Valid(values)
    } else {
        SiteFieldsOutcome::Invalid(errors)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
