//! Shared types and helpers. Import the common names with `use crate::common::prelude::*`.

/// Language codes and the app languages.
mod lang;
/// The state that the app keeps in the browser.
mod state;
/// Browser helpers.
mod util;

/// The names that pages and components import: the languages, the state and the browser helpers.
pub mod prelude {
    pub use crate::common::lang::Language;
    pub use crate::common::state::AppState;
    pub use crate::common::util::{
        document, get_preferred_language, prefers_dark, remove_classes, set_classes, window,
    };
}
