//! Day overview: calendar and overview side by side on wide screens, time recording below.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use dioxus_i18n::t;
use dto::calendar::add_months;
use dto::{daily_balances, tracking_start, TimeEntry};

use crate::api;
use crate::components::prelude::{Calendar, Overview, TimeRecorder};
use crate::components::time_recorder::{first_record_for, Pick};
use crate::components::ui::ERROR_TEXT;
use crate::STATE;

/// First and last day of the month that contains `date`.
fn month_bounds(date: NaiveDate) -> (NaiveDate, NaiveDate) {
    let first = date.with_day(1).expect("first day must exist");
    let last = add_months(first, 1)
        .pred_opt()
        .expect("day before the next month must exist");
    (first, last)
}

#[component]
/// The start page: the calendar and the overview of the selected day, with the time recording below. See the module description.
pub fn HomePage() -> Element {
    let selected = STATE.read().selected_date;
    // Only changes when the month changes, so moving between days does not reload.
    let month = use_memo(move || month_bounds(STATE.read().selected_date));

    let projects = use_resource(|| async { api::list_projects().await });
    let settings = use_resource(|| async { api::get_settings().await });
    let kinds = use_resource(|| async { api::list_absence_kinds().await });
    let holidays_res = use_resource(|| async { api::list_holidays().await });
    let all_entries = use_resource(|| async { api::list_all_entries().await });
    let mut entries = use_resource(move || {
        let (from, to) = month();
        async move { api::list_entries(from, to).await }
    });

    let loading_failed = t!("loading-failed");
    let mut picked = use_signal(|| None::<Pick>);

    let projects_now = match &*projects.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    let kinds_now = match &*kinds.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    let month_entries: Vec<TimeEntry> = match &*entries.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    let load_error = match &*entries.read() {
        Some(Err(err)) => Some(err.to_string()),
        _ => None,
    };

    let marked: BTreeSet<NaiveDate> = month_entries.iter().map(|entry| entry.date).collect();
    let day_entries: Vec<TimeEntry> = month_entries
        .iter()
        .filter(|entry| entry.date == selected)
        .cloned()
        .collect();

    // The preset from the settings is the break of a new record.
    let preset = match &*settings.read() {
        Some(Ok(current)) => current.break_minutes,
        _ => 45,
    };

    // Running overtime up to today: carry-over plus each day's difference since the first record.
    let all_now: Vec<TimeEntry> = match &*all_entries.read() {
        Some(Ok(list)) => list.clone(),
        _ => Vec::new(),
    };
    let holidays_now: BTreeSet<NaiveDate> = match &*holidays_res.read() {
        Some(Ok(list)) => list.iter().map(|holiday| holiday.date).collect(),
        _ => BTreeSet::new(),
    };
    let holiday_names: BTreeMap<NaiveDate, String> = match &*holidays_res.read() {
        Some(Ok(list)) => list
            .iter()
            .map(|holiday| (holiday.date, holiday.name.clone()))
            .collect(),
        _ => BTreeMap::new(),
    };
    let selected_holiday = holiday_names.get(&selected).cloned();
    let (carry_over, daily) = match &*settings.read() {
        Some(Ok(current)) => (current.carry_over_minutes, current.daily_minutes),
        _ => (0, 480),
    };
    // Only completed days count: today's target is not due until the day is over.
    let yesterday = chrono::Local::now().date_naive().pred_opt();
    let running_now = match (tracking_start(&all_now), yesterday) {
        (Some(start), Some(last)) => daily_balances(
            carry_over,
            start,
            last,
            daily,
            &all_now,
            &kinds_now,
            &holidays_now,
        )
        .last()
        .map_or(carry_over, |day| day.running),
        _ => carry_over,
    };

    let day_for_pick = day_entries.clone();
    rsx! {
        div { class: "flex flex-col gap-4",
            div { class: "flex flex-col lg:flex-row gap-4 items-start",
                div { class: "w-full lg:flex-1 min-w-0",
                    Calendar { marked_days: marked, holidays: holiday_names.clone() }
                }
                div { class: "w-full lg:w-80 shrink-0",
                    Overview {
                        projects: projects_now.clone(),
                        absence_kinds: kinds_now.clone(),
                        day: day_entries.clone(),
                        month: month_entries.clone(),
                        running_overtime: running_now,
                        holiday: selected_holiday.clone(),
                        on_pick: move |target: String| {
                            let entry = first_record_for(&target, &day_for_pick);
                            let serial = picked.peek().as_ref().map_or(0, |p| p.serial) + 1;
                            picked.set(Some(Pick { target, entry, serial }));
                        },
                    }
                }
            }
            if let Some(message) = load_error {
                p { class: ERROR_TEXT, "{loading_failed}: {message}" }
            }
            // Keyed by the preset: the form starts over once the settings have loaded.
            TimeRecorder {
                key: "{preset}",
                date: selected,
                day_entries: day_entries,
                projects: projects_now,
                absence_kinds: kinds_now,
                picked: picked,
                default_break: preset,
                on_saved: move |_| entries.restart(),
            }
        }
    }
}
