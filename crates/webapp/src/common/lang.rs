use serde::{Deserialize, Serialize};
use unic_langid::{langid, LanguageIdentifier};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum Language {
    English,
    German,
    Russian,
    French,
    Spanish,
    Italian,
    Portuguese,
    Dutch,
    Polish,
    Chinese,
    Japanese,
}

impl From<&str> for Language {
    fn from(short: &str) -> Self {
        match short.to_lowercase().as_str() {
            "de" => Language::German,
            "ru" => Language::Russian,
            "fr" => Language::French,
            "es" => Language::Spanish,
            "it" => Language::Italian,
            "pt" => Language::Portuguese,
            "nl" => Language::Dutch,
            "pl" => Language::Polish,
            "zh" => Language::Chinese,
            "ja" => Language::Japanese,
            _ => Language::English,
        }
    }
}

impl From<Language> for LanguageIdentifier {
    fn from(lang: Language) -> Self {
        match lang {
            Language::English => langid!("en"),
            Language::German => langid!("de"),
            Language::Russian => langid!("ru"),
            Language::French => langid!("fr"),
            Language::Spanish => langid!("es"),
            Language::Italian => langid!("it"),
            Language::Portuguese => langid!("pt"),
            Language::Dutch => langid!("nl"),
            Language::Polish => langid!("pl"),
            Language::Chinese => langid!("zh"),
            Language::Japanese => langid!("ja"),
        }
    }
}
