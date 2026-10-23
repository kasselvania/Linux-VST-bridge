//! Shared rendering for the sample preview and the installed Bridge client.
#[cfg(feature = "samples")]
pub mod preview;
pub mod presentation;
#[cfg(feature = "live")]
pub mod live;

/// What a press changed, for the native window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Refused,
    Changed,
    Navigated,
}

pub use makepad_product_ui;
pub use makepad_widgets;
#[cfg(any(feature = "live", feature = "samples"))]
pub mod ui;
