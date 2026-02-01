//! Calendar date arithmetic shared by the server and the WASM client.

use chrono::{Datelike, NaiveDate};

/// Builds the date `day` in `year`/`month`, clamping `day` to the month's length
/// (so Jan 31 moved one month forward becomes Feb 28, or Feb 29 in a leap year).
///
/// # Panics
/// If `year`/`month` do not form a valid calendar month.
pub fn clamped_date(year: i32, month: u32, day: u32) -> NaiveDate {
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("valid year and month");
    let last_day = first.num_days_in_month() as u32;
    first
        .with_day(day.min(last_day))
        .expect("clamped day must exist")
}

/// Moves a date by whole calendar months, keeping the day where possible.
pub fn add_months(date: NaiveDate, delta: i32) -> NaiveDate {
    let total = date.year() * 12 + date.month0() as i32 + delta;
    let year = total.div_euclid(12);
    let month = total.rem_euclid(12) as u32 + 1;
    clamped_date(year, month, date.day())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn clamps_day_to_end_of_shorter_month() {
        assert_eq!(clamped_date(2026, 2, 31), d(2026, 2, 28));
        assert_eq!(clamped_date(2026, 4, 31), d(2026, 4, 30));
    }

    #[test]
    fn keeps_leap_day_in_leap_year() {
        assert_eq!(clamped_date(2028, 2, 29), d(2028, 2, 29));
    }

    #[test]
    fn clamps_leap_day_in_common_year() {
        assert_eq!(clamped_date(2027, 2, 29), d(2027, 2, 28));
    }

    #[test]
    fn adding_a_month_clamps_jan_31_to_february() {
        assert_eq!(add_months(d(2026, 1, 31), 1), d(2026, 2, 28));
    }

    #[test]
    fn subtracting_a_month_crosses_the_year_boundary() {
        assert_eq!(add_months(d(2026, 1, 15), -1), d(2025, 12, 15));
    }

    #[test]
    fn adding_twelve_months_keeps_month_and_day() {
        assert_eq!(add_months(d(2026, 3, 10), 12), d(2027, 3, 10));
    }

    #[test]
    fn adding_a_year_to_leap_day_clamps() {
        assert_eq!(add_months(d(2028, 2, 29), 12), d(2029, 2, 28));
    }

    #[test]
    fn moving_back_many_months_lands_in_the_right_month() {
        assert_eq!(add_months(d(2026, 11, 30), -23), d(2024, 12, 30));
    }

    #[test]
    fn navigation_never_skips_a_month() {
        // Repeated "next month" from Jan 31 visits every month in order.
        let mut date = d(2026, 1, 31);
        for expected_month in 2..=12 {
            date = add_months(date, 1);
            assert_eq!(date.month(), expected_month);
        }
    }
}
