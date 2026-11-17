//! Data transfer objects for the t-rex REST API.
//!
//! Shared by the `server` (persistence and HTTP) and the `webapp` (WASM client),
//! so this crate must stay free of server-only dependencies.
//!
//! # Modules
//!
//! - [`entry`]: time records, the time formats and the rules that turn start, end, duration and
//!   break into a record.
//! - [`report`]: targets, actual time, overtime and the balances of days and months.
//! - [`calendar`]: date arithmetic for months.
//! - [`settings`]: per-user settings and the signed-hours format of the carry-over.
//! - [`theme`]: the built-in colour themes.
//! - [`page`]: the paging of lists.
//! - [`project`], [`absence`], [`holiday`]: the names that time is booked against and the days off.
//! - [`token`], [`me`], [`version`], [`error`]: the account, the tokens and the errors of the API.
//!
//! # Example
//!
//! A working day that starts at 09:00, works 7.5 hours and takes the default 45 minute break ends
//! at 17:15:
//!
//! ```
//! use dto::{format_clock, resolve_block, DurationUnit, Settings};
//!
//! let settings = Settings::default();
//! let block = resolve_block("09:00", "", "7.5", DurationUnit::Hours, settings.break_minutes)
//!     .expect("a valid record");
//! assert_eq!(format_clock(block.end_minute()), "17:15");
//! ```

pub mod absence;
pub mod calendar;
pub mod entry;
pub mod error;
pub mod holiday;
pub mod me;
pub mod page;
pub mod pending_start;
pub mod project;
pub mod report;
pub mod settings;
pub mod theme;
pub mod token;
pub mod version;

pub use absence::{AbsenceKind, AbsenceKindInput};
pub use entry::{
    derive_amount, derive_end, duration_minutes, duration_text, format_clock, overlaps_any,
    parse_clock, resolve_block, BlockError, DurationUnit, EntryQuery, TimeBlock, TimeEntry,
    TimeEntryInput,
};
pub use error::ErrorBody;
pub use holiday::{Holiday, HolidayImport, HolidayImportResult, HolidayImportText, HolidayInput};
pub use me::Me;
pub use page::{Page, PageQuery, DEFAULT_LIMIT, MAX_LIMIT};
pub use pending_start::PendingStart;
pub use project::{Project, ProjectInput};
pub use report::{
    actual_minutes, break_total, daily_balances, is_working_day, running_overtime, tracking_start,
    weekdays_between, DayBalance, MonthBalance,
};
pub use settings::{format_signed_hours, parse_signed_hours, Language, Settings};
pub use theme::{builtin_themes, ThemeInfo};
pub use token::{CliTokenRequest, CreatedToken, TokenInfo, TokenInput};
pub use version::VersionInfo;

/// Longest accepted name for projects and absence kinds, in characters.
pub const MAX_NAME_LEN: usize = 100;
/// Longest accepted note on a time entry, in characters.
pub const MAX_NOTE_LEN: usize = 500;
/// Upper bound for a single entry and for a working day, in minutes.
pub const MINUTES_PER_DAY: u32 = 24 * 60;
/// Colour used when none is given (matches the calendar's teal accent).
pub const DEFAULT_COLOR: &str = "#0d9488";

/// Default for the `color` field during deserialization.
pub(crate) fn default_color() -> String {
    DEFAULT_COLOR.to_string()
}

/// Checks that a name is not blank and not longer than [`MAX_NAME_LEN`].
pub(crate) fn validate_name(name: &str) -> Result<(), String> {
    let len = name.trim().chars().count();
    if len == 0 {
        return Err("name must not be empty".into());
    }
    if len > MAX_NAME_LEN {
        return Err(format!("name must be at most {MAX_NAME_LEN} characters"));
    }
    Ok(())
}

/// Checks that a colour is a `#rrggbb` hex value.
pub(crate) fn validate_color(color: &str) -> Result<(), String> {
    let valid = color.len() == 7
        && color.starts_with('#')
        && color[1..].chars().all(|c| c.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err("color must be a hex value like #0d9488".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_must_not_be_blank() {
        assert!(validate_name("").is_err());
        assert!(validate_name("   ").is_err());
        assert!(validate_name(" a ").is_ok());
    }

    #[test]
    fn name_length_limit_is_inclusive() {
        assert!(validate_name(&"x".repeat(MAX_NAME_LEN)).is_ok());
        assert!(validate_name(&"x".repeat(MAX_NAME_LEN + 1)).is_err());
    }

    #[test]
    fn color_must_be_seven_char_hex() {
        assert!(validate_color("#0d9488").is_ok());
        assert!(validate_color("#0D9488").is_ok());
        assert!(validate_color("0d9488").is_err());
        assert!(validate_color("#12345").is_err());
        assert!(validate_color("#0d95488").is_err());
        assert!(validate_color("#zzzzzz").is_err());
        assert!(validate_color("teal").is_err());
    }
}
