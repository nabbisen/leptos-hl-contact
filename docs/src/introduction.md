# Introduction

`leptos-hl-contact` is a reusable, secure contact form plugin for [Leptos](https://leptos.dev) v0.8.

## What it does

Add a working contact form to any Leptos application.  The form:

- Validates user input on the server (required, length, email format, header injection).
- Filters automated submissions with a honeypot field.
- Delivers enquiries to a configurable email address via SMTP (or any custom backend).
- Works without JavaScript via `<ActionForm/>` progressive enhancement.
- Is accessible by default: labels, ARIA attributes, keyboard navigation, screen-reader feedback.

## What it does not do

- It does not store submissions in a database (you can add this with a custom delivery backend).
- It does not include a CAPTCHA by default (guidance is provided in [Security](./security.md)).
- It does not manage multiple form definitions via a GUI.

## Three-layer architecture

```
┌────────────────────────────────────────┐
│  UI Component  (ContactForm)           │  client + server
├────────────────────────────────────────┤
│  Server Function (submit_contact)      │  server only
├────────────────────────────────────────┤
│  Delivery Backend (ContactDelivery)    │  server only
└────────────────────────────────────────┘
```

This separation means you can swap the delivery backend without touching the UI, and the UI can be styled independently of the server logic.
