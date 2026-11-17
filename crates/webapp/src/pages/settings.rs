//! Settings page: theme, language and expected working hours, stored on the server.

use dioxus::prelude::*;
use dioxus_i18n::{prelude::*, t};
use dto::{format_signed_hours, parse_signed_hours, Settings};

use crate::api;
use crate::common::prelude::{get_preferred_language, Language};
use crate::components::ui::{
    BUTTON_PRIMARY, CARD, ERROR_TEXT, INPUT, LABEL, PAGE_TITLE, SUCCESS_TEXT,
};
use crate::STATE;

/// Maps the stored language to the app's language type. `System` uses the browser's languages.
pub fn to_app_language(language: dto::Language) -> Language {
    match language {
        dto::Language::System => get_preferred_language(),
        dto::Language::En => Language::English,
        dto::Language::De => Language::German,
        dto::Language::Ru => Language::Russian,
        dto::Language::Fr => Language::French,
        dto::Language::Es => Language::Spanish,
        dto::Language::It => Language::Italian,
        dto::Language::Pt => Language::Portuguese,
        dto::Language::Nl => Language::Dutch,
        dto::Language::Pl => Language::Polish,
        dto::Language::Zh => Language::Chinese,
        dto::Language::Ja => Language::Japanese,
    }
}

/// Shows working time as hours, e.g. 450 minutes -> "7.5".
pub fn format_hours(minutes: u32) -> String {
    (f64::from(minutes) / 60.0).to_string()
}

#[derive(Debug, Clone, PartialEq)]
/// What the settings page shows after a save: success, or the reason it failed.
enum Notice {
    Saved,
    InvalidHours,
    InvalidCarryOver,
    Failed(String),
}

#[component]
/// The settings page. See the module description.
pub fn SettingsPage() -> Element {
    let saved = use_resource(|| async { api::get_settings().await });
    let mut theme = use_signal(|| dto::theme::SYSTEM.to_string());
    let mut language = use_signal(|| dto::Language::System);
    let mut hours = use_signal(|| "8".to_string());
    let mut show_email = use_signal(|| false);
    let mut break_minutes = use_signal(|| 45u32);
    let mut carry_over_text = use_signal(String::new);
    let mut loaded = use_signal(|| false);
    let mut notice = use_signal(|| None::<Notice>);
    let mut stored = use_signal(|| None::<Settings>);
    let mut i18n = i18n();

    let title = t!("settings-title");
    let theme_label = t!("theme-label");
    let theme_system = t!("theme-system");
    let language_system = t!("language-system");
    let language_label = t!("language-label");
    let hours_label = t!("daily-hours-label");
    let save_label = t!("save");
    let saved_text = t!("settings-saved");
    let invalid_text = t!("invalid-hours");
    let save_failed = t!("save-failed");
    let show_email_label = t!("settings-show-email");
    let carry_over_label = t!("settings-carry-over-label");
    let carry_over_invalid = t!("invalid-carry-over");
    let break_label = t!("settings-break-label");

    // Fill the form once, when the saved settings arrive.
    use_effect(move || {
        if loaded() {
            return;
        }
        if let Some(Ok(current)) = &*saved.read() {
            theme.set(current.theme.clone());
            language.set(current.language);
            hours.set(format_hours(current.daily_minutes));
            show_email.set(current.show_email);
            break_minutes.set(current.break_minutes);
            carry_over_text.set(format_signed_hours(current.carry_over_minutes));
            loaded.set(true);
        }
    });

    // After a successful save, apply the theme and language immediately.
    use_effect(move || {
        if let Some(current) = stored() {
            STATE.write().theme = current.theme.clone();
            STATE.write().language = to_app_language(current.language);
            STATE.write().show_email = current.show_email;
            i18n.set_language(to_app_language(current.language).into());
        }
    });

    let save = move |evt: FormEvent| {
        evt.prevent_default();
        let minutes = match hours().trim().replace(',', ".").parse::<f64>() {
            Ok(value) if (0.0..=24.0).contains(&value) => (value * 60.0).round() as u32,
            _ => {
                notice.set(Some(Notice::InvalidHours));
                return;
            }
        };
        let Some(carry_over_minutes) = parse_signed_hours(&carry_over_text()) else {
            notice.set(Some(Notice::InvalidCarryOver));
            return;
        };
        let settings = Settings {
            theme: theme(),
            language: language(),
            daily_minutes: minutes,
            show_email: show_email(),
            break_minutes: break_minutes(),
            carry_over_minutes,
        };
        spawn(async move {
            match api::save_settings(&settings).await {
                Ok(result) => {
                    stored.set(Some(result));
                    notice.set(Some(Notice::Saved));
                }
                Err(err) => notice.set(Some(Notice::Failed(err.to_string()))),
            }
        });
    };

    let notice_view = match notice() {
        None => rsx! {},
        Some(Notice::Saved) => rsx! {
            p { class: SUCCESS_TEXT, {saved_text.clone()} }
        },
        Some(Notice::InvalidHours) => rsx! {
            p { class: ERROR_TEXT, {invalid_text.clone()} }
        },
        Some(Notice::InvalidCarryOver) => rsx! {
            p { class: ERROR_TEXT, {carry_over_invalid.clone()} }
        },
        Some(Notice::Failed(message)) => rsx! {
            p { class: ERROR_TEXT, "{save_failed}: {message}" }
        },
    };

    rsx! {
        div { class: "flex flex-col gap-4",
            h1 { class: PAGE_TITLE, {title} }
            form { class: format!("{CARD} flex flex-col gap-4"), onsubmit: save,
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, {theme_label} }
                    select {
                        class: INPUT,
                        onchange: move |evt| theme.set(evt.value()),
                        option { value: dto::theme::SYSTEM, selected: theme() == dto::theme::SYSTEM, {theme_system} }
                        {dto::builtin_themes().into_iter().map(|option_theme| rsx! {
                            option { value: "{option_theme.id}", selected: theme() == option_theme.id, "{option_theme.name}" }
                        })}
                    }
                }
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, {language_label} }
                    select {
                        class: INPUT,
                        onchange: move |evt| {
                            language.set(dto::Language::parse(&evt.value()).unwrap_or(dto::Language::System));
                        },
                        option { value: "system", selected: "{language() == dto::Language::System}", {language_system} }
                        option { value: "en", selected: "{language() == dto::Language::En}", "English" }
                        option { value: "de", selected: "{language() == dto::Language::De}", "Deutsch" }
                        option { value: "ru", selected: "{language() == dto::Language::Ru}", "Русский" }
                        option { value: "fr", selected: "{language() == dto::Language::Fr}", "Français" }
                        option { value: "es", selected: "{language() == dto::Language::Es}", "Español" }
                        option { value: "it", selected: "{language() == dto::Language::It}", "Italiano" }
                        option { value: "pt", selected: "{language() == dto::Language::Pt}", "Português" }
                        option { value: "nl", selected: "{language() == dto::Language::Nl}", "Nederlands" }
                        option { value: "pl", selected: "{language() == dto::Language::Pl}", "Polski" }
                        option { value: "zh", selected: "{language() == dto::Language::Zh}", "中文" }
                        option { value: "ja", selected: "{language() == dto::Language::Ja}", "日本語" }
                    }
                }
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, {hours_label} }
                    input {
                        class: INPUT,
                        r#type: "number",
                        min: "0",
                        max: "24",
                        step: "0.25",
                        value: hours(),
                        oninput: move |evt| hours.set(evt.value()),
                    }
                }
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, {break_label} }
                    input {
                        class: INPUT,
                        r#type: "number",
                        min: "0",
                        max: "1440",
                        step: "1",
                        value: "{break_minutes()}",
                        oninput: move |evt| {
                            if let Ok(value) = evt.value().trim().parse::<u32>() {
                                break_minutes.set(value);
                            }
                        },
                    }
                }
                div { class: "flex flex-col gap-1",
                    label { class: LABEL, {carry_over_label} }
                    input {
                        class: INPUT,
                        r#type: "text",
                        inputmode: "decimal",
                        placeholder: "+0:00",
                        value: "{carry_over_text}",
                        oninput: move |evt| carry_over_text.set(evt.value()),
                    }
                }
                div { class: "flex items-center gap-2",
                    input {
                        class: "h-5 w-5 cursor-pointer",
                        r#type: "checkbox",
                        id: "show-email",
                        checked: show_email(),
                        onchange: move |evt| show_email.set(evt.checked()),
                    }
                    label { class: LABEL, r#for: "show-email", {show_email_label} }
                }
                div { class: "flex flex-wrap gap-2",
                    button { class: BUTTON_PRIMARY, r#type: "submit", {save_label} }
                }
            }
            {notice_view}
        }
    }
}
