//! The browser harness: a mounted form, a stubbed `fetch`, a clock the test
//! controls, and vendor globals.
//!
//! Install stubs before mounting, and declare them before the form, so the
//! form is dropped (and its timers and widget cleaned up) while they are still
//! in place.

mod clock;
mod fetch;
mod mount;
mod vendor;

pub use clock::Clock;
pub use fetch::FetchStub;
pub use mount::{Mounted, document, settle};
pub use vendor::VendorStub;
