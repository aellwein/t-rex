use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{MAX_NOTE_LEN, MINUTES_PER_DAY};

/// One time record: work on a project or an absence, with the break taken within it.
///
/// The record occupies `start_minute` for `minutes + break_minutes` minutes on `date`.
/// `minutes` is the booked work time; the break is unpaid and not counted as work.
/// Times are minutes after midnight, so seconds never take part. Records on the same day
/// never overlap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: i64,
    pub date: NaiveDate,
    /// Minutes after midnight. `None` for entries recorded before start times existed.
    pub start_minute: Option<u32>,
    pub minutes: u32,
    /// Unpaid break taken within the record.
    pub break_minutes: u32,
    pub project_id: Option<i64>,
    pub absence_kind_id: Option<i64>,
    pub note: String,
}

/// Request body for creating or replacing a time entry.
///
/// Exactly one of `project_id` and `absence_kind_id` must be set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeEntryInput {
    pub date: NaiveDate,
    /// Minutes after midnight, `0..1440`.
    pub start_minute: u32,
    /// Booked work time in minutes.
    pub minutes: u32,
    /// Unpaid break within the record. Omitted means no break.
    #[serde(default)]
    pub break_minutes: u32,
    #[serde(default)]
    pub project_id: Option<i64>,
    #[serde(default)]
    pub absence_kind_id: Option<i64>,
    #[serde(default)]
    pub note: String,
}

impl TimeEntryInput {
    /// Trims the note. Call before [`TimeEntryInput::validate`].
    pub fn normalize(&mut self) {
        self.note = self.note.trim().to_string();
    }

    /// Minutes after midnight at which the record ends (exclusive), break included.
    pub fn end_minute(&self) -> u32 {
        self.start_minute
            .saturating_add(self.minutes)
            .saturating_add(self.break_minutes)
    }

    /// Checks the input without touching the database.
    pub fn validate(&self) -> Result<(), String> {
        if self.minutes == 0 {
            return Err("minutes must be at least 1".into());
        }
        if self.break_minutes > MINUTES_PER_DAY {
            return Err(format!("break_minutes must be at most {MINUTES_PER_DAY}"));
        }
        if self.start_minute >= MINUTES_PER_DAY {
            return Err(format!("start_minute must be below {MINUTES_PER_DAY}"));
        }
        if self.end_minute() > MINUTES_PER_DAY {
            return Err("a time record must end on the same day".into());
        }
        match (self.project_id, self.absence_kind_id) {
            (Some(_), None) | (None, Some(_)) => {}
            _ => {
                return Err("an entry needs exactly one of project_id or absence_kind_id".into());
            }
        }
        if self.note.chars().count() > MAX_NOTE_LEN {
            return Err(format!("note must be at most {MAX_NOTE_LEN} characters"));
        }
        Ok(())
    }
}

/// Query parameters for listing entries. Both bounds are inclusive.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

/// Parses a clock time `HH:MM` (24-hour) into minutes after midnight.
pub fn parse_clock(value: &str) -> Option<u32> {
    let (hours, minutes) = value.trim().split_once(':')?;
    if hours.len() != 2 || minutes.len() != 2 {
        return None;
    }
    let hours: u32 = hours.parse().ok()?;
    let minutes: u32 = minutes.parse().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(hours * 60 + minutes)
}

/// Formats minutes after midnight as `HH:MM`. The end of the day is `24:00`.
pub fn format_clock(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// Unit of a typed duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationUnit {
    Minutes,
    Hours,
}

/// Converts a typed duration to whole minutes, rounding to the nearest minute.
///
/// Returns `None` if the text is not a non-negative number. Zero is returned as `Some(0)`.
pub fn duration_minutes(amount: &str, unit: DurationUnit) -> Option<u32> {
    let value: f64 = amount.trim().replace(',', ".").parse().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let minutes = match unit {
        DurationUnit::Minutes => value,
        DurationUnit::Hours => value * 60.0,
    };
    let minutes = minutes.round();
    if minutes > f64::from(MINUTES_PER_DAY) {
        return None;
    }
    Some(minutes as u32)
}

/// Shows whole minutes in the given unit, e.g. 90 minutes as `1.5` hours.
pub fn duration_text(minutes: u32, unit: DurationUnit) -> String {
    match unit {
        DurationUnit::Minutes => minutes.to_string(),
        DurationUnit::Hours => {
            let hours = (f64::from(minutes) / 60.0 * 100.0).round() / 100.0;
            hours.to_string()
        }
    }
}

/// A time record ready to be saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeBlock {
    pub start_minute: u32,
    /// Booked work time.
    pub minutes: u32,
    pub break_minutes: u32,
}

impl TimeBlock {
    /// Minutes after midnight at which the record ends (exclusive), break included.
    pub fn end_minute(self) -> u32 {
        self.start_minute + self.minutes + self.break_minutes
    }
}

/// Why the time fields of a record do not form a valid record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockError {
    StartMissing,
    StartInvalid,
    EndInvalid,
    DurationMissing,
    DurationInvalid,
    EndNotAfterStart,
    DurationMismatch,
    BreakTooLong,
    PastMidnight,
}

/// Works out the record from the time fields. Start, end and duration are related by
/// `end = start + duration + break`: any two of start, end and duration are enough, and if
/// all three are given they must agree.
pub fn resolve_block(
    start: &str,
    end: &str,
    amount: &str,
    unit: DurationUnit,
    break_minutes: u32,
) -> Result<TimeBlock, BlockError> {
    if start.trim().is_empty() {
        return Err(BlockError::StartMissing);
    }
    let start_minute = parse_clock(start).ok_or(BlockError::StartInvalid)?;

    let from_amount = if amount.trim().is_empty() {
        None
    } else {
        match duration_minutes(amount, unit) {
            Some(minutes) if minutes > 0 => Some(minutes),
            _ => return Err(BlockError::DurationInvalid),
        }
    };
    let from_end = if end.trim().is_empty() {
        None
    } else {
        Some(parse_clock(end).ok_or(BlockError::EndInvalid)?)
    };

    let minutes = match (from_amount, from_end) {
        (Some(minutes), Some(end_minute)) => {
            if end_minute <= start_minute {
                return Err(BlockError::EndNotAfterStart);
            }
            if end_minute - start_minute != minutes + break_minutes {
                return Err(BlockError::DurationMismatch);
            }
            minutes
        }
        (Some(minutes), None) => minutes,
        (None, Some(end_minute)) => {
            if end_minute <= start_minute {
                return Err(BlockError::EndNotAfterStart);
            }
            let span = end_minute - start_minute;
            if span < break_minutes {
                return Err(BlockError::BreakTooLong);
            }
            if span == break_minutes {
                return Err(BlockError::DurationInvalid);
            }
            span - break_minutes
        }
        (None, None) => return Err(BlockError::DurationMissing),
    };

    let block = TimeBlock {
        start_minute,
        minutes,
        break_minutes,
    };
    if block.end_minute() > MINUTES_PER_DAY {
        return Err(BlockError::PastMidnight);
    }
    Ok(block)
}

/// The end time that follows from a start, a duration and a break, if it stays on the day.
pub fn derive_end(
    start: &str,
    amount: &str,
    unit: DurationUnit,
    break_minutes: u32,
) -> Option<String> {
    let start_minute = parse_clock(start)?;
    let minutes = duration_minutes(amount, unit)?;
    let end = start_minute
        .checked_add(minutes)?
        .checked_add(break_minutes)?;
    (end <= MINUTES_PER_DAY).then(|| format_clock(end))
}

/// The work duration that follows from a start, an end and a break, if the end is after the
/// start and the break fits in between.
pub fn derive_amount(
    start: &str,
    end: &str,
    unit: DurationUnit,
    break_minutes: u32,
) -> Option<String> {
    let start_minute = parse_clock(start)?;
    let end_minute = parse_clock(end)?;
    let span = end_minute.checked_sub(start_minute)?;
    span.checked_sub(break_minutes)
        .map(|work| duration_text(work, unit))
}

/// Whether `block` shares any minute with a record of the same day, breaks included. Records
/// without a start time never conflict.
pub fn overlaps_any(block: TimeBlock, entries: &[TimeEntry]) -> bool {
    entries.iter().any(|entry| {
        entry.start_minute.is_some_and(|start| {
            let entry_end = start + entry.minutes + entry.break_minutes;
            start < block.end_minute() && entry_end > block.start_minute
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 5).unwrap()
    }

    fn project_entry(minutes: u32) -> TimeEntryInput {
        TimeEntryInput {
            date: date(),
            start_minute: 0,
            minutes,
            break_minutes: 0,
            project_id: Some(1),
            absence_kind_id: None,
            note: String::new(),
        }
    }

    #[test]
    fn zero_minutes_are_rejected() {
        assert!(project_entry(0).validate().is_err());
    }

    #[test]
    fn a_full_day_is_accepted_but_more_is_not() {
        assert!(project_entry(MINUTES_PER_DAY).validate().is_ok());
        assert!(project_entry(MINUTES_PER_DAY + 1).validate().is_err());
    }

    #[test]
    fn work_and_break_must_end_on_the_same_day() {
        let late = TimeEntryInput {
            start_minute: 23 * 60,
            ..project_entry(45)
        };
        assert!(late.validate().is_ok());
        let too_late = TimeEntryInput {
            start_minute: 23 * 60,
            break_minutes: 15,
            ..project_entry(46)
        };
        assert!(too_late.validate().is_err());
    }

    #[test]
    fn start_must_be_within_the_day() {
        let at_midnight_next_day = TimeEntryInput {
            start_minute: MINUTES_PER_DAY,
            ..project_entry(1)
        };
        assert!(at_midnight_next_day.validate().is_err());
    }

    #[test]
    fn entry_needs_exactly_one_reference() {
        let both = TimeEntryInput {
            absence_kind_id: Some(2),
            ..project_entry(30)
        };
        let neither = TimeEntryInput {
            project_id: None,
            ..project_entry(30)
        };
        assert!(both.validate().is_err());
        assert!(neither.validate().is_err());
    }

    #[test]
    fn project_or_absence_alone_is_accepted() {
        let absence = TimeEntryInput {
            project_id: None,
            absence_kind_id: Some(3),
            ..project_entry(480)
        };
        assert!(project_entry(90).validate().is_ok());
        assert!(absence.validate().is_ok());
    }

    #[test]
    fn overlong_note_is_rejected() {
        let mut entry = project_entry(30);
        entry.note = "n".repeat(MAX_NOTE_LEN + 1);
        assert!(entry.validate().is_err());
    }

    #[test]
    fn date_is_serialized_as_iso_day() {
        let json = serde_json::to_value(project_entry(30)).unwrap();
        assert_eq!(json["date"], "2026-11-05");
    }

    #[test]
    fn optional_fields_may_be_omitted_in_json() {
        let parsed: TimeEntryInput = serde_json::from_str(
            r#"{"date":"2026-11-05","start_minute":540,"minutes":30,"project_id":4}"#,
        )
        .unwrap();
        assert_eq!(parsed.start_minute, 540);
        assert_eq!(parsed.break_minutes, 0);
        assert_eq!(parsed.project_id, Some(4));
        assert_eq!(parsed.absence_kind_id, None);
        assert_eq!(parsed.note, "");
    }

    #[test]
    fn query_bounds_are_optional() {
        let parsed: EntryQuery = serde_json::from_str(r#"{"from":"2026-11-01"}"#).unwrap();
        assert_eq!(parsed.from, NaiveDate::from_ymd_opt(2026, 11, 1));
        assert_eq!(parsed.to, None);
    }

    #[test]
    fn clock_times_convert_both_ways() {
        assert_eq!(parse_clock("00:00"), Some(0));
        assert_eq!(parse_clock("09:30"), Some(570));
        assert_eq!(parse_clock("23:59"), Some(1439));
        assert_eq!(format_clock(570), "09:30");
        assert_eq!(format_clock(MINUTES_PER_DAY), "24:00");
    }

    #[test]
    fn malformed_clock_times_are_rejected() {
        for bad in ["24:00", "9:30", "12:60", "12-30", "ab:cd", "", "12:30:00"] {
            assert_eq!(parse_clock(bad), None, "{bad} should not parse");
        }
    }

    #[test]
    fn durations_convert_and_round_to_whole_minutes() {
        assert_eq!(duration_minutes("90", DurationUnit::Minutes), Some(90));
        assert_eq!(duration_minutes("1,5", DurationUnit::Hours), Some(90));
        assert_eq!(duration_minutes("0.33", DurationUnit::Hours), Some(20));
        assert_eq!(duration_minutes("-1", DurationUnit::Minutes), None);
        assert_eq!(duration_minutes("abc", DurationUnit::Hours), None);
        assert_eq!(duration_text(90, DurationUnit::Hours), "1.5");
        assert_eq!(duration_text(20, DurationUnit::Hours), "0.33");
        assert_eq!(duration_text(20, DurationUnit::Minutes), "20");
    }

    #[test]
    fn start_and_duration_give_the_end_including_the_break() {
        // 8 hours of work with a 45 minute break: 08:00 to 16:45.
        assert_eq!(
            derive_end("08:00", "8", DurationUnit::Hours, 45).as_deref(),
            Some("16:45")
        );
        assert_eq!(derive_end("23:00", "120", DurationUnit::Minutes, 0), None);
    }

    #[test]
    fn start_and_end_give_the_work_without_the_break() {
        assert_eq!(
            derive_amount("08:00", "16:45", DurationUnit::Hours, 45).as_deref(),
            Some("8")
        );
        assert_eq!(
            derive_amount("08:00", "16:45", DurationUnit::Minutes, 45).as_deref(),
            Some("480")
        );
        assert_eq!(
            derive_amount("10:00", "09:00", DurationUnit::Minutes, 0),
            None
        );
        assert_eq!(
            derive_amount("08:00", "08:30", DurationUnit::Minutes, 45),
            None
        );
    }

    #[test]
    fn any_two_time_fields_resolve_the_record() {
        let expected = TimeBlock {
            start_minute: 480,
            minutes: 480,
            break_minutes: 45,
        };
        assert_eq!(
            resolve_block("08:00", "", "480", DurationUnit::Minutes, 45),
            Ok(expected)
        );
        assert_eq!(
            resolve_block("08:00", "16:45", "", DurationUnit::Minutes, 45),
            Ok(expected)
        );
        assert_eq!(
            resolve_block("08:00", "16:45", "480", DurationUnit::Minutes, 45),
            Ok(expected)
        );
    }

    #[test]
    fn contradicting_fields_are_reported() {
        assert_eq!(
            resolve_block("08:00", "16:45", "420", DurationUnit::Minutes, 45),
            Err(BlockError::DurationMismatch)
        );
        assert_eq!(
            resolve_block("10:00", "09:00", "", DurationUnit::Minutes, 0),
            Err(BlockError::EndNotAfterStart)
        );
        assert_eq!(
            resolve_block("", "10:00", "30", DurationUnit::Minutes, 0),
            Err(BlockError::StartMissing)
        );
        assert_eq!(
            resolve_block("09:00", "", "", DurationUnit::Minutes, 0),
            Err(BlockError::DurationMissing)
        );
        assert_eq!(
            resolve_block("09:00", "", "0", DurationUnit::Minutes, 0),
            Err(BlockError::DurationInvalid)
        );
        assert_eq!(
            resolve_block("08:00", "08:30", "", DurationUnit::Minutes, 45),
            Err(BlockError::BreakTooLong)
        );
        assert_eq!(
            resolve_block("23:30", "", "60", DurationUnit::Minutes, 0),
            Err(BlockError::PastMidnight)
        );
    }

    #[test]
    fn overlap_is_detected_but_touching_records_are_fine() {
        let entry = |start: Option<u32>, minutes: u32, break_minutes: u32| TimeEntry {
            id: 1,
            date: date(),
            start_minute: start,
            minutes,
            break_minutes,
            project_id: Some(1),
            absence_kind_id: None,
            note: String::new(),
        };
        // 08:00-08:00+8h work+45 break = until 16:45.
        let existing = [entry(Some(480), 480, 45), entry(None, 60, 0)];
        assert!(!overlaps_any(
            TimeBlock {
                start_minute: 1005,
                minutes: 60,
                break_minutes: 0
            },
            &existing
        ));
        assert!(overlaps_any(
            TimeBlock {
                start_minute: 1000,
                minutes: 60,
                break_minutes: 0
            },
            &existing
        ));
        assert!(overlaps_any(
            TimeBlock {
                start_minute: 400,
                minutes: 120,
                break_minutes: 0
            },
            &existing
        ));
        assert!(!overlaps_any(
            TimeBlock {
                start_minute: 0,
                minutes: 480,
                break_minutes: 0
            },
            &existing
        ));
    }
}
