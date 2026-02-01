pub mod calendar;
pub mod overview;
mod theme;
pub mod time_recorder;
pub mod ui;

pub mod prelude {
    pub use crate::components::calendar::Calendar;
    pub use crate::components::overview::Overview;
    pub use crate::components::theme::set_theme;
    pub use crate::components::time_recorder::TimeRecorder;
}
