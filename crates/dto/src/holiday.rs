//! Public holidays: days without working time. They are left out of the targets and the reports.
//!
//! The calendar file is read on the server (see `server/src/api/holidays.rs`).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::validate_name;

/// A day without working time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holiday {
    pub id: i64,
    pub date: NaiveDate,
    pub name: String,
}

/// Request body for creating or replacing a holiday.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolidayInput {
    pub date: NaiveDate,
    pub name: String,
}

impl HolidayInput {
    /// Trims the name so stored values are canonical.
    pub fn normalize(&mut self) {
        self.name = self.name.trim().to_string();
    }

    /// Checks the input. Call [`HolidayInput::normalize`] first.
    pub fn validate(&self) -> Result<(), String> {
        validate_name(&self.name)
    }
}

/// Request body for importing holidays from an iCalendar (`.ics`) address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolidayImport {
    pub url: String,
}

/// Request body for importing holidays from the pasted content of an iCalendar (`.ics`) file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolidayImportText {
    pub text: String,
}

/// What an import did: new holidays added, and days that already had one (kept as they are).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolidayImportResult {
    pub added: u32,
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
