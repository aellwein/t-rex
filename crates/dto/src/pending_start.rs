//! Pending starts: work that has begun today but has no time record yet.
//!
//! The clients store the start when the user presses start. The record is added later, when the work ends,
//! and takes the start from here.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::MINUTES_PER_DAY;

/// Work that has started on `date` at `start_minute`, but has no time record yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingStart {
    /// The day the work started.
    pub date: NaiveDate,
    /// Minutes after midnight, `0..1440`.
    pub start_minute: u32,
}

impl PendingStart {
    /// Checks that the start is a time of the day, `0..1440` minutes.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::NaiveDate;
    /// use dto::PendingStart;
    ///
    /// let date = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
    /// assert!(PendingStart { date, start_minute: 9 * 60 }.validate().is_ok());
    /// assert!(PendingStart { date, start_minute: 1440 }.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.start_minute >= MINUTES_PER_DAY {
            return Err(format!("start_minute must be below {MINUTES_PER_DAY}"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_must_be_within_the_day() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        assert!(PendingStart {
            date,
            start_minute: 1439
        }
        .validate()
        .is_ok());
        assert!(PendingStart {
            date,
            start_minute: MINUTES_PER_DAY
        }
        .validate()
        .is_err());
    }
}
