//! Components: the parts of the pages that are used more than once. Each module has its own description.

/// The month calendar of the start page.
pub mod calendar;
/// The overview of the selected day and month.
pub mod overview;
/// Applying the colour theme to the page.
mod theme;
/// The form to book time on a day, and the records of that day.
pub mod time_recorder;
/// Style constants shared by all pages, so that the look stays the same everywhere.
pub mod ui;

/// The components that the pages import.
pub mod prelude {
    pub use crate::components::calendar::Calendar;
    pub use crate::components::overview::Overview;
    pub use crate::components::theme::set_theme;
    pub use crate::components::time_recorder::TimeRecorder;
}
