use dioxus::logger::tracing::Level;
use dioxus::prelude::*;
use dioxus_i18n::prelude::*;
use dioxus_sdk_storage::{use_storage, LocalStorage};
use unic_langid::langid;

use crate::common::prelude::*;
use crate::components::prelude::*;
use crate::routes::Route;

mod api;
mod common;
mod components;
mod pages;
mod routes;

const FAVICON: Asset = asset!("/assets/images/favicon.ico");
const APP_CSS: Asset = asset!("/assets/css/app.css");
const STORAGE_KEY: &str = "t-rex.state";
pub static STATE: GlobalSignal<AppState> = Signal::global(AppState::default);

fn main() {
    dioxus::logger::init(Level::INFO).expect("unable to init logger");
    dioxus::launch(App);
}

fn i18n_config() -> I18nConfig {
    I18nConfig::new(langid!("en"))
        .with_locale((langid!("en"), include_str!("../i18n/en.ftl")))
        .with_locale((langid!("de"), include_str!("../i18n/de.ftl")))
        .with_locale((langid!("ru"), include_str!("../i18n/ru.ftl")))
        .with_locale((langid!("fr"), include_str!("../i18n/fr.ftl")))
        .with_locale((langid!("es"), include_str!("../i18n/es.ftl")))
        .with_locale((langid!("it"), include_str!("../i18n/it.ftl")))
        .with_locale((langid!("pt"), include_str!("../i18n/pt.ftl")))
        .with_locale((langid!("nl"), include_str!("../i18n/nl.ftl")))
        .with_locale((langid!("pl"), include_str!("../i18n/pl.ftl")))
        .with_locale((langid!("zh"), include_str!("../i18n/zh.ftl")))
        .with_locale((langid!("ja"), include_str!("../i18n/ja.ftl")))
}

#[component]
fn App() -> Element {
    use_init_i18n(i18n_config);

    let mut storage_state =
        use_storage::<LocalStorage, AppState>(STORAGE_KEY.to_string(), AppState::default);
    let mut initialized_from_storage = use_signal(|| false);

    if !*initialized_from_storage.read() {
        *STATE.write() = storage_state.read().clone();
        *initialized_from_storage.write() = true;
        // The language switcher is gone, so the initial language comes from the state here.
        i18n().set_language(STATE.read().language.into());
    }

    use_effect(move || {
        let current_state = STATE.read().clone();
        if storage_state.read().clone() != current_state {
            *storage_state.write() = current_state;
        }
    });

    set_theme(&STATE.read().theme);

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: APP_CSS }
        document::Title { "t-rex" }
        Router::<Route> {}
    }
}
