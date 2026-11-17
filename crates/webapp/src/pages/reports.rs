//! Reports: booked time as tables, per project, per month or per year.
//!
//! Work is the time on projects, absences are the time off, and the break is the unpaid
//! break of the days that have records. Export uses the browser's print dialog ("Save as PDF").

use std::collections::BTreeSet;

use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use dioxus_i18n::t;
use dto::calendar::add_months;
use dto::{
    actual_minutes, break_total, daily_balances, format_clock, format_signed_hours,
    running_overtime, tracking_start, weekdays_between, AbsenceKind, MonthBalance, Project,
    Settings, TimeEntry,
};

use crate::api;
use crate::common::prelude::window;
use crate::components::calendar::month_name;
use crate::components::overview::format_duration;
use crate::components::ui::{BUTTON, CARD, ERROR_TEXT, INPUT, LABEL, PAGE_TITLE};
use crate::STATE;

#[derive(Clone, Copy, PartialEq, Eq)]
/// The views of the reports page: by project, by month, by year, and by day.
enum Mode {
    Project,
    Month,
    Year,
    Day,
    Details,
}

/// One line of a report: a project, an absence kind, or a total over all of them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Line {
    Project(i64),
    Absence(i64),
    Work,
    Absences,
    Break,
}

/// A period as a predicate over days, with its column or row label.
struct Period {
    label: String,
    contains: Box<dyn Fn(NaiveDate) -> bool>,
}

/// Sums the minutes of the entries that fall in the period and match `belongs`.
fn sum_where(
    entries: &[TimeEntry],
    in_period: &dyn Fn(NaiveDate) -> bool,
    belongs: impl Fn(&TimeEntry) -> bool,
) -> u32 {
    entries
        .iter()
        .filter(|entry| in_period(entry.date) && belongs(entry))
        .map(|entry| entry.minutes)
        .sum()
}

impl Line {
    /// The minutes of this line in the entries of the period.
    fn minutes(self, entries: &[TimeEntry], in_period: &dyn Fn(NaiveDate) -> bool) -> u32 {
        match self {
            Line::Project(id) => sum_where(entries, in_period, |e| e.project_id == Some(id)),
            Line::Absence(id) => sum_where(entries, in_period, |e| e.absence_kind_id == Some(id)),
            Line::Work => sum_where(entries, in_period, |e| e.project_id.is_some()),
            Line::Absences => sum_where(entries, in_period, |e| e.absence_kind_id.is_some()),
            Line::Break => break_total(entries, in_period),
        }
    }
}

/// Name and colour of a line.
fn line_label(line: Line, projects: &[Project], kinds: &[AbsenceKind]) -> (String, Option<String>) {
    match line {
        Line::Project(id) => projects
            .iter()
            .find(|p| p.id == id)
            .map(|p| (p.name.clone(), Some(p.color.clone())))
            .unwrap_or_else(|| ("?".into(), None)),
        Line::Absence(id) => kinds
            .iter()
            .find(|k| k.id == id)
            .map(|k| (k.name.clone(), Some(k.color.clone())))
            .unwrap_or_else(|| ("?".into(), None)),
        Line::Work => (t!("report-work"), None),
        Line::Absences => (t!("report-absences"), None),
        Line::Break => (t!("report-break"), None),
    }
}

/// Lines that have time in at least one of the periods, plus the three totals.
fn visible_lines(
    projects: &[Project],
    kinds: &[AbsenceKind],
    entries: &[TimeEntry],
    periods: &[Period],
) -> Vec<Line> {
    let candidates: Vec<Line> = projects
        .iter()
        .map(|p| Line::Project(p.id))
        .chain(kinds.iter().map(|k| Line::Absence(k.id)))
        .collect();
    let mut lines: Vec<Line> = candidates
        .into_iter()
        .filter(|line| {
            periods
                .iter()
                .any(|p| line.minutes(entries, &*p.contains) > 0)
        })
        .collect();
    lines.extend([Line::Work, Line::Absences, Line::Break]);
    lines
}

/// A table cell: the minutes as `h:mm`, or a dash for zero.
fn cell(minutes: u32) -> String {
    if minutes == 0 {
        "–".to_string()
    } else {
        format_duration(minutes)
    }
}

/// Periods as rows, lines as columns.
fn table_by_period(
    lines: &[Line],
    periods: &[Period],
    total: &Period,
    projects: &[Project],
    kinds: &[AbsenceKind],
    entries: &[TimeEntry],
) -> Element {
    let period_header = t!("report-period");
    let headers: Vec<Element> = lines
        .iter()
        .map(|line| {
            let (name, color) = line_label(*line, projects, kinds);
            rsx! {
                th { class: "px-2 py-1 text-right font-medium whitespace-nowrap",
                    span { class: "inline-flex items-center gap-1",
                        if let Some(color) = color {
                            span { class: "inline-block h-3 w-3 rounded-full", style: "background-color: {color}" }
                        }
                        {name}
                    }
                }
            }
        })
        .collect();
    let body: Vec<Element> = periods
        .iter()
        .map(|period| {
            let cells: Vec<Element> = lines
                .iter()
                .map(|line| {
                    let text = cell(line.minutes(entries, &*period.contains));
                    rsx! { td { class: "px-2 py-1 text-right tabular-nums", {text} } }
                })
                .collect();
            rsx! {
                tr { class: "border-t border-neutral-200 dark:border-neutral-600",
                    th { class: "px-2 py-1 text-left font-medium whitespace-nowrap", {period.label.clone()} }
                    {cells.into_iter()}
                }
            }
        })
        .collect();
    let total_cells: Vec<Element> = lines
        .iter()
        .map(|line| {
            let text = cell(line.minutes(entries, &*total.contains));
            rsx! { td { class: "px-2 py-1 text-right tabular-nums font-bold", {text} } }
        })
        .collect();
    let total_label = t!("report-total");
    rsx! {
        table { class: "min-w-full text-sm border-collapse",
            thead {
                tr { class: "border-b border-neutral-400 dark:border-neutral-500",
                    th { class: "px-2 py-1 text-left font-medium", {period_header} }
                    {headers.into_iter()}
                }
            }
            tbody {
                {body.into_iter()}
                tr { class: "border-t-2 border-neutral-400 dark:border-neutral-500",
                    th { class: "px-2 py-1 text-left font-bold", {total_label} }
                    {total_cells.into_iter()}
                }
            }
        }
    }
}

/// Lines as rows, periods as columns (the per-project view).
fn table_by_line(
    lines: &[Line],
    periods: &[Period],
    total: &Period,
    projects: &[Project],
    kinds: &[AbsenceKind],
    entries: &[TimeEntry],
) -> Element {
    let line_header = t!("report-line");
    let headers: Vec<Element> = periods
        .iter()
        .map(|p| rsx! { th { class: "px-2 py-1 text-right font-medium whitespace-nowrap", {p.label.clone()} } })
        .collect();
    let body: Vec<Element> = lines
        .iter()
        .map(|line| {
            let (name, color) = line_label(*line, projects, kinds);
            let cells: Vec<Element> = periods
                .iter()
                .map(|period| {
                    let text = cell(line.minutes(entries, &*period.contains));
                    rsx! { td { class: "px-2 py-1 text-right tabular-nums", {text} } }
                })
                .collect();
            let total_text = cell(line.minutes(entries, &*total.contains));
            let is_total = matches!(line, Line::Work | Line::Absences | Line::Break);
            let row_class = if is_total {
                "border-t-2 border-neutral-400 dark:border-neutral-500"
            } else {
                "border-t border-neutral-200 dark:border-neutral-600"
            };
            rsx! {
                tr { class: row_class,
                    th { class: "px-2 py-1 text-left font-medium whitespace-nowrap",
                        span { class: "inline-flex items-center gap-2",
                            if let Some(color) = color {
                                span { class: "inline-block h-3 w-3 rounded-full", style: "background-color: {color}" }
                            }
                            {name}
                        }
                    }
                    {cells.into_iter()}
                    td { class: "px-2 py-1 text-right tabular-nums font-bold", {total_text} }
                }
            }
        })
        .collect();
    let total_label = t!("report-total");
    rsx! {
        table { class: "min-w-full text-sm border-collapse",
            thead {
                tr { class: "border-b border-neutral-400 dark:border-neutral-500",
                    th { class: "px-2 py-1 text-left font-medium", {line_header} }
                    {headers.into_iter()}
                    th { class: "px-2 py-1 text-right font-bold", {total_label} }
                }
            }
            tbody { {body.into_iter()} }
        }
    }
}

#[component]
/// The reports page. See the module description.
pub fn ReportsPage() -> Element {
    let projects = use_resource(|| async { api::list_projects().await });
    let kinds = use_resource(|| async { api::list_absence_kinds().await });
    let entries_res = use_resource(|| async { api::list_all_entries().await });
    let settings_res = use_resource(|| async { api::get_settings().await });
    let holidays_res = use_resource(|| async { api::list_holidays().await });

    let mut mode = use_signal(|| Mode::Month);
    let mut year = use_signal(|| STATE.read().selected_date.year());
    let mut month_pick = use_signal(|| STATE.read().selected_date.month());

    let title = t!("report-title");
    let mode_project = t!("report-by-project");
    let mode_month = t!("report-by-month");
    let mode_year = t!("report-by-year");
    let mode_day = t!("report-by-day");
    let mode_details = t!("report-by-details");
    let date_label = t!("report-date");
    let difference_label = t!("report-difference");
    let export_label = t!("report-export-pdf");
    let loading_failed = t!("loading-failed");
    let none_yet = t!("report-empty");

    let projects_now = match &*projects.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    let kinds_now = match &*kinds.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    let entries_now = match &*entries_res.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    // Holidays have no target and are left out of the day table.
    let holidays_now: BTreeSet<NaiveDate> = match &*holidays_res.read() {
        Some(Ok(list)) => list.iter().map(|holiday| holiday.date).collect(),
        _ => BTreeSet::new(),
    };
    let load_error = match &*entries_res.read() {
        Some(Err(err)) => Some(err.to_string()),
        _ => None,
    };

    let selected_year = year();
    let current_mode = mode();

    // Running overtime: the carry-over plus each month's actual minus target, up to today.
    let carry_over = match &*settings_res.read() {
        Some(Ok(current)) => current.carry_over_minutes,
        _ => Settings::default().carry_over_minutes,
    };
    let daily = match &*settings_res.read() {
        Some(Ok(current)) => current.daily_minutes,
        _ => Settings::default().daily_minutes,
    };
    let today = chrono::Local::now().date_naive();
    // Only completed days count: today's target is not due until the day is over.
    let yesterday = today.pred_opt().unwrap_or(today);
    // The balance starts with the first month that has records; earlier months are not tracked.
    let start_month = tracking_start(&entries_now).unwrap_or(today.with_day(1).unwrap_or(today));
    let first_year = start_month.year();
    let months: Vec<MonthBalance> = (first_year..=today.year())
        .flat_map(|year| (1..=12u32).map(move |month| (year, month)))
        .filter_map(|(year, month)| {
            let from = NaiveDate::from_ymd_opt(year, month, 1)?;
            if from > yesterday || from < start_month {
                return None;
            }
            let to = add_months(from, 1).pred_opt()?.min(yesterday);
            Some(MonthBalance {
                year,
                month,
                target: weekdays_between(from, to, &holidays_now) * daily,
                actual: actual_minutes(&entries_now, &kinds_now, from, to),
            })
        })
        .collect();
    let running = running_overtime(carry_over, &months);
    let balance = running.last().copied().unwrap_or(carry_over);

    let work_period = sum_where(
        &entries_now,
        &move |d: NaiveDate| current_mode == Mode::Year || d.year() == selected_year,
        |e| e.project_id.is_some(),
    );
    let carry_label = t!("report-carry-over");
    let work_label = t!("report-work-period");
    let balance_label = t!("report-balance");

    // Years that have any data, for the per-year view.
    let years: BTreeSet<i32> = entries_now.iter().map(|e| e.date.year()).collect();

    let month_periods: Vec<Period> = (1..=12u32)
        .map(|m| Period {
            label: month_name(m),
            contains: Box::new(move |d: NaiveDate| d.year() == selected_year && d.month() == m),
        })
        .collect();
    let year_periods: Vec<Period> = if years.is_empty() {
        vec![Period {
            label: selected_year.to_string(),
            contains: Box::new(move |d: NaiveDate| d.year() == selected_year),
        }]
    } else {
        years
            .iter()
            .map(|&y| Period {
                label: y.to_string(),
                contains: Box::new(move |d: NaiveDate| d.year() == y),
            })
            .collect()
    };
    let year_total = Period {
        label: String::new(),
        contains: Box::new(move |d: NaiveDate| d.year() == selected_year),
    };
    let all_total = Period {
        label: String::new(),
        contains: Box::new(|_| true),
    };

    // Table per month (of the selected year) or per year: target, actual, the period's
    // difference, and the running overtime at the end of the period.
    let row = |label: String, target: u32, actual: u32, running_value: i32, class: &'static str| {
        let difference = actual as i32 - target as i32;
        rsx! {
            tr { class: class,
                th { class: "px-2 py-1 text-left font-medium whitespace-nowrap", {label} }
                td { class: "px-2 py-1 text-right tabular-nums", {format_duration(target)} }
                td { class: "px-2 py-1 text-right tabular-nums", {format_duration(actual)} }
                td { class: "px-2 py-1 text-right tabular-nums", {format_signed_hours(difference)} }
                td { class: "px-2 py-1 text-right tabular-nums font-bold", {format_signed_hours(running_value)} }
            }
        }
    };
    let balance_rows: Vec<Element> = if current_mode == Mode::Year {
        (first_year..=today.year())
            .map(|year| {
                let (target, actual, end_running) = months
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| m.year == year)
                    .fold((0u32, 0u32, carry_over), |acc, (i, m)| {
                        (acc.0 + m.target, acc.1 + m.actual, running[i])
                    });
                row(
                    year.to_string(),
                    target,
                    actual,
                    end_running,
                    "border-t border-neutral-200 dark:border-neutral-600",
                )
            })
            .collect()
    } else {
        months
            .iter()
            .enumerate()
            .filter(|(_, m)| m.year == selected_year)
            .map(|(i, m)| {
                row(
                    month_name(m.month),
                    m.target,
                    m.actual,
                    running[i],
                    "border-t border-neutral-200 dark:border-neutral-600",
                )
            })
            .collect()
    };
    let balance_title = t!("report-balance-title");
    let balance_note = t!("report-balance-note");
    let target_label = t!("report-target-hours");
    let actual_label = t!("report-actual-hours");
    let overtime_label = t!("report-overtime");
    let running_label = t!("report-running");
    let period_label = t!("report-period");

    let mode_value = match current_mode {
        Mode::Project => "project",
        Mode::Month => "month",
        Mode::Year => "year",
        Mode::Day => "day",
        Mode::Details => "details",
    };

    // Per day of the selected month: target, actual, difference and the running overtime.
    let day_body = {
        let balances = daily_balances(
            carry_over,
            start_month,
            yesterday,
            daily,
            &entries_now,
            &kinds_now,
            &holidays_now,
        );
        let rows: Vec<Element> = balances
            .iter()
            .filter(|b| b.date.year() == selected_year && b.date.month() == month_pick())
            // A holiday is left out unless time was booked on it anyway.
            .filter(|b| !holidays_now.contains(&b.date) || b.actual > 0)
            .map(|b| {
                let date_text = b.date.format("%d.%m.%Y").to_string();
                let difference = b.actual as i32 - b.target as i32;
                let running_value = b.running;
                rsx! {
                    tr { class: "border-t border-neutral-200 dark:border-neutral-600",
                        th { class: "px-2 py-1 text-left font-medium whitespace-nowrap", {date_text} }
                        td { class: "px-2 py-1 text-right tabular-nums", {format_duration(b.target)} }
                        td { class: "px-2 py-1 text-right tabular-nums", {format_duration(b.actual)} }
                        td { class: "px-2 py-1 text-right tabular-nums", {format_signed_hours(difference)} }
                        td { class: "px-2 py-1 text-right tabular-nums font-bold", {format_signed_hours(running_value)} }
                    }
                }
            })
            .collect();
        rsx! {
            table { class: "min-w-full text-sm border-collapse",
                thead {
                    tr { class: "border-b border-neutral-400 dark:border-neutral-500",
                        th { class: "px-2 py-1 text-left font-medium", {date_label.clone()} }
                        th { class: "px-2 py-1 text-right font-medium", {target_label.clone()} }
                        th { class: "px-2 py-1 text-right font-medium", {actual_label.clone()} }
                        th { class: "px-2 py-1 text-right font-medium", {difference_label.clone()} }
                        th { class: "px-2 py-1 text-right font-bold", {running_label.clone()} }
                    }
                }
                tbody { {rows.into_iter()} }
            }
        }
    };

    // Monthly detail: every booking of the selected month, day by day, then the totals of the month.
    let in_month = move |d: NaiveDate| d.year() == selected_year && d.month() == month_pick();
    let details_body = {
        let mut month_entries: Vec<TimeEntry> = entries_now
            .iter()
            .filter(|entry| in_month(entry.date))
            .cloned()
            .collect();
        month_entries
            .sort_by_key(|entry| (entry.date, entry.start_minute.unwrap_or(u32::MAX), entry.id));
        if month_entries.is_empty() {
            let none_text = t!("report-no-entries");
            rsx! { p { class: "text-neutral-500", {none_text} } }
        } else {
            let weekdays = [
                t!("Monday"),
                t!("Tuesday"),
                t!("Wednesday"),
                t!("Thursday"),
                t!("Friday"),
                t!("Saturday"),
                t!("Sunday"),
            ];
            let col_time = t!("report-col-time");
            let col_duration = t!("report-col-duration");
            let col_note = t!("report-col-note");
            let col_line = t!("report-line");
            let work_label = t!("report-work");
            let absences_label = t!("report-absences");
            let break_label = t!("report-break");
            let totals_title = t!("report-details-totals");

            let mut days: Vec<NaiveDate> = month_entries.iter().map(|entry| entry.date).collect();
            days.dedup();
            let day_sections: Vec<Element> = days
                .iter()
                .map(|&day| {
                    let day_entries: Vec<&TimeEntry> =
                        month_entries.iter().filter(|entry| entry.date == day).collect();
                    let work: u32 = day_entries
                        .iter()
                        .filter(|entry| entry.project_id.is_some())
                        .map(|entry| entry.minutes)
                        .sum();
                    let absence: u32 = day_entries
                        .iter()
                        .filter(|entry| entry.absence_kind_id.is_some())
                        .map(|entry| entry.minutes)
                        .sum();
                    let breaks: u32 = day_entries.iter().map(|entry| entry.break_minutes).sum();
                    let heading = format!(
                        "{} {}",
                        weekdays[day.weekday().num_days_from_monday() as usize],
                        day.format("%d.%m.%Y")
                    );
                    let rows: Vec<Element> = day_entries
                        .iter()
                        .map(|entry| {
                            let (name, color) = match entry.project_id {
                                Some(id) => line_label(Line::Project(id), &projects_now, &kinds_now),
                                None => line_label(
                                    Line::Absence(entry.absence_kind_id.unwrap_or(0)),
                                    &projects_now,
                                    &kinds_now,
                                ),
                            };
                            let time = match entry.start_minute {
                                Some(start) => format!(
                                    "{}–{}",
                                    format_clock(start),
                                    format_clock(start + entry.minutes + entry.break_minutes)
                                ),
                                None => "–".to_string(),
                            };
                            let duration = format_duration(entry.minutes);
                            let break_text = cell(entry.break_minutes);
                            let note = entry.note.clone();
                            rsx! {
                                tr { class: "border-t border-neutral-200 dark:border-neutral-600",
                                    td { class: "px-2 py-1 whitespace-nowrap tabular-nums", {time} }
                                    td { class: "px-2 py-1 whitespace-nowrap",
                                        span { class: "inline-flex items-center gap-2",
                                            if let Some(color) = color {
                                                span { class: "inline-block h-3 w-3 rounded-full", style: "background-color: {color}" }
                                            }
                                            {name}
                                        }
                                    }
                                    td { class: "px-2 py-1 text-right tabular-nums", {duration} }
                                    td { class: "px-2 py-1 text-right tabular-nums", {break_text} }
                                    td { class: "px-2 py-1", {note} }
                                }
                            }
                        })
                        .collect();
                    let summary = format!(
                        "{}: {}  ·  {}: {}  ·  {}: {}",
                        work_label.clone(),
                        format_duration(work),
                        absences_label.clone(),
                        format_duration(absence),
                        break_label.clone(),
                        format_duration(breaks)
                    );
                    rsx! {
                        section { class: "break-inside-avoid",
                            h3 { class: "font-bold mb-1", {heading} }
                            table { class: "min-w-full text-sm border-collapse",
                                thead {
                                    tr { class: "border-b border-neutral-400 dark:border-neutral-500",
                                        th { class: "px-2 py-1 text-left font-medium", {col_time.clone()} }
                                        th { class: "px-2 py-1 text-left font-medium", {col_line.clone()} }
                                        th { class: "px-2 py-1 text-right font-medium", {col_duration.clone()} }
                                        th { class: "px-2 py-1 text-right font-medium", {break_label.clone()} }
                                        th { class: "px-2 py-1 text-left font-medium", {col_note.clone()} }
                                    }
                                }
                                tbody { {rows.into_iter()} }
                            }
                            p { class: "mt-1 text-sm text-neutral-500 dark:text-neutral-400 tabular-nums", {summary} }
                        }
                    }
                })
                .collect();

            // Totals of the month, per project and absence kind, with the same lines as the other views.
            let month_period = [Period {
                label: String::new(),
                contains: Box::new(in_month),
            }];
            let month_lines = visible_lines(&projects_now, &kinds_now, &entries_now, &month_period);
            let total_rows: Vec<Element> = month_lines
                .iter()
                .map(|line| {
                    let (name, color) = line_label(*line, &projects_now, &kinds_now);
                    let text = cell(line.minutes(&entries_now, &in_month));
                    let is_total = matches!(line, Line::Work | Line::Absences | Line::Break);
                    let row_class = if is_total {
                        "border-t-2 border-neutral-400 dark:border-neutral-500"
                    } else {
                        "border-t border-neutral-200 dark:border-neutral-600"
                    };
                    rsx! {
                        tr { class: row_class,
                            th { class: "px-2 py-1 text-left font-medium whitespace-nowrap",
                                span { class: "inline-flex items-center gap-2",
                                    if let Some(color) = color {
                                        span { class: "inline-block h-3 w-3 rounded-full", style: "background-color: {color}" }
                                    }
                                    {name}
                                }
                            }
                            td { class: "px-2 py-1 text-right tabular-nums font-bold", {text} }
                        }
                    }
                })
                .collect();

            // Target: working days of the month (no weekends, no holidays) times the daily working time.
            // Actual: project work plus paid absence, as in the balance table.
            let month_start =
                NaiveDate::from_ymd_opt(selected_year, month_pick(), 1).unwrap_or(today);
            let month_end = add_months(month_start, 1).pred_opt().unwrap_or(month_start);
            let target = weekdays_between(month_start, month_end, &holidays_now) * daily;
            let actual = actual_minutes(&entries_now, &kinds_now, month_start, month_end);
            let difference = actual as i32 - target as i32;
            let target_row = t!("report-target-hours");
            let actual_row = t!("report-actual-hours");
            let difference_row = t!("report-difference");
            let balance_title_text = t!("report-balance-title");

            rsx! {
                div { class: "flex flex-col gap-6", {day_sections.into_iter()} }
                section { class: "mt-6",
                    h2 { class: "font-bold mb-1", {balance_title_text} }
                    table { class: "min-w-full text-sm border-collapse",
                        tbody {
                            tr { class: "border-t border-neutral-200 dark:border-neutral-600",
                                th { class: "px-2 py-1 text-left font-medium", {target_row} }
                                td { class: "px-2 py-1 text-right tabular-nums", {format_duration(target)} }
                            }
                            tr { class: "border-t border-neutral-200 dark:border-neutral-600",
                                th { class: "px-2 py-1 text-left font-medium", {actual_row} }
                                td { class: "px-2 py-1 text-right tabular-nums", {format_duration(actual)} }
                            }
                            tr { class: "border-t-2 border-neutral-400 dark:border-neutral-500",
                                th { class: "px-2 py-1 text-left font-bold", {difference_row} }
                                td { class: "px-2 py-1 text-right tabular-nums font-bold", {format_signed_hours(difference)} }
                            }
                        }
                    }
                }
                section { class: "mt-6",
                    h2 { class: "font-bold mb-1", {totals_title} }
                    table { class: "min-w-full text-sm border-collapse",
                        tbody { {total_rows.into_iter()} }
                    }
                }
            }
        }
    };

    let body = if current_mode == Mode::Day {
        day_body
    } else if current_mode == Mode::Details {
        details_body
    } else if entries_now.is_empty() && load_error.is_none() {
        rsx! { p { class: "text-neutral-500", {none_yet} } }
    } else {
        match current_mode {
            Mode::Project => {
                // Projects as rows, months as columns, and the year as a total column.
                let lines = visible_lines(&projects_now, &kinds_now, &entries_now, &month_periods);
                let total = Period {
                    label: t!("report-total"),
                    contains: Box::new(move |d: NaiveDate| d.year() == selected_year),
                };
                table_by_line(
                    &lines,
                    &month_periods,
                    &total,
                    &projects_now,
                    &kinds_now,
                    &entries_now,
                )
            }
            Mode::Month => {
                let lines = visible_lines(&projects_now, &kinds_now, &entries_now, &month_periods);
                table_by_period(
                    &lines,
                    &month_periods,
                    &year_total,
                    &projects_now,
                    &kinds_now,
                    &entries_now,
                )
            }
            Mode::Day | Mode::Details => rsx! {},
            Mode::Year => {
                let lines = visible_lines(&projects_now, &kinds_now, &entries_now, &year_periods);
                table_by_period(
                    &lines,
                    &year_periods,
                    &all_total,
                    &projects_now,
                    &kinds_now,
                    &entries_now,
                )
            }
        }
    };

    rsx! {
        div { class: "flex flex-col gap-4",
            div { class: "flex flex-wrap items-end gap-3 print:hidden",
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, {title.clone()} }
                    select {
                        class: INPUT,
                        onchange: move |evt| {
                            mode.set(match evt.value().as_str() {
                                "project" => Mode::Project,
                                "year" => Mode::Year,
                                "day" => Mode::Day,
                                "details" => Mode::Details,
                                _ => Mode::Month,
                            });
                        },
                        option { value: "project", selected: mode_value == "project", {mode_project} }
                        option { value: "month", selected: mode_value == "month", {mode_month} }
                        option { value: "year", selected: mode_value == "year", {mode_year} }
                        option { value: "day", selected: mode_value == "day", {mode_day} }
                        option { value: "details", selected: mode_value == "details", {mode_details} }
                    }
                }
                if current_mode == Mode::Day || current_mode == Mode::Details {
                    select {
                        class: INPUT,
                        onchange: move |evt| {
                            if let Ok(value) = evt.value().parse::<u32>() {
                                month_pick.set(value);
                            }
                        },
                        {(1..=12u32).map(|m| rsx! {
                            option { value: "{m}", selected: month_pick() == m, {month_name(m)} }
                        })}
                    }
                }
                if current_mode != Mode::Year {
                    div { class: "flex items-center gap-2",
                        button {
                            class: BUTTON,
                            r#type: "button",
                            onclick: move |_| year.set(year() - 1),
                            "<<"
                        }
                        input {
                            class: format!("{INPUT} w-28"),
                            r#type: "number",
                            value: "{selected_year}",
                            onchange: move |evt| {
                                if let Ok(value) = evt.value().parse::<i32>() {
                                    year.set(value);
                                }
                            },
                        }
                        button {
                            class: BUTTON,
                            r#type: "button",
                            onclick: move |_| year.set(year() + 1),
                            ">>"
                        }
                    }
                }
                button {
                    class: format!("{BUTTON} ml-auto"),
                    r#type: "button",
                    onclick: move |_| {
                        // Opens the print dialog; "Save as PDF" there produces the file.
                        let _ = window().print();
                    },
                    {export_label}
                }
            }
            h1 { class: format!("{PAGE_TITLE} print:text-black"),
                if current_mode == Mode::Year {
                    {title}
                } else {
                    "{title} {selected_year}"
                }
            }
            div { class: "grid grid-cols-1 gap-3 sm:grid-cols-3 print:text-black",
                section { class: CARD,
                    div { class: LABEL, {carry_label} }
                    div { class: "text-lg tabular-nums", {format_signed_hours(carry_over)} }
                }
                section { class: CARD,
                    div { class: LABEL, {work_label} }
                    div { class: "text-lg tabular-nums", {format_duration(work_period)} }
                }
                section { class: CARD,
                    div { class: LABEL, {balance_label} }
                    div { class: "text-lg font-bold tabular-nums", {format_signed_hours(balance)} }
                }
            }
            if let Some(message) = load_error {
                p { class: ERROR_TEXT, "{loading_failed}: {message}" }
            }
            section { class: format!("{CARD} overflow-x-auto"),
                {body}
            }
            if current_mode == Mode::Month || current_mode == Mode::Year {
                section { class: format!("{CARD} overflow-x-auto"),
                    h2 { class: "font-bold mb-1", {balance_title} }
                    p { class: "text-sm text-neutral-500 dark:text-neutral-400 mb-2", {balance_note} }
                    table { class: "min-w-full text-sm border-collapse",
                        thead {
                            tr { class: "border-b border-neutral-400 dark:border-neutral-500",
                                th { class: "px-2 py-1 text-left font-medium", {period_label} }
                                th { class: "px-2 py-1 text-right font-medium", {target_label} }
                                th { class: "px-2 py-1 text-right font-medium", {actual_label} }
                                th { class: "px-2 py-1 text-right font-medium", {overtime_label} }
                                th { class: "px-2 py-1 text-right font-bold", {running_label} }
                            }
                        }
                        tbody { {balance_rows.into_iter()} }
                    }
                }
            }
        }
    }
}
