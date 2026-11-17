//! Public holidays: days without working time. They are left out of the targets and the reports.
//!
//! The calendar file is read on the server (see `server/src/api/holidays.rs`).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::validate_name;

/// A day without working time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holiday {
    /// Id of the holiday, assigned by the server.
    pub id: i64,
    /// The day without working time.
    pub date: NaiveDate,
    /// Name of the holiday, for example "Christmas Day".
    pub name: String,
}

/// Request body for creating or replacing a holiday.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolidayInput {
    /// The day of the holiday.
    pub date: NaiveDate,
    /// Name of the holiday. Must not be blank.
    pub name: String,
}

impl HolidayInput {
    /// Trims the name so stored values are canonical.
    /// Trims the name so stored values are canonical.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::NaiveDate;
    /// use dto::HolidayInput;
    ///
    /// let mut input = HolidayInput { date: NaiveDate::from_ymd_opt(2026, 12, 25).unwrap(), name: "  Christmas ".into() };
    /// input.normalize();
    /// assert_eq!(input.name, "Christmas");
    /// assert!(input.validate().is_ok());
    /// ```
    pub fn normalize(&mut self) {
        self.name = self.name.trim().to_string();
    }

    /// Checks the input. Call [`HolidayInput::normalize`] first.
    /// Checks that the name is not blank and not too long. Call [`HolidayInput::normalize`] first.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::NaiveDate;
    /// use dto::HolidayInput;
    ///
    /// let date = NaiveDate::from_ymd_opt(2026, 12, 25).unwrap();
    /// assert!(HolidayInput { date, name: "Christmas".into() }.validate().is_ok());
    /// assert!(HolidayInput { date, name: "   ".into() }.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        validate_name(&self.name)
    }
}

/// Request body for importing holidays from an iCalendar (`.ics`) address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolidayImport {
    /// Address of an `.ics` file, for example a public holiday calendar.
    pub url: String,
}

/// Request body for importing holidays from the pasted content of an iCalendar (`.ics`) file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolidayImportText {
    /// The content of an `.ics` file.
    pub text: String,
}

/// What an import did: new holidays added, and days that already had one (kept as they are).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolidayImportResult {
    /// Number of holidays that were added.
    pub added: u32,
    /// Number of days that already had a holiday. They keep their existing entry.
    pub existing: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_name_must_not_be_blank() {
        let mut input = HolidayInput {
            date: NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            name: "  ".into(),
        };
        input.normalize();
        assert!(input.validate().is_err());
    }
}
