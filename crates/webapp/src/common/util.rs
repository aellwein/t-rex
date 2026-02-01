use crate::common::prelude::*;

pub fn window() -> web_sys::Window {
    web_sys::window().expect("unable to get window element")
}

pub fn document() -> web_sys::Document {
    window().document().expect("unable to get document element")
}

/// Whether the browser asks for a dark colour scheme.
pub fn prefers_dark() -> bool {
    matches!(window().match_media("(prefers-color-scheme: dark)"), Ok(Some(query)) if query.matches())
}

pub fn set_classes(class: &str) {
    document()
        .document_element()
        .expect("unable to get body element")
        .class_list()
        .add_1(class)
        .expect("unable to add class");
}

pub fn remove_classes(class: &str) {
    document()
        .document_element()
        .expect("unable to get body element")
        .class_list()
        .remove_1(class)
        .expect("unable to remove class");
}

/// The first language of the browser's list that the app supports, English otherwise.
pub fn get_preferred_language() -> Language {
    window()
        .navigator()
        .languages()
        .iter()
        .filter_map(|tag| tag.as_string())
        .find_map(|tag| language_from_tag(&tag))
        .unwrap_or(Language::English)
}

/// Maps a BCP 47 tag such as `de-AT` or `ru` to a supported language.
pub fn language_from_tag(tag: &str) -> Option<Language> {
    match tag.split(['-', '_']).next()?.to_lowercase().as_str() {
        "en" => Some(Language::English),
        "de" => Some(Language::German),
        "ru" => Some(Language::Russian),
        "fr" => Some(Language::French),
        "es" => Some(Language::Spanish),
        "it" => Some(Language::Italian),
        "pt" => Some(Language::Portuguese),
        "nl" => Some(Language::Dutch),
        "pl" => Some(Language::Polish),
        // Simplified Chinese is the only Chinese translation; all `zh` tags use it.
        "zh" => Some(Language::Chinese),
        "ja" => Some(Language::Japanese),
        _ => None,
    }
}
