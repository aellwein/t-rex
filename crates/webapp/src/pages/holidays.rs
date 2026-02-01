//! Holidays page: the days without working time, per year. Edit or add by hand, or import
//! the all-day events of an iCalendar (.ics) address.

use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use dioxus_i18n::t;
use dto::{Holiday, HolidayInput};

use crate::api;
use crate::components::ui::{
    Failed, BUTTON, BUTTON_DANGER, BUTTON_PRIMARY, CARD, ERROR_TEXT, INPUT, LABEL, PAGE_TITLE,
    SUCCESS_TEXT,
};
use crate::STATE;

#[component]
pub fn HolidaysPage() -> Element {
    let mut holidays = use_resource(|| async { api::list_holidays().await });
    let mut year = use_signal(|| STATE.read().selected_date.year());
    let mut editing = use_signal(|| None::<i64>);
    let mut date_text = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut failure = use_signal(|| None::<(Failed, String)>);
    let mut url = use_signal(String::new);
    let mut import_done = use_signal(|| None::<(u32, u32)>);
    let mut import_failure = use_signal(|| None::<String>);
    let mut pasted = use_signal(String::new);

    let title = t!("holidays-title");
    let note = t!("holidays-note");
    let loading_text = t!("loading");
    let load_failed = t!("loading-failed");
    let save_failed = t!("save-failed");
    let delete_failed = t!("delete-failed");
    let invalid_date = t!("holidays-invalid-date");
    let none_yet = t!("holidays-empty");
    let date_label = t!("holidays-date");
    let name_label = t!("name");
    let add_label = t!("holidays-add");
    let save_label = t!("save");
    let cancel_label = t!("cancel");
    let edit_label = t!("edit");
    let delete_label = t!("delete");
    let import_title = t!("holidays-import-title");
    let import_hint = t!("holidays-import-hint");
    let url_label = t!("holidays-url");
    let import_label = t!("holidays-import");
    let import_failed = t!("holidays-import-failed");
    let paste_title = t!("holidays-paste-title");
    let paste_hint = t!("holidays-paste-hint");
    let paste_label = t!("holidays-paste-button");

    let save = move |evt: FormEvent| {
        evt.prevent_default();
        let Ok(date) = NaiveDate::parse_from_str(date_text().trim(), "%Y-%m-%d") else {
            failure.set(Some((Failed::Save, invalid_date.clone())));
            return;
        };
        let input = HolidayInput { date, name: name() };
        let target = editing();
        spawn(async move {
            let result = match target {
                Some(id) => api::update_holiday(id, &input).await,
                None => api::create_holiday(&input).await,
            };
            match result {
                Ok(_) => {
                    editing.set(None);
                    date_text.set(String::new());
                    name.set(String::new());
                    failure.set(None);
                    holidays.restart();
                }
                Err(err) => failure.set(Some((Failed::Save, err.to_string()))),
            }
        });
    };

    let cancel = move |_: MouseEvent| {
        editing.set(None);
        date_text.set(String::new());
        name.set(String::new());
    };

    let import = move |evt: FormEvent| {
        evt.prevent_default();
        let address = url().trim().to_string();
        spawn(async move {
            match api::import_holidays(&address).await {
                Ok(result) => {
                    import_failure.set(None);
                    import_done.set(Some((result.added, result.existing)));
                    holidays.restart();
                }
                Err(err) => {
                    import_done.set(None);
                    import_failure.set(Some(err.to_string()));
                }
            }
        });
    };

    let paste_import = move |evt: FormEvent| {
        evt.prevent_default();
        let text = pasted();
        spawn(async move {
            match api::import_holiday_text(&text).await {
                Ok(result) => {
                    import_failure.set(None);
                    import_done.set(Some((result.added, result.existing)));
                    pasted.set(String::new());
                    holidays.restart();
                }
                Err(err) => {
                    import_done.set(None);
                    import_failure.set(Some(err.to_string()));
                }
            }
        });
    };

    let selected_year = year();
    let list_view = match &*holidays.read() {
        None => rsx! { p { {loading_text.clone()} } },
        Some(Err(err)) => rsx! { p { class: ERROR_TEXT, "{load_failed}: {err}" } },
        Some(Ok(list)) => {
            let rows: Vec<Holiday> = list
                .iter()
                .filter(|h| h.date.year() == selected_year)
                .cloned()
                .collect();
            if rows.is_empty() {
                rsx! { p { {none_yet.clone()} } }
            } else {
                rsx! {
                    ul { class: "flex flex-col gap-2",
                        for holiday in rows {
                            {
                                let id = holiday.id;
                                let date_value = holiday.date.to_string();
                                let date_text_shown = holiday.date.format("%d.%m.%Y").to_string();
                                let row_name = holiday.name.clone();
                                let edit_name = holiday.name.clone();
                                rsx! {
                                    li { class: format!("{CARD} flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between"),
                                        div { class: "flex flex-wrap items-center gap-3 min-w-0",
                                            span { class: "font-medium tabular-nums", "{date_text_shown}" }
                                            span { class: "truncate", "{row_name}" }
                                        }
                                        div { class: "flex gap-2 shrink-0",
                                            button {
                                                class: BUTTON,
                                                r#type: "button",
                                                onclick: move |_| {
                                                    editing.set(Some(id));
                                                    date_text.set(date_value.clone());
                                                    name.set(edit_name.clone());
                                                },
                                                {edit_label.clone()}
                                            }
                                            button {
                                                class: BUTTON_DANGER,
                                                r#type: "button",
                                                onclick: move |_| {
                                                    spawn(async move {
                                                        match api::delete_holiday(id).await {
                                                            Ok(()) => {
                                                                failure.set(None);
                                                                holidays.restart();
                                                            }
                                                            Err(err) => failure.set(Some((Failed::Delete, err.to_string()))),
                                                        }
                                                    });
                                                },
                                                {delete_label.clone()}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    rsx! {
        div { class: "flex flex-col gap-4",
            h1 { class: PAGE_TITLE, {title} }
            p { class: "text-sm text-neutral-500 dark:text-neutral-400", {note} }

            section { class: format!("{CARD} flex flex-col gap-3"),
                h2 { class: "font-bold", {import_title} }
                p { class: "text-sm text-neutral-500 dark:text-neutral-400", {import_hint} }
                form { class: "flex flex-col gap-3 sm:flex-row sm:items-end", onsubmit: import,
                    div { class: "flex flex-col gap-1 flex-1 min-w-0",
                        label { class: LABEL, {url_label} }
                        input {
                            class: INPUT,
                            r#type: "url",
                            required: true,
                            placeholder: "https://",
                            value: url(),
                            oninput: move |evt| url.set(evt.value()),
                        }
                    }
                    button { class: BUTTON_PRIMARY, r#type: "submit", {import_label.clone()} }
                }
                if let Some((added, existing)) = import_done() {
                    p { class: SUCCESS_TEXT, {t!("holidays-import-done", added: added, existing: existing)} }
                }
                if let Some(message) = import_failure() {
                    p { class: ERROR_TEXT, "{import_failed}: {message}" }
                }
            }

            section { class: format!("{CARD} flex flex-col gap-3"),
                h2 { class: "font-bold", {paste_title} }
                p { class: "text-sm text-neutral-500 dark:text-neutral-400", {paste_hint} }
                form { class: "flex flex-col gap-3", onsubmit: paste_import,
                    textarea {
                        class: format!("{INPUT} min-h-40 font-mono text-xs"),
                        required: true,
                        placeholder: "BEGIN:VCALENDAR",
                        value: pasted(),
                        oninput: move |evt| pasted.set(evt.value()),
                    }
                    button { class: format!("{BUTTON_PRIMARY} self-start"), r#type: "submit", {paste_label.clone()} }
                }
            }

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

            if let Some((kind, message)) = failure() {
                p { class: ERROR_TEXT,
                    {match kind {
                        Failed::Save => save_failed.clone(),
                        Failed::Delete => delete_failed.clone(),
                    }}
                    ": {message}"
                }
            }

            form { class: format!("{CARD} flex flex-col gap-4"), onsubmit: save,
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, {date_label} }
                    input {
                        class: INPUT,
                        r#type: "date",
                        required: true,
                        value: date_text(),
                        oninput: move |evt| date_text.set(evt.value()),
                    }
                }
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, {name_label} }
                    input {
                        class: INPUT,
                        r#type: "text",
                        required: true,
                        value: name(),
                        oninput: move |evt| name.set(evt.value()),
                    }
                }
                div { class: "flex flex-wrap gap-2",
                    button { class: BUTTON_PRIMARY, r#type: "submit",
                        if editing().is_some() {
                            {save_label.clone()}
                        } else {
                            {add_label.clone()}
                        }
                    }
                    if editing().is_some() {
                        button { class: BUTTON, r#type: "button", onclick: cancel, {cancel_label.clone()} }
                    }
                }
            }

            {list_view}
        }
    }
}
