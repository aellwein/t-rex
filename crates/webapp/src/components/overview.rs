//! Quick overview of the selected day and month: booked time per project and per absence kind.

use dioxus::prelude::*;
use dioxus_i18n::t;
use dto::{format_signed_hours, AbsenceKind, Project, TimeEntry};

use crate::components::ui::{CARD, LABEL};

/// Formats minutes as `hours:minutes`, e.g. 90 as `1:30`.
pub fn format_duration(minutes: u32) -> String {
    format!("{}:{:02}", minutes / 60, minutes % 60)
}

/// Minutes booked in `entries` that satisfy `belongs`.
pub fn booked<F>(entries: &[TimeEntry], belongs: F) -> u32
where
    F: Fn(&TimeEntry) -> bool,
{
    entries
        .iter()
        .filter(|entry| belongs(entry))
        .map(|entry| entry.minutes)
        .sum()
}

/// One line of the overview: a coloured name with the time booked on the day and in the month.
/// Clicking it picks the project or absence, see [`Overview`].
#[component]
fn OverviewRow(
    name: String,
    color: String,
    day: u32,
    month: u32,
    on_click: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "w-full flex items-center justify-between gap-2 py-1 px-1 rounded text-left hover:bg-neutral-200 dark:hover:bg-neutral-600 cursor-pointer",
            onclick: move |_| on_click.call(()),

            div { class: "flex items-center gap-2 min-w-0",
                span {
                    class: "inline-block h-3 w-3 rounded-full shrink-0",
                    style: "background-color: {color}",
                }
                span { class: "truncate", "{name}" }
            }
            div { class: "flex shrink-0 gap-3 tabular-nums text-right",
                span { class: "w-12", {format_duration(day)} }
                span { class: "w-12 text-neutral-500 dark:text-neutral-400", {format_duration(month)} }
            }
        }
    }
}

#[component]
pub fn Overview(
    projects: Vec<Project>,
    absence_kinds: Vec<AbsenceKind>,
    day: Vec<TimeEntry>,
    month: Vec<TimeEntry>,
    /// Running overtime up to today, in minutes (carry-over included).
    running_overtime: i32,
    /// Name of the holiday on the selected day, if it is one.
    holiday: Option<String>,
    /// Called with the target (`p:<id>` or `a:<id>`) of the clicked project or absence.
    on_pick: EventHandler<String>,
) -> Element {
    let holiday_label = t!("holiday-label");
    let title = t!("overview-title");
    let running_title = t!("overview-running");
    let day_label = t!("overview-day");
    let month_label = t!("overview-month");
    let projects_label = t!("overview-projects");
    let absences_label = t!("overview-absences");
    let none_yet = t!("no-projects");

    // Archived projects only show up while they still have time booked this month.
    let project_rows: Vec<Element> = projects
        .iter()
        .filter_map(|project| {
            let day_minutes = booked(&day, |e| e.project_id == Some(project.id));
            let month_minutes = booked(&month, |e| e.project_id == Some(project.id));
            if project.archived && month_minutes == 0 {
                return None;
            }
            let target = format!("p:{}", project.id);
            Some(rsx! {
                OverviewRow {
                    name: project.name.clone(),
                    color: project.color.clone(),
                    day: day_minutes,
                    month: month_minutes,
                    on_click: move |_| on_pick.call(target.clone()),
                }
            })
        })
        .collect();

    let absence_rows: Vec<Element> = absence_kinds
        .iter()
        .map(|kind| {
            let day_minutes = booked(&day, |e| e.absence_kind_id == Some(kind.id));
            let month_minutes = booked(&month, |e| e.absence_kind_id == Some(kind.id));
            let target = format!("a:{}", kind.id);
            rsx! {
                OverviewRow {
                    name: kind.name.clone(),
                    color: kind.color.clone(),
                    day: day_minutes,
                    month: month_minutes,
                    on_click: move |_| on_pick.call(target.clone()),
                }
            }
        })
        .collect();

    rsx! {
        section { class: format!("{CARD} flex flex-col gap-2 text-sm"),
            div { class: "flex items-center justify-between gap-2",
                h2 { class: "font-bold text-base", {title} }
                div { class: "flex shrink-0 gap-3 text-right font-medium",
                    span { class: "w-12", {day_label} }
                    span { class: "w-12", {month_label} }
                }
            }
            if let Some(name) = holiday {
                div { class: "flex items-center gap-2 rounded-lg border border-red-500 px-2 py-1 text-red-600 dark:text-red-400 font-medium",
                    span { class: "inline-block h-2.5 w-2.5 shrink-0 rounded-full bg-red-500" }
                    span { "{holiday_label}: {name}" }
                }
            }
            // Running overtime up to today: the number to check against the master time record.
            div { class: "rounded-lg bg-neutral-100 dark:bg-neutral-800 p-2",
                div { class: LABEL, {running_title} }
                div { class: "text-2xl font-bold tabular-nums", {format_signed_hours(running_overtime)} }
            }
            div { class: LABEL, {projects_label} }
            if project_rows.is_empty() && absence_rows.is_empty() {
                p { class: "text-neutral-500", {none_yet} }
            }
            {project_rows.into_iter()}
            div { class: format!("{LABEL} mt-2"), {absences_label} }
            {absence_rows.into_iter()}
        }
    }
}
