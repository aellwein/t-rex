//! The month calendar of the start page. Each day of the shown month can be selected; days with records get a
//! dot, and holidays a red marker with their name.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, NaiveDate, Weekday};
use dioxus::prelude::*;
use dioxus_i18n::t;
use dto::calendar::{add_months, clamped_date};

use crate::STATE;

/// The first year the calendar can be moved to.
const MIN_YEAR: i32 = 1970;
/// The last year the calendar can be moved to.
const MAX_YEAR: i32 = 2100;

/// The name of a month, `1` for January to `12` for December, in the current language.
pub fn month_name(month: u32) -> String {
    match month {
        1 => t!("January"),
        2 => t!("February"),
        3 => t!("March"),
        4 => t!("April"),
        5 => t!("May"),
        6 => t!("June"),
        7 => t!("July"),
        8 => t!("August"),
        9 => t!("September"),
        10 => t!("October"),
        11 => t!("November"),
        12 => t!("December"),
        _ => panic!("Invalid month number: {}", month),
    }
}

/// Renders the days of the month of `date`. Days in `marked` get a dot, holidays (with their name) a red one.
fn render_month_days(
    date: NaiveDate,
    marked: &BTreeSet<NaiveDate>,
    holidays: &BTreeMap<NaiveDate, String>,
) -> Vec<Element> {
    let mut elements: Vec<Element> = Vec::new();
    let first_day = date.with_day(1).expect("first day must exist");
    let days_in_month = first_day.num_days_in_month();
    let mut run_weekday = 0;
    let base_classes =
        "text-center p-1 sm:p-2 hover:bg-neutral-500 hover:text-white dark:hover:bg-neutral-400 cursor-pointer";

    for day in 1..=days_in_month {
        let mut eff_classes: String;
        let loop_date = date.with_day(day.into()).expect("day must exist");
        /* Before Monday we add calendar week number */
        if day == 1 || loop_date.weekday() == Weekday::Mon {
            elements.push(rsx! {
                div { class: "text-right p-2 text-teal-500",
                    {loop_date.iso_week().week().to_string()}
                }
            });
        }
        run_weekday += 1;
        if run_weekday == 7 {
            run_weekday = 0;
        }
        if day == 1 {
            let empty_cells = first_day.weekday().num_days_from_monday();
            for _ in 0..empty_cells {
                elements.push(rsx! {
                    div {}
                });
            }
        }
        if loop_date.weekday() == Weekday::Sat || loop_date.weekday() == Weekday::Sun {
            eff_classes = base_classes.to_string() + " text-red-500";
        } else {
            eff_classes = base_classes.to_string();
        }
        // Highlight current day
        if loop_date == date {
            eff_classes += " bg-neutral-500 text-white dark:bg-neutral-400 dark:text-black";
        }
        let has_records = marked.contains(&loop_date);
        let holiday_name = holidays.get(&loop_date).cloned();
        // The selected day has a dark background, so its dot is light (Nord snow / polar night).
        let dot_class = if loop_date == date {
            "block mx-auto mt-0.5 h-1.5 w-1.5 rounded-full bg-white dark:bg-black"
        } else {
            "block mx-auto mt-0.5 h-1.5 w-1.5 rounded-full bg-teal-500"
        };
        elements.push(rsx! {
            div {
                class: eff_classes,
                onclick: move |_| {
                    let mut new_state = STATE.read().clone();
                    new_state.selected_date = loop_date;
                    *STATE.write() = new_state;
                },
                {day.to_string()}
                if has_records {
                    span { class: dot_class, title: t!("has-records") }
                }
                if let Some(name) = &holiday_name {
                    span { class: "block mx-auto mt-0.5 h-1.5 w-1.5 rounded-full bg-red-500", title: name.clone() }
                }
            }
        });
    }

    elements
}

/// Month view. `marked_days` lists the days that have at least one time record,
/// `holidays` the holidays with their names.
#[component]
pub fn Calendar(
    marked_days: BTreeSet<NaiveDate>,
    holidays: BTreeMap<NaiveDate, String>,
) -> Element {
    rsx! {
        div { class: "border-neutral-300 dark:border-neutral-500 border rounded-xl p-2 sm:p-4",
            div { class: "flex flex-col flex-nowrap",
                div { class: "flex flex-wrap gap-2 items-center mb-4",
                    div { class: "grow-0",
                        button {
                            class: "rounded-lg border border-neutral-400 hover:bg-neutral-500 hover:text-white hover:cursor-pointer p-2",
                            onclick: move |_| {
                                let mut new_state = STATE.read().clone();
                                let new_date = add_months(new_state.selected_date, -1);
                                if new_date.year() >= MIN_YEAR {
                                    new_state.selected_date = new_date;
                                    *STATE.write() = new_state;
                                }
                            },
                            "<<"
                        }
                    }
                    div { class: "order-first w-full sm:order-none sm:w-auto flex flex-row gap-0.5 mx-auto items-center justify-center",
                        select {
                            class: "appearance-none",
                            onchange: move |evt| {
                                if let Ok(selected_month) = evt.value().parse::<u32>() {
                                    let mut new_state = STATE.read().clone();
                                    let sel_date = new_state.selected_date;
                                    new_state.selected_date = clamped_date(sel_date.year(), selected_month + 1, sel_date.day());
                                    *STATE.write() = new_state;
                                }
                            },

                            {(0..=11).map(|month| rsx! {
                                option {
                                    value: "{month}",
                                    selected: "{STATE.read().selected_date.month() == month + 1}",
                                    "{month_name(month + 1)}"
                                }
                            })}
                        }
                        input {
                            class: "appearance-none",
                            min: "{MIN_YEAR}",
                            max: "{MAX_YEAR}",
                            r#type: "number",
                            value: "{STATE.read().selected_date.year()}",
                            onchange: move |evt| {
                                let mut new_state = STATE.read().clone();
                                if let Ok(selected_year) = evt.value().parse::<i32>() {
                                    if !(MIN_YEAR..=MAX_YEAR).contains(&selected_year) {
                                        return;
                                    }
                                    let sel_date = new_state.selected_date;
                                    new_state.selected_date = clamped_date(selected_year, sel_date.month(), sel_date.day());
                                    *STATE.write() = new_state;
                                }
                            },
                        }
                    }
                    div {
                        button {
                            class: "rounded-lg border border-neutral-400 hover:bg-neutral-500 hover:text-white hover:cursor-pointer p-2",
                            onclick: move |_| {
                                let mut new_state = STATE.read().clone();
                                new_state.selected_date = chrono::Local::now().naive_local().date();
                                *STATE.write() = new_state;
                            },
                            title: t!("jump-to-current-day"),
                            "🎯"
                        }
                    }
                    div { class: "grow-0",
                        button {
                            class: "rounded-lg border border-neutral-400 hover:bg-neutral-500 hover:text-white hover:cursor-pointer p-2",
                            onclick: move |_| {
                                let mut new_state = STATE.read().clone();
                                let new_date = add_months(new_state.selected_date, 1);
                                if new_date.year() <= MAX_YEAR {
                                    new_state.selected_date = new_date;
                                    *STATE.write() = new_state;
                                }
                            },
                            ">>"
                        }
                    }
                }
                div { class: "grid grid-cols-8 gap-1 sm:gap-2",
                    div {} // for calender week number
                    div { class: "text-right font-bold p-2", {t!("Monday")} }
                    div { class: "text-right font-bold p-2", {t!("Tuesday")} }
                    div { class: "text-right font-bold p-2", {t!("Wednesday")} }
                    div { class: "text-right font-bold p-2", {t!("Thursday")} }
                    div { class: "text-right font-bold p-2", {t!("Friday")} }
                    div { class: "text-right font-bold p-2", {t!("Saturday")} }
                    div { class: "text-right font-bold p-2", {t!("Sunday")} }
                    {render_month_days(STATE.read().selected_date, &marked_days, &holidays).iter().map(|day| rsx! {
                        {day}
                    })
                    }
                }
            }
        }
    }
}
