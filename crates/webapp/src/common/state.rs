//! The state that the app keeps in the browser's local storage: the theme, the language, the selected day
//! and the header's email display.

use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::common::prelude::*;

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
/// The state kept in the browser: theme, language, selected day, and the header's email display.
pub struct AppState {
    /// `system`, or the id of a colour theme. See `dto::theme`.
    pub theme: String,
    pub language: Language,
    pub selected_date: NaiveDate,
    /// Show the email instead of the name in the header. Mirrors the saved setting.
    #[serde(default)]
    pub show_email: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            theme: dto::theme::SYSTEM.to_string(),
            language: get_preferred_language(),
            selected_date: Local::now().naive_local().date(),
            show_email: false,
        }
    }
}
