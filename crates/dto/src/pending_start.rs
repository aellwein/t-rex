use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::MINUTES_PER_DAY;

/// Work that has started on `date` at `start_minute`, but has no time record yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingStart {
    pub date: NaiveDate,
    /// Minutes after midnight, `0..1440`.
    pub start_minute: u32,
}

impl PendingStart {
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
