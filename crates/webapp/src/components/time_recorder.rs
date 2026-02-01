//! Time recording: the form that adds or edits a time record of the selected day, and the day's records.
//!
//! A record is start, work duration and break. The end is `start + work + break`, and the
//! break is unpaid, so it is not booked as work.

use chrono::{NaiveDate, Timelike};
use dioxus::document;
use dioxus::html::Key;
use dioxus::prelude::*;
use dioxus_i18n::t;
use dto::{
    derive_amount, derive_end, duration_minutes, duration_text, format_clock, overlaps_any,
    parse_clock, resolve_block, AbsenceKind, BlockError, DurationUnit, PendingStart, Project,
    TimeEntry, TimeEntryInput,
};

use crate::api;
use crate::components::overview::format_duration;
use crate::components::ui::{
    BUTTON, BUTTON_DANGER, BUTTON_PRIMARY, CARD, ERROR_TEXT, INPUT, LABEL, PAGE_TITLE,
};
use crate::STATE;

/// Moves keyboard focus to the element with `id`.
fn focus(id: &str) {
    let _ = document::eval(&format!("document.getElementById('{id}')?.focus();"));
}

/// A project or absence clicked in the overview. `entry` is the record of that item on the
/// selected day that the form should load, if there is one.
#[derive(Debug, Clone, PartialEq)]
pub struct Pick {
    pub target: String,
    pub entry: Option<TimeEntry>,
    /// Changes with every click, so clicking the same item again still reloads the form.
    pub serial: u64,
}

/// Which of end time and work duration the user typed last. The other one is derived from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Anchor {
    Duration,
    End,
}

/// What was just saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Notice {
    Added,
    Updated,
}

/// Why a record could not be saved.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Problem {
    Block(BlockError),
    BreakInvalid,
    NoTarget,
    Overlap,
    Failed(String),
}

fn block_error_text(error: BlockError) -> String {
    match error {
        BlockError::StartMissing => t!("err-start-missing"),
        BlockError::StartInvalid => t!("err-start-invalid"),
        BlockError::EndInvalid => t!("err-end-invalid"),
        BlockError::DurationMissing => t!("err-duration-missing"),
        BlockError::DurationInvalid => t!("err-duration-invalid"),
        BlockError::EndNotAfterStart => t!("err-end-before-start"),
        BlockError::DurationMismatch => t!("err-duration-mismatch"),
        BlockError::BreakTooLong => t!("err-break-too-long"),
        BlockError::PastMidnight => t!("err-past-midnight"),
    }
}

/// Target select values: `p:<id>` for a project, `a:<id>` for an absence kind.
fn parse_target(value: &str) -> Option<(Option<i64>, Option<i64>)> {
    let (kind, id) = value.split_once(':')?;
    let id: i64 = id.parse().ok()?;
    match kind {
        "p" => Some((Some(id), None)),
        "a" => Some((None, Some(id))),
        _ => None,
    }
}

/// The first record of `target` in `entries`, ordered by start time.
pub fn first_record_for(target: &str, entries: &[TimeEntry]) -> Option<TimeEntry> {
    let (project_id, absence_kind_id) = parse_target(target)?;
    entries
        .iter()
        .filter(|entry| entry.project_id == project_id && entry.absence_kind_id == absence_kind_id)
        .min_by_key(|entry| entry.start_minute.unwrap_or(u32::MAX))
        .cloned()
}

/// Name and colour of the project or absence an entry is booked on.
fn entry_label(entry: &TimeEntry, projects: &[Project], kinds: &[AbsenceKind]) -> (String, String) {
    if let Some(project) = projects.iter().find(|p| Some(p.id) == entry.project_id) {
        return (project.name.clone(), project.color.clone());
    }
    if let Some(kind) = kinds.iter().find(|k| Some(k.id) == entry.absence_kind_id) {
        return (kind.name.clone(), kind.color.clone());
    }
    ("?".to_string(), "#737373".to_string())
}

/// Parses the break field. Empty means no break.
fn parse_break(text: &str) -> Option<u32> {
    if text.trim().is_empty() {
        return Some(0);
    }
    text.trim()
        .parse::<u32>()
        .ok()
        .filter(|minutes| *minutes <= 24 * 60)
}

#[component]
pub fn TimeRecorder(
    date: NaiveDate,
    day_entries: Vec<TimeEntry>,
    projects: Vec<Project>,
    absence_kinds: Vec<AbsenceKind>,
    picked: Signal<Option<Pick>>,
    /// Break preset from the settings, used for new records.
    default_break: u32,
    on_saved: EventHandler<()>,
) -> Element {
    let mut start = use_signal(String::new);
    let mut end = use_signal(String::new);
    let mut amount = use_signal(String::new);
    let mut break_text = use_signal(|| default_break.to_string());
    let mut unit = use_signal(|| DurationUnit::Minutes);
    let mut anchor = use_signal(|| Anchor::Duration);
    let mut target = use_signal(String::new);
    let mut problem = use_signal(|| None::<Problem>);
    let mut notice = use_signal(|| None::<Notice>);
    // The record being edited, or `None` when the form adds a new one.
    let mut editing = use_signal(|| None::<i64>);
    let mut last_date = use_signal(|| date);
    // Fills the start field from the saved start of unfinished work, if that belongs to `day` and
    // the user has not typed a start yet. Runs on mount and whenever the selected day changes.
    let prefill_start = move |day: NaiveDate| {
        spawn(async move {
            if let Ok(Some(saved)) = api::get_pending_start().await {
                if saved.date == day && start.peek().is_empty() {
                    start.set(format_clock(saved.start_minute));
                }
            }
        });
    };
    use_effect(move || prefill_start(date));
    let now_label = t!("record-now");

    let title = t!("record-title");
    let start_label = t!("record-start");
    let end_label = t!("record-end");
    let duration_label = t!("record-duration");
    let break_label = t!("record-break");
    let unit_minutes = t!("unit-minutes");
    let unit_hours = t!("unit-hours");
    let target_label = t!("record-target");
    let choose_label = t!("record-choose");
    let projects_group = t!("overview-projects");
    let absences_group = t!("overview-absences");
    let add_label = t!("add");
    let save_label = t!("save");
    let cancel_label = t!("cancel");
    let edit_label = t!("edit");
    let delete_label = t!("delete");
    let added_text = t!("record-added");
    let updated_text = t!("record-updated");
    let records_title = t!("records-of-day");
    let no_records = t!("no-records");
    let save_failed = t!("save-failed");
    let delete_failed = t!("delete-failed");
    let overlap_text = t!("err-overlap");
    let target_text = t!("err-target-missing");
    let break_invalid = t!("err-break-invalid");

    // Empties the form, back to the preset break, and leaves edit mode.
    let mut clear_form = move || {
        start.set(String::new());
        end.set(String::new());
        amount.set(String::new());
        break_text.set(default_break.to_string());
        anchor.set(Anchor::Duration);
        target.set(String::new());
        problem.set(None);
        notice.set(None);
        editing.set(None);
    };

    // Fills the form with an existing record and switches to edit mode.
    let mut load_entry = move |entry: TimeEntry| {
        start.set(entry.start_minute.map(format_clock).unwrap_or_default());
        end.set(match entry.start_minute {
            Some(s) if s + entry.minutes + entry.break_minutes <= 24 * 60 => {
                format_clock(s + entry.minutes + entry.break_minutes)
            }
            _ => String::new(),
        });
        amount.set(duration_text(entry.minutes, unit()));
        break_text.set(entry.break_minutes.to_string());
        anchor.set(Anchor::Duration);
        target.set(match (entry.project_id, entry.absence_kind_id) {
            (Some(id), _) => format!("p:{id}"),
            (_, Some(id)) => format!("a:{id}"),
            _ => String::new(),
        });
        problem.set(None);
        notice.set(None);
        editing.set(Some(entry.id));
    };

    // A click in the overview loads that item's record of the day, or preselects the item.
    use_effect(move || {
        let Some(pick) = picked() else { return };
        match pick.entry {
            Some(entry) => load_entry(entry),
            None => {
                clear_form();
                target.set(pick.target);
            }
        }
    });

    // Moving to another day drops any record that was being edited.
    use_effect(move || {
        let current = STATE.read().selected_date;
        if *last_date.peek() != current {
            last_date.set(current);
            clear_form();
            prefill_start(current);
        }
    });

    // Saves the start of unfinished work. An empty field clears it.
    let pending_failed = save_failed.clone();
    let save_pending = use_callback(move |text: String| {
        let save_failed = pending_failed.clone();
        spawn(async move {
            let result = match parse_clock(&text) {
                Some(start_minute) => api::put_pending_start(&PendingStart { date, start_minute })
                    .await
                    .map(|_| ()),
                None if text.trim().is_empty() => api::delete_pending_start().await,
                None => return,
            };
            if let Err(err) = result {
                problem.set(Some(Problem::Failed(format!("{save_failed}: {err}"))));
            }
        });
    });

    // Start, end, duration and break are kept consistent: the field the user did not
    // touch last is derived from the others.
    let mut on_start = move |value: String| {
        start.set(value.clone());
        let Some(brk) = parse_break(&break_text()) else {
            return;
        };
        match anchor() {
            Anchor::End => {
                if let Some(derived) = derive_amount(&value, &end(), unit(), brk) {
                    amount.set(derived);
                }
            }
            Anchor::Duration => {
                if let Some(derived) = derive_end(&value, &amount(), unit(), brk) {
                    end.set(derived);
                }
            }
        }
    };
    let now_start = move |_: MouseEvent| {
        let now = chrono::Local::now().naive_local().time();
        let text = format_clock(now.hour() * 60 + now.minute());
        on_start(text.clone());
        save_pending.call(text);
    };
    let mut on_end = move |value: String| {
        anchor.set(Anchor::End);
        end.set(value.clone());
        if let Some(brk) = parse_break(&break_text()) {
            if let Some(derived) = derive_amount(&start(), &value, unit(), brk) {
                amount.set(derived);
            }
        }
    };
    let mut on_amount = move |value: String| {
        anchor.set(Anchor::Duration);
        amount.set(value.clone());
        if let Some(brk) = parse_break(&break_text()) {
            if let Some(derived) = derive_end(&start(), &value, unit(), brk) {
                end.set(derived);
            }
        }
    };
    let mut on_break = move |value: String| {
        break_text.set(value.clone());
        let Some(brk) = parse_break(&value) else {
            return;
        };
        match anchor() {
            Anchor::End => {
                if let Some(derived) = derive_amount(&start(), &end(), unit(), brk) {
                    amount.set(derived);
                }
            }
            Anchor::Duration => {
                if let Some(derived) = derive_end(&start(), &amount(), unit(), brk) {
                    end.set(derived);
                }
            }
        }
    };
    let mut on_unit = move |value: String| {
        let new_unit = if value == "hours" {
            DurationUnit::Hours
        } else {
            DurationUnit::Minutes
        };
        // Keep the same work duration, shown in the new unit.
        if let Some(minutes) = duration_minutes(&amount(), unit()) {
            amount.set(duration_text(minutes, new_unit));
        }
        unit.set(new_unit);
    };

    let day_for_submit = day_entries.clone();
    let submit = move |evt: FormEvent| {
        evt.prevent_default();
        let Some(brk) = parse_break(&break_text()) else {
            problem.set(Some(Problem::BreakInvalid));
            return;
        };
        let block = match resolve_block(&start(), &end(), &amount(), unit(), brk) {
            Ok(block) => block,
            Err(error) => {
                problem.set(Some(Problem::Block(error)));
                return;
            }
        };
        let editing_id = editing();
        // Editing a record may keep its own time, so it is not compared with itself.
        let others: Vec<TimeEntry> = day_for_submit
            .iter()
            .filter(|entry| Some(entry.id) != editing_id)
            .cloned()
            .collect();
        if overlaps_any(block, &others) {
            problem.set(Some(Problem::Overlap));
            return;
        }
        let Some((project_id, absence_kind_id)) = parse_target(&target()) else {
            problem.set(Some(Problem::NoTarget));
            return;
        };
        let input = TimeEntryInput {
            date,
            start_minute: block.start_minute,
            minutes: block.minutes,
            break_minutes: block.break_minutes,
            project_id,
            absence_kind_id,
            note: String::new(),
        };
        spawn(async move {
            let result = match editing_id {
                Some(id) => api::update_entry(id, &input).await,
                None => api::create_entry(&input).await,
            };
            match result {
                Ok(_) => {
                    let was_editing = editing_id.is_some();
                    let next_start = block.end_minute();
                    clear_form();
                    if was_editing {
                        notice.set(Some(Notice::Updated));
                    } else {
                        // The next record usually starts where this one ended, so that start is
                        // kept as the pending start.
                        if next_start < 24 * 60 {
                            start.set(format_clock(next_start));
                            let _ = api::put_pending_start(&PendingStart {
                                date,
                                start_minute: next_start,
                            })
                            .await;
                        } else {
                            let _ = api::delete_pending_start().await;
                        }
                        notice.set(Some(Notice::Added));
                    }
                    on_saved.call(());
                }
                Err(err) => problem.set(Some(Problem::Failed(err.to_string()))),
            }
        });
    };

    let delete_entry = use_callback(move |id: i64| {
        let delete_failed = delete_failed.clone();
        spawn(async move {
            match api::delete_entry(id).await {
                Ok(()) => {
                    if editing() == Some(id) {
                        clear_form();
                    }
                    on_saved.call(());
                }
                Err(err) => problem.set(Some(Problem::Failed(format!("{delete_failed}: {err}")))),
            }
        });
    });

    let mut sorted = day_entries.clone();
    sorted.sort_by_key(|entry| entry.start_minute.unwrap_or(u32::MAX));
    let records: Vec<Element> = sorted
        .iter()
        .map(|entry| {
            let (name, color) = entry_label(entry, &projects, &absence_kinds);
            let details = match entry.start_minute {
                Some(start) => format!(
                    "{}–{} · {} · {} {}",
                    format_clock(start),
                    format_clock(start + entry.minutes + entry.break_minutes),
                    format_duration(entry.minutes),
                    break_label,
                    format_duration(entry.break_minutes)
                ),
                None => format!(
                    "{} · {} {}",
                    format_duration(entry.minutes),
                    break_label,
                    format_duration(entry.break_minutes)
                ),
            };
            let id = entry.id;
            let row_class = if editing() == Some(id) {
                "flex flex-wrap items-center justify-between gap-2 py-2 px-1 border-b border-neutral-200 dark:border-neutral-600 bg-teal-50 dark:bg-teal-900"
            } else {
                "flex flex-wrap items-center justify-between gap-2 py-2 px-1 border-b border-neutral-200 dark:border-neutral-600"
            };
            let entry_for_edit = entry.clone();
            rsx! {
                li { class: row_class,
                    span { class: "tabular-nums", "{details}" }
                    span { class: "flex items-center gap-2 min-w-0",
                        span {
                            class: "inline-block h-3 w-3 rounded-full shrink-0",
                            style: "background-color: {color}",
                        }
                        span { class: "truncate", "{name}" }
                    }
                    div { class: "flex gap-2",
                        button {
                            class: BUTTON,
                            r#type: "button",
                            onclick: move |_| load_entry(entry_for_edit.clone()),
                            {edit_label.clone()}
                        }
                        button {
                            class: BUTTON_DANGER,
                            r#type: "button",
                            onclick: move |_| delete_entry.call(id),
                            {delete_label.clone()}
                        }
                    }
                }
            }
        })
        .collect();

    let problem_view = match problem() {
        None => rsx! {},
        Some(Problem::Block(error)) => rsx! { p { class: ERROR_TEXT, {block_error_text(error)} } },
        Some(Problem::BreakInvalid) => rsx! { p { class: ERROR_TEXT, {break_invalid.clone()} } },
        Some(Problem::NoTarget) => rsx! { p { class: ERROR_TEXT, {target_text.clone()} } },
        Some(Problem::Overlap) => rsx! { p { class: ERROR_TEXT, {overlap_text.clone()} } },
        Some(Problem::Failed(message)) => {
            rsx! { p { class: ERROR_TEXT, "{save_failed}: {message}" } }
        }
    };
    let notice_view = match notice() {
        None => rsx! {},
        Some(Notice::Added) => {
            rsx! { p { class: "text-teal-600 dark:text-teal-400", {added_text.clone()} } }
        }
        Some(Notice::Updated) => {
            rsx! { p { class: "text-teal-600 dark:text-teal-400", {updated_text.clone()} } }
        }
    };

    let is_editing = editing().is_some();
    let submit_label = if is_editing { save_label } else { add_label };
    let duration_step = if unit() == DurationUnit::Hours {
        "any"
    } else {
        "1"
    };
    let unit_value = if unit() == DurationUnit::Hours {
        "hours"
    } else {
        "minutes"
    };

    rsx! {
        section { class: format!("{CARD} flex flex-col gap-4"),
            h2 { class: PAGE_TITLE, {title} }
            form {
                class: "grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-7 items-end",
                onsubmit: submit,
                // Escape leaves edit mode without saving.
                onkeydown: move |evt| {
                    if evt.key() == Key::Escape && editing().is_some() {
                        clear_form();
                    }
                },
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, r#for: "rec-start", {start_label} }
                    input {
                            id: "rec-start",
                            class: INPUT,
                            r#type: "time",
                            step: "60",
                            value: "{start}",
                            oninput: move |evt| {
                                let value = evt.value();
                                on_start(value.clone());
                                // Save each complete time as it is typed. Partial input is not saved,
                                // and clearing the field is still saved on change.
                                if parse_clock(&value).is_some() {
                                    save_pending.call(value);
                                }
                            },
                            onchange: move |evt| save_pending.call(evt.value()),
                            onkeydown: move |evt| {
                                if evt.key() == Key::Enter {
                                    evt.prevent_default();
                                    save_pending.call(start());
                                    focus("rec-end");
                                }
                            },
                        }
                }
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, r#for: "rec-end", {end_label} }
                    input {
                        id: "rec-end",
                        class: INPUT,
                        r#type: "time",
                        step: "60",
                        value: "{end}",
                        oninput: move |evt| on_end(evt.value()),
                        onkeydown: move |evt| {
                            if evt.key() == Key::Enter {
                                evt.prevent_default();
                                focus("rec-amount");
                            }
                        },
                    }
                }
                div { class: "flex flex-col gap-1 lg:col-span-2",
                    label { class: LABEL, {duration_label} }
                    div { class: "flex gap-2",
                        input {
                            id: "rec-amount",
                            class: INPUT,
                            r#type: "number",
                            min: "0",
                            step: duration_step,
                            value: "{amount}",
                            oninput: move |evt| on_amount(evt.value()),
                            onkeydown: move |evt| {
                            if evt.key() == Key::Enter {
                                evt.prevent_default();
                                focus("rec-break");
                            }
                        },
                        }
                        select {
                            class: format!("{INPUT} w-auto"),
                            onchange: move |evt| on_unit(evt.value()),
                            option { value: "minutes", selected: unit_value == "minutes", {unit_minutes} }
                            option { value: "hours", selected: unit_value == "hours", {unit_hours} }
                        }
                    }
                }
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, {break_label.clone()} }
                    input {
                        id: "rec-break",
                        class: INPUT,
                        r#type: "number",
                        min: "0",
                        max: "1440",
                        step: "1",
                        value: "{break_text}",
                        oninput: move |evt| on_break(evt.value()),
                        onkeydown: move |evt| {
                            if evt.key() == Key::Enter {
                                evt.prevent_default();
                                focus("rec-target");
                            }
                        },
                    }
                }
                div { class: "flex flex-col gap-1 lg:col-span-2",
                    label { class: LABEL, {target_label} }
                    select {
                        id: "rec-target",
                        class: INPUT,
                        onchange: move |evt| target.set(evt.value()),
                        onkeydown: move |evt| {
                            if evt.key() == Key::Enter {
                                evt.prevent_default();
                                focus("rec-submit");
                            }
                        },
                        option { value: "", {choose_label} }
                        optgroup { label: "{projects_group}",
                            {projects.iter().filter(|p| !p.archived || target() == format!("p:{}", p.id)).map(|p| rsx! {
                                option { value: "p:{p.id}", selected: target() == format!("p:{}", p.id), "{p.name}" }
                            })}
                        }
                        optgroup { label: "{absences_group}",
                            {absence_kinds.iter().map(|k| rsx! {
                                option { value: "a:{k.id}", selected: target() == format!("a:{}", k.id), "{k.name}" }
                            })}
                        }
                    }
                }
                div { class: "flex gap-2",
                    button {
                        class: BUTTON,
                        r#type: "button",
                        title: "{now_label}",
                        onclick: now_start,
                        {now_label.clone()}
                    }
                    button { id: "rec-submit", class: BUTTON_PRIMARY, r#type: "submit", {submit_label} }
                    if is_editing {
                        button {
                            class: BUTTON,
                            r#type: "button",
                            onclick: move |_| clear_form(),
                            {cancel_label}
                        }
                    }
                }
            }
            {problem_view}
            {notice_view}
            div { class: "flex flex-col gap-1",
                h3 { class: LABEL, {records_title} }
                if records.is_empty() {
                    p { class: "text-neutral-500", {no_records} }
                }
                ul { {records.into_iter()} }
            }
        }
    }
}
