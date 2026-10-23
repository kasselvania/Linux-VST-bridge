//! Words the service and its windows share: the busy-lock refusal the service
//! writes and a window recognises and names a button in, and how a package
//! version is shown. The manager re-exports them where they used to live
//! (`operator_lock`, `update_feed`).

/// What a musician reads when an action or check met a lock still held after
/// its bounded wait. Nothing was changed; the same step can simply run again.
/// Where the card's next step is known, the window names its button instead
/// (`busy_sentence_naming`); a card's "Retry" can start a new installer attempt.
pub const BUSY_SENTENCE: &str = "The manager was busy finishing another task; run the same step again.";
/// The same sentence naming the button that repeats the step on that card.
pub fn busy_sentence_naming(button: &str) -> String {
    format!("The manager was busy finishing another task; choose \u{201c}{button}\u{201d}.")
}
/// A lock refusal a musician can retry, from the text a receipt or journey
/// code kept of it: the manager's busy refusal, or a bounded lock wait that
/// expired. Other refusals keep their exact reason.
pub fn is_busy_text(text: &str) -> bool {
    text == "operation already running"
        || (text.starts_with("Manager ") && text.contains(" lock Timeout after "))
}

/// `0.12.0test148` is shown as `Test148`, the build number testers know.
pub fn display_version(version: &str) -> String {
    match version.rsplit_once("test") {
        Some((_, number)) if !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()) =>
            format!("Test{number}"),
        _ => version.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busy_refusals_are_recognised_from_their_retained_text() {
        assert!(is_busy_text("operation already running"));
        assert!(is_busy_text("Manager Canonical lock Timeout after 986 attempts"));
        assert!(!is_busy_text("candidate_absent"));
        assert!(!is_busy_text("Manager Registry lock Error after 1 attempts"));
        assert_eq!(busy_sentence_naming("Retry"),
            "The manager was busy finishing another task; choose \u{201c}Retry\u{201d}.");
    }

    #[test]
    fn test_builds_are_shown_by_their_number() {
        assert_eq!(display_version("0.12.0test148"), "Test148");
        assert_eq!(display_version("1.0.0"), "1.0.0");
        assert_eq!(display_version("0.12.0test"), "0.12.0test");
        assert_eq!(display_version("0.12.0preview"), "0.12.0preview");
    }
}
