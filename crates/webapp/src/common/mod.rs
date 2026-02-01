mod lang;
mod state;
mod util;

pub mod prelude {
    pub use crate::common::lang::Language;
    pub use crate::common::state::AppState;
    pub use crate::common::util::{
        document, get_preferred_language, prefers_dark, remove_classes, set_classes, window,
    };
}
