//! Helpers for the reports: the break of a record counts toward the break total, and is
//! not part of the work time.

use std::collections::BTreeSet;

use chrono::{Datelike, NaiveDate};

use crate::{AbsenceKind, TimeEntry};

/// Sum of the breaks of the records whose day matches `in_period`.
pub fn break_total<F>(entries: &[TimeEntry], in_period: F) -> u32
where
    F: Fn(NaiveDate) -> bool,
{
    entries
        .iter()
        .filter(|entry| in_period(entry.date))
        .map(|entry| entry.break_minutes)
        .sum()
}

/// Whether a day has working time: Monday to Friday, and not a public holiday.
pub fn is_working_day(day: NaiveDate, holidays: &BTreeSet<NaiveDate>) -> bool {
    day.weekday().num_days_from_monday() < 5 && !holidays.contains(&day)
}

/// Working days between `from` and `to`, both included: Monday to Friday without the holidays.
pub fn weekdays_between(from: NaiveDate, to: NaiveDate, holidays: &BTreeSet<NaiveDate>) -> u32 {
    from.iter_days()
        .take_while(|day| *day <= to)
        .filter(|day| is_working_day(*day, holidays))
        .count() as u32
}

/// Actual time in the days from `from` to `to`: project work plus paid absences.
pub fn actual_minutes(
    entries: &[TimeEntry],
    kinds: &[AbsenceKind],
    from: NaiveDate,
    to: NaiveDate,
) -> u32 {
    entries
        .iter()
        .filter(|entry| entry.date >= from && entry.date <= to)
        .filter(|entry| {
            entry.project_id.is_some()
                || kinds
                    .iter()
                    .any(|kind| Some(kind.id) == entry.absence_kind_id && kind.paid)
        })
        .map(|entry| entry.minutes)
        .sum()
}

/// Target and actual of one month, in minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthBalance {
    pub year: i32,
    pub month: u32,
    pub target: u32,
    pub actual: u32,
}

/// Running overtime after each month: the carry-over plus the sum of (actual − target) so far.
pub fn running_overtime(carry_over: i32, months: &[MonthBalance]) -> Vec<i32> {
    let mut running = carry_over;
    months
        .iter()
        .map(|month| {
            running += month.actual as i32 - month.target as i32;
            running
        })
        .collect()
}

/// One day of the running overtime: target, actual, and the running overtime after the day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayBalance {
    pub date: NaiveDate,
    pub target: u32,
    pub actual: u32,
    pub running: i32,
}

/// Every day from `from` to `to`, both included, with its target and actual and the running overtime.
///
/// The target is `daily` on working days and 0 on weekends and holidays. The actual is the project work
/// plus paid absence of that day. The running overtime starts at `carry_over` and adds each day's difference.
pub fn daily_balances(
    carry_over: i32,
    from: NaiveDate,
    to: NaiveDate,
    daily: u32,
    entries: &[TimeEntry],
    kinds: &[AbsenceKind],
    holidays: &BTreeSet<NaiveDate>,
) -> Vec<DayBalance> {
    let mut running = carry_over;
    from.iter_days()
        .take_while(|day| *day <= to)
        .map(|day| {
            let target = if is_working_day(day, holidays) {
                daily
            } else {
                0
            };
            let actual = actual_minutes(entries, kinds, day, day);
            running += actual as i32 - target as i32;
            DayBalance {
                date: day,
                target,
                actual,
                running,
            }
        })
        .collect()
}

/// First day of the month of the earliest time entry: where the running overtime starts.
/// `None` if nothing is recorded yet.
pub fn tracking_start(entries: &[TimeEntry]) -> Option<NaiveDate> {
    entries
        .iter()
        .map(|entry| entry.date)
        .min()
        .and_then(|first| first.with_day(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, d).unwrap()
    }

    fn entry(date: NaiveDate, break_minutes: u32) -> TimeEntry {
        TimeEntry {
            id: 1,
            date,
            start_minute: None,
            minutes: 60,
            break_minutes,
            project_id: Some(1),
            absence_kind_id: None,
            note: String::new(),
        }
    }

    #[test]
    fn breaks_add_up_per_record() {
        let entries = [entry(day(2), 45), entry(day(2), 15), entry(day(3), 0)];
        assert_eq!(break_total(&entries, |_| true), 60);
    }

    #[test]
    fn breaks_outside_the_period_are_ignored() {
        let entries = [entry(day(2), 45), entry(day(3), 30)];
        assert_eq!(break_total(&entries, |d| d == day(3)), 30);
    }

    fn none() -> BTreeSet<NaiveDate> {
        BTreeSet::new()
    }

    #[test]
    fn weekdays_skip_saturday_and_sunday() {
        // 2026-11-02 is a Monday; 2026-11-08 is the following Sunday.
        assert_eq!(weekdays_between(day(2), day(8), &none()), 5);
        assert_eq!(weekdays_between(day(7), day(8), &none()), 0);
        assert_eq!(weekdays_between(day(5), day(5), &none()), 1);
    }

    #[test]
    fn holidays_are_not_working_days() {
        let holidays: BTreeSet<NaiveDate> = [day(2), day(4)].into_iter().collect();
        // Monday and Wednesday are holidays: only Tuesday, Thursday and Friday remain.
        assert_eq!(weekdays_between(day(2), day(8), &holidays), 3);
        assert!(!is_working_day(day(2), &holidays));
        assert!(is_working_day(day(3), &holidays));
    }

    #[test]
    fn a_holiday_has_no_target_in_the_running_overtime() {
        let holidays: BTreeSet<NaiveDate> = [day(3)].into_iter().collect();
        let balances = daily_balances(0, day(2), day(4), 480, &[], &[], &holidays);
        assert_eq!(balances[0].target, 480);
        assert_eq!(balances[1].target, 0);
        // The holiday neither owes nor adds time, so the running overtime does not move on it.
        assert_eq!(balances[1].running, balances[0].running);
        assert_eq!(balances[2].running, balances[0].running - 480);
    }

    #[test]
    fn actual_counts_work_and_paid_absence_only() {
        let kinds = [
            AbsenceKind {
                id: 1,
                name: "Vacation".into(),
                color: "#000000".into(),
                paid: true,
            },
            AbsenceKind {
                id: 2,
                name: "Unpaid".into(),
                color: "#000000".into(),
                paid: false,
            },
        ];
        let work = entry(day(2), 0);
        let mut vacation = entry(day(3), 0);
        vacation.project_id = None;
        vacation.absence_kind_id = Some(1);
        vacation.minutes = 480;
        let mut unpaid = vacation.clone();
        unpaid.absence_kind_id = Some(2);
        let entries = [work, vacation, unpaid];
        // 60 minutes of project work plus 480 minutes of paid vacation; unpaid time is left out.
        assert_eq!(actual_minutes(&entries, &kinds, day(1), day(30)), 540);
    }

    #[test]
    fn running_overtime_adds_each_month_to_the_carry_over() {
        // The example from the user: carry-over 8:43, 50 h worked against 40 h target.
        let carry = 8 * 60 + 43;
        let months = [
            MonthBalance {
                year: 2026,
                month: 10,
                target: 40 * 60,
                actual: 40 * 60,
            },
            MonthBalance {
                year: 2026,
                month: 11,
                target: 40 * 60,
                actual: 50 * 60,
            },
        ];
        assert_eq!(
            running_overtime(carry, &months),
            vec![carry, carry + 10 * 60]
        );
        assert_eq!(running_overtime(carry, &[]), Vec::<i32>::new());
    }

    #[test]
    fn running_overtime_follows_the_days_and_the_carry_over() {
        // Carry-over 8:43. A working week of 50 hours against 40 hours target gives 18:43.
        let carry = 8 * 60 + 43;
        let mut entries = Vec::new();
        for offset in 0..5 {
            let mut e = entry(day(2 + offset), 0);
            e.minutes = 600; // 10 hours each day
            entries.push(e);
        }
        let balances = daily_balances(carry, day(2), day(8), 480, &entries, &[], &none());
        assert_eq!(balances.len(), 7);
        // Monday to Friday: 10 h against 8 h each day, so +2 h per day.
        assert_eq!(balances[0].running, carry + 120);
        assert_eq!(balances[4].running, carry + 600);
        // Saturday and Sunday: no target, no work, nothing changes.
        assert_eq!(balances[5].running, carry + 600);
        assert_eq!(balances[6].target, 0);
    }

    #[test]
    fn a_holiday_booked_as_paid_absence_counts_as_worked() {
        let kinds = [AbsenceKind {
            id: 1,
            name: "Holiday".into(),
            color: "#000000".into(),
            paid: true,
        }];
        let mut holiday = entry(day(2), 0);
        holiday.project_id = None;
        holiday.absence_kind_id = Some(1);
        holiday.minutes = 480;
        let balances = daily_balances(0, day(2), day(2), 480, &[holiday], &kinds, &none());
        assert_eq!(balances[0].actual, 480);
        assert_eq!(balances[0].running, 0);
    }

    #[test]
    fn tracking_starts_on_the_first_day_of_the_first_month_with_records() {
        assert_eq!(tracking_start(&[]), None);
        let mut late = entry(day(17), 0);
        late.minutes = 60;
        let mut early = entry(day(3), 0);
        early.minutes = 60;
        assert_eq!(
            tracking_start(&[late, early]),
            NaiveDate::from_ymd_opt(2026, 11, 1)
        );
    }

    #[test]
    fn running_overtime_matches_a_real_month_so_far() {
        // Carry-over 8:43; November 2 to 9 has six working days at 8 hours target.
        // Work 50:17 in total gives 50:17 - 48:00 = +2:17, so the running overtime is 11:00.
        let carry = 8 * 60 + 43;
        let worked = [503u32, 503, 503, 503, 502, 503];
        let entries: Vec<TimeEntry> = [2u32, 3, 4, 5, 6, 9]
            .iter()
            .zip(worked)
            .map(|(&d, minutes)| {
                let mut e = entry(day(d), 0);
                e.minutes = minutes;
                e
            })
            .collect();
        let start = tracking_start(&entries).unwrap();
        let balances = daily_balances(carry, start, day(9), 480, &entries, &[], &none());
        assert_eq!(balances.last().unwrap().running, 11 * 60);
    }
}
