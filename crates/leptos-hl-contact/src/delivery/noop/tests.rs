// tests.rs — unit tests for the parent module.

use super::*;
fn sample_input() -> ContactInput {
        ContactInput::from_raw(
            "Bob".into(),
            "bob@example.com".into(),
            Some("Test".into()),
            "Hello, world!".into(),
            String::new(),
        )
    }

    #[tokio::test]
    async fn noop_delivery_succeeds() {
        let result = NoopDelivery.deliver(sample_input()).await;
        assert!(result.is_ok());
    }
