// tests.rs — unit tests for the parent module.

use super::*;
use crate::delivery::noop::NoopDelivery;
    use std::sync::Arc;

    #[test]
    fn delivery_context_fn_is_clone() {
        let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
        let ctx = delivery_context_fn(delivery);
        // Should be cloneable (required by both Axum handler sites).
        let _ctx2 = ctx.clone();
    }
