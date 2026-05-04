// tests.rs — unit tests for the parent module.

use super::*;
#[test]
    fn classes_default_is_all_empty() {
        let c = ContactFormClasses::default();
        assert!(c.root.is_empty());
        assert!(c.button.is_empty());
    }

    #[test]
    fn labels_default_has_english_text() {
        let l = ContactFormLabels::default();
        assert!(!l.submit.is_empty());
        assert!(!l.success.is_empty());
    }

    #[test]
    fn options_default_shows_subject() {
        let o = ContactFormOptions::default();
        assert!(o.show_subject);
        assert_eq!(o.max_message_len, 4000);
    }
